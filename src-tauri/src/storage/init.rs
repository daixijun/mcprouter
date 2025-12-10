// SQLite storage initialization utilities
#![allow(dead_code)]

use crate::error::{McpError, Result};
use crate::storage::{UnifiedStorageManager, StorageConfigManager};
use std::path::PathBuf;
use std::sync::Arc;

/// Initialize SQLite storage manager
pub async fn init_storage_manager(config_dir: &PathBuf) -> Result<Arc<UnifiedStorageManager>> {
    // Load storage configuration
    let config_manager = StorageConfigManager::new()
        .map_err(|e| McpError::InvalidConfiguration(format!("Failed to load storage config: {}", e)))?;

    let mut config = config_manager.config().clone();

    // Set default path if not configured
    if config.sqlite_path.is_none() {
        config.sqlite_path = Some(config_dir.join("mcprouter.db"));
    }

    // Create storage manager
    let storage_manager = UnifiedStorageManager::new(config).await
        .map_err(|e| McpError::DatabaseInitializationError(format!("Failed to initialize storage manager: {}", e)))?;

    Ok(Arc::new(storage_manager))
}