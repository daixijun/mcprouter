// MCP Server Management - Configuration File Version

use crate::config::{AppConfig, McpServerRepository};
use crate::error::{McpError, Result};
use crate::MCP_CLIENT_MANAGER;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::RwLock;

pub use crate::types::{McpServerConfig, McpServerInfo, ServiceStatus, ServiceVersionCache};

#[derive(Clone)]
pub struct McpServerManager {
    mcp_servers: Arc<RwLock<HashMap<String, McpServerConfig>>>,
    config: Arc<RwLock<AppConfig>>,
    pub version_cache: Arc<RwLock<HashMap<String, ServiceVersionCache>>>,
    pub tools_cache_entries: Arc<RwLock<HashMap<String, ToolsCacheEntry>>>,
    pub resources_cache_entries: Arc<RwLock<HashMap<String, ResourcesCacheEntry>>>,
    pub prompts_cache_entries: Arc<RwLock<HashMap<String, PromptsCacheEntry>>>,
    tools_cache_ttl: std::time::Duration,
}

impl McpServerManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            mcp_servers: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
            version_cache: Arc::new(RwLock::new(HashMap::new())),
            tools_cache_entries: Arc::new(RwLock::new(HashMap::new())),
            resources_cache_entries: Arc::new(RwLock::new(HashMap::new())),
            prompts_cache_entries: Arc::new(RwLock::new(HashMap::new())),
            tools_cache_ttl: std::time::Duration::from_secs(600),
        }
    }

    pub fn get_tools_cache_ttl_seconds(&self) -> u64 {
        self.tools_cache_ttl.as_secs()
    }

    pub async fn set_tools_cache_entry(&self, server_name: &str, raw: Vec<rmcp::model::Tool>) {
        let now = chrono::Utc::now();
        let infos: Vec<crate::types::McpToolInfo> = raw
            .iter()
            .map(|tool| crate::types::McpToolInfo {
                id: tool.name.to_string(),
                name: tool.name.to_string(),
                description: tool.description.clone().unwrap_or_default().to_string(),
                enabled: true,
                created_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            })
            .collect();
        let mut entries = self.tools_cache_entries.write().await;
        entries.insert(
            server_name.to_string(),
            ToolsCacheEntry {
                raw,
                infos: infos.clone(),
                last_updated: now,
                count: infos.len(),
            },
        );
    }

    pub async fn get_cached_tools_raw(&self, server_name: &str) -> Option<Vec<rmcp::model::Tool>> {
        let entries = self.tools_cache_entries.read().await;
        entries.get(server_name).map(|e| e.raw.clone())
    }

    pub async fn get_cached_tools_infos(
        &self,
        server_name: &str,
    ) -> Option<Vec<crate::types::McpToolInfo>> {
        let entries = self.tools_cache_entries.read().await;
        entries.get(server_name).map(|e| e.infos.clone())
    }

    pub async fn get_raw_cached_tools(&self, server_name: &str) -> Option<Vec<rmcp::model::Tool>> {
        self.get_cached_tools_raw(server_name).await
    }

    // Resources cache management methods
    pub async fn set_resources_cache_entry(
        &self,
        server_name: &str,
        raw: Vec<rmcp::model::Resource>,
    ) {
        let now = chrono::Utc::now();
        let infos: Vec<crate::types::McpResourceInfo> = raw
            .iter()
            .map(|resource| crate::types::McpResourceInfo {
                id: resource.uri.to_string(),
                uri: resource.uri.to_string(),
                name: resource.name.to_string(),
                description: resource.description.clone(),
                mime_type: resource.mime_type.clone(),
                enabled: true,
                created_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            })
            .collect();
        let count = infos.len();
        let mut entries = self.resources_cache_entries.write().await;
        entries.insert(
            server_name.to_string(),
            ResourcesCacheEntry {
                raw,
                infos: infos.clone(),
                last_updated: now,
                count,
            },
        );

        // Use the cached fields for logging
        tracing::debug!(
            "Cached {} resources for server '{}' (updated at: {})",
            count,
            server_name,
            now.to_rfc3339()
        );
    }

    pub async fn get_cached_resources_raw(
        &self,
        server_name: &str,
    ) -> Option<Vec<rmcp::model::Resource>> {
        let entries = self.resources_cache_entries.read().await;
        entries.get(server_name).map(|e| e.raw.clone())
    }

    pub async fn get_cached_resources_infos(
        &self,
        server_name: &str,
    ) -> Option<Vec<crate::types::McpResourceInfo>> {
        let entries = self.resources_cache_entries.read().await;
        entries.get(server_name).map(|e| {
            // Use the fields to avoid dead code warning
            let _last_updated = e.last_updated;
            let _count = e.count;
            tracing::debug!(
                "Resource cache for '{}': {} items, updated at {}",
                server_name,
                _count,
                _last_updated.to_rfc3339()
            );
            e.infos.clone()
        })
    }

    // Prompts cache management methods
    pub async fn set_prompts_cache_entry(&self, server_name: &str, raw: Vec<rmcp::model::Prompt>) {
        let now = chrono::Utc::now();
        let infos: Vec<crate::types::McpPromptInfo> = raw
            .iter()
            .map(|prompt| crate::types::McpPromptInfo {
                id: prompt.name.to_string(),
                name: prompt.name.to_string(),
                description: prompt.description.clone(),
                enabled: true,
                created_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            })
            .collect();
        let count = infos.len();
        let mut entries = self.prompts_cache_entries.write().await;
        entries.insert(
            server_name.to_string(),
            PromptsCacheEntry {
                raw,
                infos: infos.clone(),
                last_updated: now,
                count,
            },
        );

        // Use the cached fields for logging
        tracing::debug!(
            "Cached {} prompts for server '{}' (updated at: {})",
            count,
            server_name,
            now.to_rfc3339()
        );
    }

    pub async fn get_cached_prompts_raw(
        &self,
        server_name: &str,
    ) -> Option<Vec<rmcp::model::Prompt>> {
        let entries = self.prompts_cache_entries.read().await;
        entries.get(server_name).map(|e| e.raw.clone())
    }

    pub async fn get_cached_prompts_infos(
        &self,
        server_name: &str,
    ) -> Option<Vec<crate::types::McpPromptInfo>> {
        let entries = self.prompts_cache_entries.read().await;
        entries.get(server_name).map(|e| {
            // Use the fields to avoid dead code warning
            let _last_updated = e.last_updated;
            let _count = e.count;
            tracing::debug!(
                "Prompt cache for '{}': {} items, updated at {}",
                server_name,
                _count,
                _last_updated.to_rfc3339()
            );
            e.infos.clone()
        })
    }

    /// Clear all cached data (tools, resources, prompts) for a specific server
    pub async fn clear_server_cache(&self, server_name: &str) {
        let mut tools_cache = self.tools_cache_entries.write().await;
        let mut resources_cache = self.resources_cache_entries.write().await;
        let mut prompts_cache = self.prompts_cache_entries.write().await;
        let mut version_cache = self.version_cache.write().await;

        // Clear tools cache
        if tools_cache.remove(server_name).is_some() {
            tracing::info!("Cleared tools cache for server '{}'", server_name);
        }

        // Clear resources cache
        if resources_cache.remove(server_name).is_some() {
            tracing::info!("Cleared resources cache for server '{}'", server_name);
        }

        // Clear prompts cache
        if prompts_cache.remove(server_name).is_some() {
            tracing::info!("Cleared prompts cache for server '{}'", server_name);
        }

        // Clear version cache
        if version_cache.remove(server_name).is_some() {
            tracing::info!("Cleared version cache for server '{}'", server_name);
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
        // Save the updated configuration to file
        config.save()?;
        Ok(())
    }

    pub async fn load_mcp_servers(&self, app_handle: &tauri::AppHandle) -> Result<()> {
        self.sync_with_config_file(app_handle).await
    }

    pub async fn get_mcp_servers(&self) -> Arc<RwLock<HashMap<String, McpServerConfig>>> {
        self.mcp_servers.clone()
    }

    /// list_mcp_servers with optional app_handle, supports getting tool count
    pub async fn list_mcp_servers(
        &self,
        _app_handle: Option<&tauri::AppHandle>,
    ) -> Result<Vec<McpServerInfo>> {
        let services = self.mcp_servers.read().await;
        let mut result = Vec::new();

        for (name, config) in services.iter() {
            let status_option = self.get_mcp_server_status(name).await.ok();
            let status_string = status_option
                .as_ref()
                .map(|s| s.status.clone())
                .unwrap_or_else(|| "disconnected".to_string());
            let error_message = status_option.and_then(|s| s.error_message);

            // Version from cache
            let version = {
                let cache = self.version_cache.read().await;
                cache.get(name).and_then(|v| v.version.clone())
            };

            // Tool count from cache
            let tool_count = {
                let entries = self.tools_cache_entries.read().await;
                entries.get(name).map(|e| e.count)
            };

            // Set different fields based on transport type, set unneeded fields to None to skip during serialization
            let (transport_str, url, headers, command, args, env_data) = match config.transport {
                // STDIO transport: remove url/headers fields
                crate::types::ServiceTransport::Stdio => {
                    (
                        "stdio".to_string(),
                        None, // Remove url field
                        None, // Remove headers field
                        config.command.clone(),
                        config.args.clone(),
                        config.env.clone(),
                    )
                }
                // SSE transport: remove command/args/env_vars fields
                crate::types::ServiceTransport::Sse => {
                    (
                        "sse".to_string(),
                        config.url.clone(),
                        config.headers.clone(),
                        None, // Remove command field
                        None, // Remove args field
                        None, // Remove env_vars field
                    )
                }
                // HTTP transport: remove command/args/env_vars fields
                crate::types::ServiceTransport::Http => {
                    (
                        "http".to_string(),
                        config.url.clone(),
                        config.headers.clone(),
                        None, // Remove command field
                        None, // Remove args field
                        None, // Remove env_vars field
                    )
                }
            };

            result.push(McpServerInfo {
                name: name.clone(),
                enabled: config.enabled,
                status: status_string,
                version,
                error_message,
                transport: transport_str,
                url,
                description: config.description.clone(), // Read description from config
                env: env_data,
                headers,
                command,
                args,
                tool_count,
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

        let (status, error_message) = MCP_CLIENT_MANAGER.get_connection_status(name).await;

        let final_status = if !service_config.enabled {
            "disconnected".to_string()
        } else {
            status
        };

        // Version from cache
        let version = {
            let cache = self.version_cache.read().await;
            cache.get(name).and_then(|v| v.version.clone())
        };

        Ok(ServiceStatus {
            name: name.to_string(),
            enabled: service_config.enabled,
            status: final_status,
            pid: None,
            port: None,
            version,
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
            tracing::debug!("Checking version info for service '{}'...", name);

            // If version not in server_info, try to get from cached_version
            let version = connection
                .server_info
                .as_ref()
                .map(|info| info.server_info.version.clone());

            if let Some(version_str) = version {
                tracing::info!("Extracted version for service '{}': {}", name, version_str);

                // Update version cache
                let mut version_cache = self.version_cache.write().await;
                version_cache.insert(
                    name.to_string(),
                    ServiceVersionCache {
                        version: Some(version_str.clone()),
                    },
                );
                tracing::info!("Updated version cache for service '{}'", name);
            } else {
                tracing::warn!("Failed to extract version info for service '{}'", name);
                tracing::debug!("server_info: {:?}", connection.server_info);
            }
        }

        Ok(())
    }

    pub async fn add_mcp_server(
        &self,
        app_handle: &tauri::AppHandle,
        config: McpServerConfig,
    ) -> Result<()> {
        tracing::info!(
            "McpServerManager::add_mcp_server starting to add server: {}",
            config.name
        );
        let mut repo = McpServerRepository::new(app_handle).await.map_err(|e| {
            tracing::error!("Failed to create repository: {}", e);
            McpError::ConfigError(format!("Failed to create repository: {}", e))
        })?;

        tracing::info!("Repository created successfully, starting to add config");
        repo.add(config.clone()).await.map_err(|e| {
            tracing::error!("Failed to add config: {}", e);
            McpError::ConfigError(format!("Failed to add MCP server: {}", e))
        })?;

        tracing::info!("Config added successfully, syncing memory state");
        self.sync_with_config_file(app_handle).await?;

        tracing::info!(
            "Memory sync successful, starting to connect to service to get version and tool list"
        );

        // Try to connect to service to get version and tool list
        if let Err(e) = self.check_service_with_version(&config.name).await {
            tracing::warn!(
                "Failed to connect to service, will get version info on first use: {}",
                e
            );
        } else {
            tracing::info!("Service connected successfully, version info updated");
        }

        // Sync tool list and update to config file
        if let Err(e) = self
            .sync_server_tools_from_service(&config.name, app_handle)
            .await
        {
            tracing::warn!(
                "Failed to get tool list for service '{}': {}",
                config.name,
                e
            );
        } else {
            tracing::info!("Service '{}' tool list updated", config.name);
        }

        // Sync resources list
        if let Err(e) = self
            .sync_server_resources_from_service(&config.name, app_handle)
            .await
        {
            tracing::warn!(
                "Failed to get resources list for service '{}': {}",
                config.name,
                e
            );
        } else {
            tracing::info!("Service '{}' resources list updated", config.name);
        }

        // Sync prompts list
        if let Err(e) = self
            .sync_server_prompts_from_service(&config.name, app_handle)
            .await
        {
            tracing::warn!(
                "Failed to get prompts list for service '{}': {}",
                config.name,
                e
            );
        } else {
            tracing::info!("Service '{}' prompts list updated", config.name);
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

        tracing::info!("Config updated successfully, syncing memory state");
        self.sync_with_config_file(app_handle).await?;

        // If service is enabled, try to connect to get latest version info
        if config.enabled {
            tracing::info!(
                "Service '{}' updated, starting to connect to get latest version info",
                server_name
            );
            if let Err(e) = self.check_service_with_version(&server_name).await {
                tracing::warn!("Failed to connect to service: {}", e);
            } else {
                tracing::info!("Service connected successfully, version info updated");

                // Automatically sync tool list for the updated service
                tracing::info!("Syncing tool list for updated service '{}'", server_name);
                if let Err(e) = self
                    .sync_server_tools_from_service(&server_name, app_handle)
                    .await
                {
                    tracing::warn!(
                        "Failed to sync tool list for service '{}': {}",
                        server_name,
                        e
                    );
                } else {
                    tracing::info!("Service '{}' tool list synced successfully", server_name);
                }

                // Automatically sync resources list for the updated service
                tracing::info!(
                    "Syncing resources list for updated service '{}'",
                    server_name
                );
                if let Err(e) = self
                    .sync_server_resources_from_service(&server_name, app_handle)
                    .await
                {
                    tracing::warn!(
                        "Failed to sync resources list for service '{}': {}",
                        server_name,
                        e
                    );
                } else {
                    tracing::info!(
                        "Service '{}' resources list synced successfully",
                        server_name
                    );
                }

                // Automatically sync prompts list for the updated service
                tracing::info!("Syncing prompts list for updated service '{}'", server_name);
                if let Err(e) = self
                    .sync_server_prompts_from_service(&server_name, app_handle)
                    .await
                {
                    tracing::warn!(
                        "Failed to sync prompts list for service '{}': {}",
                        server_name,
                        e
                    );
                } else {
                    tracing::info!("Service '{}' prompts list synced successfully", server_name);
                }
            }
        } else {
            tracing::info!("Service '{}' is currently disabled", server_name);

            // Clear cached tools, resources, and prompts when service is disabled
            self.clear_server_cache(&server_name).await;

            // Disconnect from the service to release resources
            tracing::info!("Disconnecting from disabled service '{}'", server_name);
            if let Err(e) = MCP_CLIENT_MANAGER.disconnect_mcp_server(&server_name).await {
                tracing::warn!(
                    "Failed to disconnect from disabled service '{}': {}",
                    server_name,
                    e
                );
            } else {
                tracing::info!("Successfully disconnected from service '{}'", server_name);
            }
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

        tracing::info!("Config deleted successfully, syncing memory state");
        self.sync_with_config_file(app_handle).await?;

        // Clear all cached data for the removed server
        tracing::info!("Clearing cache for removed server '{}'", name);
        self.clear_server_cache(name).await;

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

        tracing::info!("Config updated successfully, syncing memory state");
        self.sync_with_config_file(app_handle).await?;

        // If service is enabled, try to connect to get version info
        if new_state {
            tracing::info!(
                "Service '{}' enabled, starting to connect to get version info",
                name
            );
            if let Err(e) = self.check_service_with_version(name).await {
                tracing::warn!("Failed to connect to service: {}", e);
            } else {
                tracing::info!("Service connected successfully, version info updated");

                // Automatically sync tool list for the enabled service
                tracing::info!("Syncing tool list for enabled service '{}'", name);
                if let Err(e) = self.sync_server_tools_from_service(name, app_handle).await {
                    tracing::warn!("Failed to sync tool list for service '{}': {}", name, e);
                } else {
                    tracing::info!("Service '{}' tool list synced successfully", name);
                }

                // Automatically sync resources list for the enabled service
                tracing::info!("Syncing resources list for enabled service '{}'", name);
                if let Err(e) = self
                    .sync_server_resources_from_service(name, app_handle)
                    .await
                {
                    tracing::warn!(
                        "Failed to sync resources list for service '{}': {}",
                        name,
                        e
                    );
                } else {
                    tracing::info!("Service '{}' resources list synced successfully", name);
                }

                // Automatically sync prompts list for the enabled service
                tracing::info!("Syncing prompts list for enabled service '{}'", name);
                if let Err(e) = self
                    .sync_server_prompts_from_service(name, app_handle)
                    .await
                {
                    tracing::warn!("Failed to sync prompts list for service '{}': {}", name, e);
                } else {
                    tracing::info!("Service '{}' prompts list synced successfully", name);
                }
            }
        } else {
            tracing::info!("Service '{}' is disabled, clearing cache", name);
            // Clear cached tools, resources, and prompts when service is disabled
            self.clear_server_cache(name).await;

            // Disconnect from the service to release resources
            tracing::info!("Disconnecting from disabled service '{}'", name);
            if let Err(e) = MCP_CLIENT_MANAGER.disconnect_mcp_server(name).await {
                tracing::warn!(
                    "Failed to disconnect from disabled service '{}': {}",
                    name,
                    e
                );
            } else {
                tracing::info!("Successfully disconnected from service '{}'", name);
            }
        }

        Ok(new_state)
    }

    /// Sync memory state from config file
    pub async fn sync_with_config_file(&self, app_handle: &tauri::AppHandle) -> Result<()> {
        let repo = McpServerRepository::new(app_handle)
            .await
            .map_err(|e| McpError::ConfigError(format!("Failed to create repository: {}", e)))?;

        let servers = repo.get_all();
        let mut mcp_servers_map = HashMap::new();

        for server in servers {
            let config = McpServerConfig {
                name: server.name.clone(),
                description: server.description.clone(),
                command: server.command.clone(),
                args: server.args.clone(),
                transport: server.transport.clone(),
                url: server.url.clone(),
                enabled: server.enabled,
                env: server.env.clone(),
                headers: server.headers.clone(),
            };
            mcp_servers_map.insert(config.name.clone(), config);
        }

        // Update HashMap in memory
        let mut mcp_servers = self.mcp_servers.write().await;
        *mcp_servers = mcp_servers_map;

        tracing::info!("Memory state synced, total {} servers", mcp_servers.len());
        Ok(())
    }

    /// Get tool list for server (from in-memory cache)
    pub async fn list_mcp_server_tools(
        &self,
        server_name: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<crate::types::McpToolInfo>> {
        // Try cache first
        {
            if let Some(list) = self.get_cached_tools_infos(server_name).await {
                return Ok(list);
            }
        }

        // Not cached: sync from service and cache
        self.sync_server_tools_from_service(server_name, app_handle)
            .await?;

        let tools = self
            .get_cached_tools_infos(server_name)
            .await
            .unwrap_or_default();
        Ok(tools)
    }

    /// Auto-connect all enabled services on startup
    pub async fn auto_connect_enabled_services(&self, app_handle: &tauri::AppHandle) -> Result<()> {
        let services = self.mcp_servers.read().await;
        let enabled_services: Vec<String> = services
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, _)| name.clone())
            .collect();

        if enabled_services.is_empty() {
            tracing::info!("No enabled MCP services need connection");
            drop(services);
            return Ok(());
        }

        tracing::info!(
            "Auto-connecting {} enabled MCP services on startup...",
            enabled_services.len()
        );

        // Collect all enabled service configs
        let service_configs: Vec<McpServerConfig> = enabled_services
            .iter()
            .filter_map(|name| services.get(name).cloned())
            .collect();

        drop(services);

        let mut success_count = 0;
        let mut failed_count = 0;

        // Actually try to establish connection for each service
        for service_config in service_configs {
            let service_name = service_config.name.clone();

            tracing::info!("Connecting to service: {}", service_name);

            // Try to reconnect service
            match MCP_CLIENT_MANAGER.try_reconnect(&service_config).await {
                Ok(true) => {
                    tracing::info!("Service '{}' connected successfully", service_name);

                    // Try to get version info
                    if let Err(e) = self.check_service_with_version(&service_name).await {
                        tracing::warn!(
                            "Failed to get version info for service '{}': {}",
                            service_name,
                            e
                        );
                    } else {
                        tracing::info!("Service '{}' version info updated", service_name);
                    }

                    // Auto get and update tool list
                    if let Err(e) = self
                        .sync_server_tools_from_service(&service_name, app_handle)
                        .await
                    {
                        tracing::warn!(
                            "Failed to get tool list for service '{}': {}",
                            service_name,
                            e
                        );
                    } else {
                        tracing::info!("Service '{}' tool list updated", service_name);
                    }

                    // Auto get and update resources list
                    if let Err(e) = self
                        .sync_server_resources_from_service(&service_name, app_handle)
                        .await
                    {
                        tracing::warn!(
                            "Failed to get resources list for service '{}': {}",
                            service_name,
                            e
                        );
                    } else {
                        tracing::info!("Service '{}' resources list updated", service_name);
                    }

                    // Auto get and update prompts list
                    if let Err(e) = self
                        .sync_server_prompts_from_service(&service_name, app_handle)
                        .await
                    {
                        tracing::warn!(
                            "Failed to get prompts list for service '{}': {}",
                            service_name,
                            e
                        );
                    } else {
                        tracing::info!("Service '{}' prompts list updated", service_name);
                    }

                    success_count += 1;
                }
                Ok(false) => {
                    tracing::error!("Service '{}' connection failed", service_name);
                    failed_count += 1;
                }
                Err(e) => {
                    tracing::error!("Service '{}' connection error: {}", service_name, e);
                    failed_count += 1;
                }
            }
        }

        tracing::info!(
            "Auto-connect completed: {} services connected successfully, {} failed",
            success_count,
            failed_count
        );

        Ok(())
    }

    /// Sync tool list from MCP service and update in-memory cache
    pub async fn sync_server_tools_from_service(
        &self,
        server_name: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<()> {
        tracing::debug!("Starting to get tool list from service '{}'", server_name);

        // Get service config
        let services = self.mcp_servers.read().await;
        let service_config = services
            .get(server_name)
            .ok_or_else(|| McpError::ServiceNotFound(server_name.to_string()))?
            .clone();
        drop(services);

        // Connect to service (single attempt, no retry)
        let _connection = MCP_CLIENT_MANAGER
            .ensure_connection(&service_config, false)
            .await
            .map_err(|e| {
                let error_msg = format!("{}", e);
                tracing::error!(
                    "Failed to connect to service '{}': {}",
                    server_name,
                    error_msg
                );

                // Provide specific guidance for HTTP/SSE transport
                if matches!(
                    service_config.transport,
                    crate::types::ServiceTransport::Http
                ) {
                    tracing::error!(
                        "For HTTP transport services, please verify:\n\
                        1. The URL is correct and points to a valid MCP endpoint\n\
                        2. The service is running and accessible\n\
                        3. The service properly implements the MCP StreamableHttp protocol\n\
                        4. Authentication headers are correctly configured (if required)"
                    );
                } else if matches!(
                    service_config.transport,
                    crate::types::ServiceTransport::Sse
                ) {
                    tracing::error!(
                        "For SSE transport services, please verify:\n\
                        1. The URL is correct and points to a valid MCP SSE endpoint\n\
                        2. The service is running and accessible"
                    );
                }

                McpError::ConnectionError(format!(
                    "Failed to connect to service '{}': {}",
                    server_name, error_msg
                ))
            })?;

        // Get tool list (single attempt, no retry)
        let tools = MCP_CLIENT_MANAGER
            .list_tools(server_name)
            .await
            .map_err(|e| {
                McpError::ServiceError(format!(
                    "Failed to list tools from service '{}': {}",
                    server_name, e
                ))
            })?;

        // Process tools
        if !tools.is_empty() {
            tracing::info!("Got {} tools from service '{}'", tools.len(), server_name);

            self.set_tools_cache_entry(server_name, tools.clone()).await;
            let _ = app_handle.emit("tools-updated", server_name.to_string());
            tracing::info!("Updated in-memory tool list for service '{}'", server_name);
        } else {
            tracing::debug!("Service '{}' has no available tools", server_name);
        }

        Ok(())
    }

    /// Sync resources list from MCP service and update in-memory cache
    pub async fn sync_server_resources_from_service(
        &self,
        server_name: &str,
        _app_handle: &tauri::AppHandle,
    ) -> Result<()> {
        tracing::debug!(
            "Starting to get resources list from service '{}'",
            server_name
        );

        // Get service config
        let services = self.mcp_servers.read().await;
        let service_config = services
            .get(server_name)
            .ok_or_else(|| McpError::ServiceNotFound(server_name.to_string()))?
            .clone();
        drop(services);

        // Connect to service
        let _connection = MCP_CLIENT_MANAGER
            .ensure_connection(&service_config, false)
            .await
            .map_err(|e| {
                let error_msg = format!("{}", e);
                tracing::error!(
                    "Failed to connect to service '{}': {}",
                    server_name,
                    error_msg
                );
                McpError::ConnectionError(format!(
                    "Failed to connect to service '{}': {}",
                    server_name, error_msg
                ))
            })?;

        // Get resources list
        let resources = MCP_CLIENT_MANAGER
            .list_resources(server_name)
            .await
            .map_err(|e| {
                McpError::ServiceError(format!(
                    "Failed to list resources from service '{}': {}",
                    server_name, e
                ))
            })?;

        // Process resources
        if !resources.is_empty() {
            tracing::info!(
                "Got {} resources from service '{}'",
                resources.len(),
                server_name
            );
            self.set_resources_cache_entry(server_name, resources.clone())
                .await;
            tracing::info!(
                "Updated in-memory resources list for service '{}'",
                server_name
            );
        } else {
            tracing::debug!("Service '{}' has no available resources", server_name);
        }

        Ok(())
    }

    /// Sync prompts list from MCP service and update in-memory cache
    pub async fn sync_server_prompts_from_service(
        &self,
        server_name: &str,
        _app_handle: &tauri::AppHandle,
    ) -> Result<()> {
        tracing::debug!(
            "Starting to get prompts list from service '{}'",
            server_name
        );

        // Get service config
        let services = self.mcp_servers.read().await;
        let service_config = services
            .get(server_name)
            .ok_or_else(|| McpError::ServiceNotFound(server_name.to_string()))?
            .clone();
        drop(services);

        // Connect to service
        let _connection = MCP_CLIENT_MANAGER
            .ensure_connection(&service_config, false)
            .await
            .map_err(|e| {
                let error_msg = format!("{}", e);
                tracing::error!(
                    "Failed to connect to service '{}': {}",
                    server_name,
                    error_msg
                );
                McpError::ConnectionError(format!(
                    "Failed to connect to service '{}': {}",
                    server_name, error_msg
                ))
            })?;

        // Get prompts list
        let prompts = MCP_CLIENT_MANAGER
            .list_prompts(server_name)
            .await
            .map_err(|e| {
                McpError::ServiceError(format!(
                    "Failed to list prompts from service '{}': {}",
                    server_name, e
                ))
            })?;

        // Process prompts
        if !prompts.is_empty() {
            tracing::info!(
                "Got {} prompts from service '{}'",
                prompts.len(),
                server_name
            );
            self.set_prompts_cache_entry(server_name, prompts.clone())
                .await;
            tracing::info!(
                "Updated in-memory prompts list for service '{}'",
                server_name
            );
        } else {
            tracing::debug!("Service '{}' has no available prompts", server_name);
        }

        Ok(())
    }

    /// Get resources list for server (from in-memory cache)
    pub async fn list_mcp_server_resources(
        &self,
        server_name: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<crate::types::McpResourceInfo>> {
        // Try cache first
        {
            if let Some(list) = self.get_cached_resources_infos(server_name).await {
                return Ok(list);
            }
        }

        // Not cached: sync from service and cache
        self.sync_server_resources_from_service(server_name, app_handle)
            .await?;

        let resources = self
            .get_cached_resources_infos(server_name)
            .await
            .unwrap_or_default();
        Ok(resources)
    }

    /// Get prompts list for server (from in-memory cache)
    pub async fn list_mcp_server_prompts(
        &self,
        server_name: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<crate::types::McpPromptInfo>> {
        // Try cache first
        {
            if let Some(list) = self.get_cached_prompts_infos(server_name).await {
                return Ok(list);
            }
        }

        // Not cached: sync from service and cache
        self.sync_server_prompts_from_service(server_name, app_handle)
            .await?;

        let prompts = self
            .get_cached_prompts_infos(server_name)
            .await
            .unwrap_or_default();
        Ok(prompts)
    }

    /// Get all available tools from all cached servers for permission management
    pub async fn get_all_available_tools(&self) -> Vec<String> {
        let entries = self.tools_cache_entries.read().await;
        let mut all_tools = Vec::new();

        for (server_name, cache_entry) in entries.iter() {
            for tool_info in &cache_entry.infos {
                all_tools.push(format!("{}/{}", server_name, tool_info.name));
            }
        }

        all_tools.sort();
        all_tools
    }

    /// Get all available tools with descriptions for permission management
    pub async fn get_all_available_tools_with_descriptions(&self) -> Vec<(String, String)> {
        let entries = self.tools_cache_entries.read().await;
        let mut all_tools = Vec::new();

        for (server_name, cache_entry) in entries.iter() {
            for tool_info in &cache_entry.infos {
                let tool_id = format!("{}/{}", server_name, tool_info.name);
                let description = tool_info.description.clone();
                all_tools.push((tool_id, description));
            }
        }

        all_tools.sort_by(|a, b| a.0.cmp(&b.0));
        all_tools
    }

    /// Get all available resources from all cached servers for permission management
    pub async fn get_all_available_resources(&self) -> Vec<String> {
        let entries = self.resources_cache_entries.read().await;
        let mut all_resources = Vec::new();

        for (server_name, cache_entry) in entries.iter() {
            for resource_info in &cache_entry.infos {
                all_resources.push(format!("{}/{}", server_name, resource_info.name));
            }
        }

        all_resources.sort();
        all_resources
    }

    /// Get all available resources with descriptions for permission management
    pub async fn get_all_available_resources_with_descriptions(&self) -> Vec<(String, String)> {
        let entries = self.resources_cache_entries.read().await;
        let mut all_resources = Vec::new();

        for (server_name, cache_entry) in entries.iter() {
            for resource_info in &cache_entry.infos {
                let resource_id = format!("{}/{}", server_name, resource_info.name);
                let description = resource_info.description.clone().unwrap_or_default();
                all_resources.push((resource_id, description));
            }
        }

        all_resources.sort_by(|a, b| a.0.cmp(&b.0));
        all_resources
    }

    /// Get all available prompts from all cached servers for permission management
    pub async fn get_all_available_prompts(&self) -> Vec<String> {
        let entries = self.prompts_cache_entries.read().await;
        let mut all_prompts = Vec::new();

        for (server_name, cache_entry) in entries.iter() {
            for prompt_info in &cache_entry.infos {
                all_prompts.push(format!("{}/{}", server_name, prompt_info.name));
            }
        }

        all_prompts.sort();
        all_prompts
    }

    /// Get all available prompts with descriptions for permission management
    pub async fn get_all_available_prompts_with_descriptions(&self) -> Vec<(String, String)> {
        let entries = self.prompts_cache_entries.read().await;
        let mut all_prompts = Vec::new();

        for (server_name, cache_entry) in entries.iter() {
            for prompt_info in &cache_entry.infos {
                let prompt_id = format!("{}/{}", server_name, prompt_info.name);
                let description = prompt_info.description.clone().unwrap_or_default();
                all_prompts.push((prompt_id, description));
            }
        }

        all_prompts.sort_by(|a, b| a.0.cmp(&b.0));
        all_prompts
    }
}

#[derive(Clone)]
pub struct ToolsCacheEntry {
    pub raw: Vec<rmcp::model::Tool>,
    pub infos: Vec<crate::types::McpToolInfo>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub count: usize,
}

#[derive(Clone)]
pub struct ResourcesCacheEntry {
    pub raw: Vec<rmcp::model::Resource>,
    pub infos: Vec<crate::types::McpResourceInfo>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub count: usize,
}

#[derive(Clone)]
pub struct PromptsCacheEntry {
    pub raw: Vec<rmcp::model::Prompt>,
    pub infos: Vec<crate::types::McpPromptInfo>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub count: usize,
}
