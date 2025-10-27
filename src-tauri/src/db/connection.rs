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

        // 执行数据迁移：从 tools 表迁移到 mcp_tools 表
        self.migrate_to_mcp_tools().await?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// 迁移 tools 表到 mcp_tools 表，并实现工具级别授权
    async fn migrate_to_mcp_tools(&self) -> Result<()> {
        let pool = self.get();

        // 检查是否需要迁移（tools 表存在但 mcp_tools 表不存在）
        let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(pool)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let table_names: Vec<String> = tables
            .iter()
            .filter_map(|row| row.try_get("name").ok())
            .collect();

        let has_tools = table_names.contains(&"tools".to_string());
        let has_mcp_tools = table_names.contains(&"mcp_tools".to_string());

        if has_tools && !has_mcp_tools {
            info!("检测到 tools 表，开始迁移到 mcp_tools");

            // 开启事务确保迁移的原子性
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            // 步骤 1: 重命名表
            sqlx::query("ALTER TABLE tools RENAME TO mcp_tools")
                .execute(&mut *tx)
                .await
                .map_err(|e| McpError::DatabaseError(format!("表重命名失败: {}", e)))?;

            info!("tools 表已重命名为 mcp_tools");

            // 步骤 2: 删除旧的 tools 索引
            sqlx::query("DROP INDEX IF EXISTS idx_tools_server_id")
                .execute(&mut *tx)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            sqlx::query("DROP INDEX IF EXISTS idx_tools_enabled")
                .execute(&mut *tx)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            // 步骤 3: 创建新的 mcp_tools 索引
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tools_server_id ON mcp_tools(server_id)")
                .execute(&mut *tx)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tools_enabled ON mcp_tools(enabled)")
                .execute(&mut *tx)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            // 步骤 4: 创建 api_key_tool_relations 表
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS api_key_tool_relations (
                    id TEXT PRIMARY KEY,
                    api_key_id TEXT NOT NULL,
                    tool_id TEXT NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(api_key_id, tool_id)
                )
                "#,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            // 步骤 5: 创建 api_key_tool_relations 索引
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_api_key_id ON api_key_tool_relations(api_key_id)")
                .execute(&mut *tx)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_tool_id ON api_key_tool_relations(tool_id)")
                .execute(&mut *tx)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            info!("api_key_tool_relations 表已创建");

            // 步骤 6: 数据迁移 - 从 api_key_server_relations 生成工具级授权
            let server_relations = sqlx::query("SELECT api_key_id, server_id FROM api_key_server_relations")
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            let mut migrated_count = 0;

            for relation in server_relations {
                let api_key_id: String = relation.try_get("api_key_id").unwrap_or_default();
                let server_id: String = relation.try_get("server_id").unwrap_or_default();

                // 查询该 server 下的所有工具
                let tools = sqlx::query("SELECT id FROM mcp_tools WHERE server_id = ?")
                    .bind(&server_id)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|e| McpError::DatabaseError(e.to_string()))?;

                // 为每个工具创建授权记录
                for tool_row in tools {
                    let tool_id: String = tool_row.try_get("id").unwrap_or_default();
                    let relation_id = uuid::Uuid::new_v4().to_string();

                    sqlx::query(
                        r#"
                        INSERT OR IGNORE INTO api_key_tool_relations (id, api_key_id, tool_id, created_at)
                        VALUES (?, ?, ?, CURRENT_TIMESTAMP)
                        "#,
                    )
                    .bind(&relation_id)
                    .bind(&api_key_id)
                    .bind(&tool_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| McpError::DatabaseError(e.to_string()))?;

                    migrated_count += 1;
                }
            }

            // 提交事务
            tx.commit()
                .await
                .map_err(|e| McpError::DatabaseError(format!("迁移事务提交失败: {}", e)))?;

            info!("数据迁移完成：创建了 {} 条工具级授权记录", migrated_count);
        } else if !has_mcp_tools {
            // 全新安装，直接创建 mcp_tools 表
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS mcp_tools (
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

            // 创建索引
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tools_server_id ON mcp_tools(server_id)")
                .execute(pool)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tools_enabled ON mcp_tools(enabled)")
                .execute(pool)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            // 创建 api_key_tool_relations 表
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS api_key_tool_relations (
                    id TEXT PRIMARY KEY,
                    api_key_id TEXT NOT NULL,
                    tool_id TEXT NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(api_key_id, tool_id)
                )
                "#,
            )
            .execute(pool)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_api_key_id ON api_key_tool_relations(api_key_id)")
                .execute(pool)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_tool_id ON api_key_tool_relations(tool_id)")
                .execute(pool)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

            info!("全新安装：mcp_tools 和 api_key_tool_relations 表已创建");
        }

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
