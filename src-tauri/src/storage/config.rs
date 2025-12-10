// SQLite storage configuration management
#![allow(dead_code)]

use crate::error::McpError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Storage configuration for SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// SQLite database path
    pub sqlite_path: Option<PathBuf>,

    /// Enable performance optimizations
    pub enable_optimizations: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            sqlite_path: None,
            enable_optimizations: true,
        }
    }
}

impl StorageConfig {
    /// Create new storage configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Get SQLite database path
    pub fn get_sqlite_path(&self) -> Result<PathBuf, McpError> {
        if let Some(ref path) = self.sqlite_path {
            Ok(path.clone())
        } else {
            // Default to data directory
            let mut data_dir = dirs::data_dir()
                .ok_or_else(|| McpError::Internal("Failed to get data directory".to_string()))?;
            data_dir.push("mcprouter");
            data_dir.push("data.db");
            Ok(data_dir)
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), McpError> {
        let _ = self.get_sqlite_path()?; // Validate path can be determined
        Ok(())
    }
}

/// Storage configuration manager
pub struct StorageConfigManager {
    config_path: PathBuf,
    config: StorageConfig,
}

impl StorageConfigManager {
    /// Create new config manager with default path
    pub fn new() -> Result<Self, McpError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| McpError::Internal("Failed to get config directory".to_string()))?;
        let config_path = config_dir.join("mcprouter").join("storage.json");
        Self::with_path(config_path)
    }

    /// Create new config manager with custom path
    pub fn with_path(config_path: PathBuf) -> Result<Self, McpError> {
        let config = if config_path.exists() {
            Self::load_config(&config_path)?
        } else {
            StorageConfig::default()
        };

        Ok(Self {
            config_path,
            config,
        })
    }

    /// Load configuration from file
    fn load_config(path: &PathBuf) -> Result<StorageConfig, McpError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            McpError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to read storage config: {}", e),
            ))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            McpError::SerializationError(format!("Failed to parse storage config: {}", e))
        })
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), McpError> {
        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                McpError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Failed to create config directory: {}", e),
                ))
            })?;
        }

        let content = serde_json::to_string_pretty(&self.config).map_err(|e| {
            McpError::SerializationError(format!("Failed to serialize storage config: {}", e))
        })?;

        std::fs::write(&self.config_path, content).map_err(|e| {
            McpError::IoError(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Failed to write storage config: {}", e),
            ))
        })?;

        tracing::info!("Storage configuration saved to: {:?}", self.config_path);
        Ok(())
    }

    /// Get current configuration
    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config<F>(&mut self, updater: F) -> Result<(), McpError>
    where
        F: FnOnce(&mut StorageConfig),
    {
        updater(&mut self.config);
        self.config.validate()?;
        self.save()
    }

    /// Set SQLite path
    pub fn set_sqlite_path(&mut self, path: PathBuf) -> Result<(), McpError> {
        self.update_config(|config| {
            config.sqlite_path = Some(path);
        })
    }
}
