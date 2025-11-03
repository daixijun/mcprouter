// MCP服务器管理 - 配置文件版本

use crate::config::{AppConfig, McpServerRepository};
use crate::error::{McpError, Result};
use crate::MCP_CLIENT_MANAGER;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub use crate::types::{McpServerConfig, McpServerInfo, ServiceStatus, ServiceVersionCache};

#[derive(Clone)]
pub struct McpServerManager {
    mcp_servers: Arc<RwLock<HashMap<String, McpServerConfig>>>,
    config: Arc<RwLock<AppConfig>>,
    startup_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub version_cache: Arc<RwLock<HashMap<String, ServiceVersionCache>>>,
    loading_lock: Arc<Mutex<bool>>,
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

    pub async fn update_config<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write().await;
        f(&mut config);
        Ok(())
    }

    pub async fn load_mcp_servers(&self, app_handle: &tauri::AppHandle) -> Result<()> {
        self.sync_with_config_file(app_handle).await
    }

    pub async fn get_mcp_servers(&self) -> Arc<RwLock<HashMap<String, McpServerConfig>>> {
        self.mcp_servers.clone()
    }

    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>> {
        let services = self.mcp_servers.read().await;
        let mut result = Vec::new();

        for (name, config) in services.iter() {
            let status_option = self.get_mcp_server_status(name).await.ok();
            let status_string = status_option
                .as_ref()
                .map(|s| s.status.clone())
                .unwrap_or_else(|| "disconnected".to_string());
            let error_message = status_option.and_then(|s| s.error_message);

            result.push(McpServerInfo {
                name: name.clone(),
                enabled: config.enabled,
                status: status_string,
                version: config.version.clone(),
                error_message,
                transport: format!("{:?}", config.transport),
                url: config.url.clone(),
                description: None,
                env_vars: None,
                headers: None,
                command: config.command.clone(),
                args: config.args.clone(),
                tool_count: None,
            });
        }

        Ok(result)
    }

    async fn get_mcp_server_status(&self, name: &str) -> Result<ServiceStatus> {
        let services = self.mcp_servers.read().await;
        let service_config = services
            .get(name)
            .ok_or_else(|| McpError::ServiceNotFound(name.to_string()))?
            .clone();

        let version_from_connection = MCP_CLIENT_MANAGER.get_cached_version(name).await;
        let (status, error_message) = MCP_CLIENT_MANAGER.get_connection_status(name).await;

        let final_status = if !service_config.enabled {
            "disconnected".to_string()
        } else {
            status
        };

        Ok(ServiceStatus {
            name: name.to_string(),
            enabled: service_config.enabled,
            status: final_status,
            pid: None,
            port: None,
            version: version_from_connection.or(service_config.version),
            start_time: None,
            error_message,
        })
    }

    pub async fn check_service_with_version(&self, name: &str) -> Result<()> {
        let services = self.mcp_servers.read().await;
        let service_config = services
            .get(name)
            .ok_or_else(|| McpError::ServiceNotFound(name.to_string()))?
            .clone();
        drop(services);

        let connection = MCP_CLIENT_MANAGER
            .ensure_connection(&service_config, false)
            .await
            .map_err(|e| {
                McpError::ConnectionError(format!("Failed to connect to service '{}': {}", name, e))
            })?;

        if let Some(ref _client) = connection.client {
            if let Some(version) = connection.server_info.as_ref().and_then(|info| {
                info.get("version")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            }) {
                let mut version_cache = self.version_cache.write().await;
                version_cache.insert(
                    name.to_string(),
                    ServiceVersionCache {
                        version: Some(version.clone()),
                    },
                );
            }
        }

        Ok(())
    }

    pub async fn update_version_cache(&self, name: &str, version: Option<String>) {
        let mut version_cache = self.version_cache.write().await;
        version_cache.insert(name.to_string(), ServiceVersionCache { version });
    }

    pub async fn add_mcp_server(
        &self,
        app_handle: &tauri::AppHandle,
        config: McpServerConfig,
    ) -> Result<()> {
        tracing::info!(
            "McpServerManager::add_mcp_server 开始添加服务器: {}",
            config.name
        );
        let mut repo = McpServerRepository::new(app_handle).await.map_err(|e| {
            tracing::error!("创建仓库失败: {}", e);
            McpError::ConfigError(format!("Failed to create repository: {}", e))
        })?;

        tracing::info!("仓库创建成功，开始添加配置");
        repo.add(config.clone()).await.map_err(|e| {
            tracing::error!("添加配置失败: {}", e);
            McpError::ConfigError(format!("Failed to add MCP server: {}", e))
        })?;

        tracing::info!("✅ 配置添加成功，同步内存状态");
        self.sync_with_config_file(app_handle).await?;

        tracing::info!("✅ 内存同步成功，开始连接服务获取版本和工具列表");

        // 尝试连接服务以获取版本和工具列表
        if let Err(e) = self.check_service_with_version(&config.name).await {
            tracing::warn!("⚠️ 连接服务失败，将在首次使用时获取版本信息: {}", e);
        } else {
            tracing::info!("✅ 服务连接成功，版本和工具列表已更新");
        }

        Ok(())
    }

    pub async fn update_mcp_server(
        &self,
        app_handle: &tauri::AppHandle,
        config: McpServerConfig,
    ) -> Result<()> {
        let mut repo = McpServerRepository::new(app_handle)
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to create repository: {}", e)))?;

        let server_name = config.name.clone();
        repo.update(&server_name, config.clone())
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to update MCP server: {}", e)))?;

        tracing::info!("✅ 配置更新成功，同步内存状态");
        self.sync_with_config_file(app_handle).await?;

        // 如果服务被启用，尝试连接以获取最新版本信息
        if config.enabled {
            tracing::info!("✅ 服务 '{}' 已更新，开始连接获取最新版本信息", server_name);
            if let Err(e) = self.check_service_with_version(&server_name).await {
                tracing::warn!("⚠️ 连接服务失败: {}", e);
            } else {
                tracing::info!("✅ 服务连接成功，版本信息已更新");
            }
        } else {
            tracing::info!("ℹ️ 服务 '{}' 当前为禁用状态", server_name);
        }

        Ok(())
    }

    pub async fn remove_mcp_server(&self, app_handle: &tauri::AppHandle, name: &str) -> Result<()> {
        let mut repo = McpServerRepository::new(app_handle)
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to create repository: {}", e)))?;

        repo.delete(name)
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to delete MCP server: {}", e)))?;

        tracing::info!("✅ 配置删除成功，同步内存状态");
        self.sync_with_config_file(app_handle).await?;

        Ok(())
    }

    pub async fn toggle_mcp_server(
        &self,
        app_handle: &tauri::AppHandle,
        name: &str,
    ) -> Result<bool> {
        let mut repo = McpServerRepository::new(app_handle)
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to create repository: {}", e)))?;

        let new_state = repo
            .toggle_enabled(name)
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to toggle MCP server: {}", e)))?;

        tracing::info!("✅ 配置更新成功，同步内存状态");
        self.sync_with_config_file(app_handle).await?;

        // 如果服务被启用，尝试连接以获取版本信息
        if new_state {
            tracing::info!("✅ 服务 '{}' 已启用，开始连接获取版本信息", name);
            if let Err(e) = self.check_service_with_version(name).await {
                tracing::warn!("⚠️ 连接服务失败: {}", e);
            } else {
                tracing::info!("✅ 服务连接成功，版本信息已更新");
            }
        } else {
            tracing::info!("ℹ️ 服务 '{}' 已禁用", name);
        }

        Ok(new_state)
    }

    /// 从配置文件同步内存状态
    pub async fn sync_with_config_file(&self, app_handle: &tauri::AppHandle) -> Result<()> {
        let repo = McpServerRepository::new(app_handle)
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to create repository: {}", e)))?;

        let servers = repo.get_all();
        let mut mcp_servers_map = HashMap::new();

        for server in servers {
            // 转换为 McpServerConfig（去除内部字段）
            let config = McpServerConfig {
                name: server.name.clone(),
                description: server.description.clone(),
                command: server.command.clone(),
                args: server.args.clone(),
                transport: server.transport.clone(),
                url: server.url.clone(),
                enabled: server.enabled,
                env_vars: server.env_vars.clone(),
                headers: server.headers.clone(),
                version: server.version.clone(),
            };
            mcp_servers_map.insert(config.name.clone(), config);
        }

        // 更新内存中的 HashMap
        let mut mcp_servers = self.mcp_servers.write().await;
        *mcp_servers = mcp_servers_map;

        tracing::info!("✅ 内存状态已同步，共 {} 个服务器", mcp_servers.len());
        Ok(())
    }

    /// 获取服务器的工具列表
    pub async fn list_mcp_server_tools(
        &self,
        server_name: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<String>> {
        // 首先从配置文件获取工具列表
        let repo = McpServerRepository::new(app_handle)
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to create repository: {}", e)))?;

        if let Some(server) = repo.get_by_name(server_name) {
            // 返回配置文件中存储的工具列表
            let tools: Vec<String> = server.tools.iter().map(|t| t.id.clone()).collect();
            tracing::info!("从配置文件中获取到 {} 个工具", tools.len());
            return Ok(tools);
        }

        // 如果配置文件中没有，尝试从连接中获取
        tracing::info!("配置文件中未找到工具列表，尝试从连接获取");

        // 获取内存中的服务配置
        let mcp_servers = self.mcp_servers.read().await;
        if let Some(config) = mcp_servers.get(server_name) {
            let config_clone = config.clone();
            drop(mcp_servers);
            // 尝试建立连接并获取工具
            if let Ok(_connection) = MCP_CLIENT_MANAGER
                .ensure_connection(&config_clone, false)
                .await
            {
                // TODO: 需要有方法能通过服务器名称或连接ID获取工具列表
                // 目前暂时返回空列表，后续完善
                tracing::info!("连接成功，但工具列表获取功能待完善");
            }
        }

        tracing::warn!("⚠️ 未找到服务器 '{}' 的工具列表", server_name);
        Ok(Vec::new())
    }
}
