use crate::config::AppConfig;
use crate::error::{McpError, Result};
use crate::types::{ConnectionStatus, McpConnection, McpServerConfig, McpService};
use reqwest::header;
use rmcp::model::Tool;
use rmcp::service::ServiceExt;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::transport::sse_client::SseClientConfig;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;

/// Helper function to create a reqwest client with logging for HTTP transport
fn create_http_reqwest_client(
    custom_headers: Option<&HashMap<String, String>>,
) -> Result<reqwest::Client> {
    let mut client_builder = reqwest::Client::builder()
        .user_agent("mcprouter/1.0")
        .timeout(Duration::from_secs(30));

    // Add default headers for HTTP transport
    let mut headers = header::HeaderMap::new();
    // 更宽松的Accept头，支持带charset的JSON响应
    headers.insert(
        "Accept",
        header::HeaderValue::from_static(
            "application/json, application/json; charset=utf-8, text/event-stream, */*",
        ),
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

/// Helper function to create a reqwest client with logging for SSE transport
fn create_sse_reqwest_client(
    custom_headers: Option<&HashMap<String, String>>,
) -> Result<reqwest::Client> {
    let mut client_builder = reqwest::Client::builder()
        .user_agent("mcprouter/1.0")
        .timeout(Duration::from_secs(30));

    // Add default headers for SSE
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(
        "Cache-Control",
        header::HeaderValue::from_static("no-cache"),
    );

    // Add custom headers if present
    if let Some(headers_map) = custom_headers {
        for (key, value) in headers_map {
            if let Ok(header_name) = header::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = header::HeaderValue::from_str(value) {
                    headers.insert(header_name, header_value);
                    tracing::debug!(
                        "Added SSE header: {}: {}",
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

#[derive(Clone)]
pub struct McpClientManager {
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    connection_cache_ttl: std::time::Duration,
}

impl McpClientManager {
    pub fn new(_config: AppConfig) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_cache_ttl: std::time::Duration::from_secs(300),
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
            crate::types::ServiceTransport::Sse => self.create_sse_connection(service_config).await,
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
                let error_message = Some(e.to_string());
                let failed_connection = McpConnection {
                    service_id: service_name.clone(),
                    server_info: None,
                    client: None,
                    status: ConnectionStatus {
                        is_connected: false,
                        is_connecting: false,
                        last_connected: Some(chrono::Utc::now()),
                        error_message: error_message.clone(),
                    },
                };
                self.connections
                    .write()
                    .await
                    .insert(service_name.clone(), failed_connection);
                Err(McpError::ConnectionError(error_message.unwrap_or_default()))
            }
        };

        connection
    }

    /// Create STDIO connection using rmcp 0.8.3
    async fn create_stdio_connection(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        let command = service_config.command.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("STDIO service requires command".to_string())
        })?;

        let args = service_config.args.as_ref().cloned().unwrap_or_default();
        let mut env_vars = service_config.env.clone().unwrap_or_default();

        // Load settings and apply environment configuration
        if let Ok(config) = crate::config::AppConfig::load() {
            if let Some(settings) = config.settings {
                if command.starts_with("uvx") || command.starts_with("uv") {
                    if let Some(uv_index_url) = settings.uv_index_url {
                        env_vars.insert("UV_INDEX_URL".to_string(), uv_index_url);
                    }
                } else if command.starts_with("npx") || command.starts_with("npm") {
                    if let Some(npm_registry) = settings.npm_registry {
                        env_vars.insert("NPM_CONFIG_REGISTRY".to_string(), npm_registry);
                    }
                }
            }
        }

        tracing::info!("Creating STDIO MCP service: {}", service_config.name);

        // Create transport
        let mut command_builder = Command::new(command);
        command_builder.args(&args);
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

    /// Create SSE connection using rmcp 0.8.3
    async fn create_sse_connection(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        let url = service_config.url.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("SSE service requires URL".to_string())
        })?;

        tracing::info!("Creating SSE MCP service: {}", url);

        // Create a custom reqwest client with authentication headers if provided
        let client = create_sse_reqwest_client(service_config.headers.as_ref())?;

        // Create SSE transport configuration
        let config = SseClientConfig {
            sse_endpoint: url.clone().into(),
            ..Default::default()
        };

        // Use the custom client with authentication headers
        let transport = rmcp::transport::SseClientTransport::start_with_client(client, config)
            .await
            .map_err(|e| McpError::ConnectionError(e.to_string()))?;

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
            client: Some(Arc::new(McpService::Sse(Arc::new(service)))),
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

        tracing::info!("Creating HTTP MCP service: {}", url);
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

                // Provide more helpful error messages for common issues
                if error_msg.contains("Unexpected content type") {
                    tracing::error!(
                        "Service '{}' returned unexpected content type. This may indicate:\n\
                        1. The URL is incorrect or not a valid MCP StreamableHttp endpoint\n\
                        2. The remote service is not properly configured for MCP StreamableHttp protocol\n\
                        3. The service may be using a different MCP transport (try SSE instead)\n\
                        Please verify the URL and service configuration.",
                        service_config.name
                    );
                }

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
            tracing::info!(
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
                    }
                }
            };

            tracing::info!(
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
                tracing::warn!("Service '{}' reconnection failed: {}", service_name, e);
                Ok(false)
            }
        }
    }
}
