use crate::config::ApiKeyPermissions;
use crate::error::{McpError, Result};
use crate::mcp_manager::{McpServerInfo, McpServerManager};
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    Router,
};
use rmcp::{
    model::*,
    service::{RequestContext, RoleClient, RoleServer, ServiceExt},
    transport::{
        sse_client::SseClientTransport,
        streamable_http_client::StreamableHttpClientTransport,
        streamable_http_server::{
            session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        },
        TokioChildProcess,
    },
    ErrorData as RmcpError, ServerHandler,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

// Permission wrapper for HTTP request extensions
#[derive(Clone)]
struct RequestPermissions {
    permissions: ApiKeyPermissions,
    session_id: String,
}

// Session data with timestamp for cleanup management
#[derive(Clone)]
struct SessionData {
    permissions: ApiKeyPermissions,
    created_at: std::time::Instant,
    last_accessed: std::time::Instant,
}

impl SessionData {
    fn new(permissions: ApiKeyPermissions) -> Self {
        let now = std::time::Instant::now();
        Self {
            permissions,
            created_at: now,
            last_accessed: now,
        }
    }

    fn touch(&mut self) {
        self.last_accessed = std::time::Instant::now();
    }
}

// Global session permissions storage: session_id -> SessionData
// This allows the middleware and handlers to share permission data across async boundaries
static SESSION_PERMISSIONS: std::sync::LazyLock<Arc<RwLock<HashMap<String, SessionData>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

// Thread-local storage for current session ID (this works because we set it in middleware
// and read it in handler within the same async task chain)
thread_local! {
    static CURRENT_SESSION_ID: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

#[derive(Clone)]
pub struct McpAggregator {
    mcp_server_manager: Arc<McpServerManager>,
    config: crate::config::ServerConfig,
    // Connection pool for MCP clients
    connection_pool: Arc<RwLock<HashMap<String, ManagedConnection>>>,
    // Shutdown signal
    shutdown_tx: Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

/// Managed connection with automatic cleanup
struct ManagedConnection {
    service: Arc<dyn std::any::Any + Send + Sync>,
    last_used: std::time::Instant,
}

// ConnectionType enum removed as it was unused

impl McpAggregator {
    pub fn new(
        mcp_server_manager: Arc<McpServerManager>,
        config: crate::config::ServerConfig,
    ) -> Self {
        Self {
            mcp_server_manager,
            config,
            connection_pool: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Store the sender for later use
        {
            let mut tx_guard = self.shutdown_tx.lock().await;
            *tx_guard = Some(shutdown_tx);
        }

        // Create aggregator handler
        let handler_factory = {
            let aggregator = self.clone();
            move || Ok(AggregatorHandler::new(aggregator.clone()))
        };

        // Create StreamableHttp service
        let streamable_http_service = StreamableHttpService::new(
            handler_factory,
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default(),
        );

        // Start connection cleanup task
        self.start_cleanup_task();

        // Build Axum router with API Key middleware
        let app = Router::new()
            .route_service("/mcp", streamable_http_service)
            .layer(middleware::from_fn(api_key_auth_middleware));

        tracing::info!("MCP Aggregator server listening on {}", addr);
        tracing::info!("  - Streamable HTTP endpoint: http://{}/mcp", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| McpError::NetworkError(format!("Failed to bind to {}: {}", addr, e)))?;

        // Serve with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
                tracing::info!("Received shutdown signal, stopping aggregator server");
            })
            .await
            .map_err(|e| McpError::NetworkError(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Trigger graceful shutdown of the aggregator server
    pub async fn trigger_shutdown(&self) {
        tracing::info!("Triggering aggregator shutdown...");
        let mut tx_guard = self.shutdown_tx.lock().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(());
        }
    }

    /// Start background task to clean up idle connections and expired sessions
    fn start_cleanup_task(&self) {
        let pool = self.connection_pool.clone();
        let sessions = SESSION_PERMISSIONS.clone();

        // Connection cleanup task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let mut pool = pool.write().await;
                let now = std::time::Instant::now();
                pool.retain(|name, conn| {
                    let idle_time = now.duration_since(conn.last_used);
                    if idle_time > std::time::Duration::from_secs(300) {
                        tracing::debug!("Cleaning up idle connection to {}", name);
                        false
                    } else {
                        true
                    }
                });
            }
        });

        // Session cleanup task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(120)); // Check every 2 minutes
            loop {
                interval.tick().await;
                let mut sessions = sessions.write().await;
                let now = std::time::Instant::now();
                let before_count = sessions.len();

                // Remove sessions older than 10 minutes or idle for 5 minutes
                sessions.retain(|session_id, session_data| {
                    let age = now.duration_since(session_data.created_at);
                    let idle_time = now.duration_since(session_data.last_accessed);

                    let should_retain = age < std::time::Duration::from_secs(600)
                        && idle_time < std::time::Duration::from_secs(300);

                    if !should_retain {
                        tracing::debug!(
                            "Cleaning up expired session: {} (age: {:?}, idle: {:?})",
                            session_id,
                            age,
                            idle_time
                        );
                    }

                    should_retain
                });

                let after_count = sessions.len();
                if before_count != after_count {
                    tracing::info!(
                        "Session cleanup: removed {} sessions, {} active",
                        before_count - after_count,
                        after_count
                    );
                }
            }
        });
    }

    /// Get or create a connection to an MCP server
    async fn get_or_create_connection(
        &self,
        service_name: &str,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>> {
        // Check if we have a cached connection
        {
            let mut pool = self.connection_pool.write().await;
            if let Some(conn) = pool.get_mut(service_name) {
                conn.last_used = std::time::Instant::now();
                return Ok(conn.service.clone());
            }
        }

        // Get service configuration
        let services_arc = self.mcp_server_manager.get_mcp_servers().await;
        let services = services_arc.read().await;
        let service_config = services
            .get(service_name)
            .ok_or_else(|| McpError::ServiceNotFound(service_name.to_string()))?
            .clone();
        drop(services);

        // Create new connection based on transport type
        let service =
            match service_config.transport {
                crate::config::ServiceTransport::StreamableHttp => {
                    let url = service_config.url.as_ref().ok_or_else(|| {
                        McpError::InvalidConfiguration("URL required".to_string())
                    })?;

                    tracing::info!(
                        "Creating StreamableHttp connection to {} at {}",
                        service_name,
                        url
                    );

                    let transport = StreamableHttpClientTransport::from_uri(url.as_str());
                    let client_info = ClientInfo::default();
                    let service = client_info
                        .serve(transport)
                        .await
                        .map_err(|e| McpError::ConnectionError(e.to_string()))?;

                    Arc::new(service) as Arc<dyn std::any::Any + Send + Sync>
                }
                crate::config::ServiceTransport::Sse => {
                    let url = service_config.url.as_ref().ok_or_else(|| {
                        McpError::InvalidConfiguration("URL required".to_string())
                    })?;

                    tracing::info!("Creating SSE connection to {} at {}", service_name, url);

                    let transport = SseClientTransport::start(url.clone())
                        .await
                        .map_err(|e| McpError::ConnectionError(e.to_string()))?;
                    let client_info = ClientInfo::default();
                    let service = client_info
                        .serve(transport)
                        .await
                        .map_err(|e| McpError::ConnectionError(e.to_string()))?;

                    Arc::new(service) as Arc<dyn std::any::Any + Send + Sync>
                }
                crate::config::ServiceTransport::Stdio => {
                    let command_str = service_config.command.as_ref().ok_or_else(|| {
                        McpError::InvalidConfiguration("STDIO service requires command".to_string())
                    })?;

                    tracing::info!(
                        "Creating STDIO connection to {} with command: {}",
                        service_name,
                        command_str
                    );

                    let mut command = Command::new(command_str);
                    // If running via npx, append --registry from global settings unless already provided
                    if command_str == "npx" {
                        let global_config_for_registry = self.mcp_server_manager.get_config().await;
                        if let Some(app_settings) = global_config_for_registry.settings {
                            if let Some(npm_reg) = app_settings.npm_registry {
                                let has_registry_flag = service_config
                                    .args
                                    .as_ref()
                                    .map(|args| args.iter().any(|a| a.starts_with("--registry")))
                                    .unwrap_or(false);
                                if !has_registry_flag {
                                    command.arg("--registry").arg(npm_reg);
                                }
                            }
                        }
                    }

                    if let Some(ref args) = service_config.args {
                        command.args(args);
                    }
                    command.stdout(Stdio::piped());
                    command.stdin(Stdio::piped());
                    command.stderr(Stdio::piped());

                    // Set environment variables
                    if let Some(env_vars) = &service_config.env_vars {
                        for (key, value) in env_vars {
                            command.env(key, value);
                        }
                    }

                    // Inject global mirror settings from AppConfig.settings
                    let global_config = self.mcp_server_manager.get_config().await;
                    if let Some(app_settings) = global_config.settings {
                        if let Some(uv_url) = app_settings.uv_index_url {
                            command.env("UV_INDEX_URL", uv_url.clone());
                            // Also set for uvx runner
                            command.env("UVX_INDEX_URL", uv_url);
                        }
                        if let Some(npm_reg) = app_settings.npm_registry {
                            command.env("NPM_CONFIG_REGISTRY", npm_reg);
                        }
                    }

                    let transport = TokioChildProcess::new(command).map_err(|e| {
                        McpError::ProcessError(format!("Failed to create transport: {}", e))
                    })?;

                    let client_info = ClientInfo::default();
                    let service = client_info
                        .serve(transport)
                        .await
                        .map_err(|e| McpError::ConnectionError(e.to_string()))?;

                    Arc::new(service) as Arc<dyn std::any::Any + Send + Sync>
                }
            };

        // Cache the connection
        let mut pool = self.connection_pool.write().await;
        pool.insert(
            service_name.to_string(),
            ManagedConnection {
                service: service.clone(),
                last_used: std::time::Instant::now(),
            },
        );

        Ok(service)
    }

    // shutdown method removed as it was unused

    pub async fn get_statistics(&self) -> Value {
        let services = self.mcp_server_manager.list_mcp_servers().await.ok();
        let connected_services = services
            .as_ref()
            .map(|s| s.iter().filter(|srv| srv.is_active).count())
            .unwrap_or(0);

        let pool = self.connection_pool.read().await;
        let active_connections = pool.len();

        json!({
            "connected_services": connected_services,
            "active_connections": active_connections,
            "config": {
                "port": self.config.port,
                "max_connections": self.config.max_connections,
                "timeout_seconds": self.config.timeout_seconds
            }
        })
    }
}

/// Aggregator MCP server handler
#[derive(Clone)]
struct AggregatorHandler {
    aggregator: McpAggregator,
}

impl AggregatorHandler {
    fn new(aggregator: McpAggregator) -> Self {
        Self { aggregator }
    }

    // Helper method to get permissions from current session
    async fn get_permissions(&self) -> Option<ApiKeyPermissions> {
        tracing::info!("Attempting to get permissions from session storage");

        // Try to get session_id from thread-local storage first
        let session_id = CURRENT_SESSION_ID.with(|id| id.borrow().clone());

        if let Some(session_id) = session_id {
            tracing::info!("Found session_id in thread-local storage: {}", session_id);
            let mut permissions_store = SESSION_PERMISSIONS.write().await;
            if let Some(session_data) = permissions_store.get_mut(&session_id) {
                // Update last accessed time
                session_data.touch();
                tracing::info!(
                    "SUCCESS: Found session data for session_id: {} - allowed_servers: {:?}",
                    session_id,
                    session_data.permissions.allowed_servers
                );
                return Some(session_data.permissions.clone());
            } else {
                tracing::warn!(
                    "Session_id found in thread-local but no data in storage: {}",
                    session_id
                );
            }
        } else {
            tracing::warn!("No session_id found in thread-local storage");
        }

        // If no session in thread-local storage, try to find the most recent session
        // This is a fallback for cases where thread-local storage is not available
        // This is a fallback for cases where thread-local storage is not available
        let mut permissions_store = SESSION_PERMISSIONS.write().await;
        let session_count = permissions_store.len();
        tracing::info!("Found {} sessions in global storage", session_count);

        if !permissions_store.is_empty() {
            // Find the most recently accessed session
            let mut most_recent_session_id: Option<String> = None;
            let mut most_recent_time = std::time::Instant::now();

            for (session_id, session_data) in permissions_store.iter() {
                tracing::debug!(
                    "Session: {} (age: {:?}, idle: {:?})",
                    session_id,
                    std::time::Instant::now().duration_since(session_data.created_at),
                    std::time::Instant::now().duration_since(session_data.last_accessed)
                );

                if session_data.last_accessed > most_recent_time {
                    most_recent_time = session_data.last_accessed;
                    most_recent_session_id = Some(session_id.clone());
                }
            }

            if let Some(session_id) = most_recent_session_id {
                // Update last accessed time for the fallback session
                if let Some(session_data_mut) = permissions_store.get_mut(&session_id) {
                    session_data_mut.touch();
                    tracing::info!("FALLBACK SUCCESS: Using most recent session_id: {} - allowed_servers: {:?}",
                        session_id, session_data_mut.permissions.allowed_servers);
                    return Some(session_data_mut.permissions.clone());
                }
            }
        }

        tracing::error!("PERMISSION FAILURE: No valid sessions found - will use open access mode");
        None // No session, no permissions (open access)
    }
}

impl ServerHandler for AggregatorHandler {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(Default::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "mcprouter-aggregator".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, RmcpError> {
        tracing::info!("Handling list_tools request from aggregator");

        // Try to extract permissions from RequestContext extensions first
        let permissions = if let Some(req_perms) = context.extensions.get::<RequestPermissions>() {
            tracing::info!(
                "Found permissions in RequestContext extensions for session_id: {}",
                req_perms.session_id
            );
            Some(req_perms.permissions.clone())
        } else {
            tracing::debug!("No permissions in RequestContext extensions, trying session storage");
            // Fallback to session-based permission retrieval
            self.get_permissions().await
        };

        if let Some(ref perms) = permissions {
            tracing::info!(
                "list_tools called with API key permissions: allowed_servers={:?}",
                perms.allowed_servers
            );
        } else {
            tracing::warn!("list_tools called without API key (falling back to open access mode)");
        }

        match self.collect_all_tools().await {
            Ok(tools) => {
                tracing::info!("Successfully collected {} tools", tools.len());
                if permissions.is_some() {
                    tracing::info!(
                        "API key authorization: returned {} tools filtered by permissions",
                        tools.len()
                    );
                } else {
                    tracing::info!(
                        "Open access: returned {} tools (no filtering applied)",
                        tools.len()
                    );
                }
                Ok(ListToolsResult {
                    tools,
                    next_cursor: None,
                })
            }
            Err(e) => {
                tracing::error!("Failed to collect tools: {}", e);
                Err(RmcpError {
                    code: ErrorCode(-32603),
                    message: format!("Failed to collect tools: {}", e).into(),
                    data: None,
                })
            }
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResult, RmcpError> {
        tracing::info!("Handling call_tool request: {}", request.name);

        // Parse the tool name format: serverName/toolName
        let parts: Vec<&str> = request.name.split('/').collect();
        if parts.len() != 2 {
            return Err(RmcpError {
                code: ErrorCode(-32602),
                message: format!(
                    "Invalid tool name format. Expected 'serverName/toolName', got '{}'",
                    request.name
                )
                .into(),
                data: None,
            });
        }

        let server_name = parts[0];
        let tool_name = parts[1];

        // Try to extract permissions from RequestContext extensions first
        let permissions = if let Some(req_perms) = context.extensions.get::<RequestPermissions>() {
            tracing::info!(
                "Found permissions in RequestContext extensions for session_id: {} (tool call)",
                req_perms.session_id
            );
            Some(req_perms.permissions.clone())
        } else {
            tracing::debug!(
                "No permissions in RequestContext extensions for tool call, trying session storage"
            );
            // Fallback to session-based permission retrieval
            self.get_permissions().await
        };
        if let Some(perms) = permissions {
            tracing::info!(
                "Checking permissions for tool: {}/{}",
                server_name,
                tool_name
            );

            // Check server permission
            if !Self::check_server_permission(&perms, server_name) {
                tracing::warn!("Permission denied for server: {}", server_name);
                return Err(RmcpError {
                    code: ErrorCode(-32603),
                    message: format!("Permission denied for server: {}", server_name).into(),
                    data: None,
                });
            }

            // Check tool permission
            if !Self::check_tool_permission(&perms, server_name, tool_name) {
                tracing::warn!("Permission denied for tool: {}/{}", server_name, tool_name);
                return Err(RmcpError {
                    code: ErrorCode(-32603),
                    message: format!("Permission denied for tool: {}", request.name).into(),
                    data: None,
                });
            }

            tracing::info!("Permission granted for tool: {}/{}", server_name, tool_name);
        }

        // Forward the call to the appropriate MCP server
        match self
            .forward_tool_call(server_name, tool_name, request.arguments)
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::error!("Failed to call tool {}: {}", request.name, e);
                Err(RmcpError {
                    code: ErrorCode(-32603),
                    message: format!("Failed to call tool: {}", e).into(),
                    data: None,
                })
            }
        }
    }
}

impl AggregatorHandler {
    /// Validate if an API key has permission to access a specific server
    ///
    /// # Permission Logic
    /// - If `allowed_servers` is empty, access is granted to all servers
    /// - Otherwise, the server name must be in the `allowed_servers` list
    ///
    /// # Arguments
    /// * `permissions` - The API key permissions to check
    /// * `server_name` - The name of the MCP server to access
    ///
    /// # Returns
    /// `true` if access is allowed, `false` otherwise
    fn check_server_permission(permissions: &ApiKeyPermissions, server_name: &str) -> bool {
        tracing::trace!(
            "Checking server permission for '{}' with allowed_servers: {:?}",
            server_name,
            permissions.allowed_servers
        );

        // If allowed_servers is empty, grant access to all servers
        if permissions.allowed_servers.is_empty() {
            tracing::trace!(
                "Server '{}' allowed: empty allowed_servers list (access to all servers)",
                server_name
            );
            return true;
        }

        let allowed = permissions
            .allowed_servers
            .contains(&server_name.to_string());
        tracing::trace!(
            "Server '{}' access: {} (explicit allowlist check)",
            server_name,
            allowed
        );
        allowed
    }

    /// Validate if an API key has permission to call a specific tool on a server
    ///
    /// # Permission Logic
    /// - Checks only if the server itself is accessible
    /// - When server is allowed, all tools on that server are allowed
    fn check_tool_permission(
        permissions: &ApiKeyPermissions,
        server_name: &str,
        _tool_name: &str,
    ) -> bool {
        // First check if the server is allowed
        if !Self::check_server_permission(permissions, server_name) {
            tracing::trace!("Tool access denied: server '{}' not allowed", server_name);
            return false;
        }
        // Tool-level restrictions removed: allow all tools on allowed servers
        true
    }

    /// Collect all tools from managed MCP servers
    async fn collect_all_tools(&self) -> Result<Vec<Tool>> {
        // Get permissions from session storage
        let permissions = self.get_permissions().await;

        // Get all managed MCP servers
        let mut services = self
            .aggregator
            .mcp_server_manager
            .list_mcp_servers()
            .await?;

        // If services are empty, proactively attempt a reload from DB to avoid startup race
        if services.is_empty() {
            tracing::warn!(
                "Starting tool collection with 0 services; attempting to reload from database"
            );
            if let Err(e) = self.aggregator.mcp_server_manager.load_mcp_servers().await {
                tracing::error!("Failed to reload MCP services: {}", e);
            } else {
                services = self
                    .aggregator
                    .mcp_server_manager
                    .list_mcp_servers()
                    .await?;
                tracing::info!(
                    "Service reload complete; {} services now available",
                    services.len()
                );
            }
        }

        tracing::info!("Starting tool collection from {} services", services.len());

        // Pre-filter authorized services if API key is present
        let authorized_services: Vec<&McpServerInfo> = if let Some(ref perms) = permissions {
            tracing::info!(
                "Applying permission filters: allowed_servers={:?}",
                perms.allowed_servers,
            );

            // Get current session ID for logging
            let session_id = CURRENT_SESSION_ID.with(|id| id.borrow().clone());
            if let Some(ref sid) = session_id {
                tracing::info!("Using session_id: {}", sid);
            }

            // Filter authorized services upfront
            let authorized: Vec<&McpServerInfo> = services
                .iter()
                .filter(|service| {
                    if !service.enabled {
                        return false;
                    }
                    Self::check_server_permission(perms, &service.name)
                })
                .collect();

            tracing::info!(
                "Authorization mode: API key restricted access - {} of {} services authorized",
                authorized.len(),
                services.iter().filter(|s| s.enabled).count()
            );

            // Log authorized services for clarity
            if !authorized.is_empty() {
                let service_names: Vec<&String> = authorized.iter().map(|s| &s.name).collect();
                tracing::info!("Authorized services: {:?}", service_names);
            }

            authorized
        } else {
            tracing::info!("No permission filters (open access mode)");
            services.iter().filter(|s| s.enabled).collect()
        };

        let mut all_tools = Vec::new();
        let mut processed_count = 0;
        let mut error_count = 0;
        let total_enabled_services = services.iter().filter(|s| s.enabled).count();

        // Only process pre-authorized services
        for service in authorized_services.iter() {
            tracing::info!(
                "Accessing authorized service: {} (transport: {:?})",
                service.name,
                service.transport
            );

            let service_name = &service.name;
            tracing::debug!("Fetching tools from authorized service: {}", service_name);

            // Try to connect to the service and get its tools
            match self.get_service_tools(service_name).await {
                Ok(tools) => {
                    tracing::info!(
                        "Got {} tools from authorized service {}",
                        tools.len(),
                        service_name
                    );
                    // Prefix and permission-check remain unchanged
                    for t in tools.into_iter() {
                        if let Some(ref perms) = permissions {
                            if !Self::check_tool_permission(perms, service_name, &t.name) {
                                continue;
                            }
                        }
                        let mut tool = t.clone();
                        tool.name = format!("{}/{}", service_name, t.name).into();
                        all_tools.push(tool);
                    }
                    processed_count += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch tools from service {}: {}", service_name, e);
                    error_count += 1;
                }
            }
        }

        tracing::info!(
            "Tool collection complete: {} tools from {} authorized services (processed: {}, skipped: {}, errors: {})",
            all_tools.len(),
            authorized_services.len(),
            processed_count,
            total_enabled_services.saturating_sub(processed_count),
            error_count
        );

        Ok(all_tools)
    }

    /// Get tools from a specific service using rmcp client
    async fn get_service_tools(&self, service_name: &str) -> Result<Vec<Tool>> {
        let service_any = self
            .aggregator
            .get_or_create_connection(service_name)
            .await?;

        // Try to downcast to StreamableHttp service
        if let Some(service) =
            service_any.downcast_ref::<rmcp::service::RunningService<RoleClient, ClientInfo>>()
        {
            tracing::debug!("Calling list_tools on {} via client", service_name);

            let result = service
                .list_tools(None)
                .await
                .map_err(|e| McpError::ProcessError(format!("list_tools failed: {:?}", e)))?;

            tracing::debug!("Got {} tools from {}", result.tools.len(), service_name);
            Ok(result.tools)
        } else {
            Err(McpError::ProcessError(
                "Failed to downcast service".to_string(),
            ))
        }
    }

    /// Forward a tool call to the appropriate MCP server using rmcp client
    async fn forward_tool_call(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Option<serde_json::Map<String, Value>>,
    ) -> Result<CallToolResult> {
        let service_any = self
            .aggregator
            .get_or_create_connection(server_name)
            .await?;

        // Try to downcast to client service
        if let Some(service) =
            service_any.downcast_ref::<rmcp::service::RunningService<RoleClient, ClientInfo>>()
        {
            tracing::info!("Calling tool {}/{} via client", server_name, tool_name);

            let result = service
                .call_tool(CallToolRequestParam {
                    name: tool_name.to_string().into(),
                    arguments,
                })
                .await
                .map_err(|e| McpError::ProcessError(format!("call_tool failed: {:?}", e)))?;

            Ok(result)
        } else {
            Err(McpError::ProcessError(
                "Failed to downcast service".to_string(),
            ))
        }
    }
}

/// API Key authentication middleware
///
/// This middleware validates API keys and enforces access control for the MCP aggregator.
///
/// # Authentication Flow
/// 1. Extract the API key from the Authorization header (supports both "Bearer sk-..." and "sk-..." formats)
/// 2. Validate the API key against configured keys using constant-time comparison
/// 3. If valid, store the key's permissions in the request extensions for downstream use
/// 4. If invalid or missing, return 401 Unauthorized
///
/// # Permission Model
/// API keys have the following permission structure:
/// - `allowed_servers`: List of MCP server names this key can access. Empty list = access to all servers.
///
/// # Security Notes
/// - Uses constant-time string comparison to prevent timing attacks
/// - When global `security.auth` is false, all requests are allowed (open access mode)
/// - When global `security.auth` is true and no API keys are configured, requests are rejected
/// - Disabled API keys are rejected even if the key value matches
/// - Permissions are stored in request extensions for reliable access during request processing
async fn api_key_auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;
    use crate::db::repositories::api_key_server_repository::ApiKeyServerRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // Read global auth switch from configuration
    let global_auth_required = match crate::config::AppConfig::load() {
        Ok(cfg) => cfg.security.as_ref().map(|s| s.auth).unwrap_or(true),
        Err(e) => {
            tracing::error!("Failed to load config for auth check: {}", e);
            true
        }
    };

    // If global auth is disabled, allow all requests
    if !global_auth_required {
        tracing::debug!("Global auth disabled, allowing request");
        return Ok(next.run(request).await);
    }

    // Count enabled API keys
    let api_key_count = match ApiKeyRepository::count_enabled().await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count API keys: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // If auth is required but no keys are configured, reject
    if api_key_count == 0 {
        tracing::warn!("Auth is enabled but no API keys configured; rejecting request");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Extract Authorization header
    let auth_header = headers.get("authorization").and_then(|v| v.to_str().ok());

    let api_key = match auth_header {
        Some(auth) => {
            // Support both "Bearer sk-..." and "sk-..." formats
            if auth.starts_with("Bearer ") {
                &auth[7..]
            } else {
                auth
            }
        }
        None => {
            tracing::warn!("API key authentication failed: no Authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Validate API key using database
    let verified_key = match ApiKeyRepository::verify_key(api_key).await {
        Ok(Some(key)) => key,
        Ok(None) => {
            tracing::warn!("API key authentication failed: invalid API key");
            return Err(StatusCode::UNAUTHORIZED);
        }
        Err(e) => {
            tracing::error!("API key authentication error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get server permissions
    let allowed_server_ids =
        match ApiKeyServerRepository::get_servers_by_api_key(&verified_key.id).await {
            Ok(servers) => servers,
            Err(e) => {
                tracing::error!("Failed to get server permissions: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

    // Convert server IDs to names
    let mut allowed_server_names = Vec::new();
    for server_id in &allowed_server_ids {
        if let Ok(Some(server)) = McpServerRepository::get_by_id(server_id).await {
            allowed_server_names.push(server.name);
        }
    }

    // Create permissions object for compatibility
    let permissions = crate::config::ApiKeyPermissions {
        allowed_servers: allowed_server_names.clone(),
    };

    tracing::info!(
        "API key authenticated: {} with permissions: allowed_servers={:?}",
        verified_key.name,
        allowed_server_names
    );

    // Generate unique session ID for this request
    use std::sync::atomic::{AtomicU64, Ordering};
    static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);
    let session_id = format!("session_{}", SESSION_COUNTER.fetch_add(1, Ordering::SeqCst));

    // Store permissions in global session storage with timestamp for cleanup management
    {
        let mut permissions_store = SESSION_PERMISSIONS.write().await;
        let session_data = SessionData::new(permissions.clone());
        permissions_store.insert(session_id.clone(), session_data);
    }

    // Set current session ID in thread-local storage for this request
    CURRENT_SESSION_ID.with(|id| {
        *id.borrow_mut() = Some(session_id.clone());
    });

    // Store permissions in Axum request extensions for reliable access during handler execution
    let request_permissions = RequestPermissions {
        permissions: permissions.clone(),
        session_id: session_id.clone(),
    };
    request.extensions_mut().insert(request_permissions);

    // Execute the request
    let response = next.run(request).await;

    Ok(response)
}
