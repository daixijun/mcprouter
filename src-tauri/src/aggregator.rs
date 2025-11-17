use crate::auth_context::{AuthContext, SessionIdExtension, SessionInfoExtension};
use crate::error::McpError;
use crate::mcp_client::McpClientManager;
use crate::mcp_manager::McpServerManager;
use crate::session_manager::get_session_manager;
use crate::token_manager::TokenManager;
use crate::types::ServerConfig;
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
};
use rmcp::model::{
    CallToolRequestParam, CallToolResult, ErrorCode, GetPromptRequestParam, GetPromptResult,
    InitializeRequestParam, InitializeResult, ListPromptsResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParam, ProtocolVersion, ReadResourceRequestParam,
    ReadResourceResult, Tool as McpTool,
};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig;
use rmcp::transport::streamable_http_server::tower::StreamableHttpService;
use rmcp::ErrorData as RmcpErrorData;
use rmcp::{handler::server::ServerHandler, service::RequestContext, RoleServer};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Permission validation error types
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum PermissionError {
    ToolAccessDenied { tool_name: String },
    ResourceAccessDenied { resource_uri: String },
    PromptAccessDenied { prompt_name: String },
    InvalidToken,
    TokenExpired,
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionError::ToolAccessDenied { tool_name } => {
                write!(f, "Access denied: tool '{}' is not permitted", tool_name)
            }
            PermissionError::ResourceAccessDenied { resource_uri } => {
                write!(
                    f,
                    "Access denied: resource '{}' is not permitted",
                    resource_uri
                )
            }
            PermissionError::PromptAccessDenied { prompt_name } => {
                write!(
                    f,
                    "Access denied: prompt '{}' is not permitted",
                    prompt_name
                )
            }
            PermissionError::InvalidToken => {
                write!(f, "Access denied: invalid or missing authentication token")
            }
            PermissionError::TokenExpired => {
                write!(f, "Access denied: authentication token has expired")
            }
        }
    }
}

impl std::error::Error for PermissionError {}

/// MCP Operation types for permission validation
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum McpOperation {
    ListTools,
    CallTool(String), // tool name
    ListResources,
    ReadResource(String), // resource URI
    ListPrompts,
    GetPrompt(String), // prompt name
}

/// Extract MCP operation from HTTP request path and method
/// This function extracts operation information without consuming the request body
#[allow(dead_code)]
pub fn extract_mcp_operation_from_request(req: &Request<Body>) -> Result<McpOperation, StatusCode> {
    let uri = req.uri().path();
    let method = req.method();

    // For now, we'll skip permission validation in the middleware
    // and let the actual MCP handlers perform validation
    // This avoids consuming the request body in middleware
    match (method.as_str(), uri) {
        ("POST", path) if path.contains("tools") => {
            // We can't determine the specific tool without parsing the body
            // So we'll skip validation here and let the handler do it
            Ok(McpOperation::ListTools)
        }
        ("POST", path) if path.contains("resources") => Ok(McpOperation::ListResources),
        ("POST", path) if path.contains("prompts") => Ok(McpOperation::ListPrompts),
        _ => {
            // Default case - allow through
            Ok(McpOperation::ListTools)
        }
    }
}

/// Extract MCP operation from JSON request body
/// This function is used by MCP handlers to perform detailed permission validation
#[allow(dead_code)]
pub async fn extract_mcp_operation_from_body(bytes: &[u8]) -> Result<McpOperation, StatusCode> {
    // Try to parse as JSON to determine operation type
    if bytes.is_empty() {
        return Ok(McpOperation::ListTools);
    }

    // Parse JSON body to extract operation
    let json_str = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let json_value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    // Determine operation type from JSON structure
    if let Some(method) = json_value.get("method").and_then(|v| v.as_str()) {
        match method {
            "tools/list" => Ok(McpOperation::ListTools),
            "tools/call" => {
                if let Some(params) = json_value.get("params") {
                    if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                        Ok(McpOperation::CallTool(name.to_string()))
                    } else {
                        Err(StatusCode::BAD_REQUEST)
                    }
                } else {
                    Err(StatusCode::BAD_REQUEST)
                }
            }
            "resources/list" => Ok(McpOperation::ListResources),
            "resources/read" => {
                if let Some(params) = json_value.get("params") {
                    if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                        Ok(McpOperation::ReadResource(uri.to_string()))
                    } else {
                        Err(StatusCode::BAD_REQUEST)
                    }
                } else {
                    Err(StatusCode::BAD_REQUEST)
                }
            }
            "prompts/list" => Ok(McpOperation::ListPrompts),
            "prompts/get" => {
                if let Some(params) = json_value.get("params") {
                    if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                        Ok(McpOperation::GetPrompt(name.to_string()))
                    } else {
                        Err(StatusCode::BAD_REQUEST)
                    }
                } else {
                    Err(StatusCode::BAD_REQUEST)
                }
            }
            _ => Ok(McpOperation::ListTools), // Default case
        }
    } else {
        Ok(McpOperation::ListTools) // Default if no method field
    }
}

/// Dynamic Bearer token authentication middleware using TokenManager
/// Performs basic authentication and logs the token for auditing
/// Stores token information in request extensions for later use in permission filtering
async fn dynamic_bearer_auth_middleware(
    mut req: Request,
    next: Next,
    token_manager: Arc<TokenManager>,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    // Validate Bearer token format and value
    let token_value = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            Some(&header[7..]) // Skip "Bearer "
        }
        Some(_) => {
            tracing::warn!("Authentication failed: invalid Authorization header format");
            return Err(StatusCode::UNAUTHORIZED);
        }
        None => {
            tracing::warn!("Authentication failed: missing Authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Validate token using TokenManager
    if let Some(token_value) = token_value {
        if let Some(token_id) = token_manager.validate_token(token_value).await {
            // Record usage statistics asynchronously
            let manager_clone = token_manager.clone();
            let token_id_clone = token_id.clone();
            tokio::spawn(async move {
                if let Err(e) = manager_clone.record_usage(&token_id_clone).await {
                    tracing::error!("Failed to record token usage: {}", e);
                }
            });

            // Try to get full token information and create session
            if let Ok(token) = token_manager.get_token_by_id(&token_id).await {
                // Create session for this authenticated request
                let session_id = get_session_manager().create_session(token.clone());

                // Get the complete session info
                if let Some(session_info) = get_session_manager().get_session(&session_id) {
                    // Store session info in request extensions for MCP layer to access
                    req.extensions_mut()
                        .insert(SessionInfoExtension(Arc::new(session_info)));
                    req.extensions_mut()
                        .insert(SessionIdExtension(session_id.clone()));

                    tracing::debug!(
                        "Authentication successful for token: {}, session: {}",
                        token_id,
                        session_id
                    );
                } else {
                    tracing::debug!(
                        "Authentication successful for token: {} (session info retrieval failed)",
                        token_id
                    );
                }
            } else {
                tracing::debug!(
                    "Authentication successful for token: {} (session creation skipped)",
                    token_id
                );
            }

            Ok(next.run(req).await)
        } else {
            tracing::warn!("Authentication failed: invalid token");
            Err(StatusCode::UNAUTHORIZED)
        }
    } else {
        tracing::warn!("Authentication failed: invalid Authorization header format");
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// MCP Aggregator Server - implements MCP protocol
#[derive(Clone)]
pub struct McpAggregator {
    mcp_server_manager: Arc<McpServerManager>,
    mcp_client_manager: Arc<McpClientManager>,
    config: Arc<ServerConfig>,
    token_manager: Arc<TokenManager>,
    shutdown_signal: Arc<std::sync::Mutex<Option<CancellationToken>>>,
}

impl McpAggregator {
    pub fn new(
        mcp_server_manager: Arc<McpServerManager>,
        mcp_client_manager: Arc<McpClientManager>,
        config: Arc<ServerConfig>,
        token_manager: Arc<TokenManager>,
    ) -> Self {
        Self {
            mcp_server_manager,
            mcp_client_manager,
            config,
            token_manager,
            shutdown_signal: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Check if the given token has permission to access the specified tool
    /// Returns Result with detailed error information for audit logging
    #[allow(dead_code)]
    pub async fn check_tool_permission(
        &self,
        token: &crate::token_manager::Token,
        tool_name: &str,
    ) -> Result<(), PermissionError> {
        if token.has_tool_permission(tool_name) {
            Ok(())
        } else {
            Err(PermissionError::ToolAccessDenied {
                tool_name: tool_name.to_string(),
            })
        }
    }

    /// Check if the given token has permission to access the specified resource
    #[allow(dead_code)]
    pub async fn check_resource_permission(
        &self,
        token: &crate::token_manager::Token,
        resource_uri: &str,
    ) -> Result<(), PermissionError> {
        if token.has_resource_permission(resource_uri) {
            Ok(())
        } else {
            Err(PermissionError::ResourceAccessDenied {
                resource_uri: resource_uri.to_string(),
            })
        }
    }

    /// Check if the given token has permission to access the specified prompt
    #[allow(dead_code)]
    pub async fn check_prompt_permission(
        &self,
        token: &crate::token_manager::Token,
        prompt_name: &str,
    ) -> Result<(), PermissionError> {
        if token.has_prompt_permission(prompt_name) {
            Ok(())
        } else {
            Err(PermissionError::PromptAccessDenied {
                prompt_name: prompt_name.to_string(),
            })
        }
    }

    /// Validate token status and return detailed error if invalid
    #[allow(dead_code)]
    pub async fn validate_token_status(
        &self,
        token: &crate::token_manager::Token,
    ) -> Result<(), PermissionError> {
        if token.is_expired() {
            Err(PermissionError::TokenExpired)
        } else if !token.enabled {
            Err(PermissionError::InvalidToken)
        } else {
            Ok(())
        }
    }

    /// Validate tool access permission with auth header (HTTP level)
    pub async fn validate_tool_access_with_auth(
        &self,
        tool_name: &str,
        auth_header: Option<&str>,
    ) -> Result<(), PermissionError> {
        if !self.config.is_auth_enabled() {
            return Ok(());
        }

        let token = match auth_header {
            Some(header) => self.extract_token_from_auth_header(header).await,
            None => return Err(PermissionError::InvalidToken),
        };

        let token = token.ok_or(PermissionError::InvalidToken)?;

        if self.validate_token_status(&token).await.is_ok()
            && self.check_tool_permission(&token, tool_name).await.is_ok()
        {
            Ok(())
        } else {
            Err(PermissionError::ToolAccessDenied {
                tool_name: tool_name.to_string(),
            })
        }
    }

    /// Validate resource access permission with auth header (HTTP level)
    pub async fn validate_resource_access_with_auth(
        &self,
        resource_uri: &str,
        auth_header: Option<&str>,
    ) -> Result<(), PermissionError> {
        if !self.config.is_auth_enabled() {
            return Ok(());
        }

        let token = match auth_header {
            Some(header) => self.extract_token_from_auth_header(header).await,
            None => return Err(PermissionError::InvalidToken),
        };

        let token = token.ok_or(PermissionError::InvalidToken)?;

        if self.validate_token_status(&token).await.is_ok()
            && self
                .check_resource_permission(&token, resource_uri)
                .await
                .is_ok()
        {
            Ok(())
        } else {
            Err(PermissionError::ResourceAccessDenied {
                resource_uri: resource_uri.to_string(),
            })
        }
    }

    /// Validate prompt access permission with auth header (HTTP level)
    pub async fn validate_prompt_access_with_auth(
        &self,
        prompt_name: &str,
        auth_header: Option<&str>,
    ) -> Result<(), PermissionError> {
        if !self.config.is_auth_enabled() {
            return Ok(());
        }

        let token = match auth_header {
            Some(header) => self.extract_token_from_auth_header(header).await,
            None => return Err(PermissionError::InvalidToken),
        };

        let token = token.ok_or(PermissionError::InvalidToken)?;

        if self.validate_token_status(&token).await.is_ok()
            && self
                .check_prompt_permission(&token, prompt_name)
                .await
                .is_ok()
        {
            Ok(())
        } else {
            Err(PermissionError::PromptAccessDenied {
                prompt_name: prompt_name.to_string(),
            })
        }
    }

    pub async fn start(
        self: &Arc<Self>,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("MCP Aggregator server starting...");

        // Build listen address from config
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        tracing::info!("Starting HTTP server on {}", addr);

        // Clone the Arc to pass to the service factory
        let aggregator_for_service = self.clone();

        // Create session manager
        let session_manager = Arc::new(LocalSessionManager::default());

        // Create service factory that returns aggregator handler directly
        let service_factory = move || Ok(aggregator_for_service.as_ref().clone());

        // Create server config
        let server_config = StreamableHttpServerConfig {
            sse_keep_alive: Some(std::time::Duration::from_secs(self.config.timeout_seconds)),
            stateful_mode: false, // 修改为false，与客户端allow_stateless=true保持一致
        };

        // Create StreamableHttpService
        let service = StreamableHttpService::new(service_factory, session_manager, server_config);

        // Build router with conditional authentication middleware
        let router = if self.config.is_auth_enabled() {
            tracing::info!("Authentication enabled with dynamic token management");
            let token_manager = self.token_manager.clone();
            axum::Router::new()
                .nest_service("/mcp", service)
                .layer(middleware::from_fn(move |req, next| {
                    let token_manager = token_manager.clone();
                    async move { dynamic_bearer_auth_middleware(req, next, token_manager).await }
                }))
        } else {
            tracing::info!("Authentication disabled - running without auth middleware");
            axum::Router::new().nest_service("/mcp", service)
        };

        // Bind TCP listener
        let tcp_listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            e
        })?;

        // Create cancellation token for graceful shutdown
        let ct = CancellationToken::new();

        // Store cancellation token for later use in trigger_shutdown
        {
            let mut shutdown_guard = self.shutdown_signal.lock().unwrap();
            *shutdown_guard = Some(ct.clone());
        }

        // Spawn server task
        let _server_handle = tokio::spawn({
            let ct = ct.clone();
            async move {
                tracing::info!("MCP Aggregator HTTP server running on {}", addr);
                let result = axum::serve(tcp_listener, router)
                    .with_graceful_shutdown(async move {
                        ct.cancelled_owned().await;
                        tracing::info!("MCP Aggregator server shutting down...");
                    })
                    .await;

                if let Err(e) = result {
                    tracing::error!("Server error: {}", e);
                }
            }
        });

        tracing::info!(
            "MCP Aggregator started successfully on {} (auth: {}, timeout: {}s, max_connections: {})",
            addr,
            if self.config.is_auth_enabled() { "enabled with dynamic tokens" } else { "disabled" },
            self.config.timeout_seconds,
            self.config.max_connections
        );

        Ok(())
    }

    pub async fn get_statistics(&self) -> Value {
        let entries = self.mcp_server_manager.tools_cache_entries.read().await;
        let total = entries.len();
        let ttl = self.mcp_server_manager.get_tools_cache_ttl_seconds();
        let updated_count: usize = entries.values().map(|e| e.count).sum();
        let latest = entries.values().map(|e| e.last_updated).max();
        serde_json::json!({
            "status": "running",
            "message": "Aggregator initialized",
            "tool_cache": { "enabled": true, "entries": total, "ttl_seconds": ttl, "tools_total": updated_count, "last_updated": latest.map(|d| d.to_rfc3339()) }
        })
    }

    /// Get tools directly from memory (with optional sync from config file)
    async fn get_tools_from_memory(&self) -> Result<Vec<McpTool>, McpError> {
        let mut aggregated_tools: Vec<McpTool> = Vec::new();
        let servers_lock = self.mcp_server_manager.get_mcp_servers().await;
        let servers = servers_lock.read().await;
        tracing::debug!("Found {} MCP servers in memory", servers.len());
        for (name, config) in servers.iter() {
            tracing::info!(
                "Server '{}' - enabled: {}, transport: {:?}, tools: {}",
                name,
                config.enabled,
                config.transport,
                {
                    let tc = self.mcp_server_manager.tools_cache_entries.read().await;
                    tc.get(name).map(|v| v.count).unwrap_or(0)
                }
            );
        }
        for (server_name, server_config) in servers.iter() {
            if !server_config.enabled {
                continue;
            }
            if let Some(cached) = self
                .mcp_server_manager
                .get_raw_cached_tools(server_name)
                .await
            {
                let mut prefixed = Vec::new();
                for mut tool in cached {
                    let original_name = tool.name.clone();
                    tool.name = format!("{}/{}", server_name, original_name).into();
                    if tool.description.is_none() {
                        tool.description = Some("No description".into());
                    }
                    prefixed.push(tool);
                }
                aggregated_tools.extend(prefixed);
            }
        }
        Ok(aggregated_tools)
    }

    /// Get resources directly from memory (with optional sync from config file)
    async fn get_resources_from_memory(&self) -> Result<Vec<rmcp::model::Resource>, McpError> {
        let mut aggregated_resources: Vec<rmcp::model::Resource> = Vec::new();
        let servers_lock = self.mcp_server_manager.get_mcp_servers().await;
        let servers = servers_lock.read().await;
        tracing::info!(
            "Found {} MCP servers in memory for resources",
            servers.len()
        );

        for (server_name, server_config) in servers.iter() {
            if !server_config.enabled {
                continue;
            }

            // Get from cache directly
            if let Some(cached) = self
                .mcp_server_manager
                .get_cached_resources_raw(server_name)
                .await
            {
                let mut prefixed = Vec::new();
                for resource in cached {
                    let original_uri = resource.uri.clone();
                    let prefixed_uri = format!("{}/{}", server_name, original_uri);
                    let mut prefixed_resource = resource.clone();
                    prefixed_resource.uri = prefixed_uri;
                    prefixed.push(prefixed_resource);
                }
                aggregated_resources.extend(prefixed);
            }
        }
        Ok(aggregated_resources)
    }

    /// Get prompts directly from memory (with optional sync from config file)
    async fn get_prompts_from_memory(&self) -> Result<Vec<rmcp::model::Prompt>, McpError> {
        let mut aggregated_prompts: Vec<rmcp::model::Prompt> = Vec::new();
        let servers_lock = self.mcp_server_manager.get_mcp_servers().await;
        let servers = servers_lock.read().await;
        tracing::debug!("Found {} MCP servers in memory for prompts", servers.len());

        for (server_name, server_config) in servers.iter() {
            if !server_config.enabled {
                continue;
            }

            // Get from cache directly
            if let Some(cached) = self
                .mcp_server_manager
                .get_cached_prompts_raw(server_name)
                .await
            {
                let mut prefixed = Vec::new();
                for mut prompt in cached {
                    let original_name = prompt.name.clone();
                    prompt.name = format!("{}/{}", server_name, original_name);
                    prefixed.push(prompt);
                }
                aggregated_prompts.extend(prefixed);
            }
        }
        Ok(aggregated_prompts)
    }

    /// Parse tool name with server prefix
    fn parse_tool_name(&self, tool_name: &str) -> Option<(String, String)> {
        if let Some((server_name, original_name)) = tool_name.split_once('/') {
            Some((server_name.to_string(), original_name.to_string()))
        } else {
            None
        }
    }

    /// Parse resource URI with server prefix
    fn parse_resource_uri(&self, uri: &str) -> Option<(String, String)> {
        if let Some((server_name, original_uri)) = uri.split_once('/') {
            Some((server_name.to_string(), original_uri.to_string()))
        } else {
            None
        }
    }

    /// Parse prompt name with server prefix
    fn parse_prompt_name(&self, prompt_name: &str) -> Option<(String, String)> {
        if let Some((server_name, original_name)) = prompt_name.split_once('/') {
            Some((server_name.to_string(), original_name.to_string()))
        } else {
            None
        }
    }

    /// Extract token from Authorization header (for HTTP-level operations)
    pub async fn extract_token_from_auth_header(
        &self,
        auth_header: &str,
    ) -> Option<crate::token_manager::Token> {
        if auth_header.starts_with("Bearer ") {
            let token_value = &auth_header[7..]; // Skip "Bearer "
            if let Some(token_id) = self.token_manager.validate_token(token_value).await {
                if let Ok(token) = self.token_manager.get_token_by_id(&token_id).await {
                    return Some(token);
                }
            }
        }
        None
    }

    /// Filter tools based on token permissions
    #[allow(dead_code)]
    fn filter_tools_by_permission(
        &self,
        tools: Vec<McpTool>,
        token: &crate::token_manager::Token,
    ) -> Vec<McpTool> {
        tools
            .into_iter()
            .filter(|tool| token.has_tool_permission(&tool.name))
            .collect()
    }

    /// Filter resources based on token permissions
    #[allow(dead_code)]
    fn filter_resources_by_permission(
        &self,
        resources: Vec<rmcp::model::Resource>,
        token: &crate::token_manager::Token,
    ) -> Vec<rmcp::model::Resource> {
        resources
            .into_iter()
            .filter(|resource| token.has_resource_permission(&resource.uri))
            .collect()
    }

    /// Filter prompts based on token permissions
    #[allow(dead_code)]
    fn filter_prompts_by_permission(
        &self,
        prompts: Vec<rmcp::model::Prompt>,
        token: &crate::token_manager::Token,
    ) -> Vec<rmcp::model::Prompt> {
        prompts
            .into_iter()
            .filter(|prompt| token.has_prompt_permission(&prompt.name))
            .collect()
    }

    /// List tools with permission filtering (used by HTTP endpoints)
    pub async fn list_tools_with_permission_filtering(
        &self,
        auth_header: Option<&str>,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, RmcpErrorData> {
        // If authentication is disabled, return all tools
        if !self.config.is_auth_enabled() {
            return self.list_tools_all(_request).await;
        }

        // Extract token from header
        let token = match auth_header {
            Some(header) => self.extract_token_from_auth_header(header).await,
            None => {
                tracing::warn!("Missing authorization header");
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Missing authorization header".to_string(),
                    None,
                ));
            }
        };

        let token = match token {
            Some(token) => token,
            None => {
                tracing::warn!("Invalid or expired token");
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    "Invalid or expired token".to_string(),
                    None,
                ));
            }
        };

        // Get all tools and filter by permissions
        match self.get_tools_from_memory().await {
            Ok(mut tools) => {
                tools = self.filter_tools_by_permission(tools, &token);

                let mut offset = 0usize;
                if let Some(param) = _request {
                    if let Some(cursor) = param.cursor {
                        if let Ok(v) = cursor.parse::<usize>() {
                            offset = v;
                        } else {
                            return Err(RmcpErrorData::new(
                                ErrorCode(400),
                                "Invalid cursor".to_string(),
                                None,
                            ));
                        }
                    }
                }

                let page_size = 100usize;
                let total = tools.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    tools[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };

                tracing::info!(
                    "Successfully listed {} tools for token {} (filtered from total)",
                    slice.len(),
                    token.id
                );
                Ok(ListToolsResult {
                    tools: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to list tools: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list tools: {}", e),
                    None,
                ))
            }
        }
    }

    /// List resources with permission filtering (used by HTTP endpoints)
    pub async fn list_resources_with_permission_filtering(
        &self,
        auth_header: Option<&str>,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, RmcpErrorData> {
        // If authentication is disabled, return all resources
        if !self.config.is_auth_enabled() {
            return self.list_resources_all(_request).await;
        }

        // Extract token from header
        let token = match auth_header {
            Some(header) => self.extract_token_from_auth_header(header).await,
            None => {
                tracing::warn!("Missing authorization header");
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Missing authorization header".to_string(),
                    None,
                ));
            }
        };

        let token = match token {
            Some(token) => token,
            None => {
                tracing::warn!("Invalid or expired token");
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    "Invalid or expired token".to_string(),
                    None,
                ));
            }
        };

        // Get all resources and filter by permissions
        match self.get_resources_from_memory().await {
            Ok(mut resources) => {
                resources = self.filter_resources_by_permission(resources, &token);

                let mut offset = 0usize;
                if let Some(param) = _request {
                    if let Some(cursor) = param.cursor {
                        if let Ok(v) = cursor.parse::<usize>() {
                            offset = v;
                        } else {
                            return Err(RmcpErrorData::new(
                                ErrorCode(400),
                                "Invalid cursor".to_string(),
                                None,
                            ));
                        }
                    }
                }

                let page_size = 100usize;
                let total = resources.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    resources[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };

                tracing::info!(
                    "Successfully listed {} resources for token {} (filtered from total)",
                    slice.len(),
                    token.id
                );
                Ok(ListResourcesResult {
                    resources: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to list resources: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list resources: {}", e),
                    None,
                ))
            }
        }
    }

    /// List prompts with permission filtering (used by HTTP endpoints)
    pub async fn list_prompts_with_permission_filtering(
        &self,
        auth_header: Option<&str>,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, RmcpErrorData> {
        // If authentication is disabled, return all prompts
        if !self.config.is_auth_enabled() {
            return self.list_prompts_all(_request).await;
        }

        // Extract token from header
        let token = match auth_header {
            Some(header) => self.extract_token_from_auth_header(header).await,
            None => {
                tracing::warn!("Missing authorization header");
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Missing authorization header".to_string(),
                    None,
                ));
            }
        };

        let token = match token {
            Some(token) => token,
            None => {
                tracing::warn!("Invalid or expired token");
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    "Invalid or expired token".to_string(),
                    None,
                ));
            }
        };

        // Get all prompts and filter by permissions
        match self.get_prompts_from_memory().await {
            Ok(mut prompts) => {
                prompts = self.filter_prompts_by_permission(prompts, &token);

                let mut offset = 0usize;
                if let Some(param) = _request {
                    if let Some(cursor) = param.cursor {
                        if let Ok(v) = cursor.parse::<usize>() {
                            offset = v;
                        } else {
                            return Err(RmcpErrorData::new(
                                ErrorCode(400),
                                "Invalid cursor".to_string(),
                                None,
                            ));
                        }
                    }
                }

                let page_size = 100usize;
                let total = prompts.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    prompts[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };

                tracing::info!(
                    "Successfully listed {} prompts for token {} (filtered from total)",
                    slice.len(),
                    token.id
                );
                Ok(ListPromptsResult {
                    prompts: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to list prompts: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list prompts: {}", e),
                    None,
                ))
            }
        }
    }

    /// List all tools without permission filtering (helper method)
    async fn list_tools_all(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, RmcpErrorData> {
        tracing::debug!("List tools request received (all tools)");

        let mut offset = 0usize;
        if let Some(param) = _request {
            if let Some(cursor) = param.cursor {
                if let Ok(v) = cursor.parse::<usize>() {
                    offset = v;
                } else {
                    return Err(RmcpErrorData::new(
                        ErrorCode(400),
                        "Invalid cursor".to_string(),
                        None,
                    ));
                }
            }
        }
        let page_size = 100usize;
        match self.get_tools_from_memory().await {
            Ok(tools) => {
                let total = tools.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    tools[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };
                tracing::info!("Successfully listed {} tools", total);
                Ok(ListToolsResult {
                    tools: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to list tools: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list tools: {}", e),
                    None,
                ))
            }
        }
    }

    /// List all resources without permission filtering (helper method)
    async fn list_resources_all(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, RmcpErrorData> {
        tracing::debug!("List resources request received (all resources)");

        let mut offset = 0usize;
        if let Some(param) = _request {
            if let Some(cursor) = param.cursor {
                if let Ok(v) = cursor.parse::<usize>() {
                    offset = v;
                } else {
                    return Err(RmcpErrorData::new(
                        ErrorCode(400),
                        "Invalid cursor".to_string(),
                        None,
                    ));
                }
            }
        }
        let page_size = 100usize;
        match self.get_resources_from_memory().await {
            Ok(resources) => {
                let total = resources.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    resources[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };
                tracing::info!("Successfully listed {} resources", total);
                Ok(ListResourcesResult {
                    resources: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to list resources: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list resources: {}", e),
                    None,
                ))
            }
        }
    }

    /// List all prompts without permission filtering (helper method)
    async fn list_prompts_all(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, RmcpErrorData> {
        tracing::debug!("List prompts request received (all prompts)");

        let mut offset = 0usize;
        if let Some(param) = _request {
            if let Some(cursor) = param.cursor {
                if let Ok(v) = cursor.parse::<usize>() {
                    offset = v;
                } else {
                    return Err(RmcpErrorData::new(
                        ErrorCode(400),
                        "Invalid cursor".to_string(),
                        None,
                    ));
                }
            }
        }
        let page_size = 100usize;
        match self.get_prompts_from_memory().await {
            Ok(prompts) => {
                let total = prompts.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    prompts[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };
                tracing::info!("Successfully listed {} prompts", total);
                Ok(ListPromptsResult {
                    prompts: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to list prompts: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list prompts: {}", e),
                    None,
                ))
            }
        }
    }

    pub async fn trigger_shutdown(&self) {
        tracing::info!("Triggering aggregator shutdown...");

        // Get the cancellation token and trigger shutdown
        let shutdown_guard = self.shutdown_signal.lock().unwrap();
        if let Some(ct) = shutdown_guard.as_ref() {
            ct.cancel();
            tracing::info!("Shutdown signal sent to MCP Aggregator server");
        } else {
            tracing::warn!("No active shutdown signal found for MCP Aggregator");
        }
    }
}

// Minimal ServerHandler implementation to allow compilation
// This is a placeholder that will be enhanced later
impl ServerHandler for McpAggregator {
    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, RmcpErrorData> {
        tracing::debug!("Initialize request received");

        Ok(InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: rmcp::model::ServerCapabilities {
                experimental: None,
                logging: None,
                completions: None,
                prompts: Some(rmcp::model::PromptsCapability { list_changed: None }),
                resources: Some(rmcp::model::ResourcesCapability {
                    subscribe: None,
                    list_changed: None,
                }),
                tools: Some(rmcp::model::ToolsCapability { list_changed: None }),
            },
            server_info: rmcp::model::Implementation {
                name: "MCP Router Aggregator".to_string(),
                version: "1.0.0".to_string(),
                icons: None,
                title: None,
                website_url: None,
            },
            instructions: None,
        })
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, RmcpErrorData> {
        tracing::debug!("List tools request received");

        // For now, if authentication is disabled, return all tools
        if !self.config.is_auth_enabled() {
            return self.list_tools_all(_request).await;
        }

        // Create AuthContext from RequestContext
        let auth_context = AuthContext::from_request_context(_context);

        // Check if we have a valid session with permissions
        if !auth_context.has_valid_session() {
            tracing::warn!("List tools denied: no valid session found");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Authentication required: no valid session".to_string(),
                None,
            ));
        }

        // Check if session has expired
        if auth_context.is_session_expired() {
            tracing::warn!("List tools denied: session has expired");
            return Err(RmcpErrorData::new(
                ErrorCode(403),
                "Authentication failed: session has expired".to_string(),
                None,
            ));
        }

        // Get all tools and filter by session permissions
        match self.get_tools_from_memory().await {
            Ok(mut tools) => {
                // Filter tools based on session permissions
                tools.retain(|tool| {
                    let permission_result =
                        auth_context.check_tool_permission_with_result(&tool.name);
                    match permission_result {
                        crate::auth_context::PermissionResult::Allowed => true,
                        crate::auth_context::PermissionResult::NotAuthenticated => {
                            tracing::warn!("Tool {} access denied: not authenticated", tool.name);
                            false
                        }
                        crate::auth_context::PermissionResult::SessionExpired => {
                            tracing::warn!("Tool {} access denied: session expired", tool.name);
                            false
                        }
                        crate::auth_context::PermissionResult::InsufficientPermissions => {
                            tracing::debug!(
                                "Tool {} access denied: insufficient permissions",
                                tool.name
                            );
                            false
                        }
                    }
                });

                let mut offset = 0usize;
                if let Some(param) = _request {
                    offset = param.cursor.map(|c| c.parse().unwrap_or(0)).unwrap_or(0);
                }

                let slice = if tools.len() > offset {
                    let end = std::cmp::min(offset + 100, tools.len());
                    &tools[offset..end]
                } else {
                    &[]
                };

                let next = if slice.len() == 100 && tools.len() > offset + 100 {
                    Some((offset + 100).to_string())
                } else {
                    None
                };

                tracing::info!(
                    "Successfully listed {} tools for session {} (filtered from total)",
                    slice.len(),
                    auth_context.session_id().unwrap_or("unknown")
                );
                Ok(ListToolsResult {
                    tools: slice.to_vec(),
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to get tools from memory: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Internal server error: {}", e),
                    None,
                ))
            }
        }
    }

    // Enhanced implementations for remaining methods
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, RmcpErrorData> {
        tracing::debug!("Call tool request received for name: {}", request.name);

        // 如果认证未启用，允许所有工具调用
        if !self.config.is_auth_enabled() {
            tracing::debug!("认证未启用，允许工具调用: {}", request.name);
        } else {
            // 创建AuthContext进行权限验证
            let auth_context = AuthContext::from_request_context(_context);

            // 检查是否有有效会话
            if !auth_context.has_valid_session() {
                tracing::warn!("拒绝未认证的call_tool请求: {}", request.name);
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Authentication required for call_tool".to_string(),
                    None,
                ));
            }

            // 检查会话是否过期
            if auth_context.is_session_expired() {
                tracing::warn!("拒绝过期会话的call_tool请求: {}", request.name);
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Session expired for call_tool".to_string(),
                    None,
                ));
            }

            // 检查工具权限
            if !auth_context.has_tool_permission(&request.name) {
                tracing::warn!("拒绝无权限的工具调用: {}", request.name);
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    format!("Access denied: tool '{}' is not permitted", request.name),
                    None,
                ));
            }

            tracing::debug!("工具 {} 权限验证通过", request.name);
        }

        // Parse the tool name to extract server name and original name
        let (server_name, original_name) =
            self.parse_tool_name(&request.name).ok_or_else(|| {
                RmcpErrorData::new(
                    ErrorCode(400),
                    format!("Invalid tool name format: {}", request.name),
                    None,
                )
            })?;

        tracing::info!(
            "Routing tool call to server: {}, original name: {}",
            server_name,
            original_name
        );

        // Use the MCP client manager to call the tool
        let arguments = request.arguments.map(|args| args.into_iter().collect());
        match self
            .mcp_client_manager
            .call_tool(&server_name, &original_name, arguments)
            .await
        {
            Ok(result) => {
                tracing::info!(
                    "Successfully called tool '{}' from server '{}'",
                    original_name,
                    server_name
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to call tool '{}' from server '{}': {}",
                    original_name,
                    server_name,
                    e
                );
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to call tool: {}", e),
                    None,
                ))
            }
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, RmcpErrorData> {
        tracing::debug!("List prompts request received");

        // 如果认证未启用，返回所有提示词
        if !self.config.is_auth_enabled() {
            return self.list_prompts_all(_request).await;
        }

        // 创建AuthContext进行权限验证
        let auth_context = AuthContext::from_request_context(_context);

        // 检查是否有有效会话
        if !auth_context.has_valid_session() {
            tracing::warn!("拒绝未认证的list_prompts请求");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Authentication required for list_prompts".to_string(),
                None,
            ));
        }

        // 检查会话是否过期
        if auth_context.is_session_expired() {
            tracing::warn!("拒绝过期会话的list_prompts请求");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Session expired for list_prompts".to_string(),
                None,
            ));
        }

        let mut offset = 0usize;
        if let Some(param) = _request {
            if let Some(cursor) = param.cursor {
                if let Ok(v) = cursor.parse::<usize>() {
                    offset = v;
                } else {
                    return Err(RmcpErrorData::new(
                        ErrorCode(400),
                        "Invalid cursor".to_string(),
                        None,
                    ));
                }
            }
        }

        let page_size = 100usize;
        match self.get_prompts_from_memory().await {
            Ok(prompts) => {
                // 保存原始数量用于日志记录
                let original_count = prompts.len();

                // 根据权限过滤提示词
                let filtered_prompts: Vec<_> = prompts
                    .into_iter()
                    .filter(|prompt| auth_context.has_prompt_permission(&prompt.name))
                    .collect();

                tracing::info!(
                    "权限过滤后剩余 {} 个提示词（总共 {} 个）",
                    filtered_prompts.len(),
                    original_count
                );

                let total = filtered_prompts.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    filtered_prompts[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };
                tracing::debug!("成功列出 {} 个有权限的提示词", total);
                Ok(ListPromptsResult {
                    prompts: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("获取提示词列表失败: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list prompts: {}", e),
                    None,
                ))
            }
        }
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, RmcpErrorData> {
        tracing::debug!("Get prompt request received for name: {}", request.name);

        // 如果认证未启用，允许所有提示词获取
        if !self.config.is_auth_enabled() {
            tracing::debug!("认证未启用，允许提示词获取: {}", request.name);
        } else {
            // 创建AuthContext进行权限验证
            let auth_context = AuthContext::from_request_context(_context);

            // 检查是否有有效会话
            if !auth_context.has_valid_session() {
                tracing::warn!("拒绝未认证的get_prompt请求: {}", request.name);
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Authentication required for get_prompt".to_string(),
                    None,
                ));
            }

            // 检查会话是否过期
            if auth_context.is_session_expired() {
                tracing::warn!("拒绝过期会话的get_prompt请求: {}", request.name);
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Session expired for get_prompt".to_string(),
                    None,
                ));
            }

            // 检查提示词权限
            if !auth_context.has_prompt_permission(&request.name) {
                tracing::warn!("拒绝无权限的提示词获取: {}", request.name);
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    format!("Access denied: prompt '{}' is not permitted", request.name),
                    None,
                ));
            }

            tracing::debug!("提示词 {} 权限验证通过", request.name);
        }

        // Parse the prompt name to extract server name and original name
        let (server_name, original_name) =
            self.parse_prompt_name(&request.name).ok_or_else(|| {
                RmcpErrorData::new(
                    ErrorCode(400),
                    format!("Invalid prompt name format: {}", request.name),
                    None,
                )
            })?;

        tracing::info!(
            "Routing prompt get to server: {}, original name: {}",
            server_name,
            original_name
        );

        // Use the MCP client manager to get the prompt
        let arguments = request.arguments.map(|args| {
            args.into_iter()
                .map(|(k, v)| {
                    let arg = rmcp::model::PromptArgument {
                        name: k.clone(),
                        title: v
                            .get("title")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string()),
                        description: v
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(|s| s.to_string()),
                        required: Some(
                            v.get("required").and_then(|r| r.as_bool()).unwrap_or(false),
                        ),
                    };
                    (k, arg)
                })
                .collect()
        });
        match self
            .mcp_client_manager
            .get_prompt(&server_name, &original_name, arguments)
            .await
        {
            Ok(result) => {
                tracing::info!(
                    "Successfully got prompt '{}' from server '{}'",
                    original_name,
                    server_name
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get prompt '{}' from server '{}': {}",
                    original_name,
                    server_name,
                    e
                );
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to get prompt: {}", e),
                    None,
                ))
            }
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, RmcpErrorData> {
        tracing::debug!("List resources request received");

        // 如果认证未启用，返回所有资源
        if !self.config.is_auth_enabled() {
            return self.list_resources_all(_request).await;
        }

        // 创建AuthContext进行权限验证
        let auth_context = AuthContext::from_request_context(_context);

        // 检查是否有有效会话
        if !auth_context.has_valid_session() {
            tracing::warn!("拒绝未认证的list_resources请求");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Authentication required for list_resources".to_string(),
                None,
            ));
        }

        // 检查会话是否过期
        if auth_context.is_session_expired() {
            tracing::warn!("拒绝过期会话的list_resources请求");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Session expired for list_resources".to_string(),
                None,
            ));
        }

        let mut offset = 0usize;
        if let Some(param) = _request {
            if let Some(cursor) = param.cursor {
                if let Ok(v) = cursor.parse::<usize>() {
                    offset = v;
                } else {
                    return Err(RmcpErrorData::new(
                        ErrorCode(400),
                        "Invalid cursor".to_string(),
                        None,
                    ));
                }
            }
        }

        let page_size = 100usize;
        match self.get_resources_from_memory().await {
            Ok(resources) => {
                // 保存原始数量用于日志记录
                let original_count = resources.len();

                // 根据权限过滤资源
                let filtered_resources: Vec<_> = resources
                    .into_iter()
                    .filter(|resource| auth_context.has_resource_permission(&resource.uri))
                    .collect();

                tracing::info!(
                    "权限过滤后剩余 {} 个资源（总共 {} 个）",
                    filtered_resources.len(),
                    original_count
                );

                let total = filtered_resources.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    filtered_resources[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };
                tracing::debug!("成功列出 {} 个有权限的资源", total);
                Ok(ListResourcesResult {
                    resources: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("获取资源列表失败: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list resources: {}", e),
                    None,
                ))
            }
        }
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, RmcpErrorData> {
        tracing::debug!("Read resource request received for URI: {}", request.uri);

        // 如果认证未启用，允许所有资源读取
        if !self.config.is_auth_enabled() {
            tracing::debug!("认证未启用，允许资源读取: {}", request.uri);
        } else {
            // 创建AuthContext进行权限验证
            let auth_context = AuthContext::from_request_context(_context);

            // 检查是否有有效会话
            if !auth_context.has_valid_session() {
                tracing::warn!("拒绝未认证的read_resource请求: {}", request.uri);
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Authentication required for read_resource".to_string(),
                    None,
                ));
            }

            // 检查会话是否过期
            if auth_context.is_session_expired() {
                tracing::warn!("拒绝过期会话的read_resource请求: {}", request.uri);
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Session expired for read_resource".to_string(),
                    None,
                ));
            }

            // 检查资源权限
            if !auth_context.has_resource_permission(&request.uri) {
                tracing::warn!("拒绝无权限的资源读取: {}", request.uri);
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    format!("Access denied: resource '{}' is not permitted", request.uri),
                    None,
                ));
            }

            tracing::debug!("资源 {} 权限验证通过", request.uri);
        }

        // Parse the URI to extract server name and original URI
        let (server_name, original_uri) =
            self.parse_resource_uri(&request.uri).ok_or_else(|| {
                RmcpErrorData::new(
                    ErrorCode(400),
                    format!("Invalid resource URI format: {}", request.uri),
                    None,
                )
            })?;

        tracing::info!(
            "Routing resource read to server: {}, original URI: {}",
            server_name,
            original_uri
        );

        // Use the MCP client manager to read the resource
        match self
            .mcp_client_manager
            .read_resource(&server_name, &original_uri)
            .await
        {
            Ok(result) => {
                tracing::info!(
                    "Successfully read resource '{}' from server '{}'",
                    original_uri,
                    server_name
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to read resource '{}' from server '{}': {}",
                    original_uri,
                    server_name,
                    e
                );
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to read resource: {}", e),
                    None,
                ))
            }
        }
    }
}

impl McpAggregator {
    /// Get all available tools for permission selection
    pub async fn get_all_available_tools(
        &self,
    ) -> std::result::Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        // For now, return some common tool patterns
        // TODO: Implement real server discovery
        Ok(vec![
            "filesystem/*".to_string(),
            "database/*".to_string(),
            "codegen/*".to_string(),
            "analysis/*".to_string(),
            "web/*".to_string(),
        ])
    }

    /// Get all available resources for permission selection
    pub async fn get_all_available_resources(
        &self,
    ) -> std::result::Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        // For now, return some common resource patterns
        // TODO: Implement real server discovery
        Ok(vec![
            "filesystem/*".to_string(),
            "database/*".to_string(),
            "config/*".to_string(),
            "logs/*".to_string(),
        ])
    }

    /// Get all available prompts for permission selection
    pub async fn get_all_available_prompts(
        &self,
    ) -> std::result::Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        // For now, return some common prompt patterns
        // TODO: Implement real server discovery
        Ok(vec![
            "codegen/*".to_string(),
            "analysis/*".to_string(),
            "debug/*".to_string(),
            "help/*".to_string(),
        ])
    }
}
