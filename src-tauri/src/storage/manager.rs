// Unified storage manager using SeaORM backend
use crate::error::{McpError, Result};
use crate::storage::orm_storage::Storage;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Simple storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Path to the SQLite database file
    pub db_path: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mcprouter");
        Self {
            db_path: config_dir.join("mcprouter.db"),
        }
    }
}

impl StorageConfig {
    /// Create config with custom database path
    pub fn with_db_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            db_path: path.as_ref().to_path_buf(),
        }
    }

    /// Get the database URL for SeaORM
    pub fn database_url(&self) -> String {
        format!("sqlite:{}", self.db_path.display())
    }
}

/// Storage manager using SeaORM backend
#[derive(Clone)]
pub struct StorageManager {
    /// ORM storage instance
    orm_storage: Arc<Storage>,
    /// Storage configuration
    config: StorageConfig,
}

impl StorageManager {
    /// Create new storage manager with SeaORM backend
    pub async fn new(config: StorageConfig, sql_log: bool, log_level: log::LevelFilter) -> Result<Self> {
        let database_url = config.database_url();

        // Storage::new() 内部会自动执行迁移
        let orm_storage = Storage::new(&database_url, sql_log, log_level)
            .await
            .map_err(|e| McpError::DatabaseConnectionError(format!("Failed to initialize SeaORM: {}", e)))?;

        let orm_storage = Arc::new(orm_storage);

        tracing::info!("Using SeaORM storage backend with automatic migrations");

        Ok(Self {
            orm_storage,
            config,
        })
    }

    /// Create with default configuration
    pub async fn with_default(sql_log: bool, log_level: log::LevelFilter) -> Result<Self> {
        let config = StorageConfig::default();
        Self::new(config, sql_log, log_level).await
    }

    /// Get ORM storage
    pub fn orm_storage(&self) -> Arc<Storage> {
        self.orm_storage.clone()
    }

    /// Get current storage configuration
    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    /// Health check for SeaORM storage backend
    pub async fn health_check(&self) -> Result<StorageHealth> {
        let mut healthy = true;
        let mut messages = Vec::new();

        // Check SeaORM backend
        match self.orm_storage.get_database_stats().await {
            Ok(_) => {
                messages.push("SeaORM storage is healthy".to_string());
            },
            Err(e) => {
                healthy = false;
                messages.push(format!("SeaORM health check failed: {}", e));
            }
        }

        let message = format!("SeaORM mode - {}", messages.join("; "));

        Ok(StorageHealth {
            healthy,
            message,
            stats: None,
        })
    }

    /// Create a new token manager
    pub async fn create_token_manager(&self) -> crate::error::Result<Arc<crate::token_manager::TokenManager>> {
        Ok(Arc::new(
            crate::token_manager::TokenManager::new(self.orm_storage.clone()).await?
        ))
    }

    // TODO: Add more token management methods if needed
}


/// Storage health information
#[derive(Debug, Clone)]
pub struct StorageHealth {
    pub healthy: bool,
    pub message: String,
    pub stats: Option<StorageStats>,
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub connection_count: u32,
    pub idle_connections: u32,
}