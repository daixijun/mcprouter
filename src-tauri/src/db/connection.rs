use crate::error::{McpError, Result};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::{SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
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
            McpError::DatabaseError(format!("Failed to get app data directory: {}", e))
        })?;

        // Ensure directory exists
        std::fs::create_dir_all(&app_data_dir).map_err(|e| {
            McpError::DatabaseError(format!("Failed to create app data directory: {}", e))
        })?;

        let db_path = app_data_dir.join("mcprouter.db");
        let db_url = format!("sqlite:{}", db_path.display());

        info!("Connecting to database at: {}", db_url);

        // Create connection options
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| McpError::DatabaseError(format!("Invalid database URL: {}", e)))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| {
                McpError::DatabaseError(format!("Failed to connect to database: {}", e))
            })?;

        debug!("Database connection pool created successfully");

        Ok(Self { pool })
    }

    /// 获取数据库连接池引用
    pub fn get(&self) -> &SqlitePool {
        &self.pool
    }

    /// 执行数据库初始化（创建表和索引）
    pub async fn initialize_schema(&self) -> Result<()> {
        info!("Initializing database schema");

        let pool = self.get();

        // 创建 MCP 服务器表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                description TEXT,
                command TEXT,
                args TEXT,  -- JSON array as TEXT
                transport TEXT NOT NULL,
                url TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                env_vars TEXT,  -- JSON object as TEXT
                headers TEXT,   -- JSON object as TEXT
                version TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        // 创建 tools 表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tools (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                server_id TEXT NOT NULL,
                description TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        // 创建 api_keys 表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                key_hash TEXT NOT NULL, -- 存储哈希而非明文
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        // 确保旧安装也具备 last_used_at 列
        let api_keys_columns = sqlx::query("PRAGMA table_info(api_keys)")
            .fetch_all(pool)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let has_last_used_at = api_keys_columns.iter().any(|row| {
            let name: String = row.try_get("name").unwrap_or_default();
            name == "last_used_at"
        });

        if !has_last_used_at {
            sqlx::query("ALTER TABLE api_keys ADD COLUMN last_used_at DATETIME")
                .execute(pool)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;
            info!("Added missing column 'last_used_at' to api_keys");
        }

        // 创建 api_key_server_relations 表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_key_server_relations (
                id TEXT PRIMARY KEY,
                api_key_id TEXT NOT NULL,
                server_id TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        // 创建索引以优化查询性能
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled)",
            "CREATE INDEX IF NOT EXISTS idx_mcp_servers_name ON mcp_servers(name)",
            "CREATE INDEX IF NOT EXISTS idx_tools_server_id ON tools(server_id)",
            "CREATE INDEX IF NOT EXISTS idx_tools_enabled ON tools(enabled)",
            "CREATE INDEX IF NOT EXISTS idx_api_keys_enabled ON api_keys(enabled)",
            "CREATE INDEX IF NOT EXISTS idx_api_keys_last_used_at ON api_keys(last_used_at)",
            "CREATE INDEX IF NOT EXISTS idx_api_key_relations_key_id ON api_key_server_relations(api_key_id)",
            "CREATE INDEX IF NOT EXISTS idx_api_key_relations_server_id ON api_key_server_relations(server_id)",
        ];

        for index_sql in indexes {
            sqlx::query(index_sql)
                .execute(pool)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;
        }

        info!("Database schema initialized successfully");
        Ok(())
    }
}

/// 全局数据库连接状态
pub static DB_CONNECTION: std::sync::OnceLock<
    Arc<tokio::sync::RwLock<Option<Arc<DatabaseConnection>>>>,
> = std::sync::OnceLock::new();

/// 初始化全局数据库连接
pub async fn initialize_database(app_handle: &AppHandle) -> Result<()> {
    let conn = Arc::new(DatabaseConnection::new(app_handle).await?);
    conn.initialize_schema().await?;

    let global_conn = Arc::new(tokio::sync::RwLock::new(Some(conn)));
    DB_CONNECTION
        .set(global_conn)
        .map_err(|_| McpError::DatabaseError("Database already initialized".to_string()))?;

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
                None => Err(McpError::DatabaseError(
                    "Database not initialized".to_string(),
                )),
            }
        }
        None => Err(McpError::DatabaseError(
            "Database not initialized".to_string(),
        )),
    }
}
