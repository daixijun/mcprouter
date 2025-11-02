use crate::config::AppConfig;
use crate::error::{McpError, Result};
use crate::types::{ConnectionStatus, McpConnection, McpServerConfig};
use crate::SERVICE_MANAGER;
use rust_mcp_sdk::schema::ListToolsResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct McpClientManager {
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    connection_cache_ttl: std::time::Duration,
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
    /// Note: This creates a connection tracking entry, actual MCP connection
    /// is managed by McpServerManager
    pub async fn ensure_connection(
        &self,
        service_config: &McpServerConfig,
        force_refresh: bool,
    ) -> crate::error::Result<McpConnection> {
        let service_name = &service_config.name;

        if !force_refresh && self.is_connection_cache_valid(service_name).await {
            tracing::debug!("Using cached connection for service: {}", service_name);
            if let Some(connection) = self.connections.read().await.get(service_name) {
                return Ok(connection.clone());
            }
        }

        // Need to establish new connection tracking
        tracing::info!(
            "Establishing connection tracking for MCP service: {} ({:?})",
            service_name,
            service_config.transport
        );

        // Create connection tracking entry (actual connection handled by McpServerManager)
        let connection = match service_config.transport {
            crate::types::ServiceTransport::Stdio => {
                self.create_stdio_connection(service_config).await?
            }
            crate::types::ServiceTransport::Sse => {
                self.create_sse_connection(service_config).await?
            }
            crate::types::ServiceTransport::Http => {
                self.create_http_connection(service_config).await?
            }
        };

        // Update connection cache
        self.connections
            .write()
            .await
            .insert(service_name.clone(), connection.clone());

        Ok(connection)
    }

    /// Create STDIO connection tracking entry
    async fn create_stdio_connection(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        let command = service_config.command.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("STDIO service requires command".to_string())
        })?;

        let args = service_config.args.as_ref().cloned().unwrap_or_default();

        tracing::info!(
            "Creating STDIO connection tracking for '{}': {} {:?}",
            service_config.name,
            command,
            args
        );

        Ok(McpConnection {
            service_id: service_config.name.clone(),
            server_info: None,
            client: None,
            status: ConnectionStatus {
                is_connected: true,
                last_connected: Some(chrono::Utc::now()),
                error_message: None,
            },
            cached_version: None,
        })
    }

    /// Create SSE connection tracking entry
    async fn create_sse_connection(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        let url = service_config.url.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("SSE service requires a URL".to_string())
        })?;

        tracing::info!(
            "Creating SSE connection tracking for '{}' at: {}",
            service_config.name,
            url
        );

        Ok(McpConnection {
            service_id: service_config.name.clone(),
            server_info: None,
            client: None,
            status: ConnectionStatus {
                is_connected: true,
                last_connected: Some(chrono::Utc::now()),
                error_message: None,
            },
            cached_version: None,
        })
    }

    /// Create HTTP connection tracking entry
    async fn create_http_connection(
        &self,
        service_config: &McpServerConfig,
    ) -> Result<McpConnection> {
        let base_url = service_config.url.as_ref().ok_or_else(|| {
            McpError::InvalidConfiguration("HTTP service requires a URL".to_string())
        })?;

        tracing::info!(
            "Creating HTTP connection tracking for '{}' at: {}",
            service_config.name,
            base_url
        );

        Ok(McpConnection {
            service_id: service_config.name.clone(),
            server_info: None,
            client: None,
            status: ConnectionStatus {
                is_connected: true,
                last_connected: Some(chrono::Utc::now()),
                error_message: None,
            },
            cached_version: None,
        })
    }

    /// Update connection with real server info and version
    /// This should be called after successful connection
    pub async fn update_connection_info(
        &self,
        service_name: &str,
        server_info: Option<serde_json::Value>,
        version: Option<&String>,
    ) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(service_name) {
            connection.server_info = server_info;
            connection.cached_version = version.cloned();
            connection.status.last_connected = Some(chrono::Utc::now());
            connection.status.is_connected = true;
            connection.status.error_message = None;

            tracing::debug!(
                "Updated connection info for {}: version={:?}",
                service_name,
                version
            );
        }
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

    pub async fn list_tools(&self, connection_id: &str) -> Result<Vec<crate::McpTool>> {
        let connections = self.connections.read().await;
        let connection = connections
            .get(connection_id)
            .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        // Check if connection is marked as connected
        if !connection.status.is_connected {
            return Err(McpError::ConnectionError(
                "Service not connected".to_string(),
            ));
        }

        // Get tools from database (populated by McpServerManager's background tasks)
        // TODO: migrate to config - // TODO: migrate - use crate::db::repositories::mcp_server_repository::McpServerRepository;
        // TODO: migrate to config - // TODO: migrate - use crate::db::repositories::tool_repository::ToolRepository;

        // Get server by name
        // TODO: 迁移到配置文件后重新实现
        // let server = McpServerRepository::get_by_name(connection_id).await
        //     .map_err(|e| {
        //         tracing::error!("Failed to get server {}: {}", connection_id, e);
        //         McpError::ProcessError(format!("Failed to get server {}: {}", connection_id, e))
        //     })?
        //     .ok_or_else(|| McpError::ServiceNotFound(connection_id.to_string()))?;

        // let server_id = server.id.unwrap_or_default();

        // Get all tools for this server
        // TODO: 迁移到配置文件后重新实现
        // let tool_rows = ToolRepository::get_by_server_id(&server_id).await
        //     .map_err(|e| {
        //         tracing::error!("Failed to get tools for service {}: {}", connection_id, e);
        //         McpError::ProcessError(format!(
        //             "Failed to get tools for service {}: {}",
        //             connection_id, e
        //         ))
        //     })?;

        // Convert ToolRow to Tool
        // TODO: 临时返回空列表
        let tools: Vec<crate::McpTool> = Vec::new();

        tracing::debug!(
            "Retrieved {} tools for service {}",
            tools.len(),
            connection_id
        );
        Ok(tools)
    }

    pub async fn _list_tools_on_service(&self, service_id: &str) -> Result<ListToolsResult> {
        // Get tools from configuration/cached connection
        let tools = self.list_tools(service_id).await?;

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    /// Manually set connection error for a service
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

    pub async fn _get_connection(&self, service_id: &str) -> Option<McpConnection> {
        self.connections.read().await.get(service_id).cloned()
    }

    /// Get cached version for a service without connecting
    pub async fn get_cached_version(&self, service_name: &str) -> Option<String> {
        // First try to get from memory cache if valid
        if let Some(connection) = self.connections.read().await.get(service_name) {
            if self.is_connection_cache_valid(service_name).await {
                return connection.cached_version.clone();
            }
        }

        // Fallback to configuration file if cache is invalid
        tracing::debug!(
            "Memory cache expired for service {}, checking configuration file for version",
            service_name
        );

        // Check version from loaded MCP servers configuration
        // Note: This would require access to McpServerRepository
        // For now, return None and rely on connection check for version detection
        None
    }

    /// Batch health check for multiple services
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
}
