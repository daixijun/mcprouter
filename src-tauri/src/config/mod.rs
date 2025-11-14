//! Configuration management module
//!
//! This module is responsible for managing all configuration files of the application, including:
//! - API key configuration
//! - MCP server configuration
//! - Application global configuration

pub mod file_manager;
pub mod mcp_server_config;

pub use file_manager::*;
pub use mcp_server_config::*;

// Re-export AppConfig for convenience
pub use crate::AppConfig;

use std::path::{Path, PathBuf};

/// Configuration directory relative path
const CONFIG_DIR_NAME: &str = "config";
const MCP_SERVERS_DIR_NAME: &str = "mcp_servers";

/// Get configuration directory path
pub fn get_config_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(CONFIG_DIR_NAME)
}

/// Get MCP server configuration directory path
pub fn get_mcp_servers_dir(app_data_dir: &Path) -> PathBuf {
    get_config_dir(app_data_dir).join(MCP_SERVERS_DIR_NAME)
}

// API Key configuration has been removed

/// Get application configuration file path (standardized to ~/.mcprouter/config.json)
pub fn get_app_config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("config.json")
}

/// Get single MCP server configuration file path
pub fn get_mcp_server_config_path(app_data_dir: &Path, server_name: &str) -> PathBuf {
    get_mcp_servers_dir(app_data_dir).join(format!("{}.json", server_name))
}

/// Common configuration error types
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Insufficient permissions: {0}")]
    Permission(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;
