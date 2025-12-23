use crate::auth_context::{AuthContext, SessionIdExtension, SessionInfoExtension};
use crate::commands::app_info::get_mcp_server_info;
use crate::mcp_client::McpClientManager;
// Primary implementations
pub use crate::mcp_manager::McpServerManager;
pub use crate::token_manager::TokenManager;
use crate::types::ServerConfig;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::{Json, Response},
};
use chrono;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, ErrorCode, GetPromptRequestParam, GetPromptResult,
    InitializeRequestParam, InitializeResult, ListPromptsResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParam, ProtocolVersion, ReadResourceRequestParam,
    ReadResourceResult, Resource, Tool as McpTool,
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
            tracing::debug!(
                "Authorization header preview: {}...{}",
                &header[..10],
                &header[header.len() - 10..]
            );
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
            tracing::warn!(
                "Authentication failed: missing Authorization header for {} {}",
                method,
                uri
            );
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Validate token using TokenManager
    if let Some(token_value) = token_value {
        tracing::debug!("Validating token with TokenManager...");
        match token_manager.validate_token(token_value).await {
            Ok(token_id) => {
                tracing::info!("Authentication successful for token_id: {}", token_id);

                // Record usage statistics asynchronously (usage is already recorded in validate_token)
                // No need to record again here

                // Try to get full token information and store it directly
                tracing::debug!(
                    "Retrieving full token information for token_id: {}",
                    token_id
                );
                if let Ok(Some(token)) = token_manager.get_by_id(&token_id).await {
                    tracing::debug!(
                        "Token information retrieved, storing in request extensions..."
                    );

                    // Create a session-like info object directly from the token
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_else(|e| {
                            tracing::warn!("SystemTime calculation failed: {}", e);
                            std::time::Duration::ZERO
                        })
                        .as_secs();
                    let session_info = crate::auth_context::SessionInfo {
                        id: token_id.clone(), // Use token_id as session_id
                        token_id: Some(token_id.clone()),
                        created_at: now,
                        last_used_at: Some(now),
                        expires_at: token.expires_at, // Use token expiration if available
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
            }
            Err(e) => {
                tracing::warn!("Authentication failed: {}", e);
                Err(StatusCode::UNAUTHORIZED)
            }
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
    app: tauri::AppHandle,
}

impl McpAggregator {
    pub fn new(
        mcp_server_manager: Arc<McpServerManager>,
        mcp_client_manager: Arc<McpClientManager>,
        config: Arc<ServerConfig>,
        token_manager: Arc<TokenManager>,
        app: tauri::AppHandle,
    ) -> Self {
        Self {
            mcp_server_manager,
            mcp_client_manager,
            config,
            token_manager,
            shutdown_signal: Arc::new(std::sync::Mutex::new(None)),
            app,
        }
    }

    /// Apply pagination for tools
    async fn apply_pagination_tools(
        &self,
        tools: Vec<McpTool>,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, RmcpErrorData> {
        let mut offset = 0usize;
        if let Some(param) = request {
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
        let next_cursor = if end < total {
            Some(end.to_string())
        } else {
            None
        };
        tracing::info!("Successfully listed {} tools", total);
        Ok(ListToolsResult {
            meta: None,
            tools: slice,
            next_cursor,
        })
    }

    /// Apply pagination for resources
    async fn apply_pagination_resources(
        &self,
        resources: Vec<Resource>,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, RmcpErrorData> {
        let mut offset = 0usize;
        if let Some(param) = request {
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
        let next_cursor = if end < total {
            Some(end.to_string())
        } else {
            None
        };
        tracing::info!("Successfully listed {} resources", total);
        Ok(ListResourcesResult {
            meta: None,
            resources: slice,
            next_cursor,
        })
    }

    /// Apply pagination for prompts
    async fn apply_pagination_prompts(
        &self,
        prompts: Vec<rmcp::model::Prompt>,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, RmcpErrorData> {
        let mut offset = 0usize;
        if let Some(param) = request {
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
        let next_cursor = if end < total {
            Some(end.to_string())
        } else {
            None
        };
        tracing::info!("Successfully listed {} prompts", total);
        Ok(ListPromptsResult {
            meta: None,
            prompts: slice,
            next_cursor,
        })
    }

    // 辅助函数：创建默认 schema
    fn create_default_schema() -> std::sync::Arc<serde_json::Map<String, serde_json::Value>> {
        let mut default_schema = serde_json::Map::new();
        default_schema.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        default_schema.insert(
            "properties".to_string(),
            serde_json::Value::Object(serde_json::Map::new()),
        );
        std::sync::Arc::new(default_schema)
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
        let router = {
            let mut router = axum::Router::new()
                .nest_service("/mcp", service)
                .route("/health", axum::routing::get(health_check))
                .route("/debug/status", axum::routing::post(debug_status));

            if self.config.is_auth_enabled() {
                tracing::info!("Authentication enabled with dynamic token management");
                let token_manager = self.token_manager.clone();
                router = router.layer(middleware::from_fn(move |req, next| {
                    let token_manager = token_manager.clone();
                    async move { dynamic_bearer_auth_middleware(req, next, token_manager).await }
                }));
            } else {
                tracing::info!("Authentication disabled - running without auth middleware");
            }

            router
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
            let mut shutdown_guard = self.shutdown_signal
                .lock()
                .expect("Failed to acquire shutdown_signal lock");
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
        let updated_count: usize = entries.iter().map(|e| e.0.len() + e.1.len()).sum();
        let latest = std::time::SystemTime::now();

        // Get connected servers count
        let servers = self
            .mcp_server_manager
            .list_servers(None, None)
            .await
            .unwrap_or_default();
        let connected_servers = servers.0.iter().filter(|s| s.status == "connected").count();
        let total_servers = servers.0.len();

        serde_json::json!({
            "status": "running",
            "message": "Aggregator initialized",
            "server_stats": { "total": total_servers, "connected": connected_servers },
            "tool_cache": { "enabled": true, "entries": total, "ttl_seconds": ttl, "tools_total": updated_count, "last_updated": latest.duration_since(std::time::UNIX_EPOCH).ok().map(|d| format!("{}", d.as_secs())) }
        })
    }

    /// Handle service connection status changes
    pub async fn handle_service_status_change(&self, service_id: &str, is_connected: bool) {
        tracing::info!(
            "Service '{}' connection status changed to: {}",
            service_id,
            if is_connected {
                "connected"
            } else {
                "disconnected"
            }
        );

        // If service connected, refresh its tools/resources/prompts
        if is_connected {
            tracing::info!(
                "Refreshing tools/resources/prompts for service '{}'",
                service_id
            );

            // Refresh can be done asynchronously in background
            let manager = self.mcp_server_manager.clone();
            let service_id_clone = service_id.to_string();
            tokio::spawn(async move {
                if let Err(e) = manager.sync_server_manifests(&service_id_clone).await {
                    tracing::warn!(
                        "Failed to refresh manifests for service '{}': {}",
                        service_id_clone,
                        e
                    );
                }
            });
        }
    }

    /// 根据权限过滤工具列表
    fn filter_tools_by_token_permissions(
        &self,
        tools: Vec<McpTool>,
        token_info: &crate::token_manager::TokenInfo,
    ) -> Vec<McpTool> {
        // 记录权限检查前的工具列表
        for tool in &tools {
            tracing::debug!("🔍 Tool before permission filter: {}", tool.name);
        }

        // 记录工具数量用于日志
        let original_count = tools.len();

        // 优化：使用 HashSet 加速权限查找 O(n+m) 替代 O(n*m)
        let allowed_tools_set: std::collections::HashSet<&str> = token_info
            .allowed_tools
            .iter()
            .map(|s| s.as_str())
            .collect();

        // 真实的权限过滤（精确匹配 resource_path）
        let filtered_tools: Vec<McpTool> = tools
            .into_iter()
            .filter(|tool| {
                let tool_name = &tool.name;
                let has_permission = allowed_tools_set.contains(tool_name.as_ref());

                if has_permission {
                    tracing::debug!("✅ Tool {} allowed by permission", tool_name);
                } else {
                    tracing::debug!(
                        "🚫 Tool {} filtered out - not in allowed tools: {:?}",
                        tool_name,
                        token_info.allowed_tools
                    );
                }
                has_permission
            })
            .collect();

        tracing::info!(
            "✅ Permission filtering: {} -> {} tools",
            original_count,
            filtered_tools.len()
        );

        // 记录最终返回的工具列表
        for tool in &filtered_tools {
            tracing::debug!("🎯 Tool after permission filter: {}", tool.name);
        }

        filtered_tools
    }

    /// 获取 Token 信息用于权限验证
    async fn get_token_info_for_auth(
        &self,
        authcontext: &AuthContext,
        token_manager: Arc<crate::token_manager::TokenManager>,
    ) -> Option<crate::token_manager::TokenInfo> {
        if let Some(token_id) = authcontext.token_id() {
            tracing::info!("Loading permissions for token: {}", token_id);
            match token_manager.get_token_by_id(token_id).await {
                Ok(Some(info)) => {
                    tracing::info!(
                        "Loaded {} tool permissions for token: {}",
                        info.allowed_tools.len(),
                        token_id
                    );
                    Some(info)
                }
                Ok(None) => {
                    tracing::warn!("Token not found: {}", token_id);
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to load token permissions: {}", e);
                    None
                }
            }
        } else {
            tracing::warn!("No token_id found in authcontext");
            None
        }
    }

    /// Parse resource path with server prefix
    /// Extracts server name and original resource identifier from a path in format "server__resource"
    fn parse_resource_path(&self, resource_path: &str) -> Option<(String, String)> {
        if let Some((server_name, original_name)) = resource_path.split_once("__") {
            Some((server_name.to_string(), original_name.to_string()))
        } else {
            None
        }
    }

    pub async fn trigger_shutdown(&self) {
        tracing::info!("Triggering aggregator shutdown...");

        // Get the cancellation token and trigger shutdown
        let shutdown_guard = self.shutdown_signal
            .lock()
            .expect("Failed to acquire shutdown_signal lock");
        if let Some(ct) = shutdown_guard.as_ref() {
            ct.cancel();
            tracing::info!("Shutdown signal sent to MCP Aggregator server");
        } else {
            tracing::warn!("No active shutdown signal found for MCP Aggregator");
        }
    }

    /// Fetch tools from database (merged logic from get_tools_direct)
    async fn fetch_tools_from_database(&self) -> Result<Vec<McpTool>, RmcpErrorData> {
        tracing::info!("🔍 Getting tools directly from database");

        // 通过 McpServerManager 的公共方法获取完整的工具信息，包含 input_schema
        let tools_data = self
            .mcp_server_manager
            .get_all_tools_for_aggregation()
            .await
            .map_err(|e| {
                tracing::error!("❌ Failed to fetch tools from manager: {}", e);
                RmcpErrorData::internal_error(format!("Failed to fetch tools: {}", e), None)
            })?;

        tracing::info!("📊 Retrieved {} tools from database", tools_data.len());

        // 优化：预分配 Vec 容量，避免多次重分配
        let mut mcp_tools = Vec::with_capacity(tools_data.len());

        for (_tool_id, tool_name, description, input_schema_json, server_name) in tools_data {
            // 记录原始数据
            tracing::debug!(
                "🔧 Processing tool: {} from server: {}",
                tool_name,
                server_name
            );
            tracing::debug!(
                "📝 Raw input_schema from DB: {}",
                input_schema_json
                    .as_ref()
                    .map_or("NULL".to_string(), |s| s.clone())
            );

            let server_name_str = server_name.clone(); // server_name 已经是 String 类型

            // 生成 resource_path
            let resource_path = format!("{}__{}", server_name_str, tool_name);

            // 处理 input_schema，使用数据库中存储的真实数据或创建默认的空 schema
            let input_schema: std::sync::Arc<serde_json::Map<String, serde_json::Value>> =
                if let Some(schema_str) = &input_schema_json {
                    // 尝试解析 JSON Schema
                    match serde_json::from_str::<serde_json::Value>(schema_str) {
                        Ok(schema) => {
                            tracing::debug!(
                                "✅ Successfully parsed JSON Schema for tool: {}",
                                tool_name
                            );
                            tracing::debug!("📋 Schema content: {}", schema);

                            if let serde_json::Value::Object(mut map) = schema {
                                // 确保至少有 type 字段
                                if !map.contains_key("type") {
                                    map.insert(
                                        "type".to_string(),
                                        serde_json::Value::String("object".to_string()),
                                    );
                                    tracing::debug!(
                                        "➕ Added default 'type: object' field to schema"
                                    );
                                }
                                std::sync::Arc::new(map)
                            } else {
                                tracing::warn!(
                                    "⚠️ Schema for tool {} is not an object, using default",
                                    tool_name
                                );
                                Self::create_default_schema()
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "❌ Failed to parse JSON Schema for tool {}: {}",
                                tool_name,
                                e
                            );
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
                description: Some(description.clone().into()),
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

        tracing::info!(
            "🎉 Successfully processed {} McpTool objects",
            mcp_tools.len()
        );
        Ok(mcp_tools)
    }

    /// Fetch resources from database (merged logic from get_resources_direct)
    async fn fetch_resources_from_database(&self) -> Result<Vec<Resource>, RmcpErrorData> {
        tracing::info!("🔍 Getting resources directly from database");

        // 通过 McpServerManager 的公共方法获取完整的资源信息
        let resources_data = self
            .mcp_server_manager
            .get_all_resources_for_aggregation()
            .await
            .map_err(|e| {
                tracing::error!("❌ Failed to fetch resources from manager: {}", e);
                RmcpErrorData::internal_error(format!("Failed to fetch resources: {}", e), None)
            })?;

        tracing::info!(
            "📊 Retrieved {} resources from database",
            resources_data.len()
        );

        let mut mcp_resources = Vec::new();

        for (_resource_id, uri, name, description, mime_type, server_name) in resources_data {
            // 记录原始数据
            tracing::debug!(
                "🔧 Processing resource: {} from server: {}",
                uri,
                server_name
            );

            let server_name_str = server_name.clone(); // server_name 已经是 String 类型

            // 构建完整的 resource_path (server_name__uri)
            let resource_path = format!("{}__{}", server_name_str, uri);

            // 创建 Resource 结构体
            let raw_resource = rmcp::model::RawResource {
                uri: resource_path.clone(),
                name: name.clone(),
                title: None,
                description: Some(description),
                mime_type,
                size: None,
                icons: None,
                meta: None,
            };

            let resource = Resource {
                raw: raw_resource,
                annotations: None,
            };

            mcp_resources.push(resource);
        }

        tracing::info!(
            "✅ Successfully processed {} resources",
            mcp_resources.len()
        );
        Ok(mcp_resources)
    }

    /// Fetch prompts from database (merged logic from get_prompts_direct)
    async fn fetch_prompts_from_database(&self) -> Result<Vec<rmcp::model::Prompt>, RmcpErrorData> {
        tracing::info!("🔍 Getting prompts directly from database");

        // 通过 McpServerManager 的公共方法获取完整的提示词信息
        let prompts_data = self
            .mcp_server_manager
            .get_all_prompts_for_aggregation()
            .await
            .map_err(|e| {
                tracing::error!("❌ Failed to fetch prompts from manager: {}", e);
                RmcpErrorData::internal_error(format!("Failed to fetch prompts: {}", e), None)
            })?;

        tracing::info!("📊 Retrieved {} prompts from database", prompts_data.len());

        let mut mcp_prompts = Vec::new();

        for (_prompt_id, name, description, server_name) in prompts_data {
            // 记录原始数据
            tracing::debug!(
                "🔧 Processing prompt: {} from server: {}",
                name,
                server_name
            );

            let server_name_str = server_name.clone(); // server_name 已经是 String 类型

            // 生成 resource_path (server_name__prompt_name)
            let resource_path = format!("{}__{}", server_name_str, name);

            // 创建 Prompt 结构体
            let prompt = rmcp::model::Prompt {
                name: resource_path.clone(),
                description: description.clone(),
                arguments: None, // TODO: 根据需要实现参数
                icons: None,
                meta: None,
                title: None,
            };

            mcp_prompts.push(prompt);
        }

        tracing::info!("✅ Successfully processed {} prompts", mcp_prompts.len());
        Ok(mcp_prompts)
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
            server_info: get_mcp_server_info(&self.app),
            instructions: None,
        })
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, RmcpErrorData> {
        tracing::info!("=== List Tools Handler ===");
        tracing::info!("Request parameters: {:?}", request);

        // If authentication is disabled, return all tools without pagination
        tracing::info!("Authentication enabled: {}", self.config.is_auth_enabled());
        if !self.config.is_auth_enabled() {
            tracing::info!("Authentication disabled, returning all tools");
            // Direct data retrieval - return all tools when auth is disabled
            let tools = self.fetch_tools_from_database().await?;
            tracing::info!("Successfully listed {} tools (no pagination)", tools.len());
            return Ok(ListToolsResult {
                meta: None,
                tools,
                next_cursor: None,
            });
        }

        // Create AuthContext from RequestContext
        tracing::debug!("Creating AuthContext from RequestContext");
        let authcontext = AuthContext::from_request_context(context);

        tracing::info!(
            "AuthContext created - has_valid_session: {}, is_session_expired: {}",
            authcontext.has_valid_session(),
            authcontext.is_session_expired()
        );

        // Log session details if available
        if let Some(session_id) = authcontext.session_id() {
            tracing::info!("Session ID: {}", session_id);
        }

        if let Some(token_id) = authcontext.token_id() {
            tracing::info!("Token ID: {}", token_id);
        }

        // Check if we have a valid session with permissions
        if !authcontext.has_valid_session() {
            tracing::warn!("List tools denied: no valid session found");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Authentication required: no valid session".to_string(),
                None,
            ));
        }

        // Check if session has expired
        if authcontext.is_session_expired() {
            tracing::warn!("List tools denied: session has expired");
            return Err(RmcpErrorData::new(
                ErrorCode(403),
                "Authentication failed: session has expired".to_string(),
                None,
            ));
        }

        // 内联认证和数据获取逻辑
        let tools = self.fetch_tools_from_database().await?;
        let original_count = tools.len();
        tracing::info!("📋 Retrieved {} tools from database", original_count);

        // 获取 Token 信息进行权限过滤
        let token_info = match self
            .get_token_info_for_auth(&authcontext, self.token_manager.clone())
            .await
        {
            Some(info) => info,
            None => {
                tracing::warn!("No valid token info available, returning empty tool list");
                return self.apply_pagination_tools(vec![], request).await;
            }
        };

        // 过滤工具
        let filtered_tools = self.filter_tools_by_token_permissions(tools, &token_info);

        tracing::info!(
            "Permission filtering: {} -> {} tools",
            original_count,
            filtered_tools.len()
        );

        // 应用分页逻辑
        let result = self.apply_pagination_tools(filtered_tools, request).await?;

        tracing::info!(
            "Successfully listed {} tools for session {} (filtered from total {})",
            result.tools.len(),
            authcontext.session_id().unwrap_or("unknown"),
            original_count
        );

        Ok(result)
    }

    // Enhanced implementations for remaining methods
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
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
            let authcontext = AuthContext::from_request_context(context);

            // Check if there is a valid session
            if !authcontext.has_valid_session() {
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
            if authcontext.is_session_expired() {
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
            if !authcontext.has_tool_permission(&request.name) {
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
            self.parse_resource_path(&request.name).ok_or_else(|| {
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

        // Check if the server is connected first
        let (connection_status, error_message) = self
            .mcp_client_manager
            .get_connection_status(&server_name)
            .await;
        if connection_status != "connected" {
            let error_msg = format!(
                "Server '{}' is not available (status: {}). {}",
                server_name,
                connection_status,
                error_message
                    .unwrap_or("Please check the server configuration and status.".to_string())
            );
            tracing::warn!("{} while calling tool '{}'", error_msg, original_name);
            return Err(RmcpErrorData::new(
                ErrorCode(503), // Service Unavailable
                error_msg,
                None,
            ));
        }

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
                let error_code = if e.to_string().contains("Service not found") {
                    ErrorCode(404)
                } else if e.to_string().contains("Service not connected") {
                    ErrorCode(503)
                } else {
                    ErrorCode(500)
                };
                Err(RmcpErrorData::new(
                    error_code,
                    format!("Failed to call tool: {}", e),
                    None,
                ))
            }
        }
    }

    async fn list_prompts(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, RmcpErrorData> {
        tracing::debug!("List prompts request received");

        // If authentication is disabled, return all prompts without pagination
        if !self.config.is_auth_enabled() {
            tracing::info!("Authentication disabled, returning all prompts");
            // Direct data retrieval - return all prompts when auth is disabled
            let prompts = self.fetch_prompts_from_database().await?;
            tracing::info!(
                "Successfully listed {} prompts (no pagination)",
                prompts.len()
            );
            return Ok(ListPromptsResult {
                meta: None,
                prompts,
                next_cursor: None,
            });
        }

        // Create AuthContext for permission validation
        let authcontext = AuthContext::from_request_context(context);

        // Check if there is a valid session
        if !authcontext.has_valid_session() {
            tracing::warn!("Rejected unauthenticated list_prompts request");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Authentication required for list_prompts".to_string(),
                None,
            ));
        }

        // Check if session has expired
        if authcontext.is_session_expired() {
            tracing::warn!("Rejected expired session list_prompts request");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Session expired for list_prompts".to_string(),
                None,
            ));
        }

        // 内联认证和数据获取逻辑
        let prompts = self.fetch_prompts_from_database().await?;
        let original_count = prompts.len();
        tracing::info!("📋 Retrieved {} prompts from database", original_count);

        // 获取 Token 信息进行权限过滤
        let token_info = match self
            .get_token_info_for_auth(&authcontext, self.token_manager.clone())
            .await
        {
            Some(info) => info,
            None => {
                tracing::warn!("No valid token info available, returning empty prompt list");
                return self.apply_pagination_prompts(vec![], request).await;
            }
        };

        // 过滤提示词
        let filtered_prompts: Vec<rmcp::model::Prompt> = prompts
            .into_iter()
            .filter(|prompt| {
                // 只支持精确匹配
                token_info
                    .allowed_prompts
                    .iter()
                    .any(|allowed_prompt| allowed_prompt == &prompt.name)
            })
            .collect();

        tracing::info!(
            "Permission filtering: {} -> {} prompts",
            original_count,
            filtered_prompts.len()
        );

        // 应用分页逻辑
        self.apply_pagination_prompts(filtered_prompts, request)
            .await
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        context: RequestContext<RoleServer>,
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
            let authcontext = AuthContext::from_request_context(context);

            // Check if there is a valid session
            if !authcontext.has_valid_session() {
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
            if authcontext.is_session_expired() {
                tracing::warn!(
                    "Rejected get_prompt request from expired session: {}",
                    request.name
                );
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Session expired for get_prompt".to_string(),
                    None,
                ));
            }

            // 检查提示词权限
            if !authcontext.has_prompt_permission(&request.name) {
                tracing::warn!("Access denied for prompt: {}", request.name);
                return Err(RmcpErrorData::new(
                    ErrorCode(403),
                    format!("Access denied: prompt '{}' is not permitted", request.name),
                    None,
                ));
            }

            tracing::debug!("Prompt {} permission verification passed", request.name);
        }

        // Parse the prompt name to extract server name and original name
        let (server_name, original_name) =
            self.parse_resource_path(&request.name).ok_or_else(|| {
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

        // Check if the server is connected first
        let (connection_status, error_message) = self
            .mcp_client_manager
            .get_connection_status(&server_name)
            .await;
        if connection_status != "connected" {
            let error_msg = format!(
                "Server '{}' is not available (status: {}). {}",
                server_name,
                connection_status,
                error_message
                    .unwrap_or("Please check the server configuration and status.".to_string())
            );
            tracing::warn!("{} while getting prompt '{}'", error_msg, original_name);
            return Err(RmcpErrorData::new(
                ErrorCode(503), // Service Unavailable
                error_msg,
                None,
            ));
        }

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
                let error_code = if e.to_string().contains("Service not found") {
                    ErrorCode(404)
                } else if e.to_string().contains("Service not connected") {
                    ErrorCode(503)
                } else {
                    ErrorCode(500)
                };
                Err(RmcpErrorData::new(
                    error_code,
                    format!("Failed to get prompt: {}", e),
                    None,
                ))
            }
        }
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, RmcpErrorData> {
        tracing::debug!("List resources request received");

        // If authentication is disabled, return all resources without pagination
        if !self.config.is_auth_enabled() {
            tracing::info!("Authentication disabled, returning all resources");
            // Direct data retrieval - return all resources when auth is disabled
            let resources = self.fetch_resources_from_database().await?;
            tracing::info!(
                "Successfully listed {} resources (no pagination)",
                resources.len()
            );
            return Ok(ListResourcesResult {
                meta: None,
                resources,
                next_cursor: None,
            });
        }

        // Create AuthContext for permission validation
        let authcontext = AuthContext::from_request_context(context);

        // Check if there is a valid session
        if !authcontext.has_valid_session() {
            tracing::warn!("Rejected unauthenticated list_resources request");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Authentication required for list_resources".to_string(),
                None,
            ));
        }

        // 检查会话是否过期
        if authcontext.is_session_expired() {
            tracing::warn!("Rejected list_resources request from expired session");
            return Err(RmcpErrorData::new(
                ErrorCode(401),
                "Session expired for list_resources".to_string(),
                None,
            ));
        }

        // 内联认证和数据获取逻辑
        let resources = self.fetch_resources_from_database().await?;
        let original_count = resources.len();
        tracing::info!("📋 Retrieved {} resources from database", original_count);

        // 获取 Token 信息进行权限过滤
        let token_info = match self
            .get_token_info_for_auth(&authcontext, self.token_manager.clone())
            .await
        {
            Some(info) => info,
            None => {
                tracing::warn!("No valid token info available, returning empty resource list");
                return self.apply_pagination_resources(vec![], request).await;
            }
        };

        // 过滤资源
        let filtered_resources: Vec<Resource> = resources
            .into_iter()
            .filter(|resource| {
                // 只支持精确匹配，包括 scheme://resource 格式
                token_info
                    .allowed_resources
                    .iter()
                    .any(|allowed_resource| allowed_resource == &resource.uri)
            })
            .collect();

        tracing::info!(
            "Permission filtering: {} -> {} resources",
            original_count,
            filtered_resources.len()
        );

        // 应用分页逻辑
        self.apply_pagination_resources(filtered_resources, request)
            .await
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        context: RequestContext<RoleServer>,
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
            let authcontext = AuthContext::from_request_context(context);

            // 检查是否有有效会话
            if !authcontext.has_valid_session() {
                tracing::warn!(
                    "Rejected unauthenticated read_resource request: {}",
                    request.uri
                );
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Authentication required for read_resource".to_string(),
                    None,
                ));
            }

            // 检查会话是否过期
            if authcontext.is_session_expired() {
                tracing::warn!(
                    "Rejected read_resource request from expired session: {}",
                    request.uri
                );
                return Err(RmcpErrorData::new(
                    ErrorCode(401),
                    "Session expired for read_resource".to_string(),
                    None,
                ));
            }

            // 检查资源权限
            let has_permission = authcontext.has_resource_permission(&request.uri);

            if !has_permission {
                tracing::warn!("Access denied for resource: {}", request.uri);
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
            self.parse_resource_path(&request.uri).ok_or_else(|| {
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

        // Check if the server is connected first
        let (connection_status, error_message) = self
            .mcp_client_manager
            .get_connection_status(&server_name)
            .await;
        if connection_status != "connected" {
            let error_msg = format!(
                "Server '{}' is not available (status: {}). {}",
                server_name,
                connection_status,
                error_message
                    .unwrap_or("Please check the server configuration and status.".to_string())
            );
            tracing::warn!("{} while reading resource '{}'", error_msg, original_uri);
            return Err(RmcpErrorData::new(
                ErrorCode(503), // Service Unavailable
                error_msg,
                None,
            ));
        }

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
                let error_code = if e.to_string().contains("Service not found") {
                    ErrorCode(404)
                } else if e.to_string().contains("Service not connected") {
                    ErrorCode(503)
                } else {
                    ErrorCode(500)
                };
                Err(RmcpErrorData::new(
                    error_code,
                    format!("Failed to read resource: {}", e),
                    None,
                ))
            }
        }
    }
}
