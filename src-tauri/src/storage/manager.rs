// Unified storage manager using SeaORM backend
use crate::error::{McpError, Result};
use crate::storage::orm_storage::Storage;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sea_orm_migration::prelude::*;

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

/// Unified storage manager using SeaORM backend
#[derive(Clone)]
pub struct UnifiedStorageManager {
    /// ORM storage instance
    orm_storage: Arc<Storage>,
    /// Storage configuration
    config: StorageConfig,
}

impl UnifiedStorageManager {
    /// Create new unified storage manager with SeaORM backend
    pub async fn new(config: StorageConfig) -> Result<Self> {
        let database_url = config.database_url();

        // Run migrations first with better error handling
        let db = sea_orm::Database::connect(&database_url)
            .await
            .map_err(|e| McpError::DatabaseConnectionError(format!("Failed to connect to database for migrations: {}", e)))?;

        // Run migrations with better error handling for existing indexes
        match crate::migration::Migrator::up(&db, None).await {
            Ok(_) => {
                tracing::info!("SeaORM migrations completed successfully");
            },
            Err(e) => {
                let error_msg = e.to_string();

                // Check if this is an index already exists error
                if error_msg.contains("index") && error_msg.contains("already exists") {
                    tracing::warn!("Migration index conflict detected: {}. This might be safe to continue.", error_msg);
                    tracing::info!("Attempting to verify database schema...");

                    // Try to verify the database is in a usable state
                    if let Err(verify_err) = verify_database_schema(&db).await {
                        return Err(McpError::DatabaseInitializationError(format!(
                            "Migration failed with index conflict and schema verification failed: {}\nVerification error: {}",
                            error_msg, verify_err
                        )));
                    }

                    tracing::info!("Database schema verification passed, continuing despite migration warnings");
                } else {
                    return Err(McpError::DatabaseInitializationError(format!("Failed to run SeaORM migrations: {}", e)));
                }
            }
        }

        let orm_storage = Storage::new(&database_url)
            .await
            .map_err(|e| McpError::DatabaseConnectionError(format!("Failed to initialize SeaORM: {}", e)))?;

        let orm_storage = Arc::new(orm_storage);

        tracing::info!("Using SeaORM storage backend");

        Ok(Self {
            orm_storage,
            config,
        })
    }

    /// Create with default configuration
    pub async fn with_default() -> Result<Self> {
        let config = StorageConfig::default();
        Self::new(config).await
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

/// Verify that the database schema is in a usable state
async fn verify_database_schema(db: &sea_orm::DatabaseConnection) -> std::result::Result<(), String> {
    use sea_orm::{ConnectionTrait, Statement};

    // List of tables we expect to exist
    let expected_tables = vec![
        "tokens",
        "mcp_servers",
        "mcp_server_tools",
        "mcp_server_resources",
        "mcp_server_prompts",
        "permissions",
    ];

    // Check each table exists
    for table_name in expected_tables {
        let result = db.query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{}'", table_name)
        )).await.map_err(|e| format!("Failed to check table {}: {}", table_name, e))?;

        if result.is_none() {
            return Err(format!("Required table '{}' does not exist", table_name));
        }
    }

    // Check that we can perform basic queries
    let test_query = db.query_all(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "SELECT COUNT(*) as count FROM tokens".to_string()
    )).await.map_err(|e| format!("Failed to query tokens table: {}", e))?;

    if test_query.is_empty() {
        return Err("Cannot query tokens table".to_string());
    }

    tracing::info!("Database schema verification passed");
    Ok(())
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