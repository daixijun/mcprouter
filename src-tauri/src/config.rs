use crate::error::{McpError, Result};
use serde::{Deserialize, Serialize, Serializer};

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
    pub env_vars: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
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
                // Skip env_vars for SSE and StreamableHttp
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
    StreamableHttp,
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

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: None,
            autostart: None,
            system_tray: None,
            uv_index_url: None,
            npm_registry: None,
        }
    }
}
// API Key Permission structure
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ApiKeyPermissions {
    pub allowed_servers: Vec<String>,
    pub allowed_tools: Vec<String>,
}

impl Default for ApiKeyPermissions {
    fn default() -> Self {
        Self {
            allowed_servers: Vec::new(),
            allowed_tools: Vec::new(),
        }
    }
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

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| McpError::ConfigError("Could not find home directory".to_string()))?
            .join(".mcprouter");

        std::fs::create_dir_all(&config_dir).map_err(|e| {
            McpError::ConfigError(format!("Failed to create config directory: {}", e))
        })?;

        let config_path = config_dir.join("config.json");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| McpError::ConfigError(format!("Failed to read config file: {}", e)))?;

            // 直接按新结构解析配置文件
            serde_json::from_str::<AppConfig>(&content).map_err(|e| {
                McpError::ConfigError(format!(
                    "Failed to parse AppConfig: {}. Please fix the config file or delete it to use defaults.",
                    e
                ))
            })
        } else {
            // 创建默认配置并保存（新结构）
            let default_config = AppConfig::default();
            default_config.save()?;
            tracing::info!("Created new default config file");
            Ok(default_config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| McpError::ConfigError("Could not find home directory".to_string()))?
            .join(".mcprouter");

        let config_path = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| McpError::ConfigError(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&config_path, content)
            .map_err(|e| McpError::ConfigError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }
}
