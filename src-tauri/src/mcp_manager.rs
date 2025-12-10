// MCP Server Management - SQLite Storage Version

use crate::error::{McpError, Result};

// MCP Server Manager with SQLite backend
use crate::storage::mcp_server_storage::McpServerStorage;
use crate::types::{
    McpPromptInfo, McpResourceInfo, McpServerConfig, McpServerInfo, McpToolInfo,
    ServiceVersionCache,
};
use crate::MCP_CLIENT_MANAGER;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct McpServerManager {
    storage: Arc<McpServerStorage>,
    tools_cache: Arc<RwLock<HashMap<String, Vec<McpToolInfo>>>>,
    resources_cache: Arc<RwLock<HashMap<String, Vec<McpResourceInfo>>>>,
    prompts_cache: Arc<RwLock<HashMap<String, Vec<McpPromptInfo>>>>,
    version_cache: Arc<RwLock<HashMap<String, ServiceVersionCache>>>,
    tools_cache_ttl: std::time::Duration,
}

impl McpServerManager {
    pub fn new(storage: McpServerStorage) -> Self {
        Self {
            storage: Arc::new(storage),
            tools_cache: Arc::new(RwLock::new(HashMap::new())),
            resources_cache: Arc::new(RwLock::new(HashMap::new())),
            prompts_cache: Arc::new(RwLock::new(HashMap::new())),
            version_cache: Arc::new(RwLock::new(HashMap::new())),
            tools_cache_ttl: std::time::Duration::from_secs(600),
        }
    }

    pub fn get_tools_cache_ttl_seconds(&self) -> u64 {
        self.tools_cache_ttl.as_secs()
    }

    // ============================================================================
    // Cache Management Methods
    // ============================================================================

    pub async fn set_tools_cache_entry(&self, server_name: &str, tools: Vec<rmcp::model::Tool>) {
        let now = chrono::Utc::now();
        let infos: Vec<McpToolInfo> = tools
            .iter()
            .map(|tool| McpToolInfo {
                id: tool.name.to_string(),
                name: tool.name.to_string(),
                description: tool.description.clone().unwrap_or_default().to_string(),
                enabled: true,
                created_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            })
            .collect();

        // Update in-memory cache
        let mut cache = self.tools_cache.write().await;
        cache.insert(server_name.to_string(), infos.clone());

        // Persist to SQLite
        if let Err(e) = self.storage.cache_server_tools(server_name, &tools).await {
            tracing::error!("Failed to persist tools cache to SQLite: {}", e);
        }

        tracing::debug!("Cached {} tools for server '{}'", tools.len(), server_name);
    }

    pub async fn get_cached_tools(&self, server_name: &str) -> Option<Vec<McpToolInfo>> {
        // Try in-memory cache first
        {
            let cache = self.tools_cache.read().await;
            if let Some(tools) = cache.get(server_name) {
                return Some(tools.clone());
            }
        }

        // Try loading from SQLite
        match self.storage.get_cached_server_tools(server_name).await {
            Ok(tools) if !tools.is_empty() => {
                // Update in-memory cache
                let mut cache = self.tools_cache.write().await;
                cache.insert(server_name.to_string(), tools.clone());
                Some(tools)
            }
            _ => None,
        }
    }

    // Resources cache management
    pub async fn set_resources_cache_entry(
        &self,
        server_name: &str,
        resources: Vec<rmcp::model::Resource>,
    ) {
        let now = chrono::Utc::now();
        let infos: Vec<McpResourceInfo> = resources
            .iter()
            .map(|resource| McpResourceInfo {
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

        // Update in-memory cache
        let mut cache = self.resources_cache.write().await;
        cache.insert(server_name.to_string(), infos.clone());

        // Persist to SQLite
        if let Err(e) = self
            .storage
            .cache_server_resources(server_name, &resources)
            .await
        {
            tracing::error!("Failed to persist resources cache to SQLite: {}", e);
        }

        tracing::debug!(
            "Cached {} resources for server '{}'",
            resources.len(),
            server_name
        );
    }

    pub async fn get_cached_resources(&self, server_name: &str) -> Option<Vec<McpResourceInfo>> {
        // Try in-memory cache first
        {
            let cache = self.resources_cache.read().await;
            if let Some(resources) = cache.get(server_name) {
                return Some(resources.clone());
            }
        }

        // Try loading from SQLite
        match self.storage.get_cached_server_resources(server_name).await {
            Ok(resources) if !resources.is_empty() => {
                // Update in-memory cache
                let mut cache = self.resources_cache.write().await;
                cache.insert(server_name.to_string(), resources.clone());
                Some(resources)
            }
            _ => None,
        }
    }

    // Prompts cache management
    pub async fn set_prompts_cache_entry(
        &self,
        server_name: &str,
        prompts: Vec<rmcp::model::Prompt>,
    ) {
        let now = chrono::Utc::now();
        let infos: Vec<McpPromptInfo> = prompts
            .iter()
            .map(|prompt| McpPromptInfo {
                id: prompt.name.to_string(),
                name: prompt.name.to_string(),
                description: prompt.description.clone(),
                enabled: true,
                created_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            })
            .collect();

        // Update in-memory cache
        let mut cache = self.prompts_cache.write().await;
        cache.insert(server_name.to_string(), infos.clone());

        // Persist to SQLite
        if let Err(e) = self
            .storage
            .cache_server_prompts(server_name, &prompts)
            .await
        {
            tracing::error!("Failed to persist prompts cache to SQLite: {}", e);
        }

        tracing::debug!(
            "Cached {} prompts for server '{}'",
            prompts.len(),
            server_name
        );
    }

    pub async fn get_cached_prompts(&self, server_name: &str) -> Option<Vec<McpPromptInfo>> {
        // Try in-memory cache first
        {
            let cache = self.prompts_cache.read().await;
            if let Some(prompts) = cache.get(server_name) {
                return Some(prompts.clone());
            }
        }

        // Try loading from SQLite
        match self.storage.get_cached_server_prompts(server_name).await {
            Ok(prompts) if !prompts.is_empty() => {
                // Update in-memory cache
                let mut cache = self.prompts_cache.write().await;
                cache.insert(server_name.to_string(), prompts.clone());
                Some(prompts)
            }
            _ => None,
        }
    }

    /// Clear all cached data for a specific server
    pub async fn clear_server_cache(&self, server_name: &str) -> Result<()> {
        // Clear from in-memory cache
        {
            let mut tools_cache = self.tools_cache.write().await;
            tools_cache.remove(server_name);
        }
        {
            let mut resources_cache = self.resources_cache.write().await;
            resources_cache.remove(server_name);
        }
        {
            let mut prompts_cache = self.prompts_cache.write().await;
            prompts_cache.remove(server_name);
        }
        {
            let mut version_cache = self.version_cache.write().await;
            version_cache.remove(server_name);
        }

        // Clear from SQLite
        self.storage.clear_server_cache(server_name).await?;

        tracing::info!("Cleared all cache data for server '{}'", server_name);
        Ok(())
    }

    // ============================================================================
    // Server Configuration Management
    // ============================================================================

    pub async fn load_mcp_servers(&self) -> Result<()> {
        // Load servers from SQLite (no need to sync with config file)
        let servers = self.storage.get_all_servers().await?;
        tracing::info!("Loaded {} MCP servers from SQLite", servers.len());
        Ok(())
    }

    /// Get server ID by server name
    pub async fn get_server_id_by_name(&self, server_name: &str) -> Result<String> {
        self.storage.get_server_id_by_name(server_name).await.map_err(|e| {
            McpError::DatabaseQueryError(format!("Failed to get server ID for '{}': {}", server_name, e))
        })
    }

    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>> {
        let servers = self.storage.get_all_servers().await?;
        let mut result = Vec::new();

        for server_config in servers {
            let server_name = server_config.name.clone();

            // Get connection status from MCP client manager
            let (status_string, error_message) =
                MCP_CLIENT_MANAGER.get_connection_status(&server_name).await;
            let final_status = if !server_config.enabled {
                "disconnected".to_string()
            } else {
                status_string
            };

            // Get version from cache
            let version = {
                let cache = self.version_cache.read().await;
                cache.get(&server_name).and_then(|v| v.version.clone())
            };

            // Get tool count from cache
            let tool_count = {
                let cache = self.tools_cache.read().await;
                cache.get(&server_name).map(|t| t.len())
            };

            // Get resource count from cache
            let resource_count = {
                let cache = self.resources_cache.read().await;
                cache.get(&server_name).map(|r| r.len())
            };

            // Get prompt count from cache
            let prompt_count = {
                let cache = self.prompts_cache.read().await;
                cache.get(&server_name).map(|p| p.len())
            };

            // Set different fields based on transport type
            let (transport_str, url, headers, command, args, env_data) =
                match server_config.transport {
                    crate::types::ServiceTransport::Stdio => (
                        "stdio".to_string(),
                        None,
                        None,
                        server_config.command.clone(),
                        server_config.args.clone(),
                        server_config.env.clone(),
                    ),
                    crate::types::ServiceTransport::Http => (
                        "http".to_string(),
                        server_config.url.clone(),
                        server_config.headers.clone(),
                        None,
                        None,
                        None,
                    ),
                };

            result.push(McpServerInfo {
                name: server_name,
                enabled: server_config.enabled,
                status: final_status,
                version,
                error_message,
                transport: transport_str,
                url,
                description: server_config.description,
                env: env_data,
                headers,
                command,
                args,
                tool_count,
                resource_count,
                prompt_count,
                prompt_template_count: None, // Will be calculated later if needed
            });
        }

        Ok(result)
    }

    pub async fn add_mcp_server(&self, config: McpServerConfig) -> Result<()> {
        tracing::info!("Adding MCP server '{}'", config.name);

        // Add to SQLite storage
        self.storage.add_server(&config).await?;

        // Try to connect to service to get version and capabilities
        if let Err(e) = self.check_service_with_version(&config.name).await {
            tracing::warn!("Failed to connect to service '{}': {}", config.name, e);
        }

        // Sync capabilities
        if let Err(e) = self.sync_server_capabilities(&config.name).await {
            tracing::warn!(
                "Failed to sync capabilities for service '{}': {}",
                config.name,
                e
            );
        }

        tracing::info!("✅ MCP server '{}' added successfully", config.name);
        Ok(())
    }

    pub async fn update_mcp_server(&self, config: McpServerConfig) -> Result<()> {
        tracing::info!("Updating MCP server '{}'", config.name);

        // Update in SQLite storage
        self.storage.update_server(&config.name, &config).await?;

        // Clear cache for the updated server
        self.clear_server_cache(&config.name).await?;

        // If service is enabled, reconnect and sync capabilities
        if config.enabled {
            if let Err(e) = self.check_service_with_version(&config.name).await {
                tracing::warn!(
                    "Failed to connect to updated service '{}': {}",
                    config.name,
                    e
                );
            } else {
                if let Err(e) = self.sync_server_capabilities(&config.name).await {
                    tracing::warn!(
                        "Failed to sync capabilities for service '{}': {}",
                        config.name,
                        e
                    );
                }
            }
        } else {
            // Disconnect from disabled service
            if let Err(e) = MCP_CLIENT_MANAGER.disconnect_mcp_server(&config.name).await {
                tracing::warn!(
                    "Failed to disconnect from disabled service '{}': {}",
                    config.name,
                    e
                );
            }
        }

        tracing::info!("✅ MCP server '{}' updated successfully", config.name);
        Ok(())
    }

    pub async fn remove_mcp_server(&self, name: &str) -> Result<()> {
        tracing::info!("Removing MCP server '{}'", name);

        // Disconnect from service
        if let Err(e) = MCP_CLIENT_MANAGER.disconnect_mcp_server(name).await {
            tracing::warn!("Failed to disconnect from service '{}': {}", name, e);
        }

        // Clear all cache
        self.clear_server_cache(name).await?;

        // Remove from SQLite storage
        self.storage.delete_server(name).await?;

        tracing::info!("✅ MCP server '{}' removed successfully", name);
        Ok(())
    }

    pub async fn toggle_mcp_server(&self, name: &str) -> Result<bool> {
        tracing::info!("Toggling MCP server '{}'", name);

        // Update in SQLite storage
        let new_state = self.storage.toggle_server_enabled(name).await?;

        if new_state {
            // Service enabled: connect and sync capabilities
            if let Err(e) = self.check_service_with_version(name).await {
                tracing::warn!("Failed to connect to enabled service '{}': {}", name, e);
            } else {
                if let Err(e) = self.sync_server_capabilities(name).await {
                    tracing::warn!("Failed to sync capabilities for service '{}': {}", name, e);
                }
            }
        } else {
            // Service disabled: disconnect and clear cache
            if let Err(e) = MCP_CLIENT_MANAGER.disconnect_mcp_server(name).await {
                tracing::warn!(
                    "Failed to disconnect from disabled service '{}': {}",
                    name,
                    e
                );
            }
            self.clear_server_cache(name).await?;
        }

        tracing::info!(
            "✅ MCP server '{}' toggled to {}",
            name,
            if new_state { "enabled" } else { "disabled" }
        );
        Ok(new_state)
    }

    // ============================================================================
    // Server Status and Capabilities
    // ============================================================================

    async fn check_service_with_version(&self, name: &str) -> Result<()> {
        // Get server configuration
        let server_config = self
            .storage
            .get_server_by_name(name)
            .await?
            .ok_or_else(|| McpError::ServiceNotFound(name.to_string()))?;

        // Connect to service
        let connection = MCP_CLIENT_MANAGER
            .ensure_connection(&server_config, false)
            .await
            .map_err(|e| {
                McpError::ConnectionError(format!("Failed to connect to service '{}': {}", name, e))
            })?;

        // Extract version info
        if let Some(ref _client) = connection.client {
            if let Some(info) = &connection.server_info {
                let version = info.server_info.version.clone();
                let version_clone = version.clone();

                // Update version cache
                {
                    let mut version_cache = self.version_cache.write().await;
                    version_cache.insert(
                        name.to_string(),
                        ServiceVersionCache {
                            version: Some(version.clone()),
                        },
                    );
                }

                // Persist to SQLite
                if let Err(e) = self
                    .storage
                    .update_server_version(name, Some(version))
                    .await
                {
                    tracing::error!("Failed to persist version to SQLite: {}", e);
                }

                tracing::info!("Updated version for service '{}': {}", name, version_clone);
            }
        }

        Ok(())
    }

    pub async fn sync_server_capabilities(&self, name: &str) -> Result<()> {
        // Get server configuration
        let server_config = self
            .storage
            .get_server_by_name(name)
            .await?
            .ok_or_else(|| McpError::ServiceNotFound(name.to_string()))?;

        // Connect to service
        let _connection = MCP_CLIENT_MANAGER
            .ensure_connection(&server_config, false)
            .await
            .map_err(|e| {
                McpError::ConnectionError(format!("Failed to connect to service '{}': {}", name, e))
            })?;

        // Sync tools
        match MCP_CLIENT_MANAGER.list_tools(name).await {
            Ok(tools) if !tools.is_empty() => {
                self.set_tools_cache_entry(name, tools).await;
                tracing::info!("Synced tools for service '{}'", name);
            }
            Ok(_) => {
                tracing::debug!("Service '{}' has no tools", name);
            }
            Err(e) => {
                tracing::warn!("Failed to sync tools for service '{}': {}", name, e);
            }
        }

        // Sync resources
        match MCP_CLIENT_MANAGER.list_resources(name).await {
            Ok(resources) if !resources.is_empty() => {
                self.set_resources_cache_entry(name, resources).await;
                tracing::info!("Synced resources for service '{}'", name);
            }
            Ok(_) => {
                tracing::debug!("Service '{}' has no resources", name);
            }
            Err(e) => {
                tracing::warn!("Failed to sync resources for service '{}': {}", name, e);
            }
        }

        // Sync prompts
        match MCP_CLIENT_MANAGER.list_prompts(name).await {
            Ok(prompts) if !prompts.is_empty() => {
                self.set_prompts_cache_entry(name, prompts).await;
                tracing::info!("Synced prompts for service '{}'", name);
            }
            Ok(_) => {
                tracing::debug!("Service '{}' has no prompts", name);
            }
            Err(e) => {
                tracing::warn!("Failed to sync prompts for service '{}': {}", name, e);
            }
        }

        Ok(())
    }

    pub async fn get_mcp_server_tools(&self, server_name: &str) -> Result<Vec<McpToolInfo>> {
        // Try cache first
        if let Some(tools) = self.get_cached_tools(server_name).await {
            return Ok(tools);
        }

        // Not cached: sync from service
        self.sync_server_capabilities(server_name).await?;

        // Return from cache (now should be populated)
        Ok(self.get_cached_tools(server_name).await.unwrap_or_default())
    }

    /// Toggle tool enabled state
    pub async fn toggle_tool_enabled(
        &self,
        server_name: &str,
        tool_name: &str,
        enabled: bool,
    ) -> Result<()> {
        let mut cache = self.tools_cache.write().await;

        if let Some(tools) = cache.get_mut(server_name) {
            if let Some(tool) = tools.iter_mut().find(|t| t.name == tool_name) {
                tool.enabled = enabled;
                tool.updated_at = chrono::Utc::now().to_rfc3339();
                tracing::info!("Tool '{}' on server '{}' set to enabled: {}", tool_name, server_name, enabled);
                return Ok(());
            }
        }

        Err(McpError::NotFound(format!(
            "Tool '{}' not found on server '{}'",
            tool_name, server_name
        )))
    }

    /// Enable all tools for a server
    pub async fn enable_all_tools(&self, server_name: &str) -> Result<()> {
        let mut cache = self.tools_cache.write().await;

        if let Some(tools) = cache.get_mut(server_name) {
            let now = chrono::Utc::now().to_rfc3339();
            for tool in tools.iter_mut() {
                tool.enabled = true;
                tool.updated_at = now.clone();
            }
            tracing::info!("All tools enabled for server '{}'", server_name);
            return Ok(());
        }

        Err(McpError::NotFound(format!(
            "No tools found for server '{}'",
            server_name
        )))
    }

    /// Disable all tools for a server
    pub async fn disable_all_tools(&self, server_name: &str) -> Result<()> {
        let mut cache = self.tools_cache.write().await;

        if let Some(tools) = cache.get_mut(server_name) {
            let now = chrono::Utc::now().to_rfc3339();
            for tool in tools.iter_mut() {
                tool.enabled = false;
                tool.updated_at = now.clone();
            }
            tracing::info!("All tools disabled for server '{}'", server_name);
            return Ok(());
        }

        Err(McpError::NotFound(format!(
            "No tools found for server '{}'",
            server_name
        )))
    }

    pub async fn get_mcp_server_resources(
        &self,
        server_name: &str,
    ) -> Result<Vec<McpResourceInfo>> {
        // Try cache first
        if let Some(resources) = self.get_cached_resources(server_name).await {
            return Ok(resources);
        }

        // Not cached: sync from service
        self.sync_server_capabilities(server_name).await?;

        // Return from cache (now should be populated)
        Ok(self
            .get_cached_resources(server_name)
            .await
            .unwrap_or_default())
    }

    pub async fn get_mcp_server_prompts(&self, server_name: &str) -> Result<Vec<McpPromptInfo>> {
        // Try cache first
        if let Some(prompts) = self.get_cached_prompts(server_name).await {
            return Ok(prompts);
        }

        // Not cached: sync from service
        self.sync_server_capabilities(server_name).await?;

        // Return from cache (now should be populated)
        Ok(self
            .get_cached_prompts(server_name)
            .await
            .unwrap_or_default())
    }

    // ============================================================================
    // Auto-connect on Startup
    // ============================================================================

    pub async fn auto_connect_enabled_services(&self) -> Result<()> {
        let servers = self.storage.get_all_servers().await?;
        let enabled_servers: Vec<McpServerConfig> =
            servers.into_iter().filter(|s| s.enabled).collect();

        if enabled_servers.is_empty() {
            tracing::info!("No enabled MCP services need connection");
            return Ok(());
        }

        tracing::info!(
            "Auto-connecting {} enabled MCP services...",
            enabled_servers.len()
        );

        let mut success_count = 0;
        let mut failed_count = 0;

        for server_config in enabled_servers {
            let server_name = server_config.name.clone();

            match MCP_CLIENT_MANAGER.try_reconnect(&server_config).await {
                Ok(true) => {
                    tracing::info!("Service '{}' connected successfully", server_name);

                    // Get version and capabilities
                    if let Err(e) = self.check_service_with_version(&server_name).await {
                        tracing::warn!(
                            "Failed to get version info for service '{}': {}",
                            server_name,
                            e
                        );
                    }

                    if let Err(e) = self.sync_server_capabilities(&server_name).await {
                        tracing::warn!(
                            "Failed to sync capabilities for service '{}': {}",
                            server_name,
                            e
                        );
                    }

                    success_count += 1;
                }
                Ok(false) => {
                    tracing::error!("Service '{}' connection failed", server_name);
                    failed_count += 1;
                }
                Err(e) => {
                    tracing::error!("Service '{}' connection error: {}", server_name, e);
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

    // ============================================================================
    // Permission Management Support
    // ============================================================================

    /// Get all available tools from all cached servers for permission management
    pub async fn get_all_available_tools(&self) -> Vec<String> {
        let cache = self.tools_cache.read().await;
        let mut all_tools = Vec::new();

        for (server_name, tools) in cache.iter() {
            for tool_info in tools {
                all_tools.push(format!("{}__{}", server_name, tool_info.name));
            }
        }

        all_tools.sort();
        all_tools
    }

    /// Get all available tools with descriptions for permission management
    pub async fn get_all_available_tools_with_descriptions(&self) -> Vec<(String, String, String, String, String)> {
        // Get tools directly from storage with database IDs
        match self.storage.get_all_tools_for_permissions().await {
            Ok(tools_data) => {
                // Convert storage format to our return format
                let mut result = Vec::new();
                for (tool_id, tool_name, server_name) in tools_data {
                    // Get server ID by name
                    let server_id = match self.get_server_id_by_name(&server_name).await {
                        Ok(id) => id,
                        Err(_) => server_name.clone(), // Fallback to server name if ID not found
                    };

                    // Get tool description from cache
                    let description = if let Some(cached_tools) = self.tools_cache.read().await.get(&server_name) {
                        if let Some(tool_info) = cached_tools.iter().find(|t| t.name == tool_name) {
                            tool_info.description.clone()
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    };

                    result.push((tool_id, tool_name, description, server_id, server_name));
                }
                result.sort_by(|a, b| a.0.cmp(&b.0));
                result
            }
            Err(_) => {
                // Fallback to cache-based approach if storage query fails
                let cache = self.tools_cache.read().await;
                let mut all_tools = Vec::new();

                for (server_name, tools) in cache.iter() {
                    // Get server ID by name
                    let server_id = match self.get_server_id_by_name(server_name).await {
                        Ok(id) => id,
                        Err(_) => server_name.clone(), // Fallback to server name if ID not found
                    };

                    for tool_info in tools {
                        // Try to get tool UUID from database instead of using server__name format
                        let tool_id = match self.storage.get_tool_id_by_name(&server_name, &tool_info.name).await {
                            Ok(id) => id,
                            Err(_) => {
                                // Last resort: generate a deterministic ID based on server_name and tool_name
                                // Use SHA256 hash to create a consistent ID
                                use sha2::{Sha256, Digest};
                                let mut hasher = Sha256::new();
                                hasher.update(format!("tool:{}:{}", server_name, tool_info.name));
                                let result = hasher.finalize();
                                format!("{:x}", result)
                            }
                        };
                        let name = tool_info.name.clone();
                        let description = tool_info.description.clone();
                        all_tools.push((tool_id, name, description, server_id.clone(), server_name.clone()));
                    }
                }

                all_tools.sort_by(|a, b| a.0.cmp(&b.0));
                all_tools
            }
        }
    }

    /// Get all available resources from all cached servers for permission management
    pub async fn get_all_available_resources(&self) -> Vec<String> {
        let cache = self.resources_cache.read().await;
        let mut all_resources = Vec::new();

        for (server_name, resources) in cache.iter() {
            for resource_info in resources {
                all_resources.push(format!("{}__{}", server_name, resource_info.uri));
            }
        }

        all_resources.sort();
        all_resources
    }

    /// Get all available resources with descriptions for permission management
    pub async fn get_all_available_resources_with_descriptions(&self) -> Vec<(String, String, String, String, String)> {
        // Get resources directly from storage with database IDs
        match self.storage.get_all_resources_for_permissions().await {
            Ok(resources_data) => {
                // Convert storage format to our return format
                let mut result = Vec::new();
                for (resource_id, resource_name, server_name) in resources_data {
                    // Get server ID by name
                    let server_id = match self.get_server_id_by_name(&server_name).await {
                        Ok(id) => id,
                        Err(_) => server_name.clone(), // Fallback to server name if ID not found
                    };

                    // Get resource description from cache
                    let description = if let Some(cached_resources) = self.resources_cache.read().await.get(&server_name) {
                        if let Some(resource_info) = cached_resources.iter().find(|r| r.name == resource_name) {
                            resource_info
                                .description
                                .clone()
                                .filter(|d| !d.is_empty())
                                .or_else(|| {
                                    if resource_info.name.is_empty() {
                                        None
                                    } else {
                                        Some(resource_info.name.clone())
                                    }
                                })
                                .unwrap_or_else(|| resource_info.uri.clone())
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    };

                    result.push((resource_id, resource_name, description, server_id, server_name));
                }
                result.sort_by(|a, b| a.0.cmp(&b.0));
                result
            }
            Err(_) => {
                // Fallback to cache-based approach if storage query fails
                let cache = self.resources_cache.read().await;
                let mut all_resources = Vec::new();

                for (server_name, resources) in cache.iter() {
                    // Get server ID by name
                    let server_id = match self.get_server_id_by_name(server_name).await {
                        Ok(id) => id,
                        Err(_) => server_name.clone(), // Fallback to server name if ID not found
                    };

                    for resource_info in resources {
                        // Try to get resource UUID from database instead of using server__uri format
                        let resource_id = match self.storage.get_resource_id_by_uri(&server_name, &resource_info.uri).await {
                            Ok(id) => id,
                            Err(_) => {
                                // Last resort: generate a deterministic ID based on server_name and resource_uri
                                // Use SHA256 hash to create a consistent ID
                                use sha2::{Sha256, Digest};
                                let mut hasher = Sha256::new();
                                hasher.update(format!("resource:{}:{}", server_name, resource_info.uri));
                                let result = hasher.finalize();
                                format!("{:x}", result)
                            }
                        };
                        let name = resource_info.name.clone();
                        let description = resource_info
                            .description
                            .clone()
                            .filter(|d| !d.is_empty())
                            .or_else(|| {
                                if resource_info.name.is_empty() {
                                    None
                                } else {
                                    Some(resource_info.name.clone())
                                }
                            })
                            .unwrap_or_else(|| resource_info.uri.clone());
                        all_resources.push((resource_id, name, description, server_id.clone(), server_name.clone()));
                    }
                }

                all_resources.sort_by(|a, b| a.0.cmp(&b.0));
                all_resources
            }
        }
    }

    /// Get all available prompts from all cached servers for permission management
    pub async fn get_all_available_prompts(&self) -> Vec<String> {
        let cache = self.prompts_cache.read().await;
        let mut all_prompts = Vec::new();

        for (server_name, prompts) in cache.iter() {
            for prompt_info in prompts {
                all_prompts.push(format!("{}__{}", server_name, prompt_info.name));
            }
        }

        all_prompts.sort();
        all_prompts
    }

    /// Get all available prompts with descriptions for permission management
    pub async fn get_all_available_prompts_with_descriptions(&self) -> Vec<(String, String, String, String, String)> {
        // Get prompts directly from storage with database IDs
        match self.storage.get_all_prompts_for_permissions().await {
            Ok(prompts_data) => {
                // Convert storage format to our return format
                let mut result = Vec::new();
                for (prompt_id, prompt_name, server_name) in prompts_data {
                    // Get server ID by name
                    let server_id = match self.get_server_id_by_name(&server_name).await {
                        Ok(id) => id,
                        Err(_) => server_name.clone(), // Fallback to server name if ID not found
                    };

                    // Get prompt description from cache
                    let description = if let Some(cached_prompts) = self.prompts_cache.read().await.get(&server_name) {
                        if let Some(prompt_info) = cached_prompts.iter().find(|p| p.name == prompt_name) {
                            prompt_info.description.clone().unwrap_or_default()
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    };

                    result.push((prompt_id, prompt_name, description, server_id, server_name));
                }
                result.sort_by(|a, b| a.0.cmp(&b.0));
                result
            }
            Err(_) => {
                // Fallback to cache-based approach if storage query fails
                let cache = self.prompts_cache.read().await;
                let mut all_prompts = Vec::new();

                for (server_name, prompts) in cache.iter() {
                    // Get server ID by name
                    let server_id = match self.get_server_id_by_name(server_name).await {
                        Ok(id) => id,
                        Err(_) => server_name.clone(), // Fallback to server name if ID not found
                    };

                    for prompt_info in prompts {
                        // Use server__name format as fallback for ID
                        let prompt_id = format!("{}__{}", server_name, prompt_info.name);
                        let name = prompt_info.name.clone();
                        let description = prompt_info.description.clone().unwrap_or_default();
                        all_prompts.push((prompt_id, name, description, server_id.clone(), server_name.clone()));
                    }
                }

                all_prompts.sort_by(|a, b| a.0.cmp(&b.0));
                all_prompts
            }
        }
    }

    /// Get all available prompt templates from all cached servers for permission management
    pub async fn get_all_available_prompt_templates(&self) -> Vec<String> {
        let mut templates = Vec::new();

        // Use the storage method to get prompts with template info
        for server_name in self.tools_cache.read().await.keys() {
            if let Ok(prompts_with_templates) = self
                .storage
                .get_cached_server_prompts_with_templates(server_name)
                .await
            {
                for (prompt_info, is_template) in prompts_with_templates {
                    if is_template {
                        templates.push(format!("{}__{}", server_name, prompt_info.name));
                    }
                }
            }
        }

        templates.sort();
        templates
    }

    /// Get all available prompt templates with descriptions for permission management
    pub async fn get_all_available_prompt_templates_with_descriptions(
        &self,
    ) -> Vec<(String, String, String, String, String)> {
        // Get prompt templates directly from storage with database IDs
        match self.storage.get_all_prompt_templates_for_permissions().await {
            Ok(templates_data) => {
                // Convert storage format to our return format
                let mut result = Vec::new();
                for (template_id, template_name, server_name) in templates_data {
                    // Get server ID by name
                    let server_id = match self.get_server_id_by_name(&server_name).await {
                        Ok(id) => id,
                        Err(_) => server_name.clone(), // Fallback to server name if ID not found
                    };

                    // Get template description from cache
                    let description = if let Some(cached_prompts) = self.prompts_cache.read().await.get(&server_name) {
                        if let Some(prompt_info) = cached_prompts.iter().find(|p| p.name == template_name) {
                            prompt_info
                                .description
                                .clone()
                                .filter(|d| !d.is_empty())
                                .unwrap_or_else(|| "Template with arguments".to_string())
                        } else {
                            "Template with arguments".to_string()
                        }
                    } else {
                        "Template with arguments".to_string()
                    };

                    result.push((template_id, template_name, description, server_id, server_name));
                }
                result.sort_by(|a, b| a.0.cmp(&b.0));
                result
            }
            Err(_) => {
                // Fallback to cache-based approach if storage query fails
                let mut templates = Vec::new();

                // Use the storage method to get prompts with template info
                for server_name in self.tools_cache.read().await.keys() {
                    // Get server ID by name
                    let server_id = match self.get_server_id_by_name(server_name).await {
                        Ok(id) => id,
                        Err(_) => server_name.clone(), // Fallback to server name if ID not found
                    };

                    if let Ok(prompts_with_templates) = self
                        .storage
                        .get_cached_server_prompts_with_templates(server_name)
                        .await
                    {
                        for (prompt_info, is_template) in prompts_with_templates {
                            if is_template {
                                // Use server__name format as fallback for ID
                                let template_id = format!("{}__{}", server_name, prompt_info.name);
                                let name = prompt_info.name.clone();
                                let description = prompt_info
                                    .description
                                    .clone()
                                    .filter(|d| !d.is_empty())
                                    .unwrap_or_else(|| "Template with arguments".to_string());
                                templates.push((template_id, name, description, server_id.clone(), server_name.clone()));
                            }
                        }
                    }
                }

                templates.sort_by(|a, b| a.0.cmp(&b.0));
                templates
            }
        }
    }

    // ============================================================================
    // Raw Cache Access Methods (for trait compatibility)
    // ============================================================================

    /// Get cached tools for a server (raw format)
    pub async fn get_cached_tools_raw(&self, server_name: &str) -> Option<Vec<rmcp::model::Tool>> {
        // Try to get tools in info format first, then convert to raw format
        if let Some(tool_infos) = self.get_cached_tools(server_name).await {
            tracing::debug!("Converting {} tools from info format to raw format for server '{}'",
                tool_infos.len(), server_name);

            let raw_tools: Vec<rmcp::model::Tool> = tool_infos.into_iter().map(|info| {
                rmcp::model::Tool {
                    name: info.name.into(),
                    description: Some(info.description.into()),
                    input_schema: std::sync::Arc::new(serde_json::Map::new()),
                    // Default values for other fields
                    title: None,
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    meta: None,
                }
            }).collect();

            tracing::debug!("Successfully converted {} tools to raw format", raw_tools.len());
            Some(raw_tools)
        } else {
            tracing::debug!("No cached tools found for server '{}'", server_name);
            None
        }
    }

    /// Get cached resources for a server (raw format)
    pub async fn get_cached_resources_raw(
        &self,
        _server_name: &str,
    ) -> Option<Vec<rmcp::model::Resource>> {
        // For SQLite implementation, we don't store raw resources directly
        // This would require additional storage or conversion from info format
        // For now, return None to indicate raw access is not available
        None
    }

    /// Get cached prompts for a server (raw format)
    pub async fn get_cached_prompts_raw(
        &self,
        _server_name: &str,
    ) -> Option<Vec<rmcp::model::Prompt>> {
        // For SQLite implementation, we don't store raw prompts directly
        // This would require additional storage or conversion from info format
        // For now, return None to indicate raw access is not available
        None
    }

    // ========================================================================
    // Aggregator Compatibility Methods
    // ========================================================================

    /// Get tools cache entries for aggregator
    pub fn get_tools_cache_entries(&self) -> &dashmap::DashMap<String, Vec<rmcp::model::Tool>> {
        static EMPTY_CACHE: std::sync::OnceLock<dashmap::DashMap<String, Vec<rmcp::model::Tool>>> =
            std::sync::OnceLock::new();
        EMPTY_CACHE.get_or_init(|| dashmap::DashMap::new())
    }

    /// Get raw cached tools for aggregator
    pub async fn get_raw_cached_tools(&self, server_name: &str) -> Option<Vec<rmcp::model::Tool>> {
        self.get_cached_tools_raw(server_name).await
    }

    /// Get MCP servers for aggregator
    pub async fn get_mcp_servers(&self) -> Result<Vec<McpServerInfo>> {
        self.list_mcp_servers().await
    }

}

