use crate::error::{McpError, Result};

pub use crate::types::{
    AppConfig, LoggingSettings, SecuritySettings, Settings, SystemTraySettings,
};

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
        tracing::info!("Config::save() - Starting save process");

        let config_dir = dirs::home_dir()
            .ok_or_else(|| {
                tracing::error!("Config::save() - Could not find home directory");
                McpError::ConfigError("Could not find home directory".to_string())
            })?
            .join(".mcprouter");

        tracing::info!("Config::save() - Config directory: {:?}", config_dir);

        // Ensure directory exists
        std::fs::create_dir_all(&config_dir).map_err(|e| {
            tracing::error!("Config::save() - Failed to create config directory: {}", e);
            McpError::ConfigError(format!("Failed to create config directory: {}", e))
        })?;

        let config_path = config_dir.join("config.json");
        tracing::info!("Config::save() - Config path: {:?}", config_path);

        tracing::info!("Config::save() - Serializing config...");
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            tracing::error!("Config::save() - Failed to serialize config: {}", e);
            McpError::ConfigError(format!("Failed to serialize config: {}", e))
        })?;

        tracing::info!("Config::save() - Writing {} bytes to file", content.len());
        std::fs::write(&config_path, content).map_err(|e| {
            tracing::error!("Config::save() - Failed to write config file: {}", e);
            McpError::ConfigError(format!("Failed to write config file: {}", e))
        })?;

        tracing::info!("Config::save() - Save completed successfully");
        Ok(())
    }
}
