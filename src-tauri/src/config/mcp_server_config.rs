use super::file_manager::{exists, read_dir, read_json, remove_file, write_json_atomic};
use super::{get_mcp_server_config_path, ConfigError, Result};
use crate::types::McpServerConfig;
use std::path::{Path, PathBuf};
use tauri::Manager;

/// MCP Server Repository
#[derive(Debug, Clone)]
pub struct McpServerRepository {
    app_data_dir: PathBuf,
    servers: Vec<crate::types::McpServerConfig>,
}

impl McpServerRepository {
    /// Create new MCP server repository (no caching - always fresh data)
    pub async fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| {
            tracing::error!("Failed to get application data directory: {:?}", e);
            ConfigError::Invalid("Cannot get application data directory".to_string())
        })?;

        // Always load fresh data - no caching for real-time configuration updates
        tracing::debug!("Creating fresh McpServerRepository (no caching)...");
        tracing::debug!("✅ Got application data directory: {:?}", app_data_dir);

        // Verify directory exists or can be created
        if !app_data_dir.exists() {
            tracing::warn!(
                "⚠️ Application data directory does not exist, trying to create: {:?}",
                app_data_dir
            );
            if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                tracing::error!("❌ Failed to create application data directory: {}", e);
            } else {
                tracing::info!("✅ Application data directory created successfully");
            }
        }

        let servers = Self::load_all_servers(&app_data_dir)?;
        tracing::info!("✅ Loaded {} server configs from files", servers.len());

        let repository = Self {
            app_data_dir: app_data_dir.clone(),
            servers,
        };

        Ok(repository)
    }

    /// Load all server configs
    fn load_all_servers(app_data_dir: &Path) -> Result<Vec<crate::types::McpServerConfig>> {
        let servers_dir = app_data_dir.join("config").join("mcp_servers");

        if !exists(&servers_dir) {
            return Ok(Vec::new());
        }

        let mut servers = Vec::new();
        let config_files = read_dir(&servers_dir)?;

        for config_file in config_files {
            if let Ok(server) = read_json::<_, crate::types::McpServerConfig>(&config_file) {
                servers.push(server);
            }
        }

        Ok(servers)
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
        tracing::info!("App data dir: {:?}", self.app_data_dir);

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
            env: config.env.clone(),             // Using new field name 'env'
            transport: config.transport.clone(), // Using new field name 'type'
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

        let server_path = get_mcp_server_config_path(&self.app_data_dir, &config.name);
        tracing::info!("config file path: {:?}", server_path);

        // save to file
        tracing::info!("Starting to write config file...");
        write_json_atomic(&server_path, &server_file)?;
        tracing::info!("Config file written successfully");

        self.servers.push(server_file);

        Ok(format!("MCP server '{}' added successfully", config.name))
    }

    // Version persistence removed

    /// Update server
    pub async fn update(&mut self, name: &str, config: McpServerConfig) -> Result<String> {
        // Clone required fields first to avoid borrow conflicts
        let server_name = config.name.clone();

        let server_path = get_mcp_server_config_path(&self.app_data_dir, &server_name);

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

        // save to file
        let server = self
            .get_by_name(&server_name)
            .ok_or_else(|| ConfigError::Invalid(format!("server not found: {}", name)))?;

        write_json_atomic(&server_path, server)?;

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

        // delete config file
        let server_path = get_mcp_server_config_path(&self.app_data_dir, name);
        remove_file(&server_path)?;

        // remove from memory
        self.servers.remove(server_index);

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

        // save changes
        let server_path = get_mcp_server_config_path(&self.app_data_dir, name);
        write_json_atomic(&server_path, server_mut)?;

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
