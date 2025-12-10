// Session storage implementation with SQLite
#![allow(dead_code)]

use super::{Result, StorageError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_scalar, Row, SqlitePool};
use std::time::Duration;

/// Session information stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub token_id: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

/// Session storage with SQLite backend
pub struct SessionStorage {
    pool: SqlitePool,
}

impl SessionStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        session_id: String,
        token_id: &str,
        expires_at: Option<DateTime<Utc>>,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let now = Utc::now();

        query(
            r#"
            INSERT INTO sessions (id, token_id, created_at, last_accessed, expires_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session_id)
        .bind(token_id)
        .bind(now)
        .bind(now)
        .bind(expires_at)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to create session: {}", e)))?;

        Ok(())
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>> {
        let row = query(
            r#"
            SELECT id, token_id, created_at, last_accessed, expires_at, metadata
            FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get session: {}", e)))?;

        match row {
            Some(row) => {
                let session = SessionData {
                    id: row.get("id"),
                    token_id: row.get("token_id"),
                    created_at: row.get("created_at"),
                    last_accessed: row.get("last_accessed"),
                    expires_at: row.get("expires_at"),
                    metadata: row.get("metadata"),
                };
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    /// Update session last accessed time
    pub async fn update_access_time(&self, session_id: &str) -> Result<()> {
        let now = Utc::now();

        query(
            "UPDATE sessions SET last_accessed = ? WHERE id = ?"
        )
        .bind(now)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to update session access time: {}", e)))?;

        Ok(())
    }

    /// Update session metadata
    pub async fn update_metadata(&self, session_id: &str, metadata: serde_json::Value) -> Result<()> {
        let now = Utc::now();

        query(
            "UPDATE sessions SET metadata = ?, last_accessed = ? WHERE id = ?"
        )
        .bind(metadata)
        .bind(now)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to update session metadata: {}", e)))?;

        Ok(())
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let result = query("DELETE FROM sessions WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete session: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("Session with id {} not found", session_id)));
        }

        Ok(())
    }

    /// List all active sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionData>> {
        let rows = query(
            r#"
            SELECT id, token_id, created_at, last_accessed, expires_at, metadata
            FROM sessions
            WHERE expires_at IS NULL OR expires_at > ?
            ORDER BY last_accessed DESC
            "#,
        )
        .bind(Utc::now())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to list sessions: {}", e)))?;

        let sessions = rows
            .into_iter()
            .map(|row| SessionData {
                id: row.get("id"),
                token_id: row.get("token_id"),
                created_at: row.get("created_at"),
                last_accessed: row.get("last_accessed"),
                expires_at: row.get("expires_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(sessions)
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        let now = Utc::now();

        let result = query("DELETE FROM sessions WHERE expires_at IS NOT NULL AND expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to cleanup expired sessions: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    /// Clean up idle sessions
    pub async fn cleanup_idle_sessions(&self, idle_timeout: Duration) -> Result<usize> {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(idle_timeout).unwrap_or_default();

        let result = query("DELETE FROM sessions WHERE last_accessed < ?")
            .bind(cutoff_time)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to cleanup idle sessions: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    /// Get sessions for a specific token
    pub async fn get_sessions_by_token(&self, token_id: &str) -> Result<Vec<SessionData>> {
        let rows = query(
            r#"
            SELECT id, token_id, created_at, last_accessed, expires_at, metadata
            FROM sessions
            WHERE token_id = ?
            ORDER BY last_accessed DESC
            "#,
        )
        .bind(token_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get sessions by token: {}", e)))?;

        let sessions = rows
            .into_iter()
            .map(|row| SessionData {
                id: row.get("id"),
                token_id: row.get("token_id"),
                created_at: row.get("created_at"),
                last_accessed: row.get("last_accessed"),
                expires_at: row.get("expires_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(sessions)
    }

    /// Get session count
    pub async fn get_session_count(&self) -> Result<i64> {
        let count: Option<i64> = query_scalar("SELECT COUNT(*) FROM sessions")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to get session count: {}", e)))?;

        Ok(count.unwrap_or(0))
    }
}

/// Initialize sessions table
pub async fn initialize_sessions_table(pool: &SqlitePool) -> Result<()> {
    query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            token_id TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_accessed DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at DATETIME,
            metadata TEXT,  -- JSON for additional session data
            FOREIGN KEY (token_id) REFERENCES tokens(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| StorageError::Database(format!("Failed to create sessions table: {}", e)))?;

    // Create indexes
    query("CREATE INDEX IF NOT EXISTS idx_sessions_token_id ON sessions(token_id)")
        .execute(pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to create session index: {}", e)))?;

    query("CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at)")
        .execute(pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to create expires_at index: {}", e)))?;

    query("CREATE INDEX IF NOT EXISTS idx_sessions_last_accessed ON sessions(last_accessed)")
        .execute(pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to create last_accessed index: {}", e)))?;

    Ok(())
}