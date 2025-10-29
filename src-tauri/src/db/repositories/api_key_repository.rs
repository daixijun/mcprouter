use crate::db::{get_database, models::ApiKeyRow};
use crate::error::{McpError, Result};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tracing::{debug, info};

/// API 密钥数据仓库
pub struct ApiKeyRepository;

impl ApiKeyRepository {
    /// 创建新 API 密钥
    pub async fn create(name: String, key: String) -> Result<ApiKeyRow> {
        let name_clone = name.clone();
        info!("Creating API key: {}", name);

        let key_hash = Self::hash_key(&key);
        let mut api_key = ApiKeyRow::new(name, key_hash);
        api_key.last_used_at = None; // 初始化为未使用

        let db = get_database().await?;

        sqlx::query(
            r#"
            INSERT INTO api_keys (id, name, key_hash, enabled, created_at, updated_at, last_used_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&api_key.id)
        .bind(&api_key.name)
        .bind(&api_key.key_hash)
        .bind(api_key.enabled)
        .bind(api_key.created_at.to_rfc3339())
        .bind(api_key.updated_at.to_rfc3339())
        .bind(api_key.last_used_at.map(|dt| dt.to_rfc3339()))
        .execute(&db)
        .await
        .map_err(McpError::from)?;

        info!("API key created: {}", name_clone);
        Ok(api_key)
    }

    /// 获取所有 API 密钥
    pub async fn get_all() -> Result<Vec<ApiKeyRow>> {
        debug!("Fetching all API keys");

        let db = get_database().await?;

        let rows = sqlx::query("SELECT * FROM api_keys ORDER BY created_at DESC")
            .fetch_all(&db)
            .await
            .map_err(McpError::from)?;

        let keys: Result<Vec<_>> = rows
            .into_iter()
            .map(|row| Self::row_to_api_key(row))
            .collect();

        let keys = keys?;
        debug!("Retrieved {} API keys", keys.len());
        Ok(keys)
    }

    /// 根据ID获取 API 密钥
    pub async fn get_by_id(id: &str) -> Result<Option<ApiKeyRow>> {
        debug!("Fetching API key by ID: {}", id);

        let db = get_database().await?;

        let row =
            sqlx::query("SELECT * FROM api_keys WHERE id = ? ORDER BY updated_at DESC LIMIT 1")
                .bind(id)
                .fetch_optional(&db)
                .await
                .map_err(McpError::from)?;

        match row {
            Some(r) => {
                let key = Self::row_to_api_key(r)?;
                debug!("Found API key: {}", id);
                Ok(Some(key))
            }
            None => {
                debug!("API key not found: {}", id);
                Ok(None)
            }
        }
    }

    /// 验证 API 密钥
    pub async fn verify_key(key: &str) -> Result<Option<ApiKeyRow>> {
        debug!("Verifying API key");

        let key_hash = Self::hash_key(key);

        let db = get_database().await?;

        let row = sqlx::query("SELECT * FROM api_keys WHERE key_hash = ? AND enabled = 1")
            .bind(&key_hash)
            .fetch_optional(&db)
            .await
            .map_err(McpError::from)?;

        match row {
            Some(r) => {
                let mut api_key = Self::row_to_api_key(r)?;
                // Update last_used_at and updated_at
                sqlx::query("UPDATE api_keys SET last_used_at = ?, updated_at = ? WHERE id = ?")
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(&api_key.id)
                    .execute(&db)
                    .await
                    .map_err(McpError::from)?;
                api_key.last_used_at = Some(chrono::Utc::now());
                debug!("API key verified successfully");
                Ok(Some(api_key))
            }
            None => {
                debug!("API key verification failed");
                Ok(None)
            }
        }
    }


    /// 切换 API 密钥启用状态
    pub async fn toggle_enabled(id: &str, enabled: bool) -> Result<bool> {
        info!("Toggling API key {} to enabled: {}", id, enabled);

        let db = get_database().await?;

        let result = sqlx::query("UPDATE api_keys SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(id)
            .execute(&db)
            .await
            .map_err(McpError::from)?;

        let toggled = result.rows_affected() > 0;
        info!(
            "API key toggled: {} (affected rows: {})",
            id,
            result.rows_affected()
        );
        Ok(toggled)
    }

    /// 删除 API 密钥
    pub async fn delete(id: &str) -> Result<bool> {
        info!("Deleting API key: {}", id);

        let db = get_database().await?;

        let result = sqlx::query("DELETE FROM api_keys WHERE id = ?")
            .bind(id)
            .execute(&db)
            .await
            .map_err(McpError::from)?;

        let deleted = result.rows_affected() > 0;
        info!(
            "API key deleted: {} (affected rows: {})",
            id,
            result.rows_affected()
        );
        Ok(deleted)
    }

    /// 获取启用的 API 密钥数量
    pub async fn count_enabled() -> Result<i64> {
        debug!("Counting enabled API keys");

        let db = get_database().await?;

        let row = sqlx::query("SELECT COUNT(*) as count FROM api_keys WHERE enabled = 1")
            .fetch_one(&db)
            .await
            .map_err(McpError::from)?;

        let count: i64 = row.get("count");
        debug!("Found {} enabled API keys", count);
        Ok(count)
    }

    /// 生成密钥哈希
    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// 将数据库行转换为 API 密钥对象
    fn row_to_api_key(row: sqlx::sqlite::SqliteRow) -> Result<ApiKeyRow> {
        let created_at_str: String = row
            .try_get("created_at")
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
        let updated_at_str: String = row
            .try_get("updated_at")
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
        let last_used_at_str: Option<String> = row.try_get("last_used_at").ok().flatten();

        Ok(ApiKeyRow {
            id: row.try_get("id").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            key_hash: row.try_get("key_hash").unwrap_or_default(),
            enabled: row.try_get("enabled").unwrap_or(true),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            last_used_at: last_used_at_str
                .and_then(|dt_str| chrono::DateTime::parse_from_rfc3339(&dt_str).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
        })
    }
}
