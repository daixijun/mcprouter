// API 密钥管理命令

use crate::error::{McpError, Result};
use crate::types::{ApiKey, ApiKeyPermissions};
use sqlx::Row;
use std::collections::HashSet;

/// Helper function: 从工具级别权限推导出授权的服务器列表
async fn get_allowed_servers_from_tools(api_key_id: &str) -> Result<Vec<String>> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // 获取所有授权的工具 ID
    let tool_ids = ApiKeyToolRepository::get_tools_by_api_key(api_key_id).await?;

    // 收集所有不重复的 Server
    let mut server_ids = HashSet::<String>::new();
    for tool_id in tool_ids {
        // 从 mcp_tools 表查询工具信息以获取 server_id
        if let Ok(rows) = sqlx::query("SELECT server_id FROM mcp_tools WHERE id = ?")
            .bind(&tool_id)
            .fetch_all(&crate::db::get_database().await?)
            .await
        {
            for row in rows {
                if let Ok(server_id) = row.try_get::<String, _>("server_id") {
                    server_ids.insert(server_id);
                }
            }
        }
    }

    // 将 Server ID 转换为 Server 名称
    let mut allowed_servers = Vec::new();
    for server_id in server_ids {
        if let Ok(Some(server)) = McpServerRepository::get_by_id(&server_id).await {
            allowed_servers.push(server.name);
        }
    }

    Ok(allowed_servers)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn create_api_key(name: String, permissions: ApiKeyPermissions) -> Result<ApiKey> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // Generate a new API key
    let key = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_chars: String = (0..32)
            .map(|_| {
                const CHARSET: &[u8] =
                    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        format!("sk-{}", random_chars)
    };

    // Create API key in database
    let api_key_row = ApiKeyRepository::create(name.clone(), key.clone()).await?;

    // Add tool-level permissions (批量授权 Server 的所有工具)
    for server_name in &permissions.allowed_servers {
        // Get server ID from name
        if let Some(server_row) = McpServerRepository::get_by_name(server_name).await? {
            if let Some(server_id) = server_row.id {
                ApiKeyToolRepository::grant_server_tools(&api_key_row.id, &server_id).await?;
            }
        }
    }

    tracing::info!("Created new API key: {}", api_key_row.name);

    // Return API key in the expected format
    Ok(ApiKey {
        id: api_key_row.id,
        name: api_key_row.name,
        key, // Return the actual key (only time it's shown)
        enabled: api_key_row.enabled,
        created_at: api_key_row.created_at.to_rfc3339(),
        permissions,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_api_keys() -> Result<Vec<serde_json::Value>> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;

    let api_keys = ApiKeyRepository::get_all().await?;

    let mut masked_keys = Vec::new();
    for api_key in api_keys {
        // 获取授权的工具和服务数量（排除已禁用的服务）
        let db = crate::db::get_database().await?;

        // 查询已授权的工具数量（排除所属服务被禁用的工具）
        let tool_count_row = sqlx::query(
            r#"
            SELECT COUNT(DISTINCT t.id) as tool_count
            FROM api_key_tool_relations aktr
            INNER JOIN mcp_tools t ON aktr.tool_id = t.id
            INNER JOIN mcp_servers s ON t.server_id = s.id
            WHERE aktr.api_key_id = ? AND s.enabled = 1
            "#,
        )
        .bind(&api_key.id)
        .fetch_one(&db)
        .await?;

        let tool_count: i64 = tool_count_row.get("tool_count");

        // 查询已授权的服务器数量（只统计已启用的服务器）
        let server_count_row = sqlx::query(
            r#"
            SELECT COUNT(DISTINCT s.id) as server_count
            FROM api_key_tool_relations aktr
            INNER JOIN mcp_tools t ON aktr.tool_id = t.id
            INNER JOIN mcp_servers s ON t.server_id = s.id
            WHERE aktr.api_key_id = ? AND s.enabled = 1
            "#,
        )
        .bind(&api_key.id)
        .fetch_one(&db)
        .await?;

        let server_count: i64 = server_count_row.get("server_count");

        // Mask the key (show first 6 and last 3 characters)
        let masked_key = if api_key.key_hash.len() > 9 {
            format!(
                "sk-{}...{}",
                &api_key.key_hash[..6],
                &api_key.key_hash[api_key.key_hash.len() - 3..]
            )
        } else {
            "sk-****".to_string()
        };

        masked_keys.push(serde_json::json!({
            "id": api_key.id,
            "name": api_key.name,
            "key": masked_key,
            "enabled": api_key.enabled,
            "created_at": api_key.created_at.to_rfc3339(),
            "updated_at": api_key.updated_at.to_rfc3339(),
            "authorized_server_count": server_count,
            "authorized_tool_count": tool_count,
        }));
    }

    Ok(masked_keys)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_api_key_details(id: String) -> Result<ApiKey> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;

    let api_key_row = ApiKeyRepository::get_by_id(&id)
        .await?
        .ok_or_else(|| McpError::ConfigError(format!("API key not found: {}", id)))?;

    // 获取已授权的工具 ID 列表
    let tool_ids = ApiKeyToolRepository::get_tools_by_api_key(&id).await?;
    // 通过工具权限推导出服务器名称列表
    let allowed_servers = get_allowed_servers_from_tools(&id).await?;

    Ok(ApiKey {
        id: api_key_row.id,
        name: api_key_row.name,
        key: "***".to_string(),
        enabled: api_key_row.enabled,
        created_at: api_key_row.created_at.to_rfc3339(),
        permissions: ApiKeyPermissions {
            allowed_servers,
            allowed_tools: tool_ids,
        },
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_api_key(id: String) -> Result<String> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;

    // Remove all tool permissions first
    ApiKeyToolRepository::remove_all_permissions(&id).await?;

    // Delete the API key
    let deleted = ApiKeyRepository::delete(&id).await?;

    if !deleted {
        return Err(McpError::ConfigError(format!("API key not found: {}", id)));
    }

    tracing::info!("Deleted API key: {}", id);
    Ok(format!("API key '{}' has been deleted", id))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn toggle_api_key(id: String) -> Result<bool> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;

    // Get current state
    let api_key = ApiKeyRepository::get_by_id(&id)
        .await?
        .ok_or_else(|| McpError::ConfigError(format!("API key not found: {}", id)))?;

    let new_state = !api_key.enabled;

    // Toggle the state
    ApiKeyRepository::toggle_enabled(&id, new_state).await?;

    tracing::info!(
        "Toggled API key '{}' to {}",
        id,
        if new_state { "enabled" } else { "disabled" }
    );
    Ok(new_state)
}

// Tool-level Permission Management Commands

#[tauri::command(rename_all = "snake_case")]
pub async fn get_api_key_tools(api_key_id: String) -> Result<Vec<String>> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;

    let tool_ids = ApiKeyToolRepository::get_tools_by_api_key(&api_key_id).await?;
    Ok(tool_ids)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_tool_permission(api_key_id: String, tool_id: String) -> Result<String> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;

    ApiKeyToolRepository::add_tool_permission(&api_key_id, &tool_id).await?;
    tracing::info!("Added tool permission: {} -> {}", api_key_id, tool_id);
    Ok("Tool permission added".to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn remove_tool_permission(api_key_id: String, tool_id: String) -> Result<String> {
    // First check if the tool exists
    let db = crate::db::get_database().await?;
    let tool_row = sqlx::query("SELECT id FROM mcp_tools WHERE id = ?")
        .bind(&tool_id)
        .fetch_optional(&db)
        .await
        .map_err(McpError::from)?;

    if tool_row.is_none() {
        return Err(McpError::ConfigError(format!(
            "Tool not found: {}",
            tool_id
        )));
    }

    // Remove the permission by deleting the relation
    let result =
        sqlx::query("DELETE FROM api_key_tool_relations WHERE api_key_id = ? AND tool_id = ?")
            .bind(&api_key_id)
            .bind(&tool_id)
            .execute(&db)
            .await
            .map_err(McpError::from)?;

    if result.rows_affected() > 0 {
        tracing::info!("Removed tool permission: {} -> {}", api_key_id, tool_id);
        Ok("Tool permission removed".to_string())
    } else {
        Err(McpError::ConfigError(format!(
            "Permission not found for tool: {}",
            tool_id
        )))
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn grant_server_tools_to_api_key(
    api_key_id: String,
    server_name: String,
) -> Result<String> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // Get server ID from name
    let server = McpServerRepository::get_by_name(&server_name)
        .await?
        .ok_or_else(|| McpError::ServiceNotFound(server_name.clone()))?;

    let server_id = server
        .id
        .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?;

    // Grant all tools in this server
    let granted_count = ApiKeyToolRepository::grant_server_tools(&api_key_id, &server_id).await?;

    tracing::info!(
        "Granted {} tools from server {} to API key {}",
        granted_count,
        server_name,
        api_key_id
    );
    Ok(format!(
        "Granted {} tools from server '{}'",
        granted_count, server_name
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn revoke_server_tools_from_api_key(
    api_key_id: String,
    server_name: String,
) -> Result<String> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // Get server ID from name
    let server = McpServerRepository::get_by_name(&server_name)
        .await?
        .ok_or_else(|| McpError::ServiceNotFound(server_name.clone()))?;

    let server_id = server
        .id
        .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?;

    // Revoke all tools in this server
    let revoked_count = ApiKeyToolRepository::revoke_server_tools(&api_key_id, &server_id).await?;

    tracing::info!(
        "Revoked {} tools from server {} for API key {}",
        revoked_count,
        server_name,
        api_key_id
    );
    Ok(format!(
        "Revoked {} tools from server '{}'",
        revoked_count, server_name
    ))
}
