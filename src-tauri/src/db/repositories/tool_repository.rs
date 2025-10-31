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
            INSERT INTO mcp_tools (id, name, server_id, description, enabled, created_at, updated_at)
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
        .map_err(McpError::from)?;

        info!("Created tool: {} for server: {}", tool.name, tool.server_id);
        Ok(tool_id)
    }

    /// 获取服务器的所有工具
    pub async fn get_by_server_id(server_id: &str) -> Result<Vec<ToolRow>> {
        debug!("Fetching tools for server: {}", server_id);

        let db = get_database().await?;

        let rows = sqlx::query("SELECT * FROM mcp_tools WHERE server_id = ? ORDER BY created_at")
            .bind(server_id)
            .fetch_all(&db)
            .await
            .map_err(McpError::from)?;

        let tools: Vec<ToolRow> = rows
            .into_iter()
            .filter_map(|row| Self::row_to_tool(&row).ok())
            .collect();

        debug!("Retrieved {} tools for server: {}", tools.len(), server_id);
        Ok(tools)
    }

    /// 根据名称获取工具
    pub async fn get_by_name(server_id: &str, name: &str) -> Result<Option<ToolRow>> {
        debug!("Fetching tool by name: {} for server: {}", name, server_id);

        let db = get_database().await?;

        let row = sqlx::query("SELECT * FROM mcp_tools WHERE server_id = ? AND name = ?")
            .bind(server_id)
            .bind(name)
            .fetch_optional(&db)
            .await
            .map_err(McpError::from)?;

        match row {
            Some(r) => {
                let tool = Self::row_to_tool(&r)?;
                debug!("Found tool: {}", tool.name);
                Ok(Some(tool))
            }
            None => {
                debug!("Tool not found: {}", name);
                Ok(None)
            }
        }
    }

    /// 切换工具启用状态（通过ID）
    pub async fn toggle_enabled(tool_id: &str, enabled: bool) -> Result<bool> {
        info!("Toggling tool {} to enabled: {}", tool_id, enabled);

        let db = get_database().await?;

        let result = sqlx::query("UPDATE mcp_tools SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(tool_id)
            .execute(&db)
            .await
            .map_err(McpError::from)?;

        let was_updated = result.rows_affected() > 0;
        info!(
            "Tool {} toggled to enabled: {}, updated: {}",
            tool_id, enabled, was_updated
        );
        Ok(was_updated)
    }

    /// 切换工具启用状态（通过名称）
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
            "UPDATE mcp_tools SET enabled = ?, updated_at = ? WHERE server_id = ? AND name = ?",
        )
        .bind(enabled)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(server_id)
        .bind(name)
        .execute(&db)
        .await
        .map_err(McpError::from)?;

        let was_updated = result.rows_affected() > 0;
        info!(
            "Tool {} on server {} toggled to enabled: {}, updated: {}",
            name, server_id, enabled, was_updated
        );
        Ok(was_updated)
    }

    /// 更新工具描述（通过名称）
    pub async fn update_description(
        server_id: &str,
        name: &str,
        description: Option<String>,
    ) -> Result<bool> {
        debug!(
            "Updating description for tool {} on server {}",
            name, server_id
        );

        let db = get_database().await?;

        let result = sqlx::query(
            "UPDATE mcp_tools SET description = ?, updated_at = ? WHERE server_id = ? AND name = ?",
        )
        .bind(description)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(server_id)
        .bind(name)
        .execute(&db)
        .await
        .map_err(McpError::from)?;

        let was_updated = result.rows_affected() > 0;
        debug!(
            "Updated description for tool {} on server {}: {}",
            name, server_id, was_updated
        );
        Ok(was_updated)
    }

    /// 启用服务器的所有工具
    pub async fn enable_all_by_server_id(server_id: &str) -> Result<usize> {
        info!("Enabling all tools for server: {}", server_id);

        let db = get_database().await?;

        let result =
            sqlx::query("UPDATE mcp_tools SET enabled = 1, updated_at = ? WHERE server_id = ?")
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(server_id)
                .execute(&db)
                .await
                .map_err(McpError::from)?;

        let count = result.rows_affected() as usize;
        info!("Enabled {} tools for server: {}", count, server_id);
        Ok(count)
    }

    /// 禁用服务器的所有工具
    pub async fn disable_all_by_server_id(server_id: &str) -> Result<usize> {
        info!("Disabling all tools for server: {}", server_id);

        let db = get_database().await?;

        let result =
            sqlx::query("UPDATE mcp_tools SET enabled = 0, updated_at = ? WHERE server_id = ?")
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(server_id)
                .execute(&db)
                .await
                .map_err(McpError::from)?;

        let count = result.rows_affected() as usize;
        info!("Disabled {} tools for server: {}", count, server_id);
        Ok(count)
    }

    /// 批量切换服务器下所有工具的启用状态
    pub async fn batch_toggle_server_tools(server_id: &str, enabled: bool) -> Result<usize> {
        if enabled {
            Self::enable_all_by_server_id(server_id).await
        } else {
            Self::disable_all_by_server_id(server_id).await
        }
    }

    /// 将数据库行转换为工具对象
    fn row_to_tool(row: &sqlx::sqlite::SqliteRow) -> Result<ToolRow> {
        let id: Option<String> = row.try_get("id").ok();
        let name: String = row.try_get("name").unwrap_or_default();
        let server_id: String = row.try_get("server_id").unwrap_or_default();
        let description: Option<String> = row.try_get("description").ok();
        let enabled: bool = row.try_get("enabled").unwrap_or(false);
        let created_at: chrono::DateTime<chrono::Utc> = row
            .try_get::<String, _>("created_at")
            .ok()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            })
            .unwrap_or_else(chrono::Utc::now);
        let updated_at: chrono::DateTime<chrono::Utc> = row
            .try_get::<String, _>("updated_at")
            .ok()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            })
            .unwrap_or_else(chrono::Utc::now);

        Ok(ToolRow {
            id,
            name,
            server_id,
            description,
            enabled,
            created_at,
            updated_at,
        })
    }
}
