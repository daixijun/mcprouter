//! Configuration management module
//!
//! This module is responsible for managing all configuration files of the application, including:
//! - API key configuration
//! - MCP server configuration
//! - Application global configuration

pub mod file_manager;

pub use file_manager::*;

// Re-export AppConfig for convenience
pub use crate::AppConfig;

use std::path::PathBuf;

// API Key configuration has been removed

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
