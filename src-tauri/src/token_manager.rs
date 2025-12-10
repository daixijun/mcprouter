// SQLite-based Token Manager implementation
#![allow(dead_code)]

use crate::error::{McpError, Result};
use crate::storage::token_storage::TokenStorage;
use crate::types::{PermissionType, PermissionValidationResult, Token};
use serde::{Deserialize, Serialize};

// Token Manager with SQLite backend

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
use sqlx::SqlitePool;
use std::sync::Arc;

/// Token Manager with SQLite backend
pub struct TokenManager {
    storage: Arc<TokenStorage>,
}

impl TokenManager {
    /// Create a new TokenManager with SQLite backend
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let storage = Arc::new(TokenStorage::new(pool));

        Ok(Self { storage })
    }

    /// Create a new token with generated value
    pub async fn create(&self, name: String, description: Option<String>) -> Result<TokenInfo> {
        let token_value = self.generate_token();
        self.create_with_permissions(
            name,
            description,
            None,
            None,
            None,
            Some(token_value.clone()),
        )
        .await
    }

    /// Create a new token with specified permissions
    pub async fn create_with_permissions(
        &self,
        name: String,
        description: Option<String>,
        allowed_tools: Option<Vec<String>>,
        allowed_resources: Option<Vec<String>>,
        allowed_prompts: Option<Vec<String>>,
        token_value: Option<String>,
    ) -> Result<TokenInfo> {
        let token_value = token_value.unwrap_or_else(|| self.generate_token());
        let token_id = format!("tok_{}", &token_value[4..12]); // Generate a token ID

        let token = Token {
            id: token_id.clone(),
            name: name.clone(),
            value: token_value.clone(),
            description,
            created_at: Utc::now().timestamp() as u64,
            expires_at: None,
            last_used_at: None,
            usage_count: 0,
            enabled: true,
            allowed_tools: allowed_tools.clone(),
            allowed_resources: allowed_resources.clone(),
            allowed_prompts: allowed_prompts.clone(),
            allowed_prompt_templates: None,
        };

        self.storage
            .create_token(&token)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to create token: {}", e)))?;

        // Add permissions if provided
        if let Some(tools) = allowed_tools {
            for tool in tools {
                self.storage
                    .add_permission(&token_id, "tool", &tool)
                    .await
                    .map_err(|e| {
                        McpError::InternalError(format!("Failed to add tool permission: {}", e))
                    })?;
            }
        }

        if let Some(resources) = allowed_resources {
            for resource in resources {
                self.storage
                    .add_permission(&token_id, "resource", &resource)
                    .await
                    .map_err(|e| {
                        McpError::InternalError(format!("Failed to add resource permission: {}", e))
                    })?;
            }
        }

        if let Some(prompts) = allowed_prompts {
            for prompt in prompts {
                self.storage
                    .add_permission(&token_id, "prompt", &prompt)
                    .await
                    .map_err(|e| {
                        McpError::InternalError(format!("Failed to add prompt permission: {}", e))
                    })?;
            }
        }

        let description = token.description.clone();
        let token_info = TokenInfo {
            id: token_id,
            name,
            description,
            created_at: Utc::now().timestamp() as u64,
            expires_at: token.expires_at,
            last_used_at: token.last_used_at,
            usage_count: token.usage_count,
            is_expired: self.is_token_expired(&token),
            enabled: token.enabled,
            allowed_tools: token.allowed_tools.clone().unwrap_or_default(),
            allowed_resources: token.allowed_resources.clone().unwrap_or_default(),
            allowed_prompts: token.allowed_prompts.clone().unwrap_or_default(),
            allowed_prompt_templates: token.allowed_prompt_templates.clone().unwrap_or_default(),
        };

        Ok(token_info)
    }

    /// List all tokens
    pub async fn list(&self) -> Result<Vec<TokenInfo>> {
        self.storage
            .list_tokens()
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to list tokens: {}", e)))
    }

    /// List all tokens for dashboard (minimal fields)
    pub async fn list_for_dashboard(&self) -> Result<Vec<TokenForDashboard>> {
        let token_infos = self.storage
            .list_tokens()
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to list tokens: {}", e)))?;

        let mut dashboard_tokens = Vec::new();

        for token_info in token_infos {
            // Get the full token to access the value field
            let full_token = self.storage.get_token_by_id(&token_info.id).await
                .map_err(|e| McpError::InternalError(format!("Failed to get token {}: {}", token_info.id, e)))?;

            dashboard_tokens.push(TokenForDashboard {
                id: token_info.id,
                name: token_info.name,
                token: full_token.value,
                expires_at: token_info.expires_at,
                is_expired: token_info.is_expired,
            });
        }

        Ok(dashboard_tokens)
    }

    /// Delete a token
    pub async fn delete(&self, token_id: &str) -> Result<()> {
        self.storage
            .delete_token(token_id)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to delete token: {}", e)))
    }

    /// Validate token and return token_id if valid
    pub async fn validate_token(&self, token_value: &str) -> Option<String> {
        match self.storage.get_token_by_value(token_value).await {
            Ok(Some(token)) => {
                // Check if token is expired
                if self.is_token_expired(&token) {
                    return None;
                }
                Some(token.id)
            }
            _ => None,
        }
    }

    /// Get token by ID
    pub async fn get_token_by_id(&self, token_id: &str) -> Result<Token> {
        self.storage
            .get_token_by_id(token_id)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to get token: {}", e)))
    }

    // get_token_permissions has been removed - permissions are now included in list_tokens response

    /// Get all tokens (for internal use)
    pub async fn get_all_tokens(&self) -> Result<Vec<Token>> {
        let token_infos = self
            .storage
            .list_tokens()
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to list tokens: {}", e)))?;

        let mut tokens = Vec::new();
        for token_info in token_infos {
            let token = self
                .storage
                .get_token_by_id(&token_info.id)
                .await
                .map_err(|e| {
                    McpError::InternalError(format!("Failed to get token details: {}", e))
                })?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    /// Record token usage
    pub async fn record_usage(&self, token_id: &str) -> Result<()> {
        self.storage
            .update_token_usage(token_id)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to record token usage: {}", e)))?;
        Ok(())
    }

    /// Toggle token active status
    pub async fn toggle_token(&self, token_id: &str) -> Result<bool> {
        // Get current token to determine its current state
        let token = self.get_token_by_id(token_id).await?;
        let new_status = !token.enabled;

        self.storage
            .toggle_token_enabled(token_id, new_status)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to toggle token: {}", e)))?;

        Ok(new_status)
    }

    /// Add permission to a token
    pub async fn add_permission(
        &self,
        token_id: &str,
        permission_type: PermissionType,
        pattern: String,
    ) -> Result<()> {
        let resource_type = match permission_type {
            PermissionType::Tools => "tool",
            PermissionType::Resources => "resource",
            PermissionType::Prompts => "prompt",
            PermissionType::PromptTemplates => "prompt_template",
        };

        self.storage
            .add_permission(token_id, resource_type, &pattern)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to add permission: {}", e)))
    }

    /// Remove permission from a token
    pub async fn remove_permission(
        &self,
        token_id: &str,
        permission_type: PermissionType,
        pattern: String,
    ) -> Result<()> {
        let resource_type = match permission_type {
            PermissionType::Tools => "tool",
            PermissionType::Resources => "resource",
            PermissionType::Prompts => "prompt",
            PermissionType::PromptTemplates => "prompt_template",
        };

        self.storage
            .remove_permission(token_id, resource_type, &pattern)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to remove permission: {}", e)))
    }

    /// Update token field
    pub async fn update_field(&self, token_id: &str, field: &str, value: String) -> Result<()> {
        match field {
            "name" => {
                self.storage
                    .update_token(token_id, Some(value), None)
                    .await
                    .map_err(|e| {
                        McpError::InternalError(format!("Failed to update token name: {}", e))
                    })?;
            }
            "description" => {
                self.storage
                    .update_token(token_id, None, Some(value))
                    .await
                    .map_err(|e| {
                        McpError::InternalError(format!(
                            "Failed to update token description: {}",
                            e
                        ))
                    })?;
            }
            _ => {
                return Err(McpError::InternalError(format!(
                    "Unsupported field: {}",
                    field
                )));
            }
        }
        Ok(())
    }

    /// Update permissions for a token (batch operation)
    pub async fn update_permission_typed(
        &self,
        token_id: String,
        permission_type: PermissionType,
        permissions: Vec<PermissionItem>,
    ) -> Result<()> {
        let resource_type = match permission_type {
            PermissionType::Tools => "tool",
            PermissionType::Resources => "resource",
            PermissionType::Prompts => "prompt",
            PermissionType::PromptTemplates => "prompt_template",
        };

        let permission_tuples: Vec<(String, String, bool)> = permissions
            .into_iter()
            .map(|p| (resource_type.to_string(), p.pattern, p.allowed))
            .collect();

        self.storage
            .update_permissions(&token_id, permission_tuples)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to update permissions: {}", e)))
    }

    /// Update token with new permissions
    pub async fn update_token(
        &self,
        token_id: &str,
        name: Option<String>,
        description: Option<String>,
        allowed_tools: Option<Vec<String>>,
        allowed_resources: Option<Vec<String>>,
        allowed_prompts: Option<Vec<String>>,
        allowed_prompt_templates: Option<Vec<String>>,
    ) -> Result<()> {
        // Update basic token info
        self.storage
            .update_token(token_id, name, description)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to update token: {}", e)))?;

        // Build permissions list for batch update
        let mut permissions = Vec::new();

        if let Some(tools) = allowed_tools {
            for tool in tools {
                permissions.push(("tool".to_string(), tool, true));
            }
        }

        if let Some(resources) = allowed_resources {
            for resource in resources {
                permissions.push(("resource".to_string(), resource, true));
            }
        }

        if let Some(prompts) = allowed_prompts {
            for prompt in prompts {
                permissions.push(("prompt".to_string(), prompt, true));
            }
        }

        if let Some(prompt_templates) = allowed_prompt_templates {
            for template in prompt_templates {
                permissions.push(("prompt_template".to_string(), template, true));
            }
        }

        // Batch update permissions
        if !permissions.is_empty() {
            self.storage
                .update_permissions(token_id, permissions)
                .await
                .map_err(|e| {
                    McpError::InternalError(format!("Failed to update permissions: {}", e))
                })?;
        }

        Ok(())
    }

    /// Clean up expired tokens
    pub async fn cleanup_expired(&self) -> Result<usize> {
        self.storage
            .cleanup_expired_tokens()
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to cleanup expired tokens: {}", e)))
    }

    /// Validate permission for a specific token and resource
    pub async fn validate_permission(
        &self,
        token_id: &str,
        permission_type: PermissionType,
        resource_pattern: &str,
    ) -> Result<PermissionValidationResult> {
        let resource_type = match permission_type {
            PermissionType::Tools => "tool",
            PermissionType::Resources => "resource",
            PermissionType::Prompts => "prompt",
            PermissionType::PromptTemplates => "prompt_template",
        };

        let has_permission = self
            .storage
            .has_permission(token_id, resource_type, resource_pattern)
            .await
            .map_err(|e| {
                McpError::InternalError(format!("Failed to validate permission: {}", e))
            })?;

        Ok(PermissionValidationResult {
            is_valid: has_permission,
            error: if has_permission {
                None
            } else {
                Some("Permission denied".to_string())
            },
            normalized_value: None,
        })
    }

    /// Generate a secure random token
    fn generate_token(&self) -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(&bytes)
    }

    /// Check if a token is expired
    fn is_token_expired(&self, token: &Token) -> bool {
        if let Some(expires_at) = token.expires_at {
            let now = Utc::now().timestamp() as u64;
            now > expires_at
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionItem {
    pub pattern: String,
    pub allowed: bool,
}

