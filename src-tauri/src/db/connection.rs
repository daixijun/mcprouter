use crate::error::{McpError, Result};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::{SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tracing::{debug, info};

/// 数据库连接管理器
pub struct DatabaseConnection {
    pool: SqlitePool,
}

impl DatabaseConnection {
    /// 创建新的数据库连接
    pub async fn new(app_handle: &AppHandle) -> Result<Self> {
        // Get the app data directory
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| {
            McpError::DatabaseInitializationError(format!(
                "Failed to get app data directory: {}",
                e
            ))
        })?;

        // Ensure directory exists
        std::fs::create_dir_all(&app_data_dir).map_err(|e| {
            McpError::DatabaseInitializationError(format!(
                "Failed to create app data directory: {}",
                e
            ))
        })?;

        let db_path = app_data_dir.join("mcprouter.db");
        let db_url = format!("sqlite:{}", db_path.display());

        info!("Connecting to database at: {}", db_url);

        // Create connection options
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| {
                McpError::DatabaseInitializationError(format!("Invalid database URL: {}", e))
            })?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await
            .map_err(|e| {
                McpError::DatabaseConnectionError(format!("Failed to connect to database: {}", e))
            })?;

        debug!("Database connection pool created successfully");

        Ok(Self { pool })
    }

    /// 获取数据库连接池引用
    pub fn get(&self) -> &SqlitePool {
        &self.pool
    }
}

/// 全局数据库连接状态
pub static DB_CONNECTION: std::sync::OnceLock<
    Arc<tokio::sync::RwLock<Option<Arc<DatabaseConnection>>>>,
> = std::sync::OnceLock::new();

/// 初始化全局数据库连接
pub async fn initialize_database(app_handle: &AppHandle) -> Result<()> {
    let conn = Arc::new(DatabaseConnection::new(app_handle).await?);

    let global_conn = Arc::new(tokio::sync::RwLock::new(Some(conn)));
    DB_CONNECTION.set(global_conn).map_err(|_| {
        McpError::DatabaseInitializationError("Database already initialized".to_string())
    })?;

    info!("Database connection initialized and stored globally");
    Ok(())
}

/// 获取全局数据库连接
pub async fn get_database() -> Result<SqlitePool> {
    match DB_CONNECTION.get() {
        Some(conn) => {
            let conn = conn.read().await;
            match conn.as_ref() {
                Some(db_conn) => Ok(db_conn.get().clone()),
                None => Err(McpError::DatabaseInitializationError(
                    "Database not initialized".to_string(),
                )),
            }
        }
        None => Err(McpError::DatabaseInitializationError(
            "Database not initialized".to_string(),
        )),
    }
}
