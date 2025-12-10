// SQLite storage connection pool management
#![allow(dead_code)]

use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use super::{Result, StorageError};

/// SQLite storage manager with optimized connection pooling
pub struct SqliteStorage {
    pub pool: SqlitePool,
}

impl SqliteStorage {
    /// Create a new SQLite storage with optimized connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        // Parse connection options with optimizations
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| StorageError::Database(format!("Invalid database URL: {}", e)))?
            // Enable WAL mode for better concurrency
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            // Optimize for performance
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            // Enable foreign key constraints
            .foreign_keys(true)
            // Set cache size (64MB)
            .pragma("cache_size", "-64000")
            // Set temp store to memory
            .pragma("temp_store", "memory")
            // Set memory journal mode
            .pragma("journal_mode", "WAL")
            // Enable auto vacuum
            .pragma("auto_vacuum", "FULL")
            // Set busy timeout to 30 seconds
            .pragma("busy_timeout", "30000");

        // Create connection pool with optimized settings
        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to create connection pool: {}", e)))?;

        // Configure additional performance PRAGMAs after connection
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set WAL mode: {}", e)))?;

        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set synchronous mode: {}", e)))?;

        sqlx::query("PRAGMA cache_size = -64000")
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set cache size: {}", e)))?;

        sqlx::query("PRAGMA temp_store = memory")
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set temp store: {}", e)))?;

        sqlx::query("PRAGMA mmap_size = 268435456")  // 256MB
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set mmap size: {}", e)))?;

        tracing::info!("✅ SQLite storage initialized with optimized settings");

        Ok(Self { pool })
    }

    /// Get the connection pool for use by other storage modules
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the connection pool
    pub async fn close(self) {
        self.pool.close().await;
        tracing::info!("SQLite connection pool closed");
    }

    /// Perform health check on the database
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Database health check failed: {}", e)))?;

        Ok(())
    }

    /// Get connection pool statistics
    pub async fn pool_stats(&self) -> PoolStats {
        let pool = &self.pool;
        PoolStats {
            size: pool.size() as u32,
            num_idle: pool.num_idle() as u32,
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of connections in the pool
    pub size: u32,
    /// Number of idle connections
    pub num_idle: u32,
}

