// Simplified unified storage manager for SQLite only
#![allow(dead_code)]

use crate::error::{McpError, Result};
use crate::storage::StorageConfig;
use crate::storage::mcp_server_storage::McpServerStorage;
use crate::token_manager::TokenManager;
use std::sync::Arc;
use sqlx::SqlitePool;

/// SQLite storage backend
#[derive(Clone)]
pub struct StorageBackend {
    /// SQLite connection pool
    pub pool: SqlitePool,
    /// Token manager
    pub token_manager: Arc<TokenManager>,
    /// MCP server storage
    pub mcp_server_storage: Arc<McpServerStorage>,
}

/// Simplified unified storage manager
pub struct UnifiedStorageManager {
    backend: StorageBackend,
    config: StorageConfig,
}

impl UnifiedStorageManager {
    /// Create new unified storage manager
    pub async fn new(config: StorageConfig) -> Result<Self> {
        let sqlite_path = config.get_sqlite_path()?;
        let database_url = format!("sqlite:{}", sqlite_path.display());

        // Initialize SQLite database
        let storage = crate::storage::sqlite_storage::SqliteStorage::new(&database_url)
            .await
            .map_err(|e| McpError::DatabaseConnectionError(format!("Failed to initialize SQLite: {}", e)))?;
        let pool = storage.pool;

        // Create storage instances
        let mcp_server_storage = Arc::new(McpServerStorage::new(pool.clone()));
        mcp_server_storage.init().await
            .map_err(|e| McpError::DatabaseInitializationError(format!("Failed to initialize MCP server storage: {}", e)))?;

        let token_manager = Arc::new(TokenManager::new(pool.clone()).await?);

        let backend = StorageBackend {
            pool,
            token_manager,
            mcp_server_storage,
        };

        Ok(Self {
            backend,
            config,
        })
    }

    /// Create with default configuration
    pub async fn with_default() -> Result<Self> {
        let config = StorageConfig::default();
        Self::new(config).await
    }

    /// Get token manager
    pub fn token_manager(&self) -> Arc<TokenManager> {
        self.backend.token_manager.clone()
    }

    /// Get MCP server storage
    pub fn mcp_server_storage(&self) -> Arc<McpServerStorage> {
        self.backend.mcp_server_storage.clone()
    }

    /// Get SQLite pool
    pub fn sqlite_pool(&self) -> SqlitePool {
        self.backend.pool.clone()
    }

    /// Check if current backend is SQLite (always true now)
    pub fn is_sqlite(&self) -> bool {
        true
    }

    /// Check if current backend is JSON (always false now)
    pub fn is_json(&self) -> bool {
        false
    }

    /// Get current storage configuration
    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    /// Health check for SQLite storage backend
    pub async fn health_check(&self) -> Result<StorageHealth> {
        // SQLite health check
        let pool = &self.backend.pool;
        let result = sqlx::query("SELECT 1")
            .fetch_one(pool)
            .await;

        match result {
            Ok(_) => Ok(StorageHealth {
                healthy: true,
                message: "SQLite storage is healthy".to_string(),
                stats: Some(StorageStats {
                    connection_count: pool.size() as u32,
                    idle_connections: pool.num_idle() as u32,
                }),
            }),
            Err(e) => Ok(StorageHealth {
                healthy: false,
                message: format!("SQLite health check failed: {}", e),
                stats: None,
            }),
        }
    }
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