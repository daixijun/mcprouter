use crate::error::{McpError, Result};
use crate::token_manager::TokenManager;
use crate::token_manager::{TokenForDashboard, TokenInfo};
use crate::types::{
    BatchUpdateTokenPermissionRequest, CleanupResult, CreateTokenRequest, CreateTokenResponse,
    PermissionAction, SimplePermissionUpdateResponse, TokenStats, UpdateTokenPermissionRequest,
    UpdateTokenRequest, UpdateTokenResponse, ValidationResult,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// State for managing TokenManager across the application
pub type TokenManagerState = Arc<RwLock<Option<Arc<TokenManager>>>>;

/// Create a new token
#[tauri::command]
pub async fn create_token(request: CreateTokenRequest) -> Result<CreateTokenResponse> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    let token_info = token_manager
        .create(
            request.name,
            request.description,
            request.allowed_tools,
            request.allowed_resources,
            request.allowed_prompts,
            request.allowed_prompt_templates,
            request.expires_in,
        )
        .await?;

    // Get the actual token value from storage
    let token_with_value = token_manager
        .orm_storage()
        .get_token_by_id(&token_info.id)
        .await?
        .ok_or_else(|| McpError::ValidationError("Failed to retrieve created token".to_string()))?;

    Ok(CreateTokenResponse {
        token: token_with_value,
    })
}

/// Update an existing token
#[tauri::command]
pub async fn update_token(request: UpdateTokenRequest) -> Result<UpdateTokenResponse> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    token_manager
        .update_token(
            request.name.clone().unwrap_or_default(),
            request.description.clone(),
            true, // enabled - default to true for now
        )
        .await?;

    // Get the updated token
    let token_info = token_manager.get_token_by_id(&request.id).await?;

    // Convert TokenInfo to Token
    let token = token_info
        .ok_or_else(|| crate::error::McpError::NotFound("Token not found".to_string()))?
        .into();

    Ok(UpdateTokenResponse { token })
}

/// List all tokens (without actual values for security)
#[tauri::command]
pub async fn list_tokens() -> Result<Vec<TokenInfo>> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    let tokens = token_manager.list().await?;

    Ok(tokens)
}

/// Delete a token
#[tauri::command(rename_all = "snake_case")]
pub async fn delete_token(token_id: String) -> Result<String> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    token_manager.delete(&token_id).await?;

    Ok(format!("Token '{}' deleted successfully", token_id))
}

/// Toggle token enabled status
#[tauri::command]
pub async fn toggle_token(token_id: String) -> Result<bool> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    let is_enabled = token_manager.toggle_token(&token_id).await?;

    Ok(is_enabled)
}

/// Get token statistics
#[tauri::command]
pub async fn get_token_stats() -> Result<TokenStats> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    let tokens = token_manager.list().await?;

    let total_count = tokens.len() as u64;
    let active_count = tokens.iter().filter(|t| t.enabled && !t.is_expired).count() as u64;
    let expired_count = tokens.iter().filter(|t| t.is_expired).count() as u64;

    Ok(TokenStats {
        total_count,
        active_count,
        expired_count,
        total_usage: tokens.iter().map(|t| t.usage_count).sum(),
        last_used: tokens.iter().filter_map(|t| t.last_used_at).max(),
    })
}

/// Clean up expired tokens
#[tauri::command]
pub async fn cleanup_expired_tokens() -> Result<CleanupResult> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    let removed_count = token_manager.cleanup_expired().await?;

    Ok(CleanupResult {
        removed_count: removed_count as u64,
        message: format!("Cleaned up {} expired tokens", removed_count),
    })
}

/// Validate a token
#[tauri::command]
pub async fn validate_token(token_value: String) -> Result<ValidationResult> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    let token_id = token_manager.validate_token(&token_value).await;

    match token_id {
        Ok(id) => {
            // Record token usage
            let _ = token_manager.record_usage(&id).await;

            // Get token info and convert to Token
            let token_info = token_manager.get_token_by_id(&id).await?;
            let token = token_info
                .ok_or_else(|| crate::error::McpError::NotFound("Token not found".to_string()))?
                .into();

            Ok(ValidationResult {
                valid: true,
                token_info: Some(token),
                message: "Token is valid".to_string(),
            })
        }
        Err(_) => Ok(ValidationResult {
            valid: false,
            token_info: None,
            message: "Token is invalid or expired".to_string(),
        }),
    }
}

/// Get tokens for dashboard (simplified list)
#[tauri::command]
pub async fn get_tokens_for_dashboard() -> Result<Vec<TokenForDashboard>> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    let tokens = token_manager.list_for_dashboard().await?;

    Ok(tokens)
}

/// Update token permissions with new structure (action, resource_type, resource_id, token_id)
/// Returns only success/failure status
#[tauri::command]
pub async fn update_token_permission(
    request: UpdateTokenPermissionRequest,
) -> Result<SimplePermissionUpdateResponse> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    let action_text = match request.action {
        PermissionAction::Add => "add",
        PermissionAction::Remove => "remove",
    };

    // 使用新的结构化权限管理方法（基于具体资源路径）
    match request.action {
        PermissionAction::Add => {
            token_manager
                .add_permission_by_path(
                    &request.token_id,
                    &request.resource_type,
                    &request.resource_path,
                )
                .await?
        }
        PermissionAction::Remove => {
            token_manager
                .remove_permission_by_path(
                    &request.token_id,
                    &request.resource_type,
                    &request.resource_path,
                )
                .await?
        }
    };

    Ok(SimplePermissionUpdateResponse {
        success: true,
        message: format!("Successfully {} permission", action_text),
    })
}

/// Batch update token permissions
/// Returns only success/failure status
#[tauri::command]
pub async fn batch_update_token_permissions(
    request: BatchUpdateTokenPermissionRequest,
) -> Result<SimplePermissionUpdateResponse> {
    // Use global waiting function instead of Tauri state
    let token_manager = crate::wait_for_token_manager().await?;

    // 保存权限数量，避免在循环后访问已移动的值
    let permissions_count = request.permissions.len();

    // 验证请求参数
    if request.token_id.is_empty() {
        return Err(crate::error::McpError::ValidationError(
            "Token ID is required".to_string(),
        ));
    }

    if request.permissions.is_empty() {
        return Err(crate::error::McpError::ValidationError(
            "Permissions list cannot be empty".to_string(),
        ));
    }

    // Process each permission
    for permission_path in request.permissions {
        // 验证权限路径格式
        if !permission_path.contains("__") {
            return Err(crate::error::McpError::ValidationError(format!(
                "Invalid permission path format: {}. Expected format: servername__resourcename",
                permission_path
            )));
        }

        match request.action {
            PermissionAction::Add => {
                token_manager
                    .add_permission_by_path(
                        &request.token_id,
                        &request.resource_type,
                        &permission_path,
                    )
                    .await?
            }
            PermissionAction::Remove => {
                token_manager
                    .remove_permission_by_path(
                        &request.token_id,
                        &request.resource_type,
                        &permission_path,
                    )
                    .await?
            }
        };
    }

    Ok(SimplePermissionUpdateResponse {
        success: true,
        message: format!(
            "Successfully {} {} permissions",
            match request.action {
                PermissionAction::Add => "added",
                PermissionAction::Remove => "removed",
            },
            permissions_count
        ),
    })
}

// get_token_permissions has been removed - permissions are now included in list_tokens response
