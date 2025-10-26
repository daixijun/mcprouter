use crate::db::{get_database, models::ToolRow};
use crate::error::{McpError, Result};
use sqlx::Row;
use tracing::{debug, info};

/// 工具数据仓库
pub struct ToolRepository;

impl ToolRepository {
    /// 创建新工具
    pub async fn create(tool: ToolRow) -> Result<String> {
        info!(
            "Creating tool: {} for server: {}",
            tool.name, tool.server_id
        );

        let db = get_database().await?;
        let tool_id = tool.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        sqlx::query(
            r#"
            INSERT INTO tools (id, name, server_id, description, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&tool_id)
        .bind(&tool.name)
        .bind(&tool.server_id)
        .bind(&tool.description)
        .bind(tool.enabled)
        .bind(tool.created_at.to_rfc3339())
        .bind(tool.updated_at.to_rfc3339())
        .execute(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        info!("Tool created with ID: {}", tool_id);
        Ok(tool_id)
    }

    /// 获取服务器的所有工具
    pub async fn get_by_server_id(server_id: &str) -> Result<Vec<ToolRow>> {
        debug!("Fetching tools for server: {}", server_id);

        let db = get_database().await?;

        let rows = sqlx::query("SELECT * FROM tools WHERE server_id = ? ORDER BY name")
            .bind(server_id)
            .fetch_all(&db)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let tools: Result<Vec<_>> = rows.into_iter().map(|row| Self::row_to_tool(row)).collect();

        let tools = tools?;
        debug!("Retrieved {} tools for server: {}", tools.len(), server_id);
        Ok(tools)
    }

    /// 根据名称获取工具
    pub async fn get_by_name(server_id: &str, name: &str) -> Result<Option<ToolRow>> {
        debug!("Fetching tool by name: {} for server: {}", name, server_id);

        let db = get_database().await?;

        let row = sqlx::query("SELECT * FROM tools WHERE server_id = ? AND name = ?")
            .bind(server_id)
            .bind(name)
            .fetch_optional(&db)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        match row {
            Some(r) => {
                let tool = Self::row_to_tool(r)?;
                debug!("Found tool: {} for server: {}", name, server_id);
                Ok(Some(tool))
            }
            None => {
                debug!("Tool not found: {} for server: {}", name, server_id);
                Ok(None)
            }
        }
    }

    /// 切换工具启用状态（通过ID）
    pub async fn toggle_enabled(tool_id: &str, enabled: bool) -> Result<bool> {
        info!("Toggling tool {} to enabled: {}", tool_id, enabled);

        let db = get_database().await?;

        let result = sqlx::query("UPDATE tools SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(tool_id)
            .execute(&db)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let toggled = result.rows_affected() > 0;
        info!(
            "Tool toggled: {} (affected rows: {})",
            tool_id,
            result.rows_affected()
        );
        Ok(toggled)
    }

    /// 批量切换服务器工具状态
    pub async fn batch_toggle_server_tools(server_id: &str, enabled: bool) -> Result<i64> {
        info!(
            "Toggling all tools for server {} to enabled: {}",
            server_id, enabled
        );

        let db = get_database().await?;

        let result =
            sqlx::query("UPDATE tools SET enabled = ?, updated_at = ? WHERE server_id = ?")
                .bind(enabled)
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(server_id)
                .execute(&db)
                .await
                .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let affected = result.rows_affected() as i64;
        info!("Batch toggled {} tools for server {}", affected, server_id);
        Ok(affected)
    }

    /// 更新工具描述（通过 server_id + name）
    pub async fn update_description(
        server_id: &str,
        name: &str,
        description: Option<String>,
    ) -> Result<bool> {
        info!(
            "Updating description for tool {} on server {}",
            name, server_id
        );

        let db = get_database().await?;

        let result = sqlx::query(
            "UPDATE tools SET description = ?, updated_at = ? WHERE server_id = ? AND name = ?",
        )
        .bind(&description)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(server_id)
        .bind(name)
        .execute(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// 按名称切换启用状态（无ID时兜底）
    pub async fn toggle_enabled_by_name(
        server_id: &str,
        name: &str,
        enabled: bool,
    ) -> Result<bool> {
        info!(
            "Toggling tool {} on server {} to enabled: {}",
            name, server_id, enabled
        );

        let db = get_database().await?;

        let result = sqlx::query(
            "UPDATE tools SET enabled = ?, updated_at = ? WHERE server_id = ? AND name = ?",
        )
        .bind(enabled)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(server_id)
        .bind(name)
        .execute(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// 将数据库行转换为工具对象
    fn row_to_tool(row: sqlx::sqlite::SqliteRow) -> Result<ToolRow> {
        let created_at_str: String = row
            .try_get("created_at")
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
        let updated_at_str: String = row
            .try_get("updated_at")
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());

        Ok(ToolRow {
            id: row.try_get("id").ok(),
            name: row.try_get("name").unwrap_or_default(),
            server_id: row.try_get("server_id").unwrap_or_default(),
            description: row.try_get("description").ok(),
            enabled: row.try_get("enabled").unwrap_or(true),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }
}
