use crate::db::{get_database, models::ApiKeyToolRelationRow};
use crate::error::{McpError, Result};
use sqlx::Row;
use tracing::{debug, info};

/// API 密钥-工具关联数据仓库（工具级别授权）
pub struct ApiKeyToolRepository;

impl ApiKeyToolRepository {
    /// 检查 API 密钥是否有权限访问特定工具
    pub async fn has_tool_permission(api_key_id: &str, tool_id: &str) -> Result<bool> {
        debug!("检查权限: API Key {} -> Tool {}", api_key_id, tool_id);

        let db = get_database().await?;

        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM api_key_tool_relations WHERE api_key_id = ? AND tool_id = ?"
        )
        .bind(api_key_id)
        .bind(tool_id)
        .fetch_one(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let count: i64 = row.get("count");
        let has_permission = count > 0;
        debug!(
            "权限检查结果: {} -> {} = {}",
            api_key_id, tool_id, has_permission
        );
        Ok(has_permission)
    }

    /// 获取 API 密钥的工具权限列表
    pub async fn get_tools_by_api_key(api_key_id: &str) -> Result<Vec<String>> {
        debug!("获取 API Key 的工具列表: {}", api_key_id);

        let db = get_database().await?;

        let rows = sqlx::query(
            "SELECT tool_id FROM api_key_tool_relations WHERE api_key_id = ? ORDER BY created_at"
        )
        .bind(api_key_id)
        .fetch_all(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let tool_ids: Vec<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get("tool_id").ok())
            .collect();

        debug!(
            "检索到 {} 个工具授权，API Key: {}",
            tool_ids.len(),
            api_key_id
        );
        Ok(tool_ids)
    }

    /// 添加 API 密钥对工具的权限
    pub async fn add_tool_permission(api_key_id: &str, tool_id: &str) -> Result<()> {
        info!("添加工具权限: {} -> {}", api_key_id, tool_id);

        // 检查是否已存在
        if Self::has_tool_permission(api_key_id, tool_id).await? {
            debug!("工具权限已存在: {} -> {}", api_key_id, tool_id);
            return Ok(());
        }

        let db = get_database().await?;
        let relation = ApiKeyToolRelationRow::new(api_key_id.to_string(), tool_id.to_string());
        let relation_id = relation
            .id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        sqlx::query(
            r#"
            INSERT INTO api_key_tool_relations (id, api_key_id, tool_id, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&relation_id)
        .bind(&relation.api_key_id)
        .bind(&relation.tool_id)
        .bind(relation.created_at.to_rfc3339())
        .execute(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        info!("工具权限已添加: {} -> {}", api_key_id, tool_id);
        Ok(())
    }

    /// 批量授权：授权 API Key 访问某个 Server 的所有工具
    pub async fn grant_server_tools(api_key_id: &str, server_id: &str) -> Result<usize> {
        info!(
            "批量授权 Server 的所有工具: API Key {} -> Server {}",
            api_key_id, server_id
        );

        let db = get_database().await?;

        // 查询该 Server 下的所有工具
        let tools = sqlx::query("SELECT id FROM mcp_tools WHERE server_id = ?")
            .bind(server_id)
            .fetch_all(&db)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let mut granted_count = 0;

        for tool_row in tools {
            let tool_id: String = tool_row.try_get("id").unwrap_or_default();

            // 为每个工具添加权限（跳过已存在的）
            match Self::add_tool_permission(api_key_id, &tool_id).await {
                Ok(_) => granted_count += 1,
                Err(e) => {
                    debug!("跳过工具 {}: {}", tool_id, e);
                }
            }
        }

        info!(
            "批量授权完成: 授权了 {} 个工具，API Key {} -> Server {}",
            granted_count, api_key_id, server_id
        );
        Ok(granted_count)
    }

    /// 批量撤销授权：撤销 API Key 对某个 Server 所有工具的权限
    pub async fn revoke_server_tools(api_key_id: &str, server_id: &str) -> Result<usize> {
        info!(
            "批量撤销 Server 的所有工具权限: API Key {} -> Server {}",
            api_key_id, server_id
        );

        let db = get_database().await?;

        // 查询该 Server 下的所有工具 ID
        let tools = sqlx::query("SELECT id FROM mcp_tools WHERE server_id = ?")
            .bind(server_id)
            .fetch_all(&db)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let tool_ids: Vec<String> = tools
            .into_iter()
            .filter_map(|row| row.try_get("id").ok())
            .collect();

        if tool_ids.is_empty() {
            info!("Server {} 没有工具，无需撤销权限", server_id);
            return Ok(0);
        }

        // 构建 IN 子句的占位符
        let placeholders: Vec<String> = (0..tool_ids.len()).map(|_| "?".to_string()).collect();
        let in_clause = placeholders.join(", ");

        let query_str = format!(
            "DELETE FROM api_key_tool_relations WHERE api_key_id = ? AND tool_id IN ({})",
            in_clause
        );

        let mut query = sqlx::query(&query_str).bind(api_key_id);
        for tool_id in &tool_ids {
            query = query.bind(tool_id);
        }

        let result = query
            .execute(&db)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let removed = result.rows_affected() as usize;
        info!(
            "批量撤销完成: 撤销了 {} 个工具权限，API Key {} -> Server {}",
            removed, api_key_id, server_id
        );
        Ok(removed)
    }

    /// 移除 API 密钥的所有工具权限
    pub async fn remove_all_permissions(api_key_id: &str) -> Result<i64> {
        info!("移除 API Key 的所有工具权限: {}", api_key_id);

        let db = get_database().await?;

        let result = sqlx::query("DELETE FROM api_key_tool_relations WHERE api_key_id = ?")
            .bind(api_key_id)
            .execute(&db)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let removed = result.rows_affected() as i64;
        info!(
            "已移除 {} 个工具权限，API Key: {}",
            removed, api_key_id
        );
        Ok(removed)
    }

}
