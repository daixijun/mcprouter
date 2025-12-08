use crate::error::{McpError, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Default value for token enabled field
fn default_enabled() -> bool {
    true
}

/// Token data structure with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: String,                  // Unique identifier: "tok_" + 32 random chars
    pub value: String,               // Token value: "mcp_" + 64 base64 encoded random chars
    pub name: String,                // User-friendly name
    pub description: Option<String>, // Optional description
    pub created_at: u64,             // Creation timestamp (Unix timestamp)
    pub expires_at: Option<u64>,     // Expiration timestamp (None = never expires)
    pub last_used_at: Option<u64>,   // Last usage timestamp
    pub usage_count: u64,            // Usage count statistics
    #[serde(default = "default_enabled")]
    pub enabled: bool, // Whether this token is enabled for authentication
    // Permission fields for fine-grained access control
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>, // e.g., ["filesystem/*", "database/query"]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_resources: Option<Vec<String>>, // e.g., ["filesystem/logs/*"]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_prompts: Option<Vec<String>>, // e.g., ["codegen/*"]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_prompt_templates: Option<Vec<String>>, // e.g., ["prompt-gallery__template_name"]
}

impl Token {
    /// Determine if this token has no explicit permission configuration
    fn is_unrestricted(&self) -> bool {
        self.allowed_tools.is_none()
            && self.allowed_resources.is_none()
            && self.allowed_prompts.is_none()
            && self.allowed_prompt_templates.is_none()
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = Utc::now().timestamp() as u64;
            now > expires_at
        } else {
            false
        }
    }

    /// Check if token has permission to access a specific tool
    pub fn has_tool_permission(&self, tool_name: &str) -> bool {
        match &self.allowed_tools {
            None => self.is_unrestricted(), // No restrictions configured anywhere => allow all
            Some(allowed) => allowed
                .iter()
                .any(|pattern| self.matches_pattern(pattern, tool_name)),
        }
    }

    /// Check if token has permission to access a specific resource
    pub fn has_resource_permission(&self, resource_uri: &str) -> bool {
        match &self.allowed_resources {
            None => self.is_unrestricted(), // Only unrestricted tokens get implicit access
            Some(allowed) => allowed
                .iter()
                .any(|pattern| self.matches_pattern(pattern, resource_uri)),
        }
    }

    /// Check if token has permission to access a specific prompt
    pub fn has_prompt_permission(&self, prompt_name: &str) -> bool {
        match &self.allowed_prompts {
            None => self.is_unrestricted(), // Require explicit prompts list once any permissions are set
            Some(allowed) => allowed
                .iter()
                .any(|pattern| self.matches_pattern(pattern, prompt_name)),
        }
    }

    /// Check if token has permission to access a specific prompt template
    pub fn has_prompt_template_permission(&self, template_name: &str) -> bool {
        match &self.allowed_prompt_templates {
            None => self.is_unrestricted(), // Require explicit prompt templates list once any permissions are set
            Some(allowed) => allowed
                .iter()
                .any(|pattern| self.matches_pattern(pattern, template_name)),
        }
    }

    /// Pattern matching function for permissions
    fn matches_pattern(&self, pattern: &str, item: &str) -> bool {
        match pattern {
            "*" => true, // Global wildcard
            _ if pattern.ends_with("__*") => {
                // Server wildcard: "server__*" matches "server__tool" but not "server__"
                let server = &pattern[..pattern.len() - 3];
                let prefix = format!("{}__", server);
                item.starts_with(&prefix) && item.len() > prefix.len()
            }
            _ => pattern == item, // Exact match
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Token;

    fn base_token() -> Token {
        Token {
            id: "tok-test".into(),
            value: "mcp-test".into(),
            name: "test".into(),
            description: None,
            created_at: 0,
            expires_at: None,
            last_used_at: None,
            usage_count: 0,
            enabled: true,
            allowed_tools: None,
            allowed_resources: None,
            allowed_prompts: None,
            allowed_prompt_templates: None,
        }
    }

    #[test]
    fn unrestricted_token_allows_everything() {
        let token = base_token();
        assert!(token.has_tool_permission("any__tool"));
        assert!(token.has_resource_permission("any__resource"));
        assert!(token.has_prompt_permission("any__prompt"));
        assert!(token.has_prompt_template_permission("any__template"));
    }

    #[test]
    fn restricting_tools_disallows_other_categories_by_default() {
        let mut token = base_token();
        token.allowed_tools = Some(vec!["server__tool_a".into()]);

        assert!(token.has_tool_permission("server__tool_a"));
        assert!(!token.has_tool_permission("server__tool_b"));
        assert!(!token.has_prompt_permission("server__prompt_a"));
        assert!(!token.has_resource_permission("server__resource"));
        assert!(!token.has_prompt_template_permission("server__template"));
    }

    #[test]
    fn explicit_wildcard_can_allow_category_when_other_permissions_set() {
        let mut token = base_token();
        token.allowed_prompts = Some(vec!["server__prompt_a".into()]);
        token.allowed_tools = Some(vec!["*".into()]);

        assert!(token.has_tool_permission("another__tool"));
        assert!(token.has_prompt_permission("server__prompt_a"));
        assert!(!token.has_prompt_permission("server__prompt_b"));
        assert!(!token.has_resource_permission("server__resource"));
    }

    #[test]
    fn test_double_underscore_wildcard_matching() {
        let mut token = base_token();
        token.allowed_tools = Some(vec!["server__*".into()]);

        assert!(token.has_tool_permission("server__tool_a"));
        assert!(token.has_tool_permission("server__tool_b"));
        assert!(token.has_tool_permission("server__some_complex_tool_name"));
        assert!(!token.has_tool_permission("other_server__tool"));
        assert!(!token.has_tool_permission("server"));
        assert!(!token.has_tool_permission("server__"));
    }

    #[test]
    fn test_prompt_template_permissions() {
        let mut token = base_token();
        token.allowed_prompt_templates = Some(vec![
            "prompt-gallery__code_review".into(),
            "template-system__*".into(),
        ]);

        assert!(token.has_prompt_template_permission("prompt-gallery__code_review"));
        assert!(token.has_prompt_template_permission("template-system__debug_template"));
        assert!(token.has_prompt_template_permission("template-system__custom_template"));
        assert!(!token.has_prompt_template_permission("other-gallery__code_review"));
        assert!(!token.has_prompt_template_permission("prompt-gallery__other_template"));
    }

    #[test]
    fn test_all_permission_types_with_double_underscore() {
        let mut token = base_token();
        token.allowed_tools = Some(vec!["filesystem__*".into()]);
        token.allowed_resources = Some(vec!["database__*".into()]);
        token.allowed_prompts = Some(vec!["codegen__*".into()]);
        token.allowed_prompt_templates = Some(vec!["gallery__*".into()]);

        // Test positive cases
        assert!(token.has_tool_permission("filesystem__read_file"));
        assert!(token.has_resource_permission("database__users"));
        assert!(token.has_prompt_permission("codegen__generate_class"));
        assert!(token.has_prompt_template_permission("gallery__review_template"));

        // Test negative cases
        assert!(!token.has_tool_permission("other__tool"));
        assert!(!token.has_resource_permission("other__resource"));
        assert!(!token.has_prompt_permission("other__prompt"));
        assert!(!token.has_prompt_template_permission("other__template"));
    }
}

/// Token storage structure for persistence
#[derive(Debug, Serialize, Deserialize)]
struct TokenStorage {
    pub tokens: HashMap<String, Token>,
    pub version: u32, // Storage format version for future migrations
}

impl Default for TokenStorage {
    fn default() -> Self {
        Self {
            tokens: HashMap::new(),
            version: 2,
        }
    }
}

/// Token manager for handling dynamic token lifecycle
pub struct TokenManager {
    tokens: Arc<RwLock<HashMap<String, Token>>>,
    file_path: PathBuf,
}

impl TokenManager {
    /// Create a new TokenManager with the given file path
    pub async fn new(file_path: PathBuf) -> Result<Self> {
        let manager = Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            file_path,
        };

        // Ensure directory exists
        if let Some(parent) = manager.file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                McpError::InternalError(format!("Failed to create directory: {}", e))
            })?;
        }

        // Load existing tokens
        manager.load().await?;

        Ok(manager)
    }

    /// Generate a secure random token value
    pub fn generate_secure_token() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 48]; // 48 bytes = 64 base64 chars
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut bytes);
        format!("mcp-{}", URL_SAFE_NO_PAD.encode(&bytes))
    }

    /// Generate a unique token ID
    pub fn generate_token_id() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 24]; // 24 bytes = 32 base64 chars
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut bytes);
        format!("tok-{}", URL_SAFE_NO_PAD.encode(&bytes))
    }

    /// Create a new token with given parameters
    #[allow(dead_code)]
    pub async fn create(
        &self,
        name: String,
        description: Option<String>,
        expires_in: Option<u64>, // Duration in seconds from now
    ) -> Result<Token> {
        self.create_with_permissions(name, description, expires_in, None, None, None, None)
            .await
    }

    /// Create a new token with permissions
    pub async fn create_with_permissions(
        &self,
        name: String,
        description: Option<String>,
        expires_in: Option<u64>, // Duration in seconds from now
        allowed_tools: Option<Vec<String>>,
        allowed_resources: Option<Vec<String>>,
        allowed_prompts: Option<Vec<String>>,
        allowed_prompt_templates: Option<Vec<String>>,
    ) -> Result<Token> {
        // Validate input
        if name.trim().is_empty() {
            return Err(McpError::ValidationError(
                "Token name cannot be empty".to_string(),
            ));
        }

        if name.len() > 100 {
            return Err(McpError::ValidationError(
                "Token name too long (max 100 chars)".to_string(),
            ));
        }

        if let Some(ref desc) = description {
            if desc.len() > 500 {
                return Err(McpError::ValidationError(
                    "Description too long (max 500 chars)".to_string(),
                ));
            }
        }

        // Check for duplicate names
        let tokens = self.tokens.read().await;
        if tokens.values().any(|t| t.name == name) {
            return Err(McpError::ValidationError(
                "Token name already exists".to_string(),
            ));
        }
        drop(tokens);

        let now = Utc::now().timestamp() as u64;
        let expires_at = expires_in.map(|seconds| now + seconds);

        let token = Token {
            id: Self::generate_token_id(),
            value: Self::generate_secure_token(),
            name,
            description,
            created_at: now,
            expires_at,
            last_used_at: None,
            usage_count: 0,
            enabled: true,
            // Use provided permission fields
            allowed_tools,
            allowed_resources,
            allowed_prompts,
            allowed_prompt_templates,
        };

        // Add to storage
        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(token.id.clone(), token.clone());
        }

        // Save to file
        self.save().await?;

        tracing::info!("Created new token '{}' with ID: {}", token.name, token.id);

        Ok(token)
    }

    /// List all tokens (without actual values for security)
    pub async fn list(&self) -> Result<Vec<TokenInfo>> {
        let tokens = self.tokens.read().await;
        let mut result = Vec::new();

        for token in tokens.values() {
            result.push(TokenInfo {
                id: token.id.clone(),
                name: token.name.clone(),
                description: token.description.clone(),
                created_at: token.created_at,
                expires_at: token.expires_at,
                last_used_at: token.last_used_at,
                usage_count: token.usage_count,
                is_expired: token.is_expired(),
                enabled: token.enabled,
                allowed_tools: token.allowed_tools.clone(),
                allowed_resources: token.allowed_resources.clone(),
                allowed_prompts: token.allowed_prompts.clone(),
            });
        }

        // Sort by creation time (newest first)
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(result)
    }

    /// Delete a token by ID
    pub async fn delete(&self, token_id: &str) -> Result<()> {
        let mut tokens = self.tokens.write().await;

        if let Some(token) = tokens.remove(token_id) {
            drop(tokens);
            self.save().await?;
            tracing::info!("Deleted token '{}' with ID: {}", token.name, token_id);
            Ok(())
        } else {
            Err(McpError::NotFound(format!(
                "Token with ID '{}' not found",
                token_id
            )))
        }
    }

    /// Validate a token value and return the token ID if valid
    pub async fn validate_token(&self, token_value: &str) -> Option<String> {
        let tokens = self.tokens.read().await;

        for (token_id, token) in tokens.iter() {
            if !token.is_expired()
                && token.enabled
                && constant_time_compare(token_value, &token.value)
            {
                return Some(token_id.clone());
            }
        }

        None
    }

    /// Get token by ID for permission validation
    #[allow(dead_code)]
    pub async fn get_token_by_id(&self, token_id: &str) -> Result<Token> {
        let tokens = self.tokens.read().await;

        tokens
            .get(token_id)
            .cloned()
            .ok_or_else(|| McpError::NotFound(format!("Token with ID '{}' not found", token_id)))
    }

    /// Get all tokens with full token values (for Dashboard configuration generation)
    pub async fn get_all_tokens(&self) -> Result<Vec<Token>> {
        let tokens = self.tokens.read().await;
        Ok(tokens.values().cloned().collect())
    }

    /// Record usage statistics for a token
    pub async fn record_usage(&self, token_id: &str) -> Result<()> {
        let mut tokens = self.tokens.write().await;

        if let Some(token) = tokens.get_mut(token_id) {
            let now = Utc::now().timestamp() as u64;
            token.last_used_at = Some(now);
            token.usage_count += 1;

            tracing::debug!(
                "Recorded usage for token: {} (usage count: {})",
                token_id,
                token.usage_count
            );

            // Drop the lock before saving
            drop(tokens);

            // Save immediately to ensure usage data is persisted
            let manager = self.clone();
            tokio::spawn(async move {
                if let Err(e) = manager.save().await {
                    tracing::error!("Failed to save token usage statistics: {}", e);
                } else {
                    tracing::debug!("Token usage statistics saved successfully");
                }
            });

            Ok(())
        } else {
            Err(McpError::NotFound(format!(
                "Token with ID '{}' not found",
                token_id
            )))
        }
    }

    /// Toggle a token's enabled status
    pub async fn toggle_token(&self, token_id: &str) -> Result<bool> {
        let mut tokens = self.tokens.write().await;

        if let Some(token) = tokens.get_mut(token_id) {
            token.enabled = !token.enabled;
            let new_status = token.enabled;

            drop(tokens);
            self.save().await?;

            tracing::info!(
                "Token '{}' enabled status changed to: {}",
                token_id,
                new_status
            );
            Ok(new_status)
        } else {
            Err(McpError::NotFound(format!(
                "Token with ID '{}' not found",
                token_id
            )))
        }
    }

    /// Update an existing token's permissions and metadata
    pub async fn update_token(
        &self,
        token_id: &str,
        name: Option<String>,
        description: Option<String>,
        allowed_tools: Option<Option<Vec<String>>>,
        allowed_resources: Option<Option<Vec<String>>>,
        allowed_prompts: Option<Option<Vec<String>>>,
        allowed_prompt_templates: Option<Option<Vec<String>>>,
    ) -> Result<Token> {
        let mut tokens = self.tokens.write().await;

        // Check if token exists first
        if !tokens.contains_key(token_id) {
            return Err(McpError::NotFound(format!(
                "Token with ID '{}' not found",
                token_id
            )));
        }

        // Check for duplicate names if name is being updated
        if let Some(ref new_name) = name {
            if tokens
                .values()
                .any(|t| t.id != token_id && t.name == *new_name)
            {
                return Err(McpError::ValidationError(
                    "Token name already exists".to_string(),
                ));
            }
        }

        // Now get mutable reference and update
        if let Some(token) = tokens.get_mut(token_id) {
            // Update name if provided
            if let Some(new_name) = name {
                if new_name.trim().is_empty() {
                    return Err(McpError::ValidationError(
                        "Token name cannot be empty".to_string(),
                    ));
                }
                if new_name.len() > 100 {
                    return Err(McpError::ValidationError(
                        "Token name too long (max 100 chars)".to_string(),
                    ));
                }
                token.name = new_name;
            }

            // Update description if provided
            if let Some(new_desc) = description {
                if new_desc.len() > 500 {
                    return Err(McpError::ValidationError(
                        "Description too long (max 500 chars)".to_string(),
                    ));
                }
                token.description = Some(new_desc);
            }

            // Update permissions if provided
            if let Some(new_tools) = allowed_tools {
                token.allowed_tools = new_tools;
            }
            if let Some(new_resources) = allowed_resources {
                token.allowed_resources = new_resources;
            }
            if let Some(new_prompts) = allowed_prompts {
                token.allowed_prompts = new_prompts;
            }
            if let Some(new_prompt_templates) = allowed_prompt_templates {
                token.allowed_prompt_templates = new_prompt_templates;
            }

            let updated_token = token.clone();
            drop(tokens);

            // Save to file
            self.save().await?;

            tracing::info!(
                "Updated token '{}' with ID: {}",
                updated_token.name,
                token_id
            );

            Ok(updated_token)
        } else {
            Err(McpError::NotFound(format!(
                "Token with ID '{}' not found",
                token_id
            )))
        }
    }

    /// Clean up expired tokens and return the count of removed tokens
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let mut tokens = self.tokens.write().await;
        let initial_count = tokens.len();

        tokens.retain(|_, token| !token.is_expired());
        let removed_count = initial_count - tokens.len();

        if removed_count > 0 {
            drop(tokens);
            self.save().await?;
            tracing::info!("Cleaned up {} expired tokens", removed_count);
        }

        Ok(removed_count)
    }

    /// Save tokens to file
    pub async fn save(&self) -> Result<()> {
        let tokens = self.tokens.read().await;
        let storage = TokenStorage {
            tokens: tokens.clone(),
            version: 2,
        };

        let content = serde_json::to_string_pretty(&storage)
            .map_err(|e| McpError::InternalError(format!("Failed to serialize tokens: {}", e)))?;

        // Write to temporary file first, then move to avoid corruption
        let temp_path = self.file_path.with_extension("tmp");
        fs::write(&temp_path, content)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to write tokens file: {}", e)))?;

        // Set file permissions (600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_path)
                .await
                .map_err(|e| {
                    McpError::InternalError(format!("Failed to get file metadata: {}", e))
                })?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&temp_path, perms).await.map_err(|e| {
                McpError::InternalError(format!("Failed to set file permissions: {}", e))
            })?;
        }

        // Atomic rename
        fs::rename(&temp_path, &self.file_path)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to rename tokens file: {}", e)))?;

        Ok(())
    }

    /// Migrate permission formats from '/' to '__' separator
    async fn migrate_permission_formats(&self) -> Result<()> {
        let mut tokens = self.tokens.write().await;
        let mut migration_count = 0;
        let mut migrated_tokens = Vec::new();

        for (_id, token) in tokens.iter_mut() {
            let mut needs_migration = false;
            let mut migration_details = Vec::new();

            // Check tools permissions
            if let Some(ref mut tools) = token.allowed_tools {
                let original_tools = tools.clone();
                *tools = tools.iter().map(|p| p.replace('/', "__")).collect();
                if *tools != original_tools {
                    needs_migration = true;
                    migration_details.push(format!("tools: {} -> {}", original_tools.join(", "), tools.join(", ")));
                }
            }

            // Check resources permissions
            if let Some(ref mut resources) = token.allowed_resources {
                let original_resources = resources.clone();
                *resources = resources.iter().map(|p| p.replace('/', "__")).collect();
                if *resources != original_resources {
                    needs_migration = true;
                    migration_details.push(format!("resources: {} -> {}", original_resources.join(", "), resources.join(", ")));
                }
            }

            // Check prompts permissions
            if let Some(ref mut prompts) = token.allowed_prompts {
                let original_prompts = prompts.clone();
                *prompts = prompts.iter().map(|p| p.replace('/', "__")).collect();
                if *prompts != original_prompts {
                    needs_migration = true;
                    migration_details.push(format!("prompts: {} -> {}", original_prompts.join(", "), prompts.join(", ")));
                }
            }

            // Check prompt templates permissions
            if let Some(ref mut templates) = token.allowed_prompt_templates {
                let original_templates = templates.clone();
                *templates = templates.iter().map(|p| p.replace('/', "__")).collect();
                if *templates != original_templates {
                    needs_migration = true;
                    migration_details.push(format!("prompt_templates: {} -> {}", original_templates.join(", "), templates.join(", ")));
                }
            }

            if needs_migration {
                migration_count += 1;
                migrated_tokens.push((token.name.clone(), migration_details));
            }
        }

        if migration_count > 0 {
            // Create backup before migration
            self.create_backup().await?;

            // Save migrated tokens
            drop(tokens);
            self.save().await?;

            tracing::info!(
                "Migrated permissions for {} tokens from '/' to '__' format",
                migration_count
            );

            for (token_name, details) in migrated_tokens {
                tracing::info!(
                    "Token '{}': {}",
                    token_name,
                    details.join("; ")
                );
            }
        }

        Ok(())
    }

    /// Create a backup of the current tokens file
    async fn create_backup(&self) -> Result<()> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let backup_path = self.file_path.with_extension("json.backup");
        let content = fs::read_to_string(&self.file_path).await.map_err(|e| {
            McpError::InternalError(format!("Failed to read tokens file for backup: {}", e))
        })?;

        fs::write(&backup_path, content).await.map_err(|e| {
            McpError::InternalError(format!("Failed to create backup file: {}", e))
        })?;

        tracing::info!("Created backup file: {:?}", backup_path);
        Ok(())
    }

    /// Load tokens from file
    pub async fn load(&self) -> Result<()> {
        if !self.file_path.exists() {
            // File doesn't exist, create empty storage
            tracing::info!("Tokens file not found, starting with empty token storage");
            return Ok(());
        }

        let content = fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| McpError::InternalError(format!("Failed to read tokens file: {}", e)))?;

        let storage: TokenStorage = serde_json::from_str(&content)
            .map_err(|e| McpError::InternalError(format!("Failed to parse tokens file: {}", e)))?;

        // Validate version compatibility
        match storage.version {
            1 => {
                // Version 1: Add permission fields with None values for backward compatibility
                let mut migrated_tokens = HashMap::new();
                for (id, mut token) in storage.tokens {
                    // Set permission fields to None (unrestricted access)
                    token.allowed_tools = None;
                    token.allowed_resources = None;
                    token.allowed_prompts = None;
                    token.allowed_prompt_templates = None;
                    migrated_tokens.insert(id, token);
                }

                let mut tokens = self.tokens.write().await;
                *tokens = migrated_tokens;

                tracing::info!(
                    "Migrated {} tokens from version 1 to version 2",
                    tokens.len()
                );
            }
            2 => {
                // Version 2: Current version, ensure prompt templates field exists
                let mut migrated_tokens = HashMap::new();
                for (id, mut token) in storage.tokens {
                    // Ensure prompt templates field exists (for backward compatibility)
                    if token.allowed_prompt_templates.is_none() {
                        token.allowed_prompt_templates = None;
                    }
                    migrated_tokens.insert(id, token);
                }

                let mut tokens = self.tokens.write().await;
                *tokens = migrated_tokens;
            }
            _ => {
                return Err(McpError::InternalError(format!(
                    "Unsupported token storage version: {}",
                    storage.version
                )));
            }
        }

        let tokens = self.tokens.read().await;
        tracing::info!("Loaded {} tokens from storage", tokens.len());

        // Check and migrate permission formats from '/' to '__'
        drop(tokens);
        self.migrate_permission_formats().await?;

        Ok(())
    }
}

/// Token information without the actual token value (for API responses)
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
    // Permission fields for API responses
    pub allowed_tools: Option<Vec<String>>,
    pub allowed_resources: Option<Vec<String>>,
    pub allowed_prompts: Option<Vec<String>>,
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }
    result == 0
}

// Add necessary dependencies to Cargo.toml
// chrono = { version = "0.4", features = ["serde"] }
// base64 = "0.21"
// rand = "0.8"

impl Clone for TokenManager {
    fn clone(&self) -> Self {
        Self {
            tokens: self.tokens.clone(),
            file_path: self.file_path.clone(),
        }
    }
}

// Add required imports
use base64;
