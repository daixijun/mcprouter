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
}

impl Token {
    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = Utc::now().timestamp() as u64;
            now > expires_at
        } else {
            false
        }
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
            version: 1,
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
    pub async fn create(
        &self,
        name: String,
        description: Option<String>,
        expires_in: Option<u64>, // Duration in seconds from now
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
            version: 1,
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
        if storage.version != 1 {
            return Err(McpError::InternalError(format!(
                "Unsupported token storage version: {}",
                storage.version
            )));
        }

        let mut tokens = self.tokens.write().await;
        *tokens = storage.tokens;

        tracing::info!("Loaded {} tokens from storage", tokens.len());

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
