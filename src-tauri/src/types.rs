use rmcp::model::Tool as McpToolSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Use rmcp Tool model directly instead of custom struct
pub type McpTool = McpToolSchema;

// ============================================================================
// Config related types (from config.rs)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct McpServerConfig {
    pub name: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(rename = "type")]
    pub transport: ServiceTransport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    pub enabled: bool,
}

impl McpServerConfig {
    /// Clean fields, ensure only fields related to transport type are set
    pub fn clean_fields(&mut self) {
        match self.transport {
            ServiceTransport::Stdio => {
                // stdio type: clean http related fields
                self.url = None;
                self.headers = None;
            }
            ServiceTransport::Http => {
                // http type: clean stdio related fields
                self.command = None;
                self.args = None;
                self.env = None;
            }
        }
    }

    /// Validate configuration and provide migration suggestions for deprecated features
    pub fn validate_and_warn(&self) {
        // Check if this looks like a legacy SSE configuration that might need migration
        if self.url.is_some() && self.headers.is_some() &&
           self.command.is_none() && self.args.is_none() && self.env.is_none() {
            tracing::warn!(
                "Server '{}' appears to be configured for HTTP transport. Note: SSE transport is no longer supported. Please use HTTP transport instead.",
                self.name
            );
        }
    }

    /// Create a new minimal config
    pub fn new(_id: String, name: String) -> Self {
        Self {
            name,
            description: None,
            command: None,
            args: None,
            env: None,
            transport: ServiceTransport::Stdio,
            url: None,
            headers: None,
            enabled: true,
        }
    }
}

// Conditional serialization helpers removed

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTransport {
    Stdio,
    Http,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: Option<LoggingSettings>,
    #[serde(default)]
    pub settings: Option<Settings>,
    pub mcp_servers: Vec<McpServerConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LoggingSettings {
    pub level: String,
    pub file_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SystemTraySettings {
    #[serde(default)]
    pub enabled: Option<bool>,
    pub close_to_tray: Option<bool>,
    pub start_to_tray: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct Settings {
    #[serde(default = "default_theme")]
    pub theme: Option<String>,
    #[serde(default = "default_language")]
    pub language: Option<String>,
    #[serde(default)]
    pub autostart: Option<bool>,
    #[serde(default)]
    pub system_tray: Option<SystemTraySettings>,
    pub uv_index_url: Option<String>,
    pub npm_registry: Option<String>,
    #[serde(default)]
    pub command_paths: HashMap<String, String>,
}

fn default_theme() -> Option<String> {
    Some("auto".to_string())
}

fn default_language() -> Option<String> {
    Some("zh-CN".to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub auth: bool, // Controls whether authentication is enabled
}

impl ServerConfig {
    /// Validate server configuration
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        // No specific validation needed for now
        Ok(())
    }

    /// Check if authentication is enabled
    pub fn is_auth_enabled(&self) -> bool {
        self.auth
    }
}

// ============================================================================
// Marketplace related types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceService {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub transport: String,
    pub install_command: Option<InstallCommand>,
    pub github_stars: Option<u64>,
    pub downloads: u32,
    pub last_updated: String,
    pub platform: String,
    pub logo_url: Option<String>,
    pub license: Option<String>,
    pub is_hosted: Option<bool>,
    pub is_verified: Option<bool>,
    // Fields only in detail view
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation_url: Option<String>,
    pub requirements: Vec<String>,
    pub readme: Option<String>,
    pub server_config: Option<Vec<serde_json::Value>>,
    pub env_schema: Option<EnvSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallCommand {
    pub command: String,
    pub args: Vec<String>,
    pub package_manager: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvSchema {
    #[serde(default)]
    pub properties: HashMap<String, EnvProperty>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(rename = "type")]
    pub schema_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProperty {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub prop_type: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

// Marketplace service list item without install_command and other unnecessary fields for lighter responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceServiceListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub transport: String,
    pub github_stars: Option<u64>,
    pub downloads: u32,
    pub last_updated: String,
    pub platform: String,
    pub logo_url: Option<String>,
    pub license: Option<String>,
    pub is_hosted: Option<bool>,
    pub is_verified: Option<bool>,
}

// ============================================================================
// MCP Manager related types (from mcp_manager.rs)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub enabled: bool,
    pub status: String, // "connecting", "connected", "disconnected", "failed"
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub version: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

// 合并后的响应结构体，包含状态和配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpServerInfo {
    pub name: String,
    pub enabled: bool,
    pub status: String, // "connecting", "connected", "disconnected", "failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(rename = "type")]
    pub transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template_count: Option<usize>,
}

/// MCP服务器工具信息，用于API返回
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct ServiceVersionCache {
    pub version: Option<String>,
}

// ============================================================================
// Resources and Prompts related types
// ============================================================================

/// MCP 资源信息，用于 API 返回
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpResourceInfo {
    pub id: String,
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP 提示信息，用于 API 返回
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpPromptInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP 提示消息参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

// ============================================================================
// MCP Client related types (from mcp_client.rs)
// ============================================================================

// Define enum for different service types (rmcp 0.8.3)
// Use RunningService directly to enable peer access for tool listing
pub enum McpService {
    Stdio(std::sync::Arc<rmcp::service::RunningService<rmcp::service::RoleClient, ()>>),
    Http(std::sync::Arc<rmcp::service::RunningService<rmcp::service::RoleClient, ()>>),
}

impl std::fmt::Debug for McpService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpService::Stdio(_) => write!(f, "McpService::Stdio"),
            McpService::Http(_) => write!(f, "McpService::Http"),
        }
    }
}

impl McpService {
    /// Get the peer for sending requests
    pub fn peer(&self) -> &rmcp::service::Peer<rmcp::service::RoleClient> {
        match self {
            McpService::Stdio(service) => service.peer(),
            McpService::Http(service) => service.peer(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub is_connected: bool,
    pub is_connecting: bool,
    pub last_connected: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct McpConnection {
    pub service_id: String,
    pub server_info: Option<rmcp::model::InitializeResult>,
    // Store the actual RMCP client using enum
    pub client: Option<std::sync::Arc<McpService>>,
    // Enhanced connection state management
    pub status: ConnectionStatus,
}

impl AppConfig {
    /// 从文件加载配置
    pub fn load() -> Result<Self, crate::config::ConfigError> {
        // Resolve home directory cross-platform
        let home_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let app_data_dir = std::path::PathBuf::from(format!("{}/.mcprouter", home_dir));

        let config_path = app_data_dir.join("config.json");

        // Migration: read old path if new path not exists
        if !config_path.exists() {
            let old_path = app_data_dir.join("config").join("app.json");
            if old_path.exists() {
                let file =
                    std::fs::File::open(&old_path).map_err(crate::config::ConfigError::Io)?;
                let reader = std::io::BufReader::new(file);
                let config: AppConfig =
                    serde_json::from_reader(reader).map_err(crate::config::ConfigError::Json)?;

                // Validate server configuration
                config.server.validate()?;

                // Ensure parent dir exists and write to new path
                if let Some(parent) = config_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                crate::config::write_json_atomic(&config_path, &config)?;
                return Ok(config);
            }

            // Create default when no config exists
            let default_config = Self::default();
            if let Some(parent) = config_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            crate::config::write_json_atomic(&config_path, &default_config)?;
            return Ok(default_config);
        }

        let file = std::fs::File::open(&config_path).map_err(crate::config::ConfigError::Io)?;
        let reader = std::io::BufReader::new(file);
        let config: AppConfig =
            serde_json::from_reader(reader).map_err(crate::config::ConfigError::Json)?;

        // Validate server configuration
        config.server.validate()?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), crate::config::ConfigError> {
        let home_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let app_data_dir = std::path::PathBuf::from(format!("{}/.mcprouter", home_dir));

        let config_path = app_data_dir.join("config.json");

        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        crate::config::write_json_atomic(&config_path, self)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8000,
                max_connections: 100,
                timeout_seconds: 30,
                auth: false, // Default to false for backward compatibility
            },
            logging: Some(crate::types::LoggingSettings {
                level: "info".to_string(),
                file_name: Some("mcprouter.log".to_string()),
            }),
            settings: Some(Settings {
                theme: Some("auto".to_string()),
                language: Some("zh-CN".to_string()),
                autostart: Some(false),
                system_tray: Some(SystemTraySettings {
                    enabled: Some(true),
                    close_to_tray: Some(false),
                    start_to_tray: Some(false),
                }),
                uv_index_url: None,
                npm_registry: None,
                command_paths: Default::default(),
            }),
            mcp_servers: Vec::new(),
        }
    }
}
