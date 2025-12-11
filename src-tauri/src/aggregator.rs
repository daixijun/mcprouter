use crate::auth_context::{AuthContext, SessionIdExtension, SessionInfoExtension};
use crate::mcp_client::McpClientManager;
use crate::mcp_manager::McpServerManager;
use crate::token_manager::TokenManager;
use crate::types::ServerConfig;
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::{Json, Response},
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
use chrono;

/// Permission validation error types
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
    let uri = req.uri().path();
    let method = req.method();

    tracing::debug!("=== Authentication Debug ===");
    tracing::debug!("Request: {} {}", method, uri);

    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    tracing::debug!("Authorization header present: {}", auth_header.is_some());
    if let Some(header) = auth_header {
        tracing::debug!("Authorization header length: {}", header.len());
        if header.len() > 20 {
            tracing::debug!("Authorization header preview: {}...{}",
                &header[..10], &header[header.len()-10..]);
        } else {
            tracing::debug!("Authorization header: {}", header);
        }
    }

    // Validate Bearer token format and value
    let token_value = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..]; // Skip "Bearer "
            tracing::debug!("Bearer token extracted, length: {}", token.len());
            Some(token)
        }
        Some(header) => {
            tracing::warn!("Authentication failed: invalid Authorization header format. Expected 'Bearer <token>', got: {}",
                if header.len() > 50 { format!("{}...", &header[..50]) } else { header.to_string() });
            return Err(StatusCode::UNAUTHORIZED);
        }
        None => {
            tracing::warn!("Authentication failed: missing Authorization header for {} {}", method, uri);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Validate token using TokenManager
    if let Some(token_value) = token_value {
        tracing::debug!("Validating token with TokenManager...");
        if let Some(token_id) = token_manager.validate_token(token_value).await {
            tracing::info!("Authentication successful for token_id: {}", token_id);

            // Record usage statistics asynchronously
            let manager_clone = token_manager.clone();
            let token_id_clone = token_id.clone();
            tokio::spawn(async move {
                if let Err(e) = manager_clone.record_usage(&token_id_clone).await {
                    tracing::error!("Failed to record token usage: {}", e);
                }
            });

            // Try to get full token information and store it directly
            tracing::debug!("Retrieving full token information for token_id: {}", token_id);
            if let Ok(token) = token_manager.get_token_by_id(&token_id).await {
                tracing::debug!("Token information retrieved, storing in request extensions...");

                // Create a session-like info object directly from the token
                let session_info = crate::session_manager::SessionInfo {
                    id: token_id.clone(), // Use token_id as session_id
                    token: token.clone(),
                    created_at: std::time::Instant::now(),
                    last_accessed: std::time::Instant::now(),
                    expires_at: None, // We can add token expiration logic later if needed
                };

                tracing::debug!("Session info created, storing in request extensions");
                // Store session info directly in request extensions for MCP layer to access
                req.extensions_mut()
                    .insert(SessionInfoExtension(Arc::new(session_info)));
                req.extensions_mut()
                    .insert(SessionIdExtension(token_id.clone()));

                tracing::info!(
                    "Authentication successful for token: {} (stored in request extensions)",
                    token_id
                );
            } else {
                tracing::warn!(
                    "Authentication successful for token: {} (token not found in database)",
                    token_id
                );
            }

            tracing::debug!("Proceeding to MCP handler");
            Ok(next.run(req).await)
        } else {
            tracing::warn!("Authentication failed: invalid token value. Token validation returned None.");
            Err(StatusCode::UNAUTHORIZED)
        }
    } else {
        tracing::error!("Unexpected state: token_value is None after extraction");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Health check endpoint handler
async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "MCP Aggregator"
    }))
}

/// Debug status endpoint handler
async fn debug_status(Json(_params): Json<Value>) -> Result<Json<Value>, StatusCode> {
    // This would require access to the aggregator instance
    // For now, return basic info
    Ok(Json(serde_json::json!({
        "service": "MCP Aggregator",
        "status": "running",
        "message": "Debug endpoint working"
    })))
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

    /// New constructor that accepts any TokenManager implementation
    pub fn new_with_trait(
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
    pub async fn check_tool_permission(
        &self,
        token: &crate::types::Token,
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
    pub async fn check_resource_permission(
        &self,
        token: &crate::types::Token,
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
    pub async fn check_prompt_permission(
        &self,
        token: &crate::types::Token,
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
    pub async fn validate_token_status(
        &self,
        token: &crate::types::Token,
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
        let server_info = StreamableHttpServerConfig {
            sse_keep_alive: Some(std::time::Duration::from_secs(self.config.timeout_seconds)),
            stateful_mode: false, // Set to false to match client allow_stateless=true
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        };

        // Create StreamableHttpService
        let service = StreamableHttpService::new(service_factory, session_manager, server_info);

        // Build router with conditional authentication middleware
        let router = if self.config.is_auth_enabled() {
            tracing::info!("Authentication enabled with dynamic token management");
            let token_manager = self.token_manager.clone();
            axum::Router::new()
                .nest_service("/mcp", service)
                .route("/health", axum::routing::get(health_check))
                .route("/debug/status", axum::routing::post(debug_status))
                .layer(middleware::from_fn(move |req, next| {
                    let token_manager = token_manager.clone();
                    async move { dynamic_bearer_auth_middleware(req, next, token_manager).await }
                }))
        } else {
            tracing::info!("Authentication disabled - running without auth middleware");
            axum::Router::new()
                .nest_service("/mcp", service)
                .route("/health", axum::routing::get(health_check))
                .route("/debug/status", axum::routing::post(debug_status))
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
        let entries = self.mcp_server_manager.get_tools_cache_entries();
        let total = entries.len();
        let ttl = self.mcp_server_manager.get_tools_cache_ttl_seconds();
        let updated_count: usize = entries.iter().map(|e| e.len()).sum();
        let latest = std::time::SystemTime::now();
        serde_json::json!({
            "status": "running",
            "message": "Aggregator initialized",
            "tool_cache": { "enabled": true, "entries": total, "ttl_seconds": ttl, "tools_total": updated_count, "last_updated": latest.duration_since(std::time::UNIX_EPOCH).ok().map(|d| format!("{}", d.as_secs())) }
        })
    }

    /// 获取工具 - 从数据库查询并生成 resource_path
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, RmcpErrorData> {
        tracing::info!("🔍 Starting list_tools - loading tools from database");

        // 通过 McpServerManager 的公共方法获取完整的工具信息，包含 input_schema
        let tools_data = self.mcp_server_manager.get_all_tools_for_aggregation().await
            .map_err(|e| {
                tracing::error!("❌ Failed to fetch tools from manager: {}", e);
                RmcpErrorData::internal_error(format!("Failed to fetch tools: {}", e), None)
            })?;

        tracing::info!("📊 Retrieved {} tools from database", tools_data.len());

        let mut mcp_tools = Vec::new();

        for (_tool_id, tool_name, description, input_schema_json, server_name) in tools_data {
            // 记录原始数据
            tracing::debug!("🔧 Processing tool: {} from server: {}", tool_name, server_name);
            tracing::debug!("📝 Raw input_schema from DB: {}",
                input_schema_json.as_ref().map_or("NULL".to_string(), |s| s.clone()));

            let server_name_str = server_name.clone(); // server_name 已经是 String 类型

            // 生成 resource_path
            let resource_path = format!("{}__{}", server_name_str, tool_name);

            // 处理 input_schema，使用数据库中存储的真实数据或创建默认的空 schema
            let input_schema: std::sync::Arc<serde_json::Map<String, serde_json::Value>> = if let Some(ref schema_str) = input_schema_json {
                // 尝试解析 JSON Schema
                match serde_json::from_str::<serde_json::Value>(schema_str) {
                    Ok(schema) => {
                        tracing::debug!("✅ Successfully parsed JSON Schema for tool: {}", tool_name);
                        tracing::debug!("📋 Schema content: {}", schema);

                        if let serde_json::Value::Object(mut map) = schema {
                            // 确保至少有 type 字段
                            if !map.contains_key("type") {
                                map.insert("type".to_string(), serde_json::Value::String("object".to_string()));
                                tracing::debug!("➕ Added default 'type: object' field to schema");
                            }
                            std::sync::Arc::new(map)
                        } else {
                            tracing::warn!("⚠️ Schema for tool {} is not an object, using default", tool_name);
                            Self::create_default_schema()
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to parse JSON Schema for tool {}: {}", tool_name, e);
                        tracing::error!("🔍 Original schema string: {}", schema_str);
                        Self::create_default_schema()
                    }
                }
            } else {
                tracing::debug!("⚠️ Tool {} has NULL input_schema, using default", tool_name);
                Self::create_default_schema()
            };

            mcp_tools.push(McpTool {
                name: resource_path.clone().into(), // 克隆 resource_path 并转换为 Cow
                description: Some(description.unwrap_or_else(|| "Tool from server".to_string()).into()),
                input_schema,
                // Default values for other fields
                title: None,
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            });

            tracing::debug!("✅ Processed tool: {} -> {}", tool_name, resource_path);
        }

        tracing::info!("🎉 Successfully processed {} McpTool objects", mcp_tools.len());
        Ok(mcp_tools)
    }

    // 辅助函数：创建默认 schema
    fn create_default_schema() -> std::sync::Arc<serde_json::Map<String, serde_json::Value>> {
        let mut default_schema = serde_json::Map::new();
        default_schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));
        default_schema.insert("properties".to_string(), serde_json::Value::Object(serde_json::Map::new()));
        std::sync::Arc::new(default_schema)
    }

    /// 获取资源 - 从数据库查询并生成 resource_path
    pub async fn list_resources(&self) -> Result<Vec<rmcp::model::Resource>, RmcpErrorData> {
        // 使用 McpServerManager 的新方法获取权限项
        let permissions = self.mcp_server_manager.get_available_permissions_by_type("resource").await
            .map_err(|e| RmcpErrorData::internal_error(format!("Failed to fetch resources: {}", e), None))?;

        let mut mcp_resources = Vec::new();

        for permission in permissions {
            // 从 resource_path 解析服务器名和资源名
            let parts: Vec<&str> = permission.resource_path.split("__").collect();
            if parts.len() >= 2 {
                let _server_name = parts[0];
                let resource_uri = parts[1..].join("__");

                mcp_resources.push(rmcp::model::Resource {
                    raw: rmcp::model::RawResource {
                        uri: permission.resource_path.clone(), // 使用 resource_path 作为 uri
                        name: resource_uri.clone(),
                        description: Some("Resource from server".to_string()),
                        mime_type: Some("application/octet-stream".to_string()),
                        icons: None,
                        meta: None,
                        size: None,
                        title: None,
                    },
                    annotations: None,
                });
            }
        }

        Ok(mcp_resources)
    }

    /// 获取提示词 - 从数据库查询并生成 resource_path
    pub async fn list_prompts(&self) -> Result<Vec<rmcp::model::Prompt>, RmcpErrorData> {
        // 使用 McpServerManager 的新方法获取权限项
        let permissions = self.mcp_server_manager.get_available_permissions_by_type("prompt").await
            .map_err(|e| RmcpErrorData::internal_error(format!("Failed to fetch prompts: {}", e), None))?;

        let mut mcp_prompts = Vec::new();

        for permission in permissions {
            // 从 resource_path 解析服务器名和提示词名
            let parts: Vec<&str> = permission.resource_path.split("__").collect();
            if parts.len() >= 2 {
                let _server_name = parts[0];
                let _prompt_name = parts[1..].join("__");

                mcp_prompts.push(rmcp::model::Prompt {
                    name: permission.resource_path.clone(), // 使用 resource_path 作为 name
                    description: Some("Prompt from server".to_string()),
                    arguments: Some(vec![]), // 空的参数列表
                    title: None,
                    icons: None,
                    meta: None,
                });
            }
        }

        Ok(mcp_prompts)
    }

    /// list_tools 方法现在直接使用权限验证
    pub async fn list_tools_with_auth(
        &self,
        auth_context: &AuthContext,
    ) -> Result<ListToolsResult, RmcpErrorData> {
        tracing::info!("🔐 list_tools_with_auth called with auth_context");

        let tools = self.list_tools().await?;
        tracing::info!("📋 Retrieved {} tools from database", tools.len());

        // 记录权限检查前的工具列表
        for tool in &tools {
            tracing::debug!("🔍 Tool before permission filter: {} (schema size: {})",
                tool.name,
                tool.input_schema.len());
        }

        // 记录工具数量用于日志
        let original_count = tools.len();

        // 直接使用权限过滤（精确匹配 resource_path）
        let filtered_tools: Vec<McpTool> = tools
            .into_iter()
            .filter(|tool| {
                let has_permission = auth_context.has_tool_permission(&tool.name);
                if !has_permission {
                    tracing::debug!("🚫 Tool {} filtered out due to permissions", tool.name);
                }
                has_permission
            })
            .collect();

        tracing::info!("✅ Permission filtering: {} -> {} tools", original_count, filtered_tools.len());

        // 记录最终返回的工具列表
        for tool in &filtered_tools {
            tracing::debug!("🎯 Tool after permission filter: {} (schema: {})",
                tool.name,
                serde_json::to_string(&*tool.input_schema).unwrap_or_else(|_| "INVALID".to_string()));
        }

        Ok(ListToolsResult {
            meta: None,
            tools: filtered_tools,
            next_cursor: None,
        })
    }

    /// list_resources 方法
    pub async fn list_resources_with_auth(
        &self,
        auth_context: &AuthContext,
    ) -> Result<ListResourcesResult, RmcpErrorData> {
        let resources = self.list_resources().await?;

        let filtered_resources: Vec<rmcp::model::Resource> = resources
            .iter()
            .filter(|resource| {
                auth_context.has_resource_permission(&resource.uri)
            })
            .cloned()
            .collect();

        tracing::info!("Permission filtering: {} -> {} resources", resources.len(), filtered_resources.len());

        Ok(ListResourcesResult {
            meta: None,
            resources: filtered_resources,
            next_cursor: None,
        })
    }

    /// list_prompts 方法
    pub async fn list_prompts_with_auth(
        &self,
        auth_context: &AuthContext,
    ) -> Result<ListPromptsResult, RmcpErrorData> {
        let prompts = self.list_prompts().await?;

        let filtered_prompts: Vec<rmcp::model::Prompt> = prompts
            .iter()
            .filter(|prompt| {
                auth_context.has_prompt_permission(&prompt.name)
            })
            .cloned()
            .collect();

        tracing::info!("Permission filtering: {} -> {} prompts", prompts.len(), filtered_prompts.len());

        Ok(ListPromptsResult {
            meta: None,
            prompts: filtered_prompts,
            next_cursor: None,
        })
    }

  
    /// Parse tool name with server prefix
    fn parse_tool_name(&self, tool_name: &str) -> Option<(String, String)> {
        if let Some((server_name, original_name)) = tool_name.split_once("__") {
            Some((server_name.to_string(), original_name.to_string()))
        } else {
            None
        }
    }

    /// Parse resource URI with server prefix
    fn parse_resource_uri(&self, uri: &str) -> Option<(String, String)> {
        if let Some((server_name, original_uri)) = uri.split_once("__") {
            Some((server_name.to_string(), original_uri.to_string()))
        } else {
            None
        }
    }

    /// Parse prompt name with server prefix
    fn parse_prompt_name(&self, prompt_name: &str) -> Option<(String, String)> {
        if let Some((server_name, original_name)) = prompt_name.split_once("__") {
            Some((server_name.to_string(), original_name.to_string()))
        } else {
            None
        }
    }

    /// Build a secondary permission key using the resource's display name
    fn build_resource_name_alias(&self, resource: &rmcp::model::Resource) -> Option<String> {
        if resource.name.is_empty() {
            return None;
        }
        self.parse_resource_uri(&resource.uri)
            .map(|(server_name, _)| format!("{}__{}", server_name, resource.name.clone()))
    }

    /// Check whether a token can access the given resource, supporting legacy name-based IDs
    fn token_can_access_resource(
        &self,
        token: &crate::types::Token,
        resource: &rmcp::model::Resource,
    ) -> bool {
        if token.has_resource_permission(&resource.uri) {
            return true;
        }
        if let Some(alias) = self.build_resource_name_alias(resource) {
            return token.has_resource_permission(&alias);
        }
        false
    }

    /// Evaluate resource permissions for a session with support for alias matching
    fn evaluate_session_resource_permission(
        &self,
        auth_context: &AuthContext,
        resource: &rmcp::model::Resource,
    ) -> crate::auth_context::PermissionResult {
        use crate::auth_context::PermissionResult;

        let primary = auth_context.check_resource_permission_with_result(&resource.uri);
        if matches!(primary, PermissionResult::InsufficientPermissions) {
            if let Some(alias) = self.build_resource_name_alias(resource) {
                if auth_context.has_resource_permission(&alias) {
                    return PermissionResult::Allowed;
                }
            }
        }
        primary
    }

    /// Resolve a legacy alias (server__resourceName) for a prefixed URI via the cache
    async fn resolve_resource_alias_from_uri(&self, prefixed_uri: &str) -> Option<String> {
        if let Some((server_name, original_uri)) = self.parse_resource_uri(prefixed_uri) {
            if let Some(resources) = self
                .mcp_server_manager
                .get_cached_resources_raw(&server_name)
                .await
            {
                for resource in resources {
                    if resource.uri == original_uri && !resource.name.is_empty() {
                        return Some(format!("{}__{}", server_name, resource.name));
                    }
                }
            }
        }
        None
    }

    /// Extract token from Authorization header (for HTTP-level operations)
    pub async fn extract_token_from_auth_header(
        &self,
        auth_header: &str,
    ) -> Option<crate::types::Token> {
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
    fn filter_tools_by_permission(
        &self,
        tools: Vec<McpTool>,
        token: &crate::types::Token,
    ) -> Vec<McpTool> {
        tools
            .into_iter()
            .filter(|tool| token.has_tool_permission(&tool.name))
            .collect()
    }

    /// Filter resources based on token permissions
    fn filter_resources_by_permission(
        &self,
        resources: Vec<rmcp::model::Resource>,
        token: &crate::types::Token,
    ) -> Vec<rmcp::model::Resource> {
        resources
            .into_iter()
            .filter(|resource| self.token_can_access_resource(token, resource))
            .collect()
    }

    /// Filter prompts based on token permissions
    fn filter_prompts_by_permission(
        &self,
        prompts: Vec<rmcp::model::Prompt>,
        token: &crate::types::Token,
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
        match self.list_tools().await {
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
                    meta: None,
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
        match self.list_resources().await {
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
                    meta: None,
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
        match self.list_prompts().await {
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
                    meta: None,
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
        match self.list_tools().await {
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
                    meta: None,
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
        match self.list_resources().await {
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
                    meta: None,
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
        match self.list_prompts().await {
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
                    meta: None,
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
        tracing::info!("=== List Tools Handler ===");
        tracing::info!("Request parameters: {:?}", _request);

        // For now, if authentication is disabled, return all tools
        tracing::info!("Authentication enabled: {}", self.config.is_auth_enabled());
        if !self.config.is_auth_enabled() {
            tracing::info!("Authentication disabled, returning all tools");
            return self.list_tools_all(_request).await;
        }

        // Create AuthContext from RequestContext
        tracing::debug!("Creating AuthContext from RequestContext");
        let auth_context = AuthContext::from_request_context(_context);

        tracing::info!("AuthContext created - has_valid_session: {}, is_session_expired: {}",
            auth_context.has_valid_session(),
            auth_context.is_session_expired());

        // Log session details if available
        if let Some(session_id) = auth_context.session_id() {
            tracing::info!("Session ID: {}", session_id);
        }

        if let Some(token) = auth_context.token() {
            tracing::info!("Token ID: {}", token.id);
        }

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
        tracing::info!("Getting tools from database...");
        match self.list_tools().await {
            Ok(mut tools) => {
                tracing::info!("Successfully retrieved {} tools from database", tools.len());

                // Log first few tool names for debugging
                if tools.len() > 0 {
                    let tool_names: Vec<String> = tools.iter().take(5).map(|t| t.name.to_string()).collect();
                    tracing::info!("First few tools: {:?}", tool_names);
                }

                // Filter tools based on session permissions
                let original_count = tools.len();
                tools.retain(|tool| {
                    let permission_result =
                        auth_context.check_tool_permission_with_result(&tool.name);
                    let allowed = match permission_result {
                        crate::auth_context::PermissionResult::Allowed => {
                            tracing::debug!("Tool {} allowed", tool.name);
                            true
                        }
                        crate::auth_context::PermissionResult::NotAuthenticated => {
                            tracing::warn!("Tool {} access denied: not authenticated", tool.name);
                            false
                        }
                        crate::auth_context::PermissionResult::SessionExpired => {
                            tracing::warn!("Tool {} access denied: session expired", tool.name);
                            false
                        }
                        crate::auth_context::PermissionResult::InsufficientPermissions => {
                            tracing::info!(
                                "Tool {} access denied: insufficient permissions",
                                tool.name
                            );
                            false
                        }
                    };
                    allowed
                });

                tracing::info!("Permission filtering: {} -> {} tools", original_count, tools.len());

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
                    meta: None,
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

        // If authentication is disabled, allow all tool calls
        if !self.config.is_auth_enabled() {
            tracing::debug!(
                "Authentication disabled, allowing tool call: {}",
                request.name
            );
        } else {
            // Create AuthContext for permission validation
            let auth_context = AuthContext::from_request_context(_context);

            // Check if there is a valid session
            if !auth_context.has_valid_session() {
                tracing::warn!(
                    "Rejected unauthenticated call_tool request: {}",
                    request.name
                );
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Authentication required for call_tool".to_string(),
                    None,
                ));
            }

            // Check if session has expired
            if auth_context.is_session_expired() {
                tracing::warn!(
                    "Rejected expired session call_tool request: {}",
                    request.name
                );
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Session expired for call_tool".to_string(),
                    None,
                ));
            }

            // Check tool permission
            if !auth_context.has_tool_permission(&request.name) {
                tracing::warn!("Rejected unauthorized tool call: {}", request.name);
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    format!("Access denied: tool '{}' is not permitted", request.name),
                    None,
                ));
            }

            tracing::debug!("Tool {} permission verification passed", request.name);
        }

        // Parse the tool name to extract server name and original name
        let (server_name, original_name) =
            self.parse_tool_name(&request.name).ok_or_else(|| {
                RmcpErrorData::new(
                    ErrorCode(400),
                    format!(
                        "Invalid tool name format: {}. Expected format: 'server__tool_name'",
                        request.name
                    ),
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

        // If authentication is disabled, return all prompts
        if !self.config.is_auth_enabled() {
            return self.list_prompts_all(_request).await;
        }

        // Create AuthContext for permission validation
        let auth_context = AuthContext::from_request_context(_context);

        // Check if there is a valid session
        if !auth_context.has_valid_session() {
            tracing::warn!("Rejected unauthenticated list_prompts request");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Authentication required for list_prompts".to_string(),
                None,
            ));
        }

        // Check if session has expired
        if auth_context.is_session_expired() {
            tracing::warn!("Rejected expired session list_prompts request");
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
        match self.list_prompts().await {
            Ok(prompts) => {
                // Save original count for logging
                let original_count = prompts.len();

                // Filter prompts based on permissions with detailed checking
                let filtered_prompts: Vec<_> = prompts
                    .into_iter()
                    .filter(|prompt| {
                        let permission_result =
                            auth_context.check_prompt_permission_with_result(&prompt.name);
                        match permission_result {
                            crate::auth_context::PermissionResult::Allowed => true,
                            crate::auth_context::PermissionResult::NotAuthenticated => {
                                tracing::warn!(
                                    "Prompt {} access denied: not authenticated",
                                    prompt.name
                                );
                                false
                            }
                            crate::auth_context::PermissionResult::SessionExpired => {
                                tracing::warn!(
                                    "Prompt {} access denied: session expired",
                                    prompt.name
                                );
                                false
                            }
                            crate::auth_context::PermissionResult::InsufficientPermissions => {
                                tracing::debug!(
                                    "Prompt {} access denied: insufficient permissions",
                                    prompt.name
                                );
                                false
                            }
                        }
                    })
                    .collect();

                tracing::info!(
                    "Permission filtering: {} prompts remaining (filtered from {} total)",
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
                tracing::debug!("Successfully listed {} authorized prompts", total);
                Ok(ListPromptsResult {
                    meta: None,
                    prompts: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to get prompt list: {}", e);
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

        // If authentication is disabled, allow all prompt access
        if !self.config.is_auth_enabled() {
            tracing::debug!(
                "Authentication disabled, allowing prompt access: {}",
                request.name
            );
        } else {
            // Create AuthContext for permission validation
            let auth_context = AuthContext::from_request_context(_context);

            // Check if there is a valid session
            if !auth_context.has_valid_session() {
                tracing::warn!(
                    "Rejected unauthenticated get_prompt request: {}",
                    request.name
                );
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
                    format!(
                        "Invalid prompt name format: {}. Expected format: 'server__prompt_name'",
                        request.name
                    ),
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

        // If authentication is disabled, return all resources
        if !self.config.is_auth_enabled() {
            return self.list_resources_all(_request).await;
        }

        // Create AuthContext for permission validation
        let auth_context = AuthContext::from_request_context(_context);

        // Check if there is a valid session
        if !auth_context.has_valid_session() {
            tracing::warn!("Rejected unauthenticated list_resources request");
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
        match self.list_resources().await {
            Ok(resources) => {
                // Save original count for logging
                let original_count = resources.len();

                // Filter resources based on permissions with detailed checking
                let filtered_resources: Vec<_> = resources
                    .into_iter()
                    .filter(|resource| {
                        let permission_result =
                            self.evaluate_session_resource_permission(&auth_context, resource);
                        match permission_result {
                            crate::auth_context::PermissionResult::Allowed => true,
                            crate::auth_context::PermissionResult::NotAuthenticated => {
                                tracing::warn!(
                                    "Resource {} access denied: not authenticated",
                                    resource.uri
                                );
                                false
                            }
                            crate::auth_context::PermissionResult::SessionExpired => {
                                tracing::warn!(
                                    "Resource {} access denied: session expired",
                                    resource.uri
                                );
                                false
                            }
                            crate::auth_context::PermissionResult::InsufficientPermissions => {
                                tracing::debug!(
                                    "Resource {} access denied: insufficient permissions",
                                    resource.uri
                                );
                                false
                            }
                        }
                    })
                    .collect();

                tracing::info!(
                    "Permission filtering: {} resources remaining (filtered from {} total)",
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
                tracing::debug!("Successfully listed {} authorized resources", total);
                Ok(ListResourcesResult {
                    meta: None,
                    resources: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to get resource list: {}", e);
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

        // If authentication is disabled, allow all resource reads
        if !self.config.is_auth_enabled() {
            tracing::debug!(
                "Authentication disabled, allowing resource read: {}",
                request.uri
            );
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
            let mut has_permission = auth_context.has_resource_permission(&request.uri);
            if !has_permission {
                if let Some(alias) = self.resolve_resource_alias_from_uri(&request.uri).await {
                    has_permission = auth_context.has_resource_permission(&alias);
                }
            }

            if !has_permission {
                tracing::warn!("拒绝无权限的资源读取: {}", request.uri);
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    format!("Access denied: resource '{}' is not permitted", request.uri),
                    None,
                ));
            }

            tracing::debug!("Resource {} permission verification passed", request.uri);
        }

        // Parse the URI to extract server name and original URI
        let (server_name, original_uri) =
            self.parse_resource_uri(&request.uri).ok_or_else(|| {
                RmcpErrorData::new(
                    ErrorCode(400),
                    format!(
                        "Invalid resource URI format: {}. Expected format: 'server__resource_uri'",
                        request.uri
                    ),
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
