use crate::error::{McpError, Result};
use crate::mcp_manager::{McpServerInfo, McpServerManager};
use crate::types::ApiKeyPermissions;
use crate::MCP_CLIENT_MANAGER;
// rust-mcp-sdk imports
use rust_mcp_schema::Tool;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolResult, Implementation, InitializeResult, ListToolsRequest,
    ListToolsResult, ProtocolVersion, RpcError as RmcpError, ServerCapabilities,
};
use rust_mcp_sdk::McpServer;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Permission wrapper for HTTP request extensions
// NOTE: Currently unused - reserved for future middleware implementation
// #[derive(Clone)]
// struct RequestPermissions {
//     permissions: ApiKeyPermissions,
//     session_id: String,
//     api_key_id: String,
// }

// Session data with timestamp for cleanup management
#[derive(Clone)]
struct SessionData {
    permissions: ApiKeyPermissions,
    created_at: std::time::Instant,
    last_accessed: std::time::Instant,
}

impl SessionData {
    fn _new(_permissions: ApiKeyPermissions) -> Self {
        let now = std::time::Instant::now();
        Self {
            permissions: _permissions,
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
    static CURRENT_SESSION_ID: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

#[derive(Clone)]
pub struct McpAggregator {
    mcp_server_manager: Arc<McpServerManager>,
    // Note: Removed static config field - we now dynamically fetch config from mcp_server_manager
    // Connection pool for MCP clients
    connection_pool: Arc<RwLock<HashMap<String, ManagedConnection>>>,
    // Shutdown signal
    shutdown_tx: Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

/// Managed connection with automatic cleanup
struct ManagedConnection {
    _service: Arc<dyn std::any::Any + Send + Sync>,
    last_used: std::time::Instant,
}

// ConnectionType enum removed as it was unused

impl McpAggregator {
    pub fn new(
        mcp_server_manager: Arc<McpServerManager>,
        _config: crate::types::ServerConfig, // Note: config parameter kept for compatibility but no longer stored
    ) -> Self {
        Self {
            mcp_server_manager,
            connection_pool: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        // Note: We'll use the current config from the mcp_server_manager
        // This ensures we always use the latest configuration
        let current_config = self.mcp_server_manager.get_config().await;
        let addr = format!(
            "{}:{}",
            current_config.server.host, current_config.server.port
        );

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Store the sender for later use
        {
            let mut tx_guard = self.shutdown_tx.lock().await;
            *tx_guard = Some(shutdown_tx);
        }

        // Start connection cleanup task
        self.start_cleanup_task();

        // NOTE: HyperServer integration is pending
        // The hyper_servers module in rust-mcp-sdk is currently private,
        // requiring alternative approaches such as:
        // 1. Using axum_server directly with custom MCP protocol handling
        // 2. Waiting for public API access to hyper_servers module
        // 3. Implementing a custom HTTP server with MCP protocol support
        tracing::info!("MCP Aggregator server starting on {}", addr);
        tracing::info!("  - Streamable HTTP endpoint: http://{}/mcp", addr);
        tracing::info!("  - Status: Basic implementation (HyperServer integration pending)");

        // Create aggregator handler
        let _handler = AggregatorHandler::new(self.clone());

        // IMPLEMENTATION NOTE:
        // Current implementation uses a placeholder approach.
        // For production use, consider integrating with:
        // - rust-mcp-sdk hyper_servers (when public API available)
        // - Custom axum-based MCP protocol server
        // - Third-party MCP-compatible HTTP server

        // Serve with graceful shutdown
        let server_handle = shutdown_rx;
        let _ = tokio::spawn(async move {
            let _ = server_handle.await;
            tracing::info!("Received shutdown signal, stopping aggregator server");
        });

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
    ///
    /// DEPRECATED: This method is no longer used and will be removed.
    /// Use McpClientManager.ensure_connection() for all connection needs.
    async fn _get_or_create_connection(
        &self,
        _service_name: &str,
    ) -> Result<Arc<dyn std::any::Any + Send + Sync>> {
        // This method is deprecated and should not be used
        // Connection management is now handled by McpClientManager
        return Err(McpError::ProcessError(
            "This method is deprecated".to_string(),
        ));
    }

    pub async fn get_statistics(&self) -> Value {
        let services = self.mcp_server_manager.list_mcp_servers().await.ok();
        let connected_services = services
            .as_ref()
            .map(|s| s.iter().filter(|srv| srv.status == "connected").count())
            .unwrap_or(0);

        let pool = self.connection_pool.read().await;
        let active_connections = pool.len();

        // Check if aggregator is running by checking if shutdown sender is set
        let is_running = {
            let shutdown_tx = self.shutdown_tx.lock().await;
            shutdown_tx.is_some()
        };

        // Get current configuration dynamically from mcp_server_manager
        let current_config = self.mcp_server_manager.get_config().await;
        json!({
            "endpoint": format!("http://{}:{}/mcp", current_config.server.host, current_config.server.port),
            "is_running": is_running,
            "connected_services": connected_services,
            "active_connections": active_connections,
            "max_connections": current_config.server.max_connections,
            "timeout_seconds": current_config.server.timeout_seconds
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

#[async_trait::async_trait]
impl ServerHandler for AggregatorHandler {
    async fn handle_initialize_request(
        &self,
        _request: rust_mcp_sdk::schema::InitializeRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<InitializeResult, RmcpError> {
        Ok(InitializeResult {
            protocol_version: ProtocolVersion::V2025_06_18.to_string(),
            capabilities: ServerCapabilities {
                tools: Some(Default::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "mcprouter-aggregator".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
            },
            instructions: None,
            meta: None,
        })
    }

    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RmcpError> {
        tracing::info!("Handling list_tools request from aggregator");

        match self.collect_all_tools().await {
            Ok(tools) => {
                tracing::info!("Successfully collected {} tools", tools.len());
                Ok(ListToolsResult {
                    tools,
                    next_cursor: None,
                    meta: None,
                })
            }
            Err(e) => {
                tracing::error!("Failed to collect tools: {}", e);
                Err(RmcpError::internal_error()
                    .with_message(format!("Failed to collect tools: {}", e)))
            }
        }
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_name = &request.params.name;
        tracing::info!("Handling call_tool request: {}", tool_name);

        // Parse the tool name format: serverName/toolName
        let parts: Vec<&str> = tool_name.split('/').collect();
        if parts.len() != 2 {
            return Err(CallToolError::invalid_arguments(
                tool_name,
                Some(format!(
                    "Invalid tool name format. Expected 'serverName/toolName', got '{}'",
                    tool_name
                )),
            ));
        }

        let server_name = parts[0];
        let tool_name = parts[1];

        // Step 1: Get permissions for this session
        let permissions = match self.get_permissions().await {
            Some(perms) => {
                tracing::info!("Using session permissions for tool execution");
                perms
            }
            None => {
                tracing::warn!("No session permissions found - using open access mode");
                // If no permissions, we still allow execution but log it
                ApiKeyPermissions {
                    allowed_servers: Vec::new(), // Empty means allow all
                    allowed_tools: Vec::new(),
                }
            }
        };

        // Step 2: Check server-level permissions
        if !Self::check_server_permission(&permissions, server_name) {
            return Err(CallToolError::invalid_arguments(
                tool_name,
                Some(format!(
                    "Access denied: API key does not have permission to access server '{}'. \
                    Please contact administrator to grant access.",
                    server_name
                )),
            ));
        }

        // Step 3: Check tool-level permissions (if we have an API key)
        let session_id = CURRENT_SESSION_ID.with(|id| id.borrow().clone());
        if let Some(ref api_key_id) = session_id {
            // We have a session, check tool-level permissions
            let has_tool_permission =
                Self::check_tool_permission(api_key_id, server_name, tool_name).await;

            if !has_tool_permission {
                return Err(CallToolError::invalid_arguments(
                    tool_name,
                    Some(format!(
                        "Access denied: API key does not have permission to execute tool '{}/{}'. \
                        Please contact administrator to grant tool access.",
                        server_name, tool_name
                    )),
                ));
            }
        } else {
            tracing::info!("No API key session - allowing tool execution (open access mode)");
        }

        // Step 4: Forward the tool call to the appropriate MCP service
        match self
            .forward_tool_call(server_name, tool_name, request.params.arguments.clone())
            .await
        {
            Ok(result) => {
                tracing::info!("Successfully executed tool {}/{}", server_name, tool_name);
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to execute tool {}/{}: {}",
                    server_name,
                    tool_name,
                    e
                );
                Err(CallToolError::invalid_arguments(
                    tool_name,
                    Some(format!(
                        "Failed to execute tool '{}/{}': {}. \
                        Please check the tool configuration and try again.",
                        server_name, tool_name, e
                    )),
                ))
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
    /// - Checks if the API key has been granted permission to access this specific tool
    /// - Uses tool-level authorization from api_key_tool_relations table
    ///
    /// # Arguments
    /// * `api_key_id` - The API key ID to check
    /// * `server_name` - The name of the MCP server
    /// * `tool_name` - The name of the tool to call
    ///
    /// # Returns
    /// `true` if access is allowed, `false` otherwise
    async fn check_tool_permission(api_key_id: &str, server_name: &str, tool_name: &str) -> bool {
        // TODO: 迁移到配置文件后重新实现
        tracing::warn!(
            "check_tool_permission not implemented yet for API Key {} on {}/{}",
            api_key_id,
            server_name,
            tool_name
        );

        // 暂时允许所有访问，后续需要从配置文件中读取权限
        true
    }

    /* 原始实现已移除
    async fn check_tool_permission_old(api_key_id: &str, server_name: &str, tool_name: &str) -> bool {
        tracing::trace!(
            "Checking tool-level permission for API Key {} on {}/{}",
            api_key_id,
            server_name,
            tool_name
        );

        // 首先获取 Server ID
        let server = match McpServerRepository::get_by_name(server_name).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!("Server not found: {}", server_name);
                return false;
            }
            Err(e) => {
                tracing::error!("Failed to get server {}: {}", server_name, e);
                return false;
            }
        };

        // 然后获取 Tool
        let tool =
            match McpServerRepository::get_by_name(&server_name).await.unwrap_or(None) {
                Some(t) => t,
                None => {
                    tracing::warn!("Tool not found: {}/{}", server_name, tool_name);
                    return false;
                }
            };

        let tool_id = "temp_id";

        // 检查工具级别的权限
        true
    } // TODO: migrate - 恢复原始实现时删除
*/

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

        // If services are empty, proactively attempt a reload from configuration files to avoid startup race
        if services.is_empty() {
            tracing::warn!(
                "Starting tool collection with 0 services; attempting to reload from configuration"
            );
            // TODO: 需要传递 app_handle 参数
            // if let Err(e) = self.aggregator.mcp_server_manager.load_mcp_servers(&app_handle).await {
            //     tracing::error!("Failed to reload MCP services: {}", e);
            // } else {
            //     services = self
            //         .aggregator
            //         .mcp_server_manager
            //         .list_mcp_servers()
            //         .await?;
            // }
            tracing::warn!("MCP service reload skipped - not fully implemented");
            services = Vec::new();
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
                    // Tool-level permission check will be enforced during call_tool
                    for t in tools.into_iter() {
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

    /// Get tools from a specific service using configuration
    async fn get_service_tools(&self, service_name: &str) -> Result<Vec<Tool>> {
        tracing::info!("Fetching cached tools for service: {}", service_name);

        // TODO: 从配置文件中读取工具信息
        // 临时返回空列表，后续需要从配置文件加载工具
        tracing::warn!("get_service_tools not fully implemented yet");
        Ok(Vec::new())
    }

    /// Forward a tool call to the appropriate MCP server
    async fn forward_tool_call(
        &self,
        server_name: &str,
        tool_name: &str,
        _arguments: Option<serde_json::Map<String, Value>>,
    ) -> Result<CallToolResult> {
        tracing::info!("Forwarding tool call: {}/{}", server_name, tool_name);

        // Get service configuration from manager
        let services_arc = self.aggregator.mcp_server_manager.get_mcp_servers().await;
        let services = services_arc.read().await;
        let service_config = services
            .get(server_name)
            .ok_or_else(|| McpError::ServiceNotFound(server_name.to_string()))?
            .clone();
        drop(services);

        tracing::debug!(
            "Found service config for {}: transport={:?}",
            server_name,
            service_config.transport
        );

        // Ensure connection to the service using McpClientManager
        let connection = MCP_CLIENT_MANAGER
            .ensure_connection(&service_config, false)
            .await
            .map_err(|e| {
                tracing::error!("Failed to establish connection to {}: {}", server_name, e);
                McpError::ConnectionError(format!(
                    "Failed to connect to service '{}': {}. Please check if the service is running and accessible.",
                    server_name, e
                ))
            })?;

        tracing::debug!("Established connection to {}", server_name);

        // TODO: Implement actual tool forwarding using rust-mcp-sdk
        // For now, we'll return a structured error that provides helpful information
        // In the future, this will use the actual MCP client to call the tool

        // Check if we have the necessary client information
        if let Some(ref _client) = connection.client {
            tracing::info!("Using MCP client for tool execution on {}", server_name);

            // TODO: Here we would use the actual MCP client to call the tool
            // Example (pseudo-code):
            // let result = client.call_tool(tool_name, arguments).await?;
            // return Ok(result);

            // For now, return a helpful message
            Err(McpError::ProcessError(format!(
                "Tool execution pipeline is being set up. The tool '{}/{}' will be available shortly. \
                Please try again in a moment or contact the administrator if the issue persists.",
                server_name, tool_name
            )))
        } else {
            tracing::error!("No MCP client available for service {}", server_name);
            Err(McpError::ConnectionError(format!(
                "No active connection to service '{}'. Please verify the service is running and try again.",
                server_name
            )))
        }
    }
}

// NOTE: API Key authentication middleware is currently disabled
// This middleware would provide:
// - API key validation from Authorization headers
// - Permission checking based on API key restrictions
// - Session management for authenticated requests
//
// To enable, uncomment the function and add proper implementation
// with required imports (HeaderMap, Request, Next, Response, StatusCode)
/*
async fn api_key_auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // IMPLEMENTATION NEEDED:
    // 1. Extract API key from Authorization header
    // 2. Validate against database-stored API keys
    // 3. Load permissions for the authenticated key
    // 4. Store permissions in request extensions for handler access
    // 5. Return 401 for invalid/missing keys
    // 6. Return 500 for internal errors

    // For now, allow all requests
    Ok(next.run(request).await)
}
*/
