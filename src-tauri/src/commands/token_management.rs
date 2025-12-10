use crate::error::{McpError, Result};
use crate::token_manager::{TokenInfo, TokenForDashboard};
use crate::token_manager::TokenManager;
use crate::types::{
    PermissionAction,
    CreateTokenRequest, UpdateTokenRequest, CreateTokenResponse, UpdateTokenResponse,
    TokenStats, CleanupResult, ValidationResult, UpdateTokenPermissionRequest,
    SimplePermissionUpdateResponse
};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// State for managing TokenManager across the application
pub type TokenManagerState = Arc<RwLock<Option<Arc<TokenManager>>>>;

/// Create a new token
#[tauri::command]
pub async fn create_token(
    request: CreateTokenRequest,
    state: State<'_, TokenManagerState>,
) -> Result<CreateTokenResponse> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let token_info = token_manager
        .create_with_permissions(
            request.name,
            request.description,
            request.allowed_tools,
            request.allowed_resources,
            request.allowed_prompts,
            None,
        )
        .await?;

    // Get the full token with value
    let token = token_manager
        .get_token_by_id(&token_info.id)
        .await?;

    Ok(CreateTokenResponse { token })
}

/// Update an existing token
#[tauri::command]
pub async fn update_token(
    request: UpdateTokenRequest,
    state: State<'_, TokenManagerState>,
) -> Result<UpdateTokenResponse> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    token_manager
        .update_token(
            &request.id,
            request.name,
            request.description,
            request.allowed_tools,
            request.allowed_resources,
            request.allowed_prompts,
            request.allowed_prompt_templates,
        )
        .await?;

    // Get the updated token
    let token = token_manager.get_token_by_id(&request.id).await?;

    Ok(UpdateTokenResponse { token })
}

/// List all tokens (without actual values for security)
#[tauri::command]
pub async fn list_tokens(state: State<'_, TokenManagerState>) -> Result<Vec<TokenInfo>> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let tokens = token_manager.list().await?;

    Ok(tokens)
}

/// Delete a token
#[tauri::command]
pub async fn delete_token(
    id: String,
    state: State<'_, TokenManagerState>,
) -> Result<String> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    token_manager.delete(&id).await?;

    Ok(format!("Token '{}' deleted successfully", id))
}

/// Toggle token enabled status
#[tauri::command]
pub async fn toggle_token(
    id: String,
    state: State<'_, TokenManagerState>,
) -> Result<bool> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let enabled = token_manager.toggle_token(&id).await?;

    Ok(enabled)
}

/// Get token statistics
#[tauri::command]
pub async fn get_token_stats(state: State<'_, TokenManagerState>) -> Result<TokenStats> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

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
pub async fn cleanup_expired_tokens(
    state: State<'_, TokenManagerState>,
) -> Result<CleanupResult> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let removed_count = token_manager.cleanup_expired().await?;

    Ok(CleanupResult {
        removed_count: removed_count as u64,
        message: format!("Cleaned up {} expired tokens", removed_count),
    })
}

/// Validate a token
#[tauri::command]
pub async fn validate_token(
    token_value: String,
    state: State<'_, TokenManagerState>,
) -> Result<ValidationResult> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let token_id = token_manager.validate_token(&token_value).await;

    match token_id {
        Some(id) => {
            // Record token usage
            let _ = token_manager.record_usage(&id).await;

            Ok(ValidationResult {
                valid: true,
                token_info: Some(token_manager.get_token_by_id(&id).await?),
                message: "Token is valid".to_string(),
            })
        }
        None => Ok(ValidationResult {
            valid: false,
            token_info: None,
            message: "Token is invalid or expired".to_string(),
        }),
    }
}

/// Get tokens for dashboard (simplified list)
#[tauri::command]
pub async fn get_tokens_for_dashboard(
    state: State<'_, TokenManagerState>,
) -> Result<Vec<TokenForDashboard>> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let tokens = token_manager.list_for_dashboard().await?;

    Ok(tokens)
}

/// Update token permissions with new structure (action, resource_type, resource_id, token_id)
/// Returns only success/failure status
#[tauri::command]
pub async fn update_token_permission(
    request: UpdateTokenPermissionRequest,
    state: State<'_, TokenManagerState>,
) -> Result<SimplePermissionUpdateResponse> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let action_text = match request.action {
        PermissionAction::Add => "add",
        PermissionAction::Remove => "remove",
    };

    // 使用新的结构化权限管理方法
    match request.action {
        PermissionAction::Add => {
            token_manager
                .add_permission(&request.token_id, request.resource_type.clone(), request.resource_id)
                .await?
        }
        PermissionAction::Remove => {
            token_manager
                .remove_permission(&request.token_id, request.resource_type.clone(), request.resource_id)
                .await?
        }
    };

    Ok(SimplePermissionUpdateResponse {
        success: true,
        message: format!("Successfully {} permission", action_text),
    })
}

// get_token_permissions has been removed - permissions are now included in list_tokens response
