-- Add sessions table for SQLite-based session management
-- This migration adds persistent session storage to replace in-memory DashMap

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    token_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_accessed DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME,
    metadata TEXT,  -- JSON for additional session data
    FOREIGN KEY (token_id) REFERENCES tokens(id) ON DELETE CASCADE
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_sessions_token_id ON sessions(token_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_sessions_last_accessed ON sessions(last_accessed);

-- Update schema version
INSERT OR REPLACE INTO schema_version (id, version) VALUES (3, '2.2.0');