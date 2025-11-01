use crate::config::AppConfig;
use crate::error::{McpError, Result};
use crate::http_client::HttpTransportConfig;
use crate::types::{
    ConnectionStatus, McpConnection, McpServerConfig, McpService, ServiceTransport,
};
use crate::SERVICE_MANAGER;
use rmcp::model::ClientInfo;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

// Import RMCP components
use rmcp::{
    model::ListToolsResult,
    service::ServiceExt,
    transport::{
        streamable_http_client::StreamableHttpClientTransport,
        SseClientTransport, TokioChildProcess,
    },
};

// Access global service manager for config via state manager

#[derive(Clone)]
pub struct McpClientManager {
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    // Connection management and caching
    connection_cache_ttl: std::time::Duration,
    // max_connection_attempts field removed as it was never read
}

impl McpClientManager {
    pub fn new(_config: AppConfig) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_cache_ttl: std::time::Duration::from_secs(300), // 5 minutes cache
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

    /// Smart connection management with caching
    pub async fn ensure_connection(
        &self,
        service_config: &McpServerConfig,
        force_refresh: bool,
    ) -> crate::error::Result<McpConnection> {
        let service_name = &service_config.name;

        // Check if we have a valid cached connection
        if !force_refresh && self.is_connection_cache_valid(service_name).await {
            tracing::debug!("Using cached connection for service: {}", service_name);
            if let Some(connection) = self.connections.read().await.get(service_name) {
                return Ok(connection.clone());
            }
        }

        // Need to establish new connection
        tracing::info!(
            "Establishing new connection to MCP service: {} ({:?})",
            service_name,
            service_config.transport
        );

        let connection = match service_config.transport {
            ServiceTransport::Stdio => self.connect_stdio_service(service_config).await?,
            ServiceTransport::Sse => self.connect_sse_service(service_config).await?,
            ServiceTransport::Http => self.connect_http_service(service_config).await?,
        };

        // Update connection cache
        self.connections
            .write()
            .await
            .insert(service_name.clone(), connection.clone());

        Ok(connection)
    }

    async fn connect_stdio_service(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        tracing::debug!(
            "Starting STDIO process for service: {}",
            service_config.name
        );

        let command_str = service_config.command.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("STDIO service requires command".to_string())
        })?;
        let mut command = Command::new(command_str);
        // If running via npx, append --registry from global settings unless already provided
        if command_str == "npx" {
            let global_config_for_registry = SERVICE_MANAGER.get_config().await;
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
        let global_config = SERVICE_MANAGER.get_config().await;
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
        // Create RMCP client using TokioChildProcess transport
        let transport = TokioChildProcess::new(command)
            .map_err(|e| McpError::ProcessError(format!("Failed to create transport: {}", e)))?;

        let client_service = ClientInfo::default();
        let service = client_service.serve(transport).await.map_err(|e| {
            McpError::ConnectionError(format!("Failed to connect to service: {}", e))
        })?;

        // Get server information
        let server_info_value = service.peer_info();
        let server_info = serde_json::to_value(server_info_value).map_err(|e| {
            McpError::SerializationError(format!("Failed to serialize server info: {}", e))
        })?;

        tracing::debug!("Connected to MCP server: {:?}", server_info);

        let version = extract_version_from_server_info(&server_info);
        Ok(McpConnection {
            service_id: service_config.name.clone(),
            server_info: Some(server_info),
            client: Some(Arc::new(McpService::Stdio(service))),
            status: ConnectionStatus {
                is_connected: true,
                last_connected: Some(chrono::Utc::now()),
                error_message: None,
            },
            cached_version: version,
        })
    }

    async fn connect_sse_service(&self, service_config: &McpServerConfig) -> Result<McpConnection> {
        let url = service_config.url.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("SSE service requires a URL".to_string())
        })?;

        tracing::info!("Connecting to SSE service at: {}", url);

        // Create SSE client transport using the convenience start method
        let transport = SseClientTransport::start(url.clone()).await.map_err(|e| {
            McpError::ConnectionError(format!("Failed to create SSE transport: {}", e))
        })?;

        let client_info = ClientInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: rmcp::model::ClientCapabilities::default(),
            client_info: rmcp::model::Implementation {
                name: "mcprouter".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
        };

        // Connect to the service
        let service = client_info.serve(transport).await.map_err(|e| {
            McpError::ConnectionError(format!("Failed to connect to SSE service: {}", e))
        })?;

        // Get server information
        let server_info_value = service.peer_info();
        let server_info = serde_json::to_value(server_info_value).map_err(|e| {
            McpError::SerializationError(format!("Failed to serialize server info: {}", e))
        })?;

        tracing::debug!("Connected to SSE MCP server: {:?}", server_info);

        let version = extract_version_from_server_info(&server_info);
        Ok(McpConnection {
            service_id: service_config.name.clone(),
            server_info: Some(server_info),
            client: Some(Arc::new(McpService::Sse(service))),
            status: ConnectionStatus {
                is_connected: true,
                last_connected: Some(chrono::Utc::now()),
                error_message: None,
            },
            cached_version: version,
        })
    }

    async fn connect_http_service(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        let base_url = service_config.url.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("HTTP service requires a URL".to_string())
        })?;

        tracing::info!("Connecting to HTTP service at: {}", base_url);

        // Create transport configuration using our custom HttpTransportConfig
        let mut transport_config = HttpTransportConfig::new(base_url);

        // Set custom headers if present
        if let Some(headers) = &service_config.headers {
            tracing::debug!("Using custom headers for HTTP service: {:?}", headers);
            transport_config = transport_config.headers(headers);
        } else {
            tracing::debug!("No custom headers configured for HTTP service");
        }

        // Build the actual RMCP transport configuration
        let config = transport_config.build_config()?;

        // Create transport with configuration
        let transport = StreamableHttpClientTransport::from_config(config);

        let client_info = ClientInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: rmcp::model::ClientCapabilities::default(),
            client_info: rmcp::model::Implementation {
                name: "mcprouter".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
        };

        let service = client_info.serve(transport).await.map_err(|e| {
            McpError::ConnectionError(format!("Failed to connect to HTTP service: {}", e))
        })?;

        // Get server information
        let server_info_value = service.peer_info();
        let server_info = serde_json::to_value(server_info_value).map_err(|e| {
            McpError::SerializationError(format!("Failed to serialize server info: {}", e))
        })?;

        tracing::debug!("Connected to HTTP MCP server: {:?}", server_info);

        let version = extract_version_from_server_info(&server_info);
        Ok(McpConnection {
            service_id: service_config.name.clone(),
            server_info: Some(server_info),
            client: Some(Arc::new(McpService::Http(service))),
            status: ConnectionStatus {
                is_connected: true,
                last_connected: Some(chrono::Utc::now()),
                error_message: None,
            },
            cached_version: version,
        })
    }

    pub async fn disconnect_mcp_server(&self, service_id: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.remove(service_id) {
            // If there's an RMCP service, try to close it gracefully
            if let Some(service) = connection.client {
                // Use Arc::strong_count to check if we're the only reference
                if Arc::strong_count(&service) == 1 {
                    // We're the only owner, try to unwrap and cancel
                    match Arc::try_unwrap(service) {
                        Ok(service) => match service {
                            McpService::Stdio(service) => {
                                if let Err(e) = service.cancel().await {
                                    tracing::warn!("Failed to cancel STDIO service: {}", e);
                                }
                            }
                            McpService::Sse(service) => {
                                if let Err(e) = service.cancel().await {
                                    tracing::warn!("Failed to cancel SSE service: {}", e);
                                }
                            }
                            McpService::Http(service) => {
                                if let Err(e) = service.cancel().await {
                                    tracing::warn!("Failed to cancel HTTP service: {}", e);
                                }
                            }
                        },
                        Err(_) => {
                            // This shouldn't happen since we checked strong_count
                            tracing::debug!(
                                "Unexpected: could not unwrap service even with strong_count == 1"
                            );
                        }
                    }
                } else {
                    // There are other references to this service, so we can't cancel it gracefully
                    // The service will be cleaned up when all references are dropped
                    tracing::debug!(
                        "Service {} has {} active references, will be cleaned up when all references are dropped",
                        service_id,
                        Arc::strong_count(&service)
                    );
                }
            }
            tracing::debug!("Disconnected from service: {}", service_id);
        }
        Ok(())
    }

    pub async fn list_tools(&self, connection_id: &str) -> Result<Vec<crate::McpTool>> {
        let connections = self.connections.read().await;
        let connection = connections
            .get(connection_id)
            .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        // Get the RMCP service from the connection
        let service = connection
            .client
            .as_ref()
            .ok_or_else(|| McpError::ConnectionError("Service not connected".to_string()))?;

        // Match on the service type using the enum
        let tools_result = match &**service {
            McpService::Stdio(_) => self.list_tools_on_service(connection_id).await,
            McpService::Sse(_) => self.list_tools_on_service(connection_id).await,
            McpService::Http(_) => self.list_tools_on_service(connection_id).await,
        }?;

        // Return RMCP tools directly
        Ok(tools_result.tools)
    }

    pub async fn list_tools_on_service(&self, service_id: &str) -> Result<ListToolsResult> {
        let connection = self
            .get_connection(service_id)
            .await
            .ok_or_else(|| McpError::ServiceNotFound(service_id.to_string()))?;

        // Get the RMCP service from the connection
        let service = connection
            .client
            .as_ref()
            .ok_or_else(|| McpError::ConnectionError("Service not connected".to_string()))?;

        // Match on the service type using the enum
        match &**service {
            McpService::Stdio(service) => {
                let tools_result = service
                    .list_tools(Default::default())
                    .await
                    .map_err(|e| McpError::ToolError(format!("Failed to list tools: {}", e)))?;
                Ok(tools_result)
            }
            McpService::Sse(service) => {
                let tools_result = service
                    .list_tools(Default::default())
                    .await
                    .map_err(|e| McpError::ToolError(format!("Failed to list tools: {}", e)))?;
                Ok(tools_result)
            }
            McpService::Http(service) => {
                let tools_result = service
                    .list_tools(Default::default())
                    .await
                    .map_err(|e| McpError::ToolError(format!("Failed to list tools: {}", e)))?;
                Ok(tools_result)
            }
        }
    }

    /// Manually set connection error for a service (used when connection fails but we want to track the error)
    pub async fn set_connection_error(&self, service_id: &str, error_message: String) {
        let error_connection = McpConnection {
            service_id: service_id.to_string(),
            server_info: None,
            client: None,
            status: ConnectionStatus {
                is_connected: false,
                last_connected: None,
                error_message: Some(error_message),
            },
            cached_version: None,
        };

        self.connections
            .write()
            .await
            .insert(service_id.to_string(), error_connection);
    }

    pub async fn get_connections(&self) -> Vec<McpConnection> {
        self.connections.read().await.values().cloned().collect()
    }

    pub async fn is_connected(&self, service_id: &str) -> bool {
        self.connections.read().await.contains_key(service_id)
    }

    pub async fn get_connection_status(&self, service_id: &str) -> (String, Option<String>) {
        if let Some(connection) = self.connections.read().await.get(service_id) {
            if connection.status.is_connected {
                ("connected".to_string(), None)
            } else {
                (
                    "disconnected".to_string(),
                    connection.status.error_message.clone(),
                )
            }
        } else {
            ("disconnected".to_string(), None)
        }
    }

    pub async fn get_connection(&self, service_id: &str) -> Option<McpConnection> {
        self.connections.read().await.get(service_id).cloned()
    }

    /// Get cached version for a service without connecting
    pub async fn get_cached_version(&self, service_name: &str) -> Option<String> {
        if let Some(connection) = self.connections.read().await.get(service_name) {
            if self.is_connection_cache_valid(service_name).await {
                return connection.cached_version.clone();
            }
        }
        None
    }

    /// Batch health check for multiple services with connection reuse and concurrency control
    pub async fn batch_health_check(&self, service_names: &[String]) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        let mut services_to_check = Vec::new();

        // First, check cached connections
        for service_name in service_names {
            if self.is_connection_cache_valid(service_name).await {
                results.insert(service_name.clone(), true);
                tracing::debug!("Service {} is healthy (cached)", service_name);
            } else {
                services_to_check.push(service_name.clone());
            }
        }

        // Check services that need validation with controlled concurrency
        if !services_to_check.is_empty() {
            tracing::info!(
                "Checking health for {} services with controlled concurrency",
                services_to_check.len()
            );

            // Use a semaphore to limit concurrent connections
            let semaphore = Arc::new(tokio::sync::Semaphore::new(3)); // Max 3 concurrent connections
            let mut tasks = Vec::new();

            for service_name in services_to_check {
                let semaphore_clone = semaphore.clone();
                let manager = self.clone();

                let task = tokio::spawn(async move {
                    let _permit = semaphore_clone.acquire().await.unwrap();

                    match manager.get_service_config(&service_name).await {
                        Some(config) => {
                            // Don't force refresh connection during batch health check
                            // This prevents unnecessary new connections when we already have one
                            match manager.ensure_connection(&config, false).await {
                                Ok(_) => {
                                    tracing::debug!(
                                        "Service {} is healthy (checked)",
                                        service_name
                                    );
                                    (service_name, true)
                                }
                                Err(e) => {
                                    tracing::debug!("Service {} is unhealthy: {}", service_name, e);
                                    (service_name, false)
                                }
                            }
                        }
                        None => {
                            tracing::warn!("Service configuration not found: {}", service_name);
                            (service_name, false)
                        }
                    }
                });

                tasks.push(task);
            }

            // Wait for all health checks to complete
            for task in tasks {
                if let Ok((service_name, is_healthy)) = task.await {
                    results.insert(service_name, is_healthy);
                }
            }
        }

        results
    }

    /// Get service configuration from service manager
    async fn get_service_config(&self, service_name: &str) -> Option<McpServerConfig> {
        // Access the service manager to get the config
        let services = SERVICE_MANAGER.get_mcp_servers().await;
        let services_read = services.read().await;
        services_read.get(service_name).cloned()
    }

    // update_connection_cache method removed as it was never used

    // get_server_info method removed as it was never used
}

/// Extract version information from server info JSON
fn extract_version_from_server_info(server_info: &serde_json::Value) -> Option<String> {
    if let Some(server_info_field) = server_info.get("serverInfo") {
        if let Some(version) = server_info_field.get("version").and_then(|v| v.as_str()) {
            return Some(version.to_string());
        }
    }
    None
}
