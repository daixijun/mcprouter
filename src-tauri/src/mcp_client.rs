use crate::config::AppConfig;
use crate::error::{McpError, Result};
use crate::types::{ConnectionStatus, McpConnection, McpServerConfig, McpService};
use reqwest::header;
use rmcp::model::Tool;
use rmcp::service::ServiceExt;
use rmcp::transport::child_process::TokioChildProcess;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;

/// Helper function to create a reqwest client with logging for HTTP transport
fn create_http_reqwest_client(
    custom_headers: Option<&HashMap<String, String>>,
) -> Result<reqwest::Client> {
    let user_agent = crate::commands::app_info::get_user_agent_static();
    let mut client_builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(30));

    // Add default headers for HTTP transport
    let mut headers = header::HeaderMap::new();
    // 更宽松的Accept头，支持带charset的JSON响应
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("application/json, text/event-stream; charset=utf-8"),
    );
    // 不设置固定的Content-Type，让reqwest自动处理
    // headers.insert(
    //     "Content-Type",
    //     header::HeaderValue::from_static("application/json"),
    // );

    // Add custom headers if present
    if let Some(headers_map) = custom_headers {
        for (key, value) in headers_map {
            if let Ok(header_name) = header::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = header::HeaderValue::from_str(value) {
                    headers.insert(header_name, header_value);
                    tracing::debug!(
                        "Added HTTP header: {}: {}",
                        key,
                        if key.eq_ignore_ascii_case("authorization") {
                            "****"
                        } else {
                            value
                        }
                    );
                }
            }
        }
    }

    client_builder = client_builder.default_headers(headers);
    client_builder
        .build()
        .map_err(|e| McpError::ConnectionError(e.to_string()))
}

pub struct McpClientManager {
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    connection_cache_ttl: std::time::Duration,
    tool_manager: Arc<crate::tool_manager::ToolManager>,
}

impl Clone for McpClientManager {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
            connection_cache_ttl: self.connection_cache_ttl,
            tool_manager: Arc::clone(&self.tool_manager),
        }
    }
}

impl McpClientManager {
    pub fn new(_config: AppConfig) -> Self {
        let tool_manager = Arc::new(crate::tool_manager::ToolManager::new());
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_cache_ttl: std::time::Duration::from_secs(300),
            tool_manager,
        }
    }

    /// Check if cached connection is still valid
    async fn is_connection_cache_valid(&self, service_name: &str) -> bool {
        if let Some(connection) = self.connections.read().await.get(service_name) {
            if let Some(last_connected) = connection.status.last_connected {
                let now = chrono::Utc::now();
                let cache_duration = now.signed_duration_since(last_connected);
                return cache_duration.num_seconds() < self.connection_cache_ttl.as_secs() as i64
                    && connection.status.is_connected;
            }
        }
        false
    }

    pub async fn ensure_connection(
        &self,
        service_config: &McpServerConfig,
        force_refresh: bool,
    ) -> Result<McpConnection> {
        let service_name = &service_config.name;

        if !force_refresh && self.is_connection_cache_valid(service_name).await {
            if let Some(connection) = self.connections.read().await.get(service_name) {
                return Ok(connection.clone());
            }
        }

        let connection_result = match service_config.transport {
            crate::types::ServiceTransport::Stdio => {
                self.create_stdio_connection(service_config).await
            }
            crate::types::ServiceTransport::Http => {
                self.create_http_connection(service_config).await
            }
        };

        // Store the connection result (success or failure) in cache
        let connection = match connection_result {
            Ok(conn) => {
                self.connections
                    .write()
                    .await
                    .insert(service_name.clone(), conn.clone());
                Ok(conn)
            }
            Err(e) => {
                // Store failed connection status with error message
                let error_message = e.to_string();
                let failed_connection = McpConnection {
                    service_id: service_name.clone(),
                    server_info: None,
                    client: None,
                    status: ConnectionStatus {
                        is_connected: false,
                        is_connecting: false,
                        last_connected: Some(chrono::Utc::now()),
                        error_message: Some(error_message.clone()),
                    },
                };
                self.connections
                    .write()
                    .await
                    .insert(service_name.clone(), failed_connection);
                Err(McpError::ConnectionError(error_message))
            }
        };

        connection
    }

    /// Create STDIO connection using managed tools
    async fn create_stdio_connection(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        let command = service_config.command.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("STDIO service requires command".to_string())
        })?;

        // Initialize tool manager and ensure tools are available
        self.tool_manager.initialize().await?;
        self.tool_manager.ensure_tools_for_command(command).await?;

        // Get original args from config
        let original_args = service_config.args.as_ref().cloned().unwrap_or_default();

        // For npx commands, we need to combine command and args before conversion
        let (converted_command, final_args) = {
            let first_word = command.split_whitespace().next().unwrap_or("");
            if first_word == "npx" {
                // Combine command and args into a single command string
                let full_command = if original_args.is_empty() {
                    command.to_string()
                } else {
                    format!("{} {}", command, original_args.join(" "))
                };

                // Convert the full command
                let converted = self.tool_manager.convert_command(&full_command);

                // Parse converted command and filter arguments
                let converted_parts: Vec<&str> = converted.split_whitespace().collect();
                if converted_parts.len() >= 3
                    && converted_parts[0] == "bun"
                    && converted_parts[1] == "x"
                {
                    let mut filtered_args = Vec::new();
                    for arg in converted_parts.iter() {
                        let arg_lower = arg.to_lowercase();
                        // Skip "bun" and -y/--yes, keep x and everything else
                        if *arg != "bun" && arg_lower != "-y" && arg_lower != "--yes" {
                            filtered_args.push(arg.to_string());
                        } else if *arg != "bun" {
                            tracing::debug!(
                                "Removed npx-specific argument '{}' when converting to bun x",
                                arg
                            );
                        }
                    }
                    (converted, filtered_args)
                } else {
                    // If converted command doesn't start with "bun x", use converted as command and empty args
                    (converted, vec![])
                }
            } else {
                // Non-npx commands: use regular conversion and keep original args
                (self.tool_manager.convert_command(command), original_args)
            }
        };

        let mut env_vars = service_config.env.clone().unwrap_or_default();

        // *** 新增：加载 Shell 环境变量 ***
        match crate::shell_environment::ShellEnvironment::load_environment().await {
            Ok(shell_env) => {
                // 合并环境变量（shell 环境优先）
                for (key, value) in shell_env {
                    env_vars.insert(key, value);
                }
                tracing::debug!("Loaded shell environment variables");
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load shell environment, using current process env: {}",
                    e
                );
                // 继续使用当前进程环境变量
            }
        }

        // Get the executable path
        let executable_path = {
            let first_word = command.split_whitespace().next().unwrap_or("");
            if first_word == "npx" && converted_command.starts_with("bun x") {
                // For converted npx commands, use bun executable
                self.tool_manager
                    .get_executable_path("bun")
                    .await
                    .unwrap_or_else(|_| {
                        tracing::warn!(
                            "Tool not found in managed directory, falling back to system PATH"
                        );
                        PathBuf::from("bun")
                    })
            } else {
                // For other commands, use regular path resolution
                self.tool_manager
                    .get_executable_path(&converted_command)
                    .await
                    .unwrap_or_else(|_| {
                        tracing::warn!(
                            "Tool not found in managed directory, falling back to system PATH"
                        );
                        PathBuf::from(
                            converted_command
                                .split_whitespace()
                                .next()
                                .unwrap_or(&converted_command),
                        )
                    })
            }
        };

        // Load settings and apply environment configuration
        if let Ok(config) = crate::config::AppConfig::load() {
            if let Some(settings) = config.settings {
                // Apply environment variables based on command type
                let first_word = command.split_whitespace().next().unwrap_or("");
                if first_word == "uvx" || first_word == "uv" {
                    if let Some(uv_index_url) = settings.uv_index_url {
                        env_vars.insert("UV_INDEX_URL".to_string(), uv_index_url);
                    }
                } else if first_word == "npx" || first_word == "npm" {
                    if let Some(npm_registry) = settings.npm_registry {
                        env_vars.insert("NPM_CONFIG_REGISTRY".to_string(), npm_registry);
                    }
                }
            }
        }

        tracing::debug!(
            "Creating STDIO MCP service: {} (converted to: {}), args: {:?}",
            service_config.name,
            converted_command,
            final_args
        );

        // Create transport
        let mut command_builder = Command::new(executable_path);
        command_builder.args(&final_args);
        for (key, value) in env_vars {
            command_builder.env(key, value);
        }

        let transport = TokioChildProcess::new(command_builder)
            .map_err(|e| McpError::ConnectionError(e.to_string()))?;

        // Create service
        let service =
            ().serve(transport)
                .await
                .map_err(|e| McpError::ConnectionError(e.to_string()))?;

        let server_info = service.peer_info();

        // Print server_info structure
        if let Some(ref info) = server_info {
            tracing::debug!("Service '{}' server_info: {:?}", service_config.name, info);
        }

        Ok(McpConnection {
            service_id: service_config.name.clone(),
            server_info: server_info.cloned(),
            client: Some(Arc::new(McpService::Stdio(Arc::new(service)))),
            status: ConnectionStatus {
                is_connected: true,
                is_connecting: false,
                last_connected: Some(chrono::Utc::now()),
                error_message: None,
            },
        })
    }

    /// Create HTTP connection using rmcp 0.8.3
    async fn create_http_connection(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        let url = service_config.url.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("HTTP service requires URL".to_string())
        })?;

        tracing::debug!("Creating HTTP MCP service: {}", url);
        tracing::debug!("Service '{}' URL: {}", service_config.name, url);

        // Log custom headers if present
        if let Some(headers) = &service_config.headers {
            tracing::debug!(
                "Service '{}' custom headers: {:?}",
                service_config.name,
                headers
            );
        }

        // Create HTTP client with reqwest including custom headers
        tracing::debug!("Creating HTTP client for service '{}'", service_config.name);
        let client = create_http_reqwest_client(service_config.headers.as_ref())?;

        // Create HTTP transport configuration with stateless mode enabled
        let mut config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                url.as_ref(),
            );
        // Enable stateless mode to support servers that don't support SSE streaming
        config.allow_stateless = true;

        // Use the public type alias - reqwest::Client implements StreamableHttpClient
        let transport = rmcp::transport::StreamableHttpClientTransport::with_client(client, config);

        // Add detailed error logging for connection failures
        let service = match ().serve(transport).await {
            Ok(service) => {
                tracing::info!(
                    "Successfully connected to HTTP MCP service: {}",
                    service_config.name
                );
                service
            }
            Err(e) => {
                let error_msg = e.to_string();
                tracing::error!(
                    "Failed to connect to HTTP MCP service '{}': {}",
                    service_config.name,
                    error_msg
                );

                return Err(McpError::ConnectionError(error_msg));
            }
        };

        let server_info = service.peer_info();

        // Print server_info structure
        if let Some(ref info) = server_info {
            tracing::debug!("Service '{}' server_info: {:?}", service_config.name, info);
        }

        Ok(McpConnection {
            service_id: service_config.name.clone(),
            server_info: server_info.cloned(),
            client: Some(Arc::new(McpService::Http(Arc::new(service)))),
            status: ConnectionStatus {
                is_connected: true,
                is_connecting: false,
                last_connected: Some(chrono::Utc::now()),
                error_message: None,
            },
        })
    }

    pub async fn disconnect_mcp_server(&self, service_id: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.remove(service_id) {
            if let Some(_service) = connection.client {
                tracing::debug!("Disconnected from service: {}", service_id);
            }
        }
        Ok(())
    }

    pub async fn list_tools(&self, connection_id: &str) -> Result<Vec<Tool>> {
        let connections = self.connections.read().await;
        let connection = connections
            .get(connection_id)
            .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        if !connection.status.is_connected {
            return Err(McpError::ConnectionError(
                "Service not connected".to_string(),
            ));
        }

        if let Some(ref client_arc) = connection.client {
            tracing::debug!(
                "Attempting to fetch tools from rmcp client {}",
                connection_id
            );

            // Extract the peer from the service
            let peer = client_arc.peer();

            // Create list tools request with pagination support
            let request =
                rmcp::model::ListToolsRequest::with_param(rmcp::model::PaginatedRequestParam {
                    cursor: None,
                });

            let start_time = std::time::Instant::now();
            // Send request via peer and get response
            // Note: We need to convert to ClientRequest
            let client_request: rmcp::model::ClientRequest = request.into();
            let server_result = peer
                .send_request(client_request)
                .await
                .map_err(|e| McpError::ServiceError(e.to_string()))?;
            let duration = start_time.elapsed();

            // Convert ServerResult to ListToolsResult
            let result = match server_result {
                rmcp::model::ServerResult::ListToolsResult(r) => r,
                _ => {
                    tracing::warn!("Unexpected response type from {}", connection_id);
                    rmcp::model::ListToolsResult {
                        tools: Vec::new(),
                        next_cursor: None,
                        meta: None,
                    }
                }
            };

            tracing::debug!(
                "Successfully fetched {} tools from {} ({}ms)",
                result.tools.len(),
                connection_id,
                duration.as_millis()
            );

            // Log each tool for debugging
            for tool in &result.tools {
                tracing::debug!(
                    "Tool: {} - {}",
                    tool.name,
                    tool.description
                        .as_ref()
                        .map(|d| d.as_ref())
                        .unwrap_or("No description")
                );
            }

            Ok(result.tools)
        } else {
            tracing::warn!("No client available for connection {}", connection_id);
            Ok(Vec::new())
        }
    }

    pub async fn list_resources(&self, connection_id: &str) -> Result<Vec<rmcp::model::Resource>> {
        let connections = self.connections.read().await;
        let connection = connections
            .get(connection_id)
            .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        if !connection.status.is_connected {
            return Err(McpError::ConnectionError(
                "Service not connected".to_string(),
            ));
        }

        if let Some(ref client_arc) = connection.client {
            tracing::debug!(
                "Attempting to fetch resources from rmcp client {}",
                connection_id
            );

            let peer = client_arc.peer();

            let request =
                rmcp::model::ListResourcesRequest::with_param(rmcp::model::PaginatedRequestParam {
                    cursor: None,
                });

            let start_time = std::time::Instant::now();
            let client_request: rmcp::model::ClientRequest = request.into();
            let server_result = peer
                .send_request(client_request)
                .await
                .map_err(|e| McpError::ServiceError(e.to_string()))?;
            let duration = start_time.elapsed();

            let result = match server_result {
                rmcp::model::ServerResult::ListResourcesResult(r) => r,
                _ => {
                    tracing::warn!("Unexpected response type from {}", connection_id);
                    rmcp::model::ListResourcesResult {
                        resources: Vec::new(),
                        next_cursor: None,
                        meta: None,
                    }
                }
            };

            tracing::debug!(
                "Successfully fetched {} resources from {} ({}ms)",
                result.resources.len(),
                connection_id,
                duration.as_millis()
            );

            for resource in &result.resources {
                let name = &resource.name;
                let description = &resource.description.as_deref().unwrap_or("No description");
                tracing::debug!("Resource: {} - {} ({})", name, description, resource.uri);
            }

            Ok(result.resources)
        } else {
            tracing::warn!("No client available for connection {}", connection_id);
            Ok(Vec::new())
        }
    }

    pub async fn list_prompts(&self, connection_id: &str) -> Result<Vec<rmcp::model::Prompt>> {
        let connections = self.connections.read().await;
        let connection = connections
            .get(connection_id)
            .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        if !connection.status.is_connected {
            return Err(McpError::ConnectionError(
                "Service not connected".to_string(),
            ));
        }

        if let Some(ref client_arc) = connection.client {
            tracing::debug!(
                "Attempting to fetch prompts from rmcp client {}",
                connection_id
            );

            let peer = client_arc.peer();

            let request =
                rmcp::model::ListPromptsRequest::with_param(rmcp::model::PaginatedRequestParam {
                    cursor: None,
                });

            let start_time = std::time::Instant::now();
            let client_request: rmcp::model::ClientRequest = request.into();
            let server_result = peer
                .send_request(client_request)
                .await
                .map_err(|e| McpError::ServiceError(e.to_string()))?;
            let duration = start_time.elapsed();

            let result = match server_result {
                rmcp::model::ServerResult::ListPromptsResult(r) => r,
                _ => {
                    tracing::warn!("Unexpected response type from {}", connection_id);
                    rmcp::model::ListPromptsResult {
                        prompts: Vec::new(),
                        next_cursor: None,
                        meta: None,
                    }
                }
            };

            tracing::debug!(
                "Successfully fetched {} prompts from {} ({}ms)",
                result.prompts.len(),
                connection_id,
                duration.as_millis()
            );

            for prompt in &result.prompts {
                tracing::debug!(
                    "Prompt: {} - {}",
                    prompt.name,
                    prompt
                        .description
                        .as_ref()
                        .unwrap_or(&"No description".to_string())
                );
            }

            Ok(result.prompts)
        } else {
            tracing::warn!("No client available for connection {}", connection_id);
            Ok(Vec::new())
        }
    }

    pub async fn read_resource(
        &self,
        connection_id: &str,
        uri: &str,
    ) -> Result<rmcp::model::ReadResourceResult> {
        let connections = self.connections.read().await;
        let connection = connections
            .get(connection_id)
            .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        if !connection.status.is_connected {
            return Err(McpError::ConnectionError(
                "Service not connected".to_string(),
            ));
        }

        if let Some(ref client_arc) = connection.client {
            tracing::debug!(
                "Attempting to read resource '{}' from rmcp client {}",
                uri,
                connection_id
            );

            let peer = client_arc.peer();

            let request =
                rmcp::model::Request::<_, _>::new(rmcp::model::ReadResourceRequestParam {
                    uri: uri.to_string(),
                });

            let start_time = std::time::Instant::now();
            // Send request via peer and get response
            let client_request: rmcp::model::ClientRequest = request.into();
            let server_result = peer
                .send_request(client_request)
                .await
                .map_err(|e| McpError::ServiceError(e.to_string()))?;
            let duration = start_time.elapsed();

            let result = match server_result {
                rmcp::model::ServerResult::ReadResourceResult(r) => r,
                _ => {
                    return Err(McpError::ServiceError(
                        "Unexpected response type".to_string(),
                    ));
                }
            };

            tracing::debug!(
                "Successfully read resource '{}' from {} ({}ms, {} contents)",
                uri,
                connection_id,
                duration.as_millis(),
                result.contents.len()
            );

            Ok(result)
        } else {
            Err(McpError::ServiceError(
                "No client available for connection".to_string(),
            ))
        }
    }

    pub async fn get_prompt(
        &self,
        connection_id: &str,
        name: &str,
        arguments: Option<HashMap<String, rmcp::model::PromptArgument>>,
    ) -> Result<rmcp::model::GetPromptResult> {
        let connections = self.connections.read().await;
        let connection = connections
            .get(connection_id)
            .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        if !connection.status.is_connected {
            return Err(McpError::ConnectionError(
                "Service not connected".to_string(),
            ));
        }

        if let Some(ref client_arc) = connection.client {
            tracing::debug!(
                "Attempting to get prompt '{}' from rmcp client {}",
                name,
                connection_id
            );

            let peer = client_arc.peer();

            let arguments_map = arguments
                .map(|args| {
                    args.into_iter()
                        .map(|(k, v)| (k, serde_json::json!(v)))
                        .collect::<serde_json::Map<_, _>>()
                })
                .unwrap_or_default();
            let request = rmcp::model::Request::<_, _>::new(rmcp::model::GetPromptRequestParam {
                name: name.to_string(),
                arguments: Some(arguments_map),
            });

            let start_time = std::time::Instant::now();
            // Send request via peer and get response
            let client_request: rmcp::model::ClientRequest = request.into();
            let server_result = peer
                .send_request(client_request)
                .await
                .map_err(|e| McpError::ServiceError(e.to_string()))?;
            let duration = start_time.elapsed();

            let result = match server_result {
                rmcp::model::ServerResult::GetPromptResult(r) => r,
                _ => {
                    return Err(McpError::ServiceError(
                        "Unexpected response type".to_string(),
                    ));
                }
            };

            tracing::debug!(
                "Successfully got prompt '{}' from {} ({}ms, {} messages)",
                name,
                connection_id,
                duration.as_millis(),
                result.messages.len()
            );

            Ok(result)
        } else {
            Err(McpError::ServiceError(
                "No client available for connection".to_string(),
            ))
        }
    }

    pub async fn call_tool(
        &self,
        connection_id: &str,
        name: &str,
        arguments: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<rmcp::model::CallToolResult> {
        let connections = self.connections.read().await;
        let connection = connections
            .get(connection_id)
            .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        if !connection.status.is_connected {
            return Err(McpError::ConnectionError(
                "Service not connected".to_string(),
            ));
        }

        if let Some(ref client_arc) = connection.client {
            tracing::debug!(
                "Attempting to call tool '{}' from rmcp client {}",
                name,
                connection_id
            );

            let peer = client_arc.peer();

            let arguments_map = arguments
                .map(|args| args.into_iter().collect::<serde_json::Map<_, _>>())
                .unwrap_or_default();
            let request = rmcp::model::Request::<_, _>::new(rmcp::model::CallToolRequestParam {
                name: name.to_string().into(),
                arguments: Some(arguments_map),
            });

            let start_time = std::time::Instant::now();
            // Send request via peer and get response
            let client_request: rmcp::model::ClientRequest = request.into();
            let server_result = peer
                .send_request(client_request)
                .await
                .map_err(|e| McpError::ServiceError(e.to_string()))?;
            let duration = start_time.elapsed();

            let result = match server_result {
                rmcp::model::ServerResult::CallToolResult(r) => r,
                _ => {
                    return Err(McpError::ServiceError(
                        "Unexpected response type".to_string(),
                    ));
                }
            };

            tracing::debug!(
                "Successfully called tool '{}' from {} ({}ms, {} content items)",
                name,
                connection_id,
                duration.as_millis(),
                result.content.len()
            );

            Ok(result)
        } else {
            Err(McpError::ServiceError(
                "No client available for connection".to_string(),
            ))
        }
    }

    pub async fn get_connections(&self) -> Vec<McpConnection> {
        self.connections.read().await.values().cloned().collect()
    }

    pub async fn get_connection_status(&self, service_id: &str) -> (String, Option<String>) {
        if let Some(connection) = self.connections.read().await.get(service_id) {
            if connection.status.is_connected {
                ("connected".to_string(), None)
            } else {
                // 如果有错误消息，说明连接失败，返回 failed 状态
                // 否则返回 disconnected 状态
                if connection.status.error_message.is_some() {
                    (
                        "failed".to_string(),
                        connection.status.error_message.clone(),
                    )
                } else {
                    ("disconnected".to_string(), None)
                }
            }
        } else {
            ("disconnected".to_string(), None)
        }
    }

    /// Disconnect a specific server, handling both STDIO and HTTP types properly
    pub async fn disconnect_server(&self, server_name: &str) -> Result<()> {
        let mut connections = self.connections.write().await;

        if let Some(mut connection) = connections.remove(server_name) {
            tracing::info!("Disconnecting server '{}'", server_name);

            // Close the MCP client if it exists
            if let Some(client) = connection.client.take() {
                // Drop the client to trigger cleanup
                drop(client);
                tracing::info!("Closed MCP client for server '{}'", server_name);
            }

            // Clear server info
            connection.server_info = None;

            // Update connection status
            connection.status.is_connected = false;
            connection.status.is_connecting = false;
            connection.status.error_message = Some("Disconnected by user".to_string());

            tracing::info!("Successfully disconnected server '{}'", server_name);
        } else {
            tracing::warn!("Server '{}' not found in active connections", server_name);
        }

        Ok(())
    }

    /// Try to reconnect to a specific service
    pub async fn try_reconnect(&self, service_config: &McpServerConfig) -> Result<bool> {
        let service_name = &service_config.name;

        // Remove any existing invalid connection
        {
            let mut connections = self.connections.write().await;
            if let Some(connection) = connections.get(service_name) {
                if !connection.status.is_connected {
                    connections.remove(service_name);
                }
            }
        }

        // Attempt to create a new connection
        match self.ensure_connection(service_config, true).await {
            Ok(connection) => {
                let is_connected = connection.status.is_connected;
                tracing::info!(
                    "Service '{}' reconnection attempt: {}",
                    service_name,
                    if is_connected { "success" } else { "failed" }
                );
                Ok(is_connected)
            }
            Err(e) => {
                tracing::error!("Service '{}' reconnection failed: {}", service_name, e);
                Ok(false)
            }
        }
    }

    /// 获取已连接服务器的版本信息
    pub async fn get_server_version(&self, server_name: &str) -> Option<String> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(server_name) {
            if connection.status.is_connected {
                if let Some(ref server_info) = connection.server_info {
                    return Some(server_info.server_info.version.clone());
                }
            }
        }
        None
    }
}
