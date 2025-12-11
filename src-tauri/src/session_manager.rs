// SQLite-based Session Manager implementation

use crate::error::{McpError, Result};
use crate::storage::session_storage::{SessionData, SessionStorage};
// use crate::storage::StorageError;  // 暂时注释掉，后续可能需要
use crate::types::Token;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

// Type alias for backward compatibility
pub type SessionManager = SessionManagerSqlite;

// Global session manager instance (safe for Rust 2024)
static SESSION_MANAGER: std::sync::Mutex<Option<Arc<SessionManager>>> = std::sync::Mutex::new(None);
static SESSION_MANAGER_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize the global session manager
pub fn init_session_manager(manager: Arc<SessionManager>) {
    SESSION_MANAGER_INIT.call_once(|| {
        let mut guard = SESSION_MANAGER.lock().unwrap();
        *guard = Some(manager);
    });
}

/// Get the global session manager
pub fn get_session_manager() -> Option<Arc<SessionManager>> {
    let guard = SESSION_MANAGER.lock().unwrap();
    guard.clone()
}

/// SQLite-based Session Manager
pub struct SessionManagerSqlite {
    storage: Arc<SessionStorage>,
    cleanup_interval: Duration,
    default_idle_timeout: Duration,
}

impl SessionManagerSqlite {
    /// Create a new SessionManager with SQLite backend
    pub async fn new(pool: sqlx::SqlitePool) -> Result<Self> {
        // SessionStorage::new will handle table initialization via migrations
        let storage = Arc::new(SessionStorage::new(pool));

        Ok(Self {
            storage,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            default_idle_timeout: Duration::from_secs(3600), // 1 hour
        })
    }

    /// Create a new SessionManager with custom configuration
    pub async fn new_with_config(
        pool: sqlx::SqlitePool,
        cleanup_interval: Duration,
        idle_timeout: Duration,
    ) -> Result<Self> {
        // SessionStorage::new will handle table initialization via migrations
        let storage = Arc::new(SessionStorage::new(pool));

        Ok(Self {
            storage,
            cleanup_interval,
            default_idle_timeout: idle_timeout,
        })
    }

    /// Create a new session and return its ID
    pub fn create_session(&self, token: Token) -> String {
        let session_id = Uuid::new_v4().to_string();

        // Convert token expiration time to session expiration time
        let expires_at = token.expires_at.map(|timestamp| {
            chrono::DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_else(chrono::Utc::now)
        });

        // Create session using the storage - block on async operation for now
        // This prevents tokio task queue buildup
        let storage_clone = self.storage.clone();
        let session_id_clone = session_id.clone();
        let token_id = token.id.clone();
        let token_id_for_log = token_id.clone(); // Clone for logging

        // Use block_in_place to avoid stack overflow
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                storage_clone
                    .create_session(
                        session_id_clone,
                        &token_id,
                        expires_at,
                        None, // No additional metadata for now
                    )
                    .await
            })
        });

        match result {
            Ok(_) => {
                tracing::debug!(
                    "Created SQLite session {} for token {}",
                    session_id,
                    token_id_for_log
                );
            }
            Err(e) => {
                tracing::error!("Failed to create session in database: {}", e);
            }
        }

        session_id
    }

    /// Get session information by ID
    pub fn get_session(&self, session_id: &str) -> Option<SessionData> {
        // For synchronous access, we need to block on the async call
        let storage = self.storage.clone();
        let session_id = session_id.to_string();

        // Use block_in_place to avoid stack overflow
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async move { storage.get_session(&session_id).await })
        });

        match result {
            Ok(session_data) => session_data,
            Err(e) => {
                tracing::error!("Failed to get session from database: {}", e);
                None
            }
        }
    }

    /// Get complete SessionInfo including full Token data
    /// This method should be used when creating sessions for authentication
    pub fn get_session_info_with_token(&self, session_id: &str, token_manager: &crate::token_manager::TokenManager) -> Option<SessionInfo> {
        let session_data = self.get_session(session_id)?;

        // Get full token information from TokenManager
        let token = match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                token_manager.get_token_by_id(&session_data.token_id)
            )
        }) {
            Ok(token) => {
                tracing::debug!("Successfully retrieved token for session {} - ID: {}, Enabled: {}, Tools: {}, Resources: {}, Prompts: {}",
                    session_id,
                    token.id,
                    token.enabled,
                    token.allowed_tools.as_ref().map_or(0, |t| t.len()),
                    token.allowed_resources.as_ref().map_or(0, |r| r.len()),
                    token.allowed_prompts.as_ref().map_or(0, |p| p.len())
                );
                token
            },
            Err(e) => {
                tracing::error!("Failed to get token information for session {}: {}", session_id, e);
                return None;
            }
        };

        // Convert expires_at to Instant
        let expires_at = session_data.expires_at.map(|dt| {
            std::time::Instant::now() + (dt - chrono::Utc::now()).to_std().unwrap_or_default()
        });

        Some(SessionInfo {
            id: session_data.id,
            token,
            created_at: std::time::Instant::now(), // We don't store this precisely, so use now
            last_accessed: std::time::Instant::now(),
            expires_at,
        })
    }

    /// Get session asynchronously (recommended method)
    pub async fn get_session_async(&self, session_id: &str) -> Result<Option<SessionData>> {
        self.storage
            .get_session(session_id)
            .await
            .map_err(|e| McpError::DatabaseError(format!("Session error: {}", e)))
    }

    /// Update session last accessed time
    pub fn update_access(&self, session_id: &str) {
        let storage = self.storage.clone();
        let session_id = session_id.to_string();

        // Use spawn with a lower priority to prevent stack overflow
        tokio::task::spawn_local(async move {
            if let Err(e) = storage.update_access_time(&session_id).await {
                tracing::error!("Failed to update session access time: {}", e);
            }
        });
    }

    /// Update session metadata
    pub fn update_metadata(&self, session_id: &str, metadata: serde_json::Value) {
        let storage = self.storage.clone();
        let session_id = session_id.to_string();

        // Use spawn with a lower priority to prevent stack overflow
        tokio::task::spawn_local(async move {
            if let Err(e) = storage.update_metadata(&session_id, metadata).await {
                tracing::error!("Failed to update session metadata: {}", e);
            }
        });
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        self.storage
            .delete_session(session_id)
            .await
            .map_err(|e| McpError::DatabaseError(format!("Session delete error: {}", e)))
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired(&self) -> Result<usize> {
        self.storage
            .cleanup_expired_sessions()
            .await
            .map_err(|e| McpError::DatabaseError(format!("Session cleanup error: {}", e)))
    }

    /// Clean up idle sessions
    pub async fn cleanup_idle(&self, idle_timeout: Option<Duration>) -> Result<usize> {
        let timeout = idle_timeout.unwrap_or(self.default_idle_timeout);
        self.storage
            .cleanup_idle_sessions(timeout)
            .await
            .map_err(|e| McpError::DatabaseError(format!("Session cleanup idle error: {}", e)))
    }

    /// List all active sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionData>> {
        self.storage
            .list_sessions()
            .await
            .map_err(|e| McpError::DatabaseError(format!("Session list error: {}", e)))
    }

    /// Get sessions for a specific token
    pub async fn get_sessions_by_token(&self, token_id: &str) -> Result<Vec<SessionData>> {
        self.storage
            .get_sessions_by_token(token_id)
            .await
            .map_err(|e| McpError::DatabaseError(format!("Session token query error: {}", e)))
    }

    /// Get total session count
    pub async fn get_session_count(&self) -> Result<i64> {
        self.storage
            .get_session_count()
            .await
            .map_err(|e| McpError::DatabaseError(format!("Session count error: {}", e)))
    }

    /// Check if session exists and is valid
    pub async fn is_session_valid(&self, session_id: &str) -> bool {
        match self
            .storage
            .get_session(session_id)
            .await
            .map_err(|e| McpError::DatabaseError(format!("Session error: {}", e)))
        {
            Ok(Some(session)) => {
                let now = chrono::Utc::now();
                // Check if session is expired
                if let Some(expires_at) = session.expires_at {
                    now <= expires_at
                } else {
                    true // No expiration time means session doesn't expire
                }
            }
            Ok(None) => false,
            Err(_) => false,
        }
    }

    /// Get cleanup interval
    pub fn cleanup_interval(&self) -> Duration {
        self.cleanup_interval
    }

    /// Get default idle timeout
    pub fn default_idle_timeout(&self) -> Duration {
        self.default_idle_timeout
    }
}

/// Session information wrapper for compatibility with existing code
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub token: Token,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub expires_at: Option<Instant>,
}

impl SessionInfo {
    pub fn new(token: Token, expires_at: Option<Instant>) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            token,
            created_at: now,
            last_accessed: now,
            expires_at,
        }
    }

    pub fn update_access(&mut self) {
        self.last_accessed = Instant::now();
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() > expires_at
        } else {
            self.token.is_expired()
        }
    }

    pub fn is_idle_longer_than(&self, idle_timeout: Duration) -> bool {
        self.last_accessed.elapsed() > idle_timeout
    }
}

/// Convert SessionData to SessionInfo
impl From<SessionData> for SessionInfo {
    fn from(data: SessionData) -> Self {
        // For now, we can't fully convert SessionData to SessionInfo
        // because we need the full Token object
        // This would require a TokenManager dependency
        Self {
            id: data.id,
            token: Token {
                id: data.token_id.clone(),
                name: String::new(), // Would need to fetch from database
                value: String::new(),
                description: None,
                created_at: data.created_at.timestamp() as u64,
                expires_at: data.expires_at.map(|dt| dt.timestamp() as u64),
                last_used_at: Some(data.last_accessed.timestamp() as u64),
                usage_count: 0,
                enabled: true,
                allowed_tools: None,
                allowed_resources: None,
                allowed_prompts: None,
                allowed_prompt_templates: None,
            },
            created_at: std::time::Instant::now(),
            last_accessed: std::time::Instant::now(),
            expires_at: data.expires_at.map(|dt| {
                std::time::Instant::now() + (dt - chrono::Utc::now()).to_std().unwrap_or_default()
            }),
        }
    }
}
