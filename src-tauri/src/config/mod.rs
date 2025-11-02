//! 配置管理模块
//!
//! 本模块负责管理应用的所有配置文件，包括：
//! - API密钥配置
//! - MCP服务器配置
//! - 应用全局配置

pub mod api_key_config;
pub mod file_manager;
pub mod mcp_server_config;

pub use api_key_config::*;
pub use file_manager::*;
pub use mcp_server_config::*;

// Re-export AppConfig for convenience
pub use crate::AppConfig;

use std::path::{Path, PathBuf};

/// 配置目录相对路径
const CONFIG_DIR_NAME: &str = "config";
const MCP_SERVERS_DIR_NAME: &str = "mcp_servers";

/// 获取配置目录路径
pub fn get_config_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(CONFIG_DIR_NAME)
}

/// 获取MCP服务器配置目录路径
pub fn get_mcp_servers_dir(app_data_dir: &Path) -> PathBuf {
    get_config_dir(app_data_dir).join(MCP_SERVERS_DIR_NAME)
}

/// 获取API密钥配置文件路径
pub fn get_api_keys_config_path(app_data_dir: &Path) -> PathBuf {
    get_config_dir(app_data_dir).join("api_keys.json")
}

/// 获取应用配置文件路径
pub fn get_app_config_path(app_data_dir: &Path) -> PathBuf {
    get_config_dir(app_data_dir).join("app.json")
}

/// 获取单个MCP服务器配置文件路径
pub fn get_mcp_server_config_path(app_data_dir: &Path, server_name: &str) -> PathBuf {
    get_mcp_servers_dir(app_data_dir).join(format!("{}.json", server_name))
}

/// 通用配置错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON序列化/反序列化错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("配置文件不存在: {0}")]
    NotFound(PathBuf),

    #[error("配置无效: {0}")]
    Invalid(String),

    #[error("权限不足: {0}")]
    Permission(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;
