-- Migration: Remove resource_path field from permissions table
-- Version: 4.0.0

-- ============================================================================
-- Remove resource_path column from permissions table
-- ============================================================================

-- First, clear all data from permissions table
DELETE FROM permissions;

-- Drop the old permissions table
DROP TABLE IF EXISTS permissions;

-- Create new permissions table without resource_path field
-- resource_id will now store the permission pattern (e.g., tool name, resource path)
CREATE TABLE permissions (
    id TEXT PRIMARY KEY,                    -- UUID v7 for natural ordering
    token_id TEXT NOT NULL,                 -- Foreign key to tokens table
    resource_type TEXT NOT NULL,            -- 'tool', 'resource', or 'prompt'
    resource_id TEXT,                       -- Permission pattern (e.g., tool name, resource path)
    allowed BOOLEAN NOT NULL DEFAULT 1,     -- Permission granted or denied
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Ensure resource_id is provided
    CONSTRAINT chk_resource_id CHECK (
        resource_id IS NOT NULL
    ),

    -- Unique constraint for permission entries
    UNIQUE(token_id, resource_type, resource_id)
);

-- Performance indexes for permissions
CREATE INDEX IF NOT EXISTS idx_permissions_resource_id ON permissions(resource_id);
CREATE INDEX IF NOT EXISTS idx_permissions_resource_type ON permissions(resource_type);
CREATE INDEX IF NOT EXISTS idx_permissions_token_resource_type ON permissions(token_id, resource_type);

-- ============================================================================
-- Update schema version
-- ============================================================================

INSERT OR IGNORE INTO schema_version (id, version) VALUES (4, '4.0.0');
