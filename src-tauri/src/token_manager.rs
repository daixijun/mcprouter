// Token Manager implementation

use crate::error::{McpError, Result};
use crate::storage::orm_storage::Storage;
use crate::types::{PermissionType, PermissionValidationResult, Token};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Token information for listing (without actual token value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub last_used_at: Option<u64>,
    pub usage_count: u64,
    pub is_expired: bool,
    pub enabled: bool,
    pub allowed_tools: Vec<String>,
    pub allowed_resources: Vec<String>,
    pub allowed_prompts: Vec<String>,
    pub allowed_prompt_templates: Vec<String>,
}

/// Token information for dashboard (minimal fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenForDashboard {
    pub id: String,
    pub name: String,
    pub token: String,
    pub expires_at: Option<u64>,
    pub is_expired: bool,
}

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use uuid::Uuid;

/// Token Manager
#[derive(Debug)]
pub struct TokenManager {
    orm_storage: Arc<Storage>,
}

impl TokenManager {
    /// Create a new TokenManager with SeaORM backend
    pub async fn new(orm_storage: Arc<Storage>) -> Result<Self> {
        Ok(Self { orm_storage })
    }

    /// Get access to the underlying ORM storage (for internal use)
    pub fn orm_storage(&self) -> Arc<Storage> {
        self.orm_storage.clone()
    }

    /// Generate a secure random token
    pub fn generate_token(&self) -> String {
        // Use UUID v7 for better randomness and sortable timestamps
        let uuid = Uuid::now_v7();
        let token_bytes = uuid.as_bytes();
        URL_SAFE_NO_PAD.encode(token_bytes)
    }

    /// Create a new token with generated value
    pub async fn create(&self, name: String, description: Option<String>) -> Result<TokenInfo> {
        let token_value = self.generate_token();
        let now = Utc::now();
        let id = Uuid::now_v7().to_string();

        let token = Token {
            id: id.clone(),
            name,
            value: token_value,
            description,
            created_at: now.timestamp() as u64,
            enabled: true,
            last_used_at: None,
            usage_count: 0,
            expires_at: None, // TODO: Make configurable
            allowed_tools: Some(vec![]),
            allowed_resources: Some(vec![]),
            allowed_prompts: Some(vec![]),
            allowed_prompt_templates: Some(vec![]),
        };

        self.orm_storage
            .create_token(&token)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to create token: {}", e)))?;

        self.convert_to_token_info(&token).await
    }

    /// Validate a token and return token info if valid
    pub async fn validate(
        &self,
        token_value: &str,
    ) -> Result<(PermissionValidationResult, Option<String>)> {
        let token = self
            .orm_storage
            .get_token_by_value(token_value)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to validate token: {}", e)))?;

        if let Some(token) = token {
            // Check if token is enabled
            if !token.enabled {
                return Ok((
                    PermissionValidationResult {
                        is_valid: false,
                        error: Some("Token is disabled".to_string()),
                        normalized_value: None,
                    },
                    Some(token.id),
                ));
            }

            // Check if token is expired
            if let Some(expires_at) = token.expires_at {
                let now = Utc::now().timestamp() as u64;
                if expires_at < now {
                    return Ok((
                        PermissionValidationResult {
                            is_valid: false,
                            error: Some("Token has expired".to_string()),
                            normalized_value: None,
                        },
                        Some(token.id),
                    ));
                }
            }

            // Update usage statistics
            if let Err(e) = self.orm_storage.update_token_usage(&token.id).await {
                tracing::warn!("Failed to update token usage: {}", e);
            }

            // Check permissions from database
            // For now, we'll validate token existence but detailed permission checks will be implemented later
            Ok((
                PermissionValidationResult {
                    is_valid: true,
                    error: None,
                    normalized_value: Some(token.value),
                },
                Some(token.id),
            ))
        } else {
            Ok((
                PermissionValidationResult {
                    is_valid: false,
                    error: Some("Token not found".to_string()),
                    normalized_value: None,
                },
                None,
            ))
        }
    }

    /// Get all tokens (for listing)
    pub async fn list(&self) -> Result<Vec<TokenInfo>> {
        let tokens = self
            .orm_storage
            .get_all_tokens()
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to list tokens: {}", e)))?;

        let mut token_infos = Vec::new();
        for token in tokens {
            let info = self.convert_to_token_info(&token).await?;
            token_infos.push(info);
        }

        Ok(token_infos)
    }

    /// Get token by ID
    pub async fn get_by_id(&self, token_id: &str) -> Result<Option<TokenInfo>> {
        if let Some(token) = self
            .orm_storage
            .get_token_by_id(token_id)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to get token: {}", e)))?
        {
            let info = self.convert_to_token_info(&token).await?;
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }

    /// Get token by ID (alias for get_by_id)
    pub async fn get_token_by_id(&self, token_id: &str) -> Result<Option<TokenInfo>> {
        self.get_by_id(token_id).await
    }

    /// Delete a token
    pub async fn delete(&self, token_id: &str) -> Result<()> {
        self.orm_storage
            .delete_token(token_id)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to delete token: {}", e)))
    }

    /// Enable or disable a token
    pub async fn set_enabled(&self, token_id: &str, enabled: bool) -> Result<()> {
        self.orm_storage
            .set_token_enabled(token_id, enabled)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to update token status: {}", e)))
    }

    /// Get tokens for dashboard
    pub async fn get_for_dashboard(&self) -> Result<Vec<TokenForDashboard>> {
        let tokens = self.orm_storage.get_all_tokens().await.map_err(|e| {
            McpError::ValidationError(format!("Failed to get tokens for dashboard: {}", e))
        })?;

        let now = Utc::now().timestamp() as u64;
        let token_infos: Vec<TokenForDashboard> = tokens
            .into_iter()
            .map(|token| TokenForDashboard {
                id: token.id,
                name: token.name,
                token: token.value,
                expires_at: token.expires_at,
                is_expired: token
                    .expires_at
                    .is_some_and(|expires_at| expires_at < now),
            })
            .collect();

        Ok(token_infos)
    }

    /// Check if token has permission for a specific resource
    pub async fn check_permission(
        &self,
        token_id: &str,
        resource_type: &str,
        resource_path: &str,
    ) -> Result<bool> {
        self.orm_storage
            .check_permission(token_id, resource_type, resource_path)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to check permission: {}", e)))
    }

    /// Validate token (alias for compatibility with aggregator)
    pub async fn validate_token(&self, token_value: &str) -> Result<String> {
        let (validation_result, token_id) = self.validate(token_value).await?;
        if validation_result.is_valid {
            // Return the token_id
            if let Some(id) = token_id {
                Ok(id)
            } else {
                Err(McpError::ValidationError(
                    "Token validation failed: no token_id returned".to_string(),
                ))
            }
        } else {
            Err(McpError::ValidationError(format!(
                "Token validation failed: {}",
                validation_result
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string())
            )))
        }
    }

    /// Update token permissions
    pub async fn update_permissions(
        &self,
        token_id: &str,
        permissions: Vec<PermissionType>,
    ) -> Result<()> {
        // Clear existing permissions
        let existing_permissions = self
            .orm_storage
            .get_token_permissions(token_id)
            .await
            .map_err(|e| {
                McpError::ValidationError(format!("Failed to get existing permissions: {}", e))
            })?;

        for perm in existing_permissions {
            if let Err(e) = self
                .orm_storage
                .remove_permission(token_id, &perm.resource_type, &perm.resource_path)
                .await
            {
                tracing::warn!("Failed to remove existing permission: {}", e);
            }
        }

        // Add new permissions
        // 由于 PermissionType 现在是单元变体，我们需要为每个类型创建默认模式
        for permission in permissions {
            let pattern = "*".to_string(); // 默认模式
            match permission {
                PermissionType::Tools => {
                    self.orm_storage
                        .add_permission(token_id, "tool", &pattern)
                        .await
                        .map_err(|e| {
                            McpError::ValidationError(format!(
                                "Failed to add tool permission: {}",
                                e
                            ))
                        })?;
                }
                PermissionType::Resources => {
                    self.orm_storage
                        .add_permission(token_id, "resource", &pattern)
                        .await
                        .map_err(|e| {
                            McpError::ValidationError(format!(
                                "Failed to add resource permission: {}",
                                e
                            ))
                        })?;
                }
                PermissionType::Prompts => {
                    self.orm_storage
                        .add_permission(token_id, "prompt", &pattern)
                        .await
                        .map_err(|e| {
                            McpError::ValidationError(format!(
                                "Failed to add prompt permission: {}",
                                e
                            ))
                        })?;
                }
                PermissionType::PromptTemplates => {
                    self.orm_storage
                        .add_permission(token_id, "prompt_template", &pattern)
                        .await
                        .map_err(|e| {
                            McpError::ValidationError(format!(
                                "Failed to add prompt template permission: {}",
                                e
                            ))
                        })?;
                }
            }
        }

        Ok(())
    }

    /// Get token statistics
    pub async fn get_stats(&self) -> Result<std::collections::HashMap<String, u64>> {
        let tokens =
            self.orm_storage.get_all_tokens().await.map_err(|e| {
                McpError::ValidationError(format!("Failed to get token stats: {}", e))
            })?;

        let now = Utc::now().timestamp() as u64;
        let mut stats = std::collections::HashMap::new();

        let total = tokens.len() as u64;
        let enabled = tokens.iter().filter(|t| t.enabled).count() as u64;
        let expired = tokens
            .iter()
            .filter(|t| t.expires_at.is_some_and(|expires_at| expires_at < now))
            .count() as u64;

        stats.insert("total".to_string(), total);
        stats.insert("enabled".to_string(), enabled);
        stats.insert("disabled".to_string(), total - enabled);
        stats.insert("expired".to_string(), expired);
        stats.insert("active".to_string(), enabled - expired);

        Ok(stats)
    }

    /// Update token (alias for create/update functionality)
    pub async fn update_token(
        &self,
        name: String,
        _description: Option<String>,
        enabled: bool,
    ) -> Result<TokenInfo> {
        // Find existing token by name - we need to search through all tokens
        let tokens = self.list().await?;

        if let Some(token_info) = tokens.iter().find(|t| t.name == name) {
            // Update the token using its ID
            self.set_enabled(&token_info.id, enabled).await?;

            // Get the updated token info
            if let Some(updated_token) = self.get_by_id(&token_info.id).await? {
                Ok(updated_token)
            } else {
                Err(McpError::NotFound(
                    "Token not found after update".to_string(),
                ))
            }
        } else {
            Err(McpError::NotFound(format!(
                "Token with name '{}' not found",
                name
            )))
        }
    }

    /// Toggle token enabled status
    pub async fn toggle_token(&self, token_id: &str) -> Result<bool> {
        // Get current token info
        if let Some(token_info) = self.get_by_id(token_id).await? {
            let new_enabled = !token_info.enabled;
            self.set_enabled(token_id, new_enabled).await?;
            Ok(new_enabled)
        } else {
            Err(McpError::NotFound("Token not found".to_string()))
        }
    }

    /// Record token usage (alias for update_token_usage)
    pub async fn record_usage(&self, token_id: &str) -> Result<()> {
        self.orm_storage
            .update_token_usage(token_id)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to record token usage: {}", e)))
    }

    /// List tokens for dashboard
    pub async fn list_for_dashboard(&self) -> Result<Vec<TokenForDashboard>> {
        self.get_for_dashboard().await
    }

    /// Add permission to token (by type, supports wildcards)
    pub async fn add_permission(
        &self,
        token_id: &str,
        permission_type: crate::types::PermissionType,
    ) -> Result<()> {
        let pattern = "*".to_string(); // Default pattern
        let resource_type = match permission_type {
            crate::types::PermissionType::Tools => "tool",
            crate::types::PermissionType::Resources => "resource",
            crate::types::PermissionType::Prompts => "prompt",
            crate::types::PermissionType::PromptTemplates => "prompt_template",
        };

        self.orm_storage
            .add_permission(token_id, resource_type, &pattern)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to add permission: {}", e)))
    }

    /// Add specific permission to token (by resource path)
    pub async fn add_permission_by_path(
        &self,
        token_id: &str,
        resource_type: &crate::types::PermissionType,
        resource_path: &str,
    ) -> Result<()> {
        let resource_type_str = match resource_type {
            crate::types::PermissionType::Tools => "tool",
            crate::types::PermissionType::Resources => "resource",
            crate::types::PermissionType::Prompts => "prompt",
            crate::types::PermissionType::PromptTemplates => "prompt_template",
        };

        self.orm_storage
            .add_permission(token_id, resource_type_str, resource_path)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to add permission: {}", e)))
    }

    /// Remove permission from token (by type, supports wildcards)
    pub async fn remove_permission(
        &self,
        token_id: &str,
        permission_type: crate::types::PermissionType,
    ) -> Result<()> {
        let pattern = "*".to_string(); // Default pattern
        let resource_type = match permission_type {
            crate::types::PermissionType::Tools => "tool",
            crate::types::PermissionType::Resources => "resource",
            crate::types::PermissionType::Prompts => "prompt",
            crate::types::PermissionType::PromptTemplates => "prompt_template",
        };

        self.orm_storage
            .remove_permission(token_id, resource_type, &pattern)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to remove permission: {}", e)))
    }

    /// Remove specific permission from token (by resource path)
    pub async fn remove_permission_by_path(
        &self,
        token_id: &str,
        resource_type: &crate::types::PermissionType,
        resource_path: &str,
    ) -> Result<()> {
        let resource_type_str = match resource_type {
            crate::types::PermissionType::Tools => "tool",
            crate::types::PermissionType::Resources => "resource",
            crate::types::PermissionType::Prompts => "prompt",
            crate::types::PermissionType::PromptTemplates => "prompt_template",
        };

        self.orm_storage
            .remove_permission(token_id, resource_type_str, resource_path)
            .await
            .map_err(|e| McpError::ValidationError(format!("Failed to remove permission: {}", e)))
    }

    /// Clean up expired tokens
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let tokens = self.orm_storage.get_all_tokens().await.map_err(|e| {
            McpError::ValidationError(format!("Failed to cleanup expired tokens: {}", e))
        })?;

        let now = Utc::now().timestamp() as u64;
        let expired_tokens: Vec<String> = tokens
            .into_iter()
            .filter_map(|t| {
                if t.expires_at.is_some_and(|expires_at| expires_at < now) {
                    Some(t.id)
                } else {
                    None
                }
            })
            .collect();

        let mut cleaned_count = 0;
        for token_id in expired_tokens {
            if let Err(e) = self.orm_storage.delete_token(&token_id).await {
                tracing::warn!("Failed to delete expired token {}: {}", token_id, e);
            } else {
                cleaned_count += 1;
            }
        }

        tracing::info!("Cleaned up {} expired tokens", cleaned_count);
        Ok(cleaned_count)
    }

    // ============================================================================
    // Public utility methods
    // ============================================================================

    /// Convert TokenInfo to Token for older API compatibility
    pub fn token_info_to_token(token_info: &TokenInfo) -> Token {
        // Create a Token value based on token name
        let token_value = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", token_info.id, token_info.name));

        Token {
            id: token_info.id.clone(),
            name: token_info.name.clone(),
            value: token_value,
            description: token_info.description.clone(),
            created_at: token_info.created_at,
            enabled: token_info.enabled,
            last_used_at: token_info.last_used_at,
            usage_count: token_info.usage_count,
            expires_at: token_info.expires_at,
            allowed_tools: Some(token_info.allowed_tools.clone()),
            allowed_resources: Some(token_info.allowed_resources.clone()),
            allowed_prompts: Some(token_info.allowed_prompts.clone()),
            allowed_prompt_templates: Some(token_info.allowed_prompt_templates.clone()),
        }
    }

    // ============================================================================
    // Private helper methods
    // ============================================================================

    async fn convert_to_token_info(&self, token: &Token) -> Result<TokenInfo> {
        let now = Utc::now().timestamp() as u64;

        // Get token permissions from database
        let permissions = self
            .orm_storage
            .get_token_permissions(&token.id)
            .await
            .map_err(|e| {
                McpError::ValidationError(format!("Failed to get token permissions: {}", e))
            })?;

        // Group permissions by type
        let mut allowed_tools = Vec::new();
        let mut allowed_resources = Vec::new();
        let mut allowed_prompts = Vec::new();
        let mut allowed_prompt_templates = Vec::new();

        for permission in permissions {
            if permission.allowed {
                match permission.resource_type.as_str() {
                    "tool" => allowed_tools.push(permission.resource_path),
                    "resource" => allowed_resources.push(permission.resource_path),
                    "prompt" => allowed_prompts.push(permission.resource_path),
                    "prompt_template" => allowed_prompt_templates.push(permission.resource_path),
                    _ => {
                        tracing::warn!(
                            "Unknown resource type in permission: {}",
                            permission.resource_type
                        );
                    }
                }
            }
        }

        Ok(TokenInfo {
            id: token.id.clone(),
            name: token.name.clone(),
            description: token.description.clone(),
            created_at: now, // TODO: Store created_at in database
            expires_at: token.expires_at,
            last_used_at: token.last_used_at,
            usage_count: token.usage_count,
            is_expired: token
                .expires_at
                .is_some_and(|expires_at| expires_at < now),
            enabled: token.enabled,
            allowed_tools,
            allowed_resources,
            allowed_prompts,
            allowed_prompt_templates,
        })
    }
}


impl From<TokenInfo> for Token {
    fn from(token_info: TokenInfo) -> Self {
        TokenManager::token_info_to_token(&token_info)
    }
}
