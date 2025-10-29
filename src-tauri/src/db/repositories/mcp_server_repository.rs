use crate::db::{get_database, models::McpServerRow};
use crate::error::{McpError, Result};
use chrono::Utc;
use serde_json;
use sqlx::Row;
use tracing::{debug, info};

/// MCP 服务器数据仓库
pub struct McpServerRepository;

impl McpServerRepository {
    /// 创建新的 MCP 服务器配置
    pub async fn create(mut server: McpServerRow) -> Result<String> {
        info!("Creating MCP server: {}", server.name);

        // Generate ID if not provided
        if server.id.is_none() {
            server.id = Some(uuid::Uuid::new_v4().to_string());
        }
        let server_id = server.id.clone().unwrap();

        let db = get_database().await?;
        let args_json = server
            .args
            .map(|args| serde_json::to_string(&args).unwrap_or_default());
        let env_vars_json = server
            .env_vars
            .map(|vars| serde_json::to_string(&vars).unwrap_or_default());
        let headers_json = server
            .headers
            .map(|headers| serde_json::to_string(&headers).unwrap_or_default());

        let _result = sqlx::query(
            r#"
            INSERT INTO mcp_servers (id, name, description, command, args, transport, url, enabled, env_vars, headers, version, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&server.id)
        .bind(&server.name)
        .bind(&server.description)
        .bind(&server.command)
        .bind(&args_json)
        .bind(&server.transport)
        .bind(&server.url)
        .bind(server.enabled)
        .bind(&env_vars_json)
        .bind(&headers_json)
        .bind(&server.version)
        .bind(server.created_at.to_rfc3339())
        .bind(server.updated_at.to_rfc3339())
        .execute(&db)
        .await
        .map_err(McpError::from)?;

        info!("MCP server created with ID: {}", server_id);
        Ok(server_id)
    }

    /// 获取所有 MCP 服务器
    pub async fn get_all() -> Result<Vec<McpServerRow>> {
        debug!("Fetching all MCP servers");

        let db = get_database().await?;

        let rows = sqlx::query("SELECT * FROM mcp_servers ORDER BY created_at DESC")
            .fetch_all(&db)
            .await
            .map_err(McpError::from)?;

        let servers: Result<Vec<_>> = rows
            .into_iter()
            .map(|row| Self::row_to_server(row))
            .collect();

        let servers = servers?;
        debug!("Retrieved {} MCP servers", servers.len());
        Ok(servers)
    }

    /// 根据名称获取 MCP 服务器
    pub async fn get_by_name(name: &str) -> Result<Option<McpServerRow>> {
        debug!("Fetching MCP server by name: {}", name);

        let db = get_database().await?;

        let row = sqlx::query("SELECT * FROM mcp_servers WHERE name = ?")
            .bind(name)
            .fetch_optional(&db)
            .await
            .map_err(McpError::from)?;

        match row {
            Some(r) => {
                let server = Self::row_to_server(r)?;
                debug!("Found MCP server: {}", name);
                Ok(Some(server))
            }
            None => {
                debug!("MCP server not found: {}", name);
                Ok(None)
            }
        }
    }

    /// 根据ID获取 MCP 服务器
    pub async fn get_by_id(id: &str) -> Result<Option<McpServerRow>> {
        debug!("Fetching MCP server by ID: {}", id);

        let db = get_database().await?;

        let row = sqlx::query("SELECT * FROM mcp_servers WHERE id = ?")
            .bind(id)
            .fetch_optional(&db)
            .await
            .map_err(McpError::from)?;

        match row {
            Some(r) => {
                let server = Self::row_to_server(r)?;
                debug!("Found MCP server with ID: {}", id);
                Ok(Some(server))
            }
            None => {
                debug!("MCP server not found with ID: {}", id);
                Ok(None)
            }
        }
    }

    /// 删除 MCP 服务器及其关联数据
    pub async fn delete(name: &str) -> Result<bool> {
        info!("Deleting MCP server and related data: {}", name);

        let db = get_database().await?;

        // 首先获取服务器ID
        let server_row = sqlx::query("SELECT id FROM mcp_servers WHERE name = ?")
            .bind(name)
            .fetch_optional(&db)
            .await
            .map_err(McpError::from)?;

        let Some(server_row) = server_row else {
            debug!("MCP server not found for deletion: {}", name);
            return Ok(false);
        };

        let server_id: String = server_row.get("id");

        // 开始事务删除所有相关数据
        let mut tx = db
            .begin()
            .await
            .map_err(McpError::from)?;

        // 1. 删除该服务器的所有工具
        let tools_deleted = sqlx::query("DELETE FROM tools WHERE server_id = ?")
            .bind(&server_id)
            .execute(&mut *tx)
            .await
            .map_err(McpError::from)?;

        // 2. 删除API密钥与该服务器的关联关系
        let relations_deleted =
            sqlx::query("DELETE FROM api_key_server_relations WHERE server_id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(McpError::from)?;

        // 3. 删除服务器本身
        let server_deleted = sqlx::query("DELETE FROM mcp_servers WHERE name = ?")
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(McpError::from)?;

        // 提交事务
        tx.commit()
            .await
            .map_err(McpError::from)?;

        let total_deleted = server_deleted.rows_affected() > 0;
        info!(
            "MCP server and related data deleted: {} (tools: {}, relations: {}, server: {})",
            name,
            tools_deleted.rows_affected(),
            relations_deleted.rows_affected(),
            server_deleted.rows_affected()
        );
        Ok(total_deleted)
    }

    /// 切换服务器启用状态
    pub async fn toggle_enabled(name: &str, enabled: bool) -> Result<bool> {
        info!("Toggling MCP server {} to enabled: {}", name, enabled);

        let db = get_database().await?;

        let result =
            sqlx::query("UPDATE mcp_servers SET enabled = ?, updated_at = ? WHERE name = ?")
                .bind(enabled)
                .bind(Utc::now().to_rfc3339())
                .bind(name)
                .execute(&db)
                .await
                .map_err(McpError::from)?;

        let toggled = result.rows_affected() > 0;
        info!(
            "MCP server toggled: {} (affected rows: {})",
            name,
            result.rows_affected()
        );
        Ok(toggled)
    }

    /// 将数据库行转换为服务器对象
    fn row_to_server(row: sqlx::sqlite::SqliteRow) -> Result<McpServerRow> {
        let args_json: Option<String> = row.try_get("args").ok().flatten();
        let args = args_json.and_then(|json| serde_json::from_str(&json).ok());

        let env_vars_json: Option<String> = row.try_get("env_vars").ok().flatten();
        let env_vars = env_vars_json.and_then(|json| serde_json::from_str(&json).ok());

        let headers_json: Option<String> = row.try_get("headers").ok().flatten();
        let headers = headers_json.and_then(|json| serde_json::from_str(&json).ok());

        let created_at_str: String = row
            .try_get("created_at")
            .unwrap_or_else(|_| Utc::now().to_rfc3339());
        let updated_at_str: String = row
            .try_get("updated_at")
            .unwrap_or_else(|_| Utc::now().to_rfc3339());

        Ok(McpServerRow {
            id: row.try_get("id").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            description: row.try_get("description").ok(),
            command: row.try_get("command").ok(),
            args,
            transport: row.try_get("transport").unwrap_or_default(),
            url: row.try_get("url").ok(),
            enabled: row.try_get("enabled").unwrap_or(true),
            env_vars,
            headers,
            version: row.try_get("version").ok(),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    pub async fn update_version(name: &str, version: Option<String>) -> Result<()> {
        let db = get_database().await?;
        let updated_at = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE mcp_servers
            SET version = ?, updated_at = ?
            WHERE name = ?
            "#,
        )
        .bind(&version)
        .bind(updated_at)
        .bind(name)
        .execute(&db)
        .await
        .map_err(McpError::from)?;

        info!("Updated version for server {} to {:?}", name, version);
        Ok(())
    }
}
