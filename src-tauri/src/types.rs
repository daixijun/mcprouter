// 共享类型定义

use rmcp::model::Tool as RmcpTool;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

// Use rmcp Tool model directly instead of custom struct
pub type McpTool = RmcpTool;

// ============================================================================
// Config related types (from config.rs)
// ============================================================================

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct McpServerConfig {
    pub name: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    pub transport: ServiceTransport,
    pub url: Option<String>,
    pub enabled: bool,
    pub env_vars: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub version: Option<String>,
}

impl Serialize for McpServerConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("McpServerConfig", 11)?;
        state.serialize_field("name", &self.name)?;
        if let Some(ref description) = self.description {
            state.serialize_field("description", description)?;
        }
        if let Some(ref command) = self.command {
            if !should_skip_command(command) {
                state.serialize_field("command", command)?;
            }
        }
        if let Some(ref args) = self.args {
            if !should_skip_args(args) {
                state.serialize_field("args", args)?;
            }
        }
        state.serialize_field("transport", &self.transport)?;
        if let Some(ref url) = self.url {
            state.serialize_field("url", url)?;
        }
        state.serialize_field("enabled", &self.enabled)?;

        // Only serialize env_vars if transport is Stdio
        match self.transport {
            ServiceTransport::Stdio => {
                if let Some(ref env_vars) = self.env_vars {
                    state.serialize_field("env_vars", env_vars)?;
                }
            }
            _ => {
                // Skip env_vars for SSE and Http
            }
        }

        if let Some(ref headers) = self.headers {
            state.serialize_field("headers", headers)?;
        }

        // Serialize version
        if let Some(ref version) = self.version {
            state.serialize_field("version", version)?;
        }

        state.end()
    }
}

// Helper functions for conditional serialization
fn should_skip_command(command: &str) -> bool {
    command.is_empty()
}

fn should_skip_args(args: &[String]) -> bool {
    args.is_empty()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTransport {
    Stdio,
    Sse,
    Http,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: Option<LoggingSettings>,
    pub security: Option<SecuritySettings>,
    #[serde(default)]
    pub settings: Option<Settings>,
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
    #[serde(default)]
    pub close_to_tray: Option<bool>,
    #[serde(default)]
    pub start_to_tray: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub struct Settings {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub autostart: Option<bool>,
    #[serde(default)]
    pub system_tray: Option<SystemTraySettings>,
    #[serde(default)]
    pub uv_index_url: Option<String>,
    #[serde(default)]
    pub npm_registry: Option<String>,
}


// API Key Permission structure
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub struct ApiKeyPermissions {
    pub allowed_servers: Vec<String>,
    pub allowed_tools: Vec<String>,
}


// API Key structure
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key: String,
    pub enabled: bool,
    pub created_at: String,
    pub permissions: ApiKeyPermissions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SecuritySettings {
    pub allowed_hosts: Vec<String>,
    pub auth: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub timeout_seconds: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8850,
                max_connections: 100,
                timeout_seconds: 30,
            },
            logging: Some(LoggingSettings {
                level: "info".to_string(),
                file_name: Some("mcprouter.log".to_string()),
            }),
            security: Some(SecuritySettings {
                allowed_hosts: vec!["localhost".to_string(), "127.0.0.1".to_string()],
                auth: true,
            }),
            settings: Some(Settings::default()),
        }
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
pub struct McpServerInfo {
    pub name: String,
    pub enabled: bool,
    pub status: String, // "connecting", "connected", "disconnected", "failed"
    pub version: Option<String>,
    pub error_message: Option<String>,
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
pub struct ServiceVersionCache {
    pub version: Option<String>,
}

// ============================================================================
// MCP Client related types (from mcp_client.rs)
// ============================================================================

// Define enum for different service types
#[derive(Debug)]
pub enum McpService {
    Stdio(rmcp::service::RunningService<rmcp::RoleClient, rmcp::model::ClientInfo>),
    Sse(rmcp::service::RunningService<rmcp::RoleClient, rmcp::model::ClientInfo>),
    Http(rmcp::service::RunningService<rmcp::RoleClient, rmcp::model::ClientInfo>),
}

#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub is_connected: bool,
    pub last_connected: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct McpConnection {
    pub service_id: String,
    pub server_info: Option<serde_json::Value>,
    // Store the actual RMCP client using enum
    pub client: Option<std::sync::Arc<McpService>>,
    // Enhanced connection state management
    pub status: ConnectionStatus,
    // Cached service information
    pub cached_version: Option<String>,
}
