use crate::config::{AppConfig, McpServerConfig};
use crate::db::models::{McpServerRow, ToolRow};
use crate::db::repositories::mcp_server_repository::McpServerRepository;
use crate::db::repositories::tool_repository::ToolRepository;
use crate::error::{McpError, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub enabled: bool,
    pub is_active: bool,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub version: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
}

// 合并后的响应结构体，包含状态和配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub enabled: bool,
    pub is_active: bool,
    pub version: Option<String>,
    pub start_time: Option<String>,
    pub last_error: Option<String>,
    pub transport: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub env_vars: Option<HashMap<String, String>>,
    pub headers: Option<HashMap<String, String>>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub tool_count: Option<usize>,
}

#[derive(Clone)]
pub struct McpServerManager {
    mcp_servers: Arc<RwLock<HashMap<String, McpServerConfig>>>,
    config: Arc<RwLock<AppConfig>>,
    startup_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub version_cache: Arc<RwLock<HashMap<String, ServiceVersionCache>>>,
    loading_lock: Arc<Mutex<bool>>, // Prevent concurrent loading
}

#[derive(Clone)]
pub struct ServiceVersionCache {
    pub version: Option<String>,
    // last_updated field removed as it was never read
}

impl McpServerManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            mcp_servers: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
            startup_tasks: Arc::new(Mutex::new(HashMap::new())),
            version_cache: Arc::new(RwLock::new(HashMap::new())),
            loading_lock: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn get_config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    pub async fn load_mcp_servers(&self) -> Result<()> {
        // Prevent concurrent loading
        {
            let mut loading = self.loading_lock.lock().unwrap();
            if *loading {
                tracing::debug!("MCP servers loading already in progress, skipping");
                return Ok(());
            }
            *loading = true;
        }

        // Ensure we release the lock even if there's an error
        let _guard = scopeguard::guard((), |_| {
            if let Ok(mut loading) = self.loading_lock.lock() {
                *loading = false;
            }
        });

        // Cancel any existing startup tasks first
        if let Ok(mut tasks) = self.startup_tasks.lock() {
            for (_, handle) in tasks.drain() {
                handle.abort();
            }
        }

        let mut mcp_servers = self.mcp_servers.write().await;
        mcp_servers.clear();

        // Load from database instead of config file
        let servers_from_db = McpServerRepository::get_all().await?;
        for server_row in servers_from_db {
            let service_config = self.db_row_to_config(&server_row)?;
            mcp_servers.insert(service_config.name.clone(), service_config);
        }

        // Auto-check enabled services for connectivity
        let enabled_services: Vec<_> = mcp_servers
            .iter()
            .filter(|(_, s)| s.enabled)
            .map(|(name, _)| name.clone())
            .collect();

        // Drop the lock before spawning tasks
        drop(mcp_servers);

        // Use batch health check with connection reuse and caching
        if !enabled_services.is_empty() {
            let manager = self.clone();
            let enabled_services_clone = enabled_services.clone();
            let handle = tokio::spawn(async move {
                use super::MCP_CLIENT_MANAGER;

                tracing::info!(
                    "Performing batch health check for {} enabled services",
                    enabled_services_clone.len()
                );

                // Batch check with connection reuse
                let health_results = MCP_CLIENT_MANAGER
                    .batch_health_check(&enabled_services_clone)
                    .await;

                for (service_name, is_healthy) in health_results {
                    if is_healthy {
                        // Prefer logging with concrete version if available
                        if let Some(cached_version) =
                            MCP_CLIENT_MANAGER.get_cached_version(&service_name).await
                        {
                            let mut version_cache = manager.version_cache.write().await;
                            version_cache.insert(
                                service_name.clone(),
                                ServiceVersionCache {
                                    version: Some(cached_version.clone()),
                                },
                            );
                            tracing::debug!("Synced version cache for service {}", service_name);
                            tracing::info!(
                                "Service {} is active, version {}",
                                service_name,
                                cached_version
                            );
                        } else {
                            tracing::info!("Service {} is active", service_name);
                        }

                        // 后台获取工具清单（总是尝试获取以刷新缓存）
                        if let Some(service_config) =
                            manager.mcp_servers.read().await.get(&service_name).cloned()
                        {
                            manager.background_fetch_service_tools(
                                service_name.clone(),
                                service_config,
                            );
                        }
                    } else {
                        tracing::debug!("Service {} health check failed", service_name);
                    }
                }

                // Remove the task from startup_tasks when completed
                if let Ok(mut tasks) = manager.startup_tasks.lock() {
                    for name in enabled_services_clone {
                        tasks.remove(&name);
                    }
                }
            });

            // Store the task handle for tracking
            if let Ok(mut tasks) = self.startup_tasks.lock() {
                tasks.insert("batch_health_check".to_string(), handle);
            }
        }

        Ok(())
    }

    /// Smart service check using cached connection information
    pub async fn check_service_with_version(&self, name: &str) -> Result<()> {
        use super::MCP_CLIENT_MANAGER;

        let services = self.mcp_servers.read().await;
        let service_config = services
            .get(name)
            .ok_or_else(|| McpError::ServiceNotFound(name.to_string()))?
            .clone();
        drop(services);

        tracing::debug!("Checking service {} with smart connection management", name);

        // First, try to get cached version
        if let Some(cached_version) = MCP_CLIENT_MANAGER.get_cached_version(name).await {
            tracing::debug!(
                "Using cached version for service {}: {}",
                name,
                cached_version
            );

            // Update local version cache
            let mut version_cache = self.version_cache.write().await;
            version_cache.insert(
                name.to_string(),
                ServiceVersionCache {
                    version: Some(cached_version.clone()),
                },
            );
            return Ok(());
        }

        // Need to establish connection to get version info
        // Only force refresh if we don't have a valid cached connection
        let force_refresh = !MCP_CLIENT_MANAGER.is_connected(name).await;
        match MCP_CLIENT_MANAGER
            .ensure_connection(&service_config, force_refresh)
            .await
        {
            Ok(connection) => {
                tracing::debug!(
                    "Successfully connected to service {} for version check",
                    name
                );

                // Update version information from connection
                if let Some(ref version) = connection.cached_version {
                    let mut version_cache = self.version_cache.write().await;
                    version_cache.insert(
                        name.to_string(),
                        ServiceVersionCache {
                            version: Some(version.clone()),
                        },
                    );
                    tracing::debug!("Updated version cache for service {} to {}", name, version);
                }

                // Disconnect immediately (this was just for version retrieval and connectivity check)
                let _ = MCP_CLIENT_MANAGER
                    .disconnect_mcp_server(&connection.service_id)
                    .await;

                Ok(())
            }
            Err(e) => {
                tracing::debug!(
                    "Failed to connect to service {} for version check: {}",
                    name,
                    e
                );
                Err(e)
            }
        }
    }

    pub async fn add_mcp_server(&self, service_config: McpServerConfig) -> Result<()> {
        let mut services = self.mcp_servers.write().await;

        if services.contains_key(&service_config.name) {
            return Err(McpError::ServiceAlreadyExists(service_config.name.clone()));
        }

        let service_name = service_config.name.clone();
        let is_enabled = service_config.enabled;

        // Convert config to database row and save to database
        let server_row = self.config_to_db_row(&service_config)?;
        McpServerRepository::create(server_row).await?;

        services.insert(service_config.name.clone(), service_config.clone());
        drop(services);

        // 如果服务已启用，在后台获取工具清单
        if is_enabled {
            tracing::info!(
                "New enabled service added, fetching tools in background: {}",
                service_name
            );
            self.background_fetch_service_tools(service_name, service_config);
        }

        Ok(())
    }

    pub async fn remove_mcp_server(&self, name: &str) -> Result<()> {
        let mut services = self.mcp_servers.write().await;

        if !services.contains_key(name) {
            return Err(McpError::ServiceNotFound(name.to_string()));
        }

        services.remove(name);

        // Delete from database (cascade delete will handle related tools)
        McpServerRepository::delete(name).await?;

        // 记录日志
        tracing::info!("Service removed: {}", name);

        // 清理内存中的版本缓存
        {
            let mut version_cache = self.version_cache.write().await;
            version_cache.remove(name);
        }

        Ok(())
    }

    // Toggle service enabled/disabled state
    pub async fn toggle_mcp_server(&self, name: &str) -> Result<bool> {
        let mut services = self.mcp_servers.write().await;

        if let Some(service) = services.get_mut(name) {
            service.enabled = !service.enabled;
            let new_state = service.enabled;
            let service_config_clone = service.clone();

            // Update database
            McpServerRepository::toggle_enabled(name, new_state).await?;

            // If service is now enabled, check its connectivity, version, and refresh tools
            if new_state {
                let manager = self.clone();
                let name_clone = name.to_string();
                tokio::spawn(async move {
                    use super::MCP_CLIENT_MANAGER;

                    // First try to get cached info
                    if let Some(cached_version) =
                        MCP_CLIENT_MANAGER.get_cached_version(&name_clone).await
                    {
                        tracing::info!(
                            "Service {} is active (cached), version {}",
                            name_clone,
                            cached_version
                        );

                        // Sync version cache from MCP_CLIENT_MANAGER to local cache
                        let mut version_cache = manager.version_cache.write().await;
                        version_cache.insert(
                            name_clone.clone(),
                            ServiceVersionCache {
                                version: Some(cached_version),
                            },
                        );
                        return;
                    }

                    // Fall back to full check if no cache
                    match manager.check_service_with_version(&name_clone).await {
                        Ok(_) => {
                            // Try read version from local cache, fallback to client cache
                            let version_from_cache = {
                                let cache = manager.version_cache.read().await;
                                cache.get(&name_clone).and_then(|c| c.version.clone())
                            };

                            if let Some(version) = version_from_cache {
                                tracing::info!(
                                    "Service {} is active, version {}",
                                    name_clone,
                                    version
                                );
                            } else if let Some(version) =
                                MCP_CLIENT_MANAGER.get_cached_version(&name_clone).await
                            {
                                tracing::info!(
                                    "Service {} is active, version {}",
                                    name_clone,
                                    version
                                );
                            } else {
                                tracing::info!("Service {} is active", name_clone);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Service {} check failed: {}", name_clone, e);
                        }
                    }
                });

                // Kick off background tool refresh and DB sync
                self.background_fetch_service_tools(name.to_string(), service_config_clone);
            }

            Ok(new_state)
        } else {
            Err(McpError::ServiceNotFound(name.to_string()))
        }
    }

    async fn get_mcp_server_status(&self, name: &str) -> Result<ServiceStatus> {
        let services = self.mcp_servers.read().await;
        let service_config = services
            .get(name)
            .ok_or_else(|| McpError::ServiceNotFound(name.to_string()))?
            .clone();

        // Get version from cache first
        let version_from_cache = {
            let version_cache = self.version_cache.read().await;
            version_cache
                .get(name)
                .and_then(|cache| cache.version.clone())
        };

        // Use MCP_CLIENT_MANAGER to get actual connection status
        use super::MCP_CLIENT_MANAGER;
        let is_active = MCP_CLIENT_MANAGER.is_connected(name).await;

        tracing::debug!(
            "Service {} status: enabled={}, is_active={}, version={:?}",
            name,
            service_config.enabled,
            is_active,
            version_from_cache
        );

        Ok(ServiceStatus {
            name: service_config.name.clone(),
            enabled: service_config.enabled,
            is_active,
            pid: None,
            port: None,
            version: version_from_cache,
            start_time: None,
        })
    }

    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>> {
        let services = self.mcp_servers.read().await;
        let mut server_infos = Vec::new();

        for (name, config) in services.iter() {
            if let Ok(status) = self.get_mcp_server_status(name).await {
                // Query tool count from database
                let tool_count = match crate::db::get_database().await {
                    Ok(db) => {
                        // Get tools count for this server from database
                        match sqlx::query("SELECT COUNT(*) as count FROM tools WHERE server_id = (SELECT id FROM mcp_servers WHERE name = ?)")
                            .bind(name)
                            .fetch_one(&db)
                            .await
                        {
                            Ok(row) => {
                                let count: i64 = row.get("count");
                                tracing::debug!("Service {} has {} tools in database", name, count);
                                Some(count as usize)
                            }
                            Err(e) => {
                                tracing::error!("Failed to get tool count for service {}: {}", name, e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get database connection: {}", e);
                        None
                    }
                };

                let server_info = McpServerInfo {
                    name: status.name,
                    enabled: status.enabled,
                    is_active: status.is_active,
                    version: status.version,
                    start_time: status.start_time.map(|dt| dt.to_rfc3339()),
                    last_error: None, // TODO: 从状态中获取错误信息
                    transport: match config.transport {
                        crate::config::ServiceTransport::Stdio => "stdio".to_string(),
                        crate::config::ServiceTransport::Sse => "sse".to_string(),
                        crate::config::ServiceTransport::StreamableHttp => {
                            "streamablehttp".to_string()
                        }
                    },
                    url: config.url.clone(),
                    description: config.description.clone(),
                    env_vars: config.env_vars.clone(),
                    headers: config.headers.clone(),
                    command: match config.transport {
                        crate::config::ServiceTransport::Stdio => config.command.clone(),
                        _ => None,
                    },
                    args: match config.transport {
                        crate::config::ServiceTransport::Stdio => config.args.clone(),
                        _ => None,
                    },
                    tool_count: tool_count,
                };
                server_infos.push(server_info);
            }
        }

        Ok(server_infos)
    }

    pub async fn get_mcp_servers(&self) -> Arc<RwLock<HashMap<String, McpServerConfig>>> {
        self.mcp_servers.clone()
    }
}

impl McpServerManager {
    // Generic atomic config update: apply mutation under write-lock then persist
    pub async fn update_config<F>(&self, update_fn: F) -> crate::error::Result<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write().await;
        update_fn(&mut config);
        config.save()
    }

    /// 在后台异步获取并缓存服务工具（不阻塞调用者）
    fn background_fetch_service_tools(&self, service_name: String, config: McpServerConfig) {
        let manager = self.clone();
        tokio::spawn(async move {
            tracing::info!("Background fetching tools for service: {}", service_name);
            match manager
                .fetch_and_cache_service_tools(&service_name, &config)
                .await
            {
                Ok(tools) => {
                    tracing::info!(
                        "Successfully cached {} tools for service {} in background",
                        tools.len(),
                        service_name
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Background tool fetch failed for service {}: {}",
                        service_name,
                        e
                    );
                }
            }
        });
    }

    pub async fn update_version_cache(&self, name: &str, version: Option<String>) {
        let mut cache = self.version_cache.write().await;
        cache.insert(
            name.to_string(),
            ServiceVersionCache {
                version,
            },
        );
        tracing::debug!("Updated version cache via API for {}", name);
    }

    async fn fetch_and_cache_service_tools(
        &self,
        service_name: &str,
        config: &McpServerConfig,
    ) -> Result<Vec<crate::McpTool>> {
        use super::MCP_CLIENT_MANAGER;

        tracing::info!("Fetching tools for service: {}", service_name);

        // 尝试连接到服务
        let connection = match MCP_CLIENT_MANAGER.ensure_connection(config, false).await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to connect to service {}: {}", service_name, e);
                return Err(e);
            }
        };

        // 在建立连接后同步版本缓存，并写入数据库
        if let Some(version) = connection.cached_version.clone() {
            {
                let mut cache = self.version_cache.write().await;
                cache.insert(
                    service_name.to_string(),
                    ServiceVersionCache {
                        version: Some(version.clone()),
                    },
                );
            }
            tracing::info!("Synced version cache for {} => {}", service_name, version);
            if let Err(e) = McpServerRepository::update_version(service_name, Some(version.clone())).await {
                tracing::warn!("Failed to persist version for {}: {}", service_name, e);
            }
        }

        // 获取工具列表
        let mcp_tools = match MCP_CLIENT_MANAGER.list_tools(service_name).await {
            Ok(tools) => tools,
            Err(e) => {
                tracing::warn!("Failed to list tools for service {}: {}", service_name, e);
                return Err(e);
            }
        };

        let tool_count = mcp_tools.len();

        // Save tools to database
        if let Err(e) = self.save_tools_to_database(service_name, &mcp_tools).await {
            tracing::error!(
                "Failed to save tools to database for service {}: {}",
                service_name,
                e
            );
        }

        tracing::info!(
            "Successfully fetched and cached {} tools for service: {}",
            tool_count,
            service_name
        );
        Ok(mcp_tools)
    }

    /// Save tools to database
    async fn save_tools_to_database(
        &self,
        service_name: &str,
        tools: &[crate::McpTool],
    ) -> Result<()> {
        // Get server ID from database
        let server = McpServerRepository::get_by_name(service_name)
            .await?
            .ok_or_else(|| McpError::ServiceNotFound(service_name.to_string()))?;
        let server_id = server
            .id
            .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?;

        // Get existing tools from database
        let existing_tools = ToolRepository::get_by_server_id(&server_id).await?;
        let existing_map: std::collections::HashMap<String, ToolRow> = existing_tools
            .into_iter()
            .map(|t| (t.name.clone(), t))
            .collect();

        // Build remote tool name set
        let remote_names: std::collections::HashSet<_> =
            tools.iter().map(|t| t.name.to_string()).collect();

        // Upsert tools: update description for existing, create missing
        for tool in tools {
            let tool_name = tool.name.to_string();
            let remote_desc = tool.description.as_ref().map(|d| d.to_string());

            if let Some(existing) = existing_map.get(&tool_name) {
                // Update description if changed
                if existing.description != remote_desc {
                    if let Err(e) = ToolRepository::update_description(
                        &server_id,
                        &tool_name,
                        remote_desc.clone(),
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to update description for tool {} on server {}: {}",
                            tool_name,
                            service_name,
                            e
                        );
                    } else {
                        tracing::debug!(
                            "Updated description for tool {} on server {}",
                            tool_name,
                            service_name
                        );
                    }
                }
            } else {
                // Create new tool
                let tool_row = ToolRow {
                    id: None,
                    name: tool_name.clone(),
                    server_id: server_id.clone(),
                    description: remote_desc,
                    enabled: true,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                if let Err(e) = ToolRepository::create(tool_row).await {
                    tracing::error!(
                        "Failed to create tool {} for server {}: {}",
                        tool_name,
                        service_name,
                        e
                    );
                } else {
                    tracing::info!("Created new tool {} for server {}", tool_name, service_name);
                }
            }
        }

        // Disable tools that are no longer reported by the service
        for (name, row) in existing_map.into_iter() {
            if !remote_names.contains(&name) {
                if let Some(id) = row.id.as_ref() {
                    if let Err(e) = ToolRepository::toggle_enabled(id, false).await {
                        tracing::warn!(
                            "Failed to disable missing tool {} for server {}: {}",
                            name,
                            service_name,
                            e
                        );
                    } else {
                        tracing::info!(
                            "Disabled missing tool {} for server {}",
                            name,
                            service_name
                        );
                    }
                } else {
                    // Fallback by name in rare case of missing ID
                    if let Err(e) =
                        ToolRepository::toggle_enabled_by_name(&server_id, &name, false).await
                    {
                        tracing::warn!(
                            "Failed to disable missing tool {} by name for server {}: {}",
                            name,
                            service_name,
                            e
                        );
                    } else {
                        tracing::info!(
                            "Disabled missing tool {} by name for server {}",
                            name,
                            service_name
                        );
                    }
                }
            }
        }

        tracing::info!(
            "Synced {} tools with database for service {}",
            remote_names.len(),
            service_name
        );
        Ok(())
    }

    /// Convert database row to config
    fn db_row_to_config(&self, row: &McpServerRow) -> Result<McpServerConfig> {
        use crate::config::ServiceTransport;

        let transport = match row.transport.to_lowercase().as_str() {
            "stdio" => ServiceTransport::Stdio,
            "sse" => ServiceTransport::Sse,
            "streamablehttp" => ServiceTransport::StreamableHttp,
            _ => {
                return Err(McpError::ConfigError(format!(
                    "Invalid transport: {}",
                    row.transport
                )))
            }
        };

        Ok(McpServerConfig {
            name: row.name.clone(),
            description: row.description.clone(),
            command: row.command.clone(),
            args: row.args.clone(),
            transport,
            url: row.url.clone(),
            enabled: row.enabled,
            env_vars: row.env_vars.clone(),
            headers: row.headers.clone(),
            version: row.version.clone(),
        })
    }

    /// Convert config to database row
    fn config_to_db_row(&self, config: &McpServerConfig) -> Result<McpServerRow> {
        let transport = match config.transport {
            crate::config::ServiceTransport::Stdio => "stdio",
            crate::config::ServiceTransport::Sse => "sse",
            crate::config::ServiceTransport::StreamableHttp => "streamablehttp",
        };

        Ok(McpServerRow {
            id: None, // Will be generated by repository if needed
            name: config.name.clone(),
            description: config.description.clone(),
            command: config.command.clone(),
            args: config.args.clone(),
            transport: transport.to_string(),
            url: config.url.clone(),
            enabled: config.enabled,
            env_vars: config.env_vars.clone(),
            headers: config.headers.clone(),
            version: config.version.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    // 缓存相关方法
}
