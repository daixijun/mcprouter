use super::{ConfigError, Result};
use crate::types::McpServerConfig;

/// MCP Server Repository
#[derive(Debug, Clone)]
pub struct McpServerRepository {
    servers: Vec<crate::types::McpServerConfig>,
}

impl McpServerRepository {
    /// Create new MCP server repository (load from AppConfig)
    pub async fn new(_app_handle: &tauri::AppHandle) -> Result<Self> {
        // Load servers directly from AppConfig instead of separate files
        let config = crate::AppConfig::load().map_err(|e| {
            tracing::error!("Failed to load AppConfig: {}", e);
            ConfigError::Invalid(format!("Cannot load AppConfig: {}", e))
        })?;

        tracing::debug!("Creating fresh McpServerRepository (from AppConfig)...");
        tracing::info!(
            "✅ Loaded {} server configs from AppConfig",
            config.mcp_servers.len()
        );

        let repository = Self {
            servers: config.mcp_servers,
        };

        Ok(repository)
    }

    /// Get all servers
    pub fn get_all(&self) -> &[crate::types::McpServerConfig] {
        &self.servers
    }

    /// Get server by name
    pub fn get_by_name(&self, name: &str) -> Option<&crate::types::McpServerConfig> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// Add new server
    pub async fn add(&mut self, config: McpServerConfig) -> Result<String> {
        tracing::info!("Starting to add MCP server: {}", config.name);

        // check if name already exists
        if self.get_by_name(&config.name).is_some() {
            tracing::warn!("server name already exists: {}", config.name);
            return Err(ConfigError::Invalid(format!(
                "server name already exists: {}",
                config.name
            )));
        }

        let mut server_file = crate::types::McpServerConfig {
            name: config.name.clone(),
            description: config.description,
            command: config.command,
            args: config.args,
            env: config.env.clone(),
            transport: config.transport.clone(),
            url: config.url,
            headers: config.headers,
            enabled: config.enabled,
        };

        // Clean fields: remove irrelevant fields based on transport type
        server_file.clean_fields();
        tracing::info!(
            "✅ field cleanup completed，transport type: {:?}",
            server_file.transport
        );

        // Add to in-memory list
        self.servers.push(server_file.clone());

        // Save to AppConfig
        let mut app_config = crate::AppConfig::load()?;
        app_config.mcp_servers = self.servers.clone();
        app_config.save()?;

        tracing::info!("Config saved successfully to AppConfig");

        Ok(format!("MCP server '{}' added successfully", config.name))
    }

    // Version persistence removed

    /// Update server
    pub async fn update(&mut self, name: &str, config: McpServerConfig) -> Result<String> {
        // Clone required fields first to avoid borrow conflicts
        let server_name = config.name.clone();

        // modify data structure
        {
            let server = self
                .get_by_name_mut(&server_name)
                .ok_or_else(|| ConfigError::Invalid(format!("server not found: {}", name)))?;

            server.name = config.name.clone();
            server.description = config.description;
            server.command = config.command;
            server.args = config.args;
            server.transport = config.transport.clone();
            server.url = config.url;
            server.enabled = config.enabled;
            server.env = config.env;
            server.headers = config.headers;

            // cleaning fields（bytransport type）
            server.clean_fields();
        }

        // Save to AppConfig
        let mut app_config = crate::AppConfig::load()?;
        app_config.mcp_servers = self.servers.clone();
        app_config.save()?;

        Ok(format!("MCP server '{}' updated successfully", server_name))
    }

    /// Delete server
    pub async fn delete(&mut self, name: &str) -> Result<String> {
        // find server
        let server_index = self
            .servers
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| ConfigError::Invalid(format!("server not found: {}", name)))?;

        // remove from memory
        self.servers.remove(server_index);

        // Save to AppConfig
        let mut app_config = crate::AppConfig::load()?;
        app_config.mcp_servers = self.servers.clone();
        app_config.save()?;

        Ok(format!("MCP server '{}' deleted", name))
    }

    /// Toggle server enabled status
    pub async fn toggle_enabled(&mut self, name: &str) -> Result<bool> {
        // Find server first
        let server = self
            .get_by_name(name)
            .ok_or_else(|| ConfigError::Invalid(format!("server not found: {}", name)))?;

        let new_state = !server.enabled;

        // Get modifiable reference and update
        let server_mut = self.servers.iter_mut().find(|s| s.name == name).unwrap();
        server_mut.enabled = new_state;

        // cleaning fields（bytransport type）
        server_mut.clean_fields();

        // Save to AppConfig
        let mut app_config = crate::AppConfig::load()?;
        app_config.mcp_servers = self.servers.clone();
        app_config.save()?;

        tracing::info!(
            "✅ Server '{}' enabled status updated to: {}",
            name,
            new_state
        );

        Ok(new_state)
    }

    // Tool persistence removed

    /// Get modifiable server by name
    fn get_by_name_mut(&mut self, name: &str) -> Option<&mut crate::types::McpServerConfig> {
        self.servers.iter_mut().find(|s| s.name == name)
    }
}
