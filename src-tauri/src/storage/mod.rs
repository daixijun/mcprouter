// SeaORM-based storage module

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
pub mod manager;
pub mod orm_storage;

// Re-export commonly used types
pub use manager::UnifiedStorageManager;