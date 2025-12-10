// Token storage implementation with SQLite
#![allow(dead_code)]

use super::{Result, StorageError};
use crate::types::Token;
use crate::token_manager::TokenInfo;
use chrono::{DateTime, Utc};
use sqlx::{query, Row, SqlitePool};
use uuid::Uuid;

/// Token storage with SQLite backend
pub struct TokenStorage {
    pool: SqlitePool,
}

impl TokenStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new token
    pub async fn create_token(&self, token: &Token) -> Result<()> {
        let now = Utc::now();

        query(
            r#"
            INSERT INTO tokens (id, name, token, description, created_at, updated_at, enabled, last_used_at, usage_count, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&token.id)
        .bind(&token.name)
        .bind(&token.value)
        .bind(&token.description)
        .bind(now)
        .bind(now)
        .bind(token.enabled)
        .bind(token.last_used_at.map(|ts| DateTime::from_timestamp(ts as i64, 0).unwrap_or(now)))
        .bind(token.usage_count as i64)
        .bind(token.expires_at.map(|ts| DateTime::from_timestamp(ts as i64, 0).unwrap_or(now)))
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to create token: {}", e)))?;

        Ok(())
    }

    /// Get token by database ID
    pub async fn get_token_by_id(&self, token_id: &str) -> Result<Token> {
        let row = query(
            r#"
            SELECT id, name, token, description, created_at, updated_at, enabled, last_used_at, usage_count, expires_at
            FROM tokens
            WHERE id = ?
            "#,
        )
        .bind(token_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get token: {}", e)))?;

        match row {
            Some(row) => {
                let mut token = Token {
                    id: token_id.to_string(),
                    name: row.get("name"),
                    value: row.get("token"),
                    description: row.get("description"),
                    created_at: row.get::<DateTime<Utc>, _>("created_at").timestamp() as u64,
                    expires_at: row.get::<Option<DateTime<Utc>>, _>("expires_at").map(|dt| dt.timestamp() as u64),
                    last_used_at: row.get::<Option<DateTime<Utc>>, _>("last_used_at").map(|dt| dt.timestamp() as u64),
                    usage_count: row.get::<i64, _>("usage_count") as u64,
                    enabled: row.get::<bool, _>("enabled"),
                    allowed_tools: None,
                    allowed_resources: None,
                    allowed_prompts: None,
                    allowed_prompt_templates: None,
                };

                // Load permissions for this token
                token.allowed_tools = Some(self.get_token_permissions(token_id, "tool").await?);
                token.allowed_resources = Some(self.get_token_permissions(token_id, "resource").await?);
                token.allowed_prompts = Some(self.get_token_permissions(token_id, "prompt").await?);
                token.allowed_prompt_templates = Some(self.get_token_permissions(token_id, "prompt_template").await?);

                Ok(token)
            }
            None => Err(StorageError::NotFound(format!("Token with id {} not found", token_id))),
        }
    }

    /// Get token by value (for validation)
    pub async fn get_token_by_value(&self, token_value: &str) -> Result<Option<Token>> {
        let row = query(
            r#"
            SELECT id, name, token, description, created_at, updated_at, enabled, last_used_at, usage_count, expires_at
            FROM tokens
            WHERE token = ?
            "#,
        )
        .bind(token_value)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get token by value: {}", e)))?;

        match row {
            Some(row) => {
                let token_id: String = row.get("id");
                let mut token = Token {
                    id: token_id.clone(),
                    name: row.get("name"),
                    value: row.get("token"),
                    description: row.get("description"),
                    created_at: row.get::<DateTime<Utc>, _>("created_at").timestamp() as u64,
                    expires_at: row.get::<Option<DateTime<Utc>>, _>("expires_at").map(|dt| dt.timestamp() as u64),
                    last_used_at: row.get::<Option<DateTime<Utc>>, _>("last_used_at").map(|dt| dt.timestamp() as u64),
                    usage_count: row.get::<i64, _>("usage_count") as u64,
                    enabled: row.get::<bool, _>("enabled"),
                    allowed_tools: None,
                    allowed_resources: None,
                    allowed_prompts: None,
                    allowed_prompt_templates: None,
                };

                // Load permissions for this token
                token.allowed_tools = Some(self.get_token_permissions(&token_id, "tool").await?);
                token.allowed_resources = Some(self.get_token_permissions(&token_id, "resource").await?);
                token.allowed_prompts = Some(self.get_token_permissions(&token_id, "prompt").await?);
                token.allowed_prompt_templates = Some(self.get_token_permissions(&token_id, "prompt_template").await?);

                Ok(Some(token))
            }
            None => Ok(None),
        }
    }

    /// List all tokens as TokenInfo
    pub async fn list_tokens(&self) -> Result<Vec<TokenInfo>> {
        let rows = query(
            r#"
            SELECT t.id, t.name, t.token, t.description, t.created_at, t.updated_at,
                   t.enabled, t.last_used_at, t.usage_count, t.expires_at,
                   GROUP_CONCAT(CASE
                        WHEN p.resource_type = 'tool' AND p.allowed = 1
                        THEN (SELECT mt.id FROM mcp_server_tools mt
                              JOIN mcp_servers ms ON mt.server_id = ms.id
                              WHERE ms.enabled = 1 AND (
                                -- 稳定标识符格式: server_name__tool_name
                                ms.name || '__' || mt.name = p.resource_id OR
                                -- 兼容UUID格式（如果存在）
                                mt.id = p.resource_id OR
                                -- 兼容纯工具名称格式（如果存在）
                                mt.name = p.resource_id
                              ))
                   END) as allowed_tools,
                   GROUP_CONCAT(CASE
                        WHEN p.resource_type = 'resource' AND p.allowed = 1
                        THEN (SELECT mr.id FROM mcp_server_resources mr
                              JOIN mcp_servers ms ON mr.server_id = ms.id
                              WHERE ms.enabled = 1 AND (
                                -- 稳定标识符格式: server_name__resource_uri
                                ms.name || '__' || mr.uri = p.resource_id OR
                                -- 兼容UUID格式（如果存在）
                                mr.id = p.resource_id OR
                                -- 兼容纯资源名称格式（如果存在）
                                mr.name = p.resource_id
                              ))
                   END) as allowed_resources,
                   GROUP_CONCAT(CASE
                        WHEN p.resource_type = 'prompt' AND p.allowed = 1
                        THEN (SELECT mp.id FROM mcp_server_prompts mp
                              JOIN mcp_servers ms ON mp.server_id = ms.id
                              WHERE ms.enabled = 1 AND (
                                -- 稳定标识符格式: server_name__prompt_name
                                ms.name || '__' || mp.name = p.resource_id OR
                                -- 兼容UUID格式（如果存在）
                                mp.id = p.resource_id OR
                                -- 兼容纯提示词名称格式（如果存在）
                                mp.name = p.resource_id
                              ))
                   END) as allowed_prompts,
                   '' as allowed_prompt_templates
            FROM tokens t
            LEFT JOIN permissions p ON t.id = p.token_id
            GROUP BY t.id, t.name, t.token, t.description, t.created_at, t.updated_at,
                     t.enabled, t.last_used_at, t.usage_count, t.expires_at
            ORDER BY t.created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to list tokens: {}", e)))?;

        let mut tokens = Vec::new();
        let now = Utc::now();
        for row in rows {
            let token_id: String = row.get("id");
            let expires_at: Option<DateTime<Utc>> = row.get("expires_at");
            let is_expired = expires_at.map_or(false, |exp| now > exp);

            // 处理GROUP_CONCAT结果，将逗号分隔的字符串转换为Vec<String>
            fn split_concat_string(s: Option<String>) -> Vec<String> {
                match s {
                    Some(s) if !s.is_empty() => s.split(',').map(|s| s.to_string()).collect(),
                    _ => Vec::new(),
                }
            }

            
            let token_info = TokenInfo {
                id: token_id.clone(),
                name: row.get("name"),
                description: row.get("description"),
                created_at: row.get::<DateTime<Utc>, _>("created_at").timestamp() as u64,
                expires_at: expires_at.map(|dt| dt.timestamp() as u64),
                last_used_at: row.get::<Option<DateTime<Utc>>, _>("last_used_at").map(|dt| dt.timestamp() as u64),
                usage_count: row.get::<i64, _>("usage_count") as u64,
                is_expired,
                enabled: row.get::<bool, _>("enabled"),
                allowed_tools: split_concat_string(row.get("allowed_tools")),
                allowed_resources: split_concat_string(row.get("allowed_resources")),
                allowed_prompts: split_concat_string(row.get("allowed_prompts")),
                allowed_prompt_templates: split_concat_string(row.get("allowed_prompt_templates")),
            };
            tokens.push(token_info);
        }

        Ok(tokens)
    }

    /// Update token usage statistics
    pub async fn update_token_usage(&self, token_id: &str) -> Result<()> {
        let now = Utc::now();

        query(
            r#"
            UPDATE tokens
            SET last_used_at = ?, usage_count = usage_count + 1, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(token_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to update token usage: {}", e)))?;

        Ok(())
    }

    /// Toggle token enabled status
    pub async fn toggle_token_enabled(&self, token_id: &str, enabled: bool) -> Result<()> {
        let now = Utc::now();

        query(
            "UPDATE tokens SET enabled = ?, updated_at = ? WHERE id = ?"
        )
        .bind(enabled)
        .bind(now)
        .bind(token_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to toggle token status: {}", e)))?;

        Ok(())
    }

    /// Clean up expired tokens and return count
    pub async fn cleanup_expired_tokens(&self) -> Result<usize> {
        let now = Utc::now();

        let result = query("DELETE FROM tokens WHERE expires_at IS NOT NULL AND expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to cleanup expired tokens: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    /// Get token with all permissions
    pub async fn get_token_with_permissions(&self, token_id: &str) -> Result<Token> {
        self.get_token_by_id(token_id).await
    }

    /// Update token basic info
    pub async fn update_token(&self, token_id: &str, name: Option<String>, description: Option<String>) -> Result<()> {
        let mut updates = Vec::new();
        let mut params = Vec::new();

        if let Some(name) = name {
            updates.push("name = ?");
            params.push(name);
        }

        if let Some(description) = description {
            updates.push("description = ?");
            params.push(description);
        }

        if updates.is_empty() {
            return Ok(()); // Nothing to update
        }

        updates.push("updated_at = ?");
        params.push(Utc::now().to_rfc3339());

        let sql = format!(
            "UPDATE tokens SET {} WHERE id = ?",
            updates.join(", ")
        );

        let mut query_builder = query(&sql);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        query_builder = query_builder.bind(token_id);

        query_builder
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to update token: {}", e)))?;

        Ok(())
    }

    /// Delete a token and all its permissions
    pub async fn delete_token(&self, token_id: &str) -> Result<()> {
        // Use a transaction for atomic deletion
        let mut tx = self.pool.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        // Delete permissions first
        query("DELETE FROM permissions WHERE token_id = ?")
            .bind(token_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete token permissions: {}", e)))?;

        // Delete token
        let result = query("DELETE FROM tokens WHERE id = ?")
            .bind(token_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete token: {}", e)))?;

        tx.commit().await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("Token with id {} not found", token_id)));
        }

        Ok(())
    }

    /// Add a permission to a token
    pub async fn add_permission(&self, token_id: &str, resource_type: &str, resource_id: &str) -> Result<()> {
        let permission_id = Uuid::now_v7().to_string();
        let now = Utc::now();

        // Normalize resource_id to stable identifier format
        let stable_resource_id = self.normalize_resource_id(resource_type, resource_id).await?;

        query(
            r#"
            INSERT INTO permissions (id, token_id, resource_type, resource_id, allowed, created_at, updated_at)
            VALUES (?, ?, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(&permission_id)
        .bind(token_id)
        .bind(resource_type)
        .bind(&stable_resource_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                StorageError::AlreadyExists(format!("Permission already exists for {} {}", resource_type, stable_resource_id))
            } else {
                StorageError::Database(format!("Failed to add permission: {}", e))
            }
        })?;

        Ok(())
    }

    /// Remove a permission from a token
    pub async fn remove_permission(&self, token_id: &str, resource_type: &str, resource_id: &str) -> Result<()> {
        // Normalize resource_id to stable identifier format
        let stable_resource_id = self.normalize_resource_id(resource_type, resource_id).await?;

        let result = query(
            "DELETE FROM permissions WHERE token_id = ? AND resource_type = ? AND resource_id = ?"
        )
        .bind(token_id)
        .bind(resource_type)
        .bind(&stable_resource_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to remove permission: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Permission not found for {} {}",
                resource_type, stable_resource_id
            )));
        }

        Ok(())
    }

    /// Get permissions for a specific token and resource type
    async fn get_token_permissions(&self, token_id: &str, resource_type: &str) -> Result<Vec<String>> {
        let rows = query(
            r#"
            SELECT resource_id
            FROM permissions
            WHERE token_id = ? AND resource_type = ? AND allowed = 1
            ORDER BY created_at ASC
            "#,
        )
        .bind(token_id)
        .bind(resource_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get token permissions: {}", e)))?;

        Ok(rows.into_iter()
            .map(|row| row.get::<String, _>("resource_id"))
            .collect())
    }

    /// Check if a specific permission exists for a token
    pub async fn has_permission(&self, token_id: &str, resource_type: &str, resource_id: &str) -> Result<bool> {
        let row = query(
            "SELECT COUNT(*) as count FROM permissions WHERE token_id = ? AND resource_type = ? AND resource_id = ? AND allowed = 1"
        )
        .bind(token_id)
        .bind(resource_type)
        .bind(resource_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to check permission: {}", e)))?;

        match row {
            Some(row) => {
                let count: i64 = row.get("count");
                Ok(count > 0)
            }
            None => Ok(false),
        }
    }

    /// Convert UUID to stable identifier format (server_name__resource_name)
    async fn convert_uuid_to_stable_id(&self, resource_type: &str, uuid: &str) -> Result<String> {
        match resource_type {
            "tool" => {
                let row = query(
                    r#"
                    SELECT s.name || '__' || t.name as stable_id
                    FROM mcp_server_tools t
                    JOIN mcp_servers s ON t.server_id = s.id
                    WHERE t.id = ?
                    "#,
                )
                .bind(uuid)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to convert tool UUID to stable ID: {}", e)))?;

                match row {
                    Some(row) => Ok(row.get::<String, _>("stable_id")),
                    None => Err(StorageError::NotFound(format!("Tool with UUID {} not found", uuid))),
                }
            }
            "resource" => {
                let row = query(
                    r#"
                    SELECT s.name || '__' || r.uri as stable_id
                    FROM mcp_server_resources r
                    JOIN mcp_servers s ON r.server_id = s.id
                    WHERE r.id = ?
                    "#,
                )
                .bind(uuid)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to convert resource UUID to stable ID: {}", e)))?;

                match row {
                    Some(row) => Ok(row.get::<String, _>("stable_id")),
                    None => Err(StorageError::NotFound(format!("Resource with UUID {} not found", uuid))),
                }
            }
            "prompt" => {
                let row = query(
                    r#"
                    SELECT s.name || '__' || p.name as stable_id
                    FROM mcp_server_prompts p
                    JOIN mcp_servers s ON p.server_id = s.id
                    WHERE p.id = ?
                    "#,
                )
                .bind(uuid)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to convert prompt UUID to stable ID: {}", e)))?;

                match row {
                    Some(row) => Ok(row.get::<String, _>("stable_id")),
                    None => Err(StorageError::NotFound(format!("Prompt with UUID {} not found", uuid))),
                }
            }
            _ => Err(StorageError::InvalidData(format!("Unsupported resource type: {}", resource_type))),
        }
    }

    /// Check if a string is a UUID (basic format check)
    fn is_uuid(&self, s: &str) -> bool {
        s.len() == 36 && s.chars().filter(|&c| c == '-').count() == 4
    }

    /// Normalize resource_id to stable identifier format
    async fn normalize_resource_id(&self, resource_type: &str, resource_id: &str) -> Result<String> {
        // If it's already in stable format (contains "__"), return as is
        if resource_id.contains("__") {
            return Ok(resource_id.to_string());
        }

        // If it's a UUID, convert to stable format
        if self.is_uuid(resource_id) {
            return self.convert_uuid_to_stable_id(resource_type, resource_id).await;
        }

        // Otherwise, assume it's already a stable identifier or legacy format
        Ok(resource_id.to_string())
    }

    /// Batch update permissions for a token
    pub async fn update_permissions(&self, token_id: &str, permissions: Vec<(String, String, bool)>) -> Result<()> {
        let mut tx = self.pool.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        // Clear existing permissions for this token
        query("DELETE FROM permissions WHERE token_id = ?")
            .bind(token_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to clear existing permissions: {}", e)))?;

        // Add new permissions
        for (resource_type, resource_id, allowed) in permissions {
            if allowed {
                let permission_id = Uuid::now_v7().to_string();
                let now = Utc::now();

                query(
                    r#"
                    INSERT INTO permissions (id, token_id, resource_type, resource_id, allowed, created_at, updated_at)
                    VALUES (?, ?, ?, ?, 1, ?, ?)
                    "#,
                )
                .bind(&permission_id)
                .bind(token_id)
                .bind(&resource_type)
                .bind(&resource_id)
                .bind(now)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to add permission: {}", e)))?;
            }
        }

        tx.commit().await
            .map_err(|e| StorageError::Database(format!("Failed to commit permission updates: {}", e)))?;

        Ok(())
    }
}

