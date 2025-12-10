// SQLite storage module

use sqlx::SqlitePool;
use thiserror::Error;

/// Storage error types
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Item already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Re-export storage modules
pub mod config;
pub mod init;
pub mod manager;
pub mod mcp_server_storage;
pub mod sqlite_storage;
pub mod token_storage;
pub mod permission_storage;
pub mod session_storage;

// Re-export commonly used types
pub use config::{StorageConfig, StorageConfigManager};
pub use manager::UnifiedStorageManager;