use crate::error::{McpError, Result};
use crate::token_manager::{Token, TokenInfo, TokenManager};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// Request for creating a new token
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub description: Option<String>,
    pub expires_in: Option<u64>, // Duration in seconds from now
    // Permission fields
    pub allowed_tools: Option<Vec<String>>, // e.g., ["filesystem/*", "database/query"]
    pub allowed_resources: Option<Vec<String>>, // e.g., ["filesystem/logs/*"]
    pub allowed_prompts: Option<Vec<String>>, // e.g., ["codegen/*"]
    pub allowed_prompt_templates: Option<Vec<String>>, // e.g., ["prompt-gallery__template_name"]
}

/// Request for updating an existing token
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTokenRequest {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    // Permission fields - use Option<Option<T>> to distinguish between "don't update" and "set to None"
    pub allowed_tools: Option<Option<Vec<String>>>,
    pub allowed_resources: Option<Option<Vec<String>>>,
    pub allowed_prompts: Option<Option<Vec<String>>>,
    pub allowed_prompt_templates: Option<Option<Vec<String>>>,
}

/// Response containing the created token
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenResponse {
    pub token: Token,
}

/// Response containing the updated token
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTokenResponse {
    pub token: Token,
}

/// State for managing TokenManager across the application
pub type TokenManagerState = Arc<RwLock<Option<Arc<TokenManager>>>>;

/// Initialize TokenManager with the given config directory
pub async fn init_token_manager(config_dir: &PathBuf) -> Result<Arc<TokenManager>> {
    let tokens_path = config_dir.join("tokens.json");
    let manager = TokenManager::new(tokens_path).await?;

    tracing::info!("TokenManager initialized successfully");
    Ok(Arc::new(manager))
}

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

    let token = token_manager
        .create_with_permissions(
            request.name,
            request.description,
            request.expires_in,
            request.allowed_tools,
            request.allowed_resources,
            request.allowed_prompts,
            request.allowed_prompt_templates,
        )
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

    let updated_token = token_manager
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
    Ok(UpdateTokenResponse {
        token: updated_token,
    })
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

/// Delete a token by ID
#[tauri::command]
pub async fn delete_token(token_id: String, state: State<'_, TokenManagerState>) -> Result<()> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    token_manager.delete(&token_id).await?;
    Ok(())
}

/// Toggle a token's enabled status
#[tauri::command]
pub async fn toggle_token(token_id: String, state: State<'_, TokenManagerState>) -> Result<bool> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let result = token_manager.toggle_token(&token_id).await?;
    Ok(result)
}

/// Get token statistics
#[tauri::command]
pub async fn get_token_stats(state: State<'_, TokenManagerState>) -> Result<TokenStats> {
    let token_manager_guard = state.read().await;

    // Check if TokenManager is initialized
    let token_manager = match token_manager_guard.as_ref() {
        Some(manager) => manager.clone(),
        None => {
            tracing::error!("TokenManager not initialized in state");
            return Err(McpError::InternalError(
                "TokenManager not initialized".to_string(),
            ));
        }
    };

    let tokens = token_manager.list().await?;

    let total_count = tokens.len();
    let active_count = tokens.iter().filter(|t| !t.is_expired).count();
    let expired_count = total_count - active_count;

    let total_usage: u64 = tokens.iter().map(|t| t.usage_count).sum();

    // Find most recently used token
    let last_used = tokens.iter().filter_map(|t| t.last_used_at).max();

    let stats = TokenStats {
        total_count,
        active_count,
        expired_count,
        total_usage,
        last_used,
    };

    // Debug serialization
    match serde_json::to_string(&stats) {
        Ok(json) => {
            tracing::debug!("TokenStats serialized to JSON: {}", json);
        }
        Err(e) => {
            tracing::error!("Failed to serialize TokenStats: {}", e);
        }
    }

    Ok(stats)
}

/// Clean up expired tokens
#[tauri::command]
pub async fn cleanup_expired_tokens(state: State<'_, TokenManagerState>) -> Result<CleanupResult> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let removed_count = token_manager.cleanup_expired().await?;
    Ok(CleanupResult {
        removed_count,
        message: if removed_count > 0 {
            format!("Cleaned up {} expired tokens", removed_count)
        } else {
            "No expired tokens found".to_string()
        },
    })
}

/// Token statistics information
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStats {
    pub total_count: usize,
    pub active_count: usize,
    pub expired_count: usize,
    pub total_usage: u64,
    pub last_used: Option<u64>,
}

/// Result of cleanup operation
#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupResult {
    pub removed_count: usize,
    pub message: String,
}

/// Validate a token (for testing purposes - normally this is done by auth middleware)
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

    if let Some(token_id) = token_manager.validate_token(&token_value).await {
        // Get token info without value
        let tokens = token_manager.list().await?;
        if let Some(token_info) = tokens.iter().find(|t| t.id == token_id) {
            Ok(ValidationResult {
                valid: true,
                token_info: Some(token_info.clone()),
                message: "Token is valid".to_string(),
            })
        } else {
            Ok(ValidationResult {
                valid: false,
                token_info: None,
                message: "Token validation failed - token not found".to_string(),
            })
        }
    } else {
        Ok(ValidationResult {
            valid: false,
            token_info: None,
            message: "Invalid or expired token".to_string(),
        })
    }
}

/// Get tokens for Dashboard (including actual token values for configuration generation)
#[tauri::command]
pub async fn get_tokens_for_dashboard(state: State<'_, TokenManagerState>) -> Result<Vec<Token>> {
    let token_manager_guard = state.read().await;

    let token_manager = token_manager_guard
        .as_ref()
        .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
        .clone();

    let tokens = token_manager.get_all_tokens().await?;
    Ok(tokens)
}

/// Token validation result
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub token_info: Option<TokenInfo>,
    pub message: String,
}
