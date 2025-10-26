use crate::db::{get_database, models::ApiKeyServerRelationRow};
use crate::error::{McpError, Result};
use sqlx::Row;
use tracing::{debug, info};

/// API 密钥-服务器关联数据仓库
pub struct ApiKeyServerRepository;

impl ApiKeyServerRepository {
    /// 获取 API 密钥的服务器权限列表
    pub async fn get_servers_by_api_key(api_key_id: &str) -> Result<Vec<String>> {
        debug!("Fetching servers for API key: {}", api_key_id);

        let db = get_database().await?;

        let rows = sqlx::query(
            "SELECT server_id FROM api_key_server_relations WHERE api_key_id = ? ORDER BY created_at"
        )
        .bind(api_key_id)
        .fetch_all(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let server_ids: Vec<String> = rows
            .into_iter()
            .filter_map(|row| row.try_get("server_id").ok())
            .collect();

        debug!(
            "Retrieved {} servers for API key: {}",
            server_ids.len(),
            api_key_id
        );
        Ok(server_ids)
    }

    /// 检查 API 密钥是否有服务器访问权限
    pub async fn has_permission(api_key_id: &str, server_id: &str) -> Result<bool> {
        debug!("Checking permission: {} -> {}", api_key_id, server_id);

        let db = get_database().await?;

        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM api_key_server_relations WHERE api_key_id = ? AND server_id = ?"
        )
        .bind(api_key_id)
        .bind(server_id)
        .fetch_one(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let count: i64 = row.get("count");
        let has_permission = count > 0;
        debug!(
            "Permission check result: {} -> {} = {}",
            api_key_id, server_id, has_permission
        );
        Ok(has_permission)
    }

    /// 添加 API 密钥对服务器的权限
    pub async fn add_permission(api_key_id: &str, server_id: &str) -> Result<()> {
        info!("Adding permission: {} -> {}", api_key_id, server_id);

        // 检查是否已存在
        if Self::has_permission(api_key_id, server_id).await? {
            debug!("Permission already exists: {} -> {}", api_key_id, server_id);
            return Ok(());
        }

        let db = get_database().await?;
        let relation = ApiKeyServerRelationRow::new(api_key_id.to_string(), server_id.to_string());
        let relation_id = relation
            .id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        sqlx::query(
            r#"
            INSERT INTO api_key_server_relations (id, api_key_id, server_id, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&relation_id)
        .bind(&relation.api_key_id)
        .bind(&relation.server_id)
        .bind(relation.created_at.to_rfc3339())
        .execute(&db)
        .await
        .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        info!("Permission added: {} -> {}", api_key_id, server_id);
        Ok(())
    }

    /// 移除 API 密钥的所有权限
    pub async fn remove_all_permissions(api_key_id: &str) -> Result<i64> {
        info!("Removing all permissions for API key: {}", api_key_id);

        let db = get_database().await?;

        let result = sqlx::query("DELETE FROM api_key_server_relations WHERE api_key_id = ?")
            .bind(api_key_id)
            .execute(&db)
            .await
            .map_err(|e| McpError::DatabaseError(e.to_string()))?;

        let removed = result.rows_affected() as i64;
        info!(
            "Removed {} permissions for API key: {}",
            removed, api_key_id
        );
        Ok(removed)
    }
}
