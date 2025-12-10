-- Add missing token fields for full compatibility
-- This migration adds fields that were present in JSON token format

-- Add token status and usage tracking fields to tokens table
ALTER TABLE tokens ADD COLUMN enabled BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE tokens ADD COLUMN last_used_at DATETIME;
ALTER TABLE tokens ADD COLUMN usage_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tokens ADD COLUMN expires_at DATETIME;

-- Create indexes for the new fields
CREATE INDEX IF NOT EXISTS idx_tokens_enabled ON tokens(enabled);
CREATE INDEX IF NOT EXISTS idx_tokens_last_used ON tokens(last_used_at);
CREATE INDEX IF NOT EXISTS idx_tokens_expires_at ON tokens(expires_at);

-- Add permission column for prompt templates to support all permission types
ALTER TABLE permissions ADD COLUMN prompt_template TEXT;

-- Create compound index for better permission lookup performance
CREATE INDEX IF NOT EXISTS idx_permissions_token_resource ON permissions(token_id, resource_type, resource_path);

-- Update schema version
INSERT OR REPLACE INTO schema_version (id, version) VALUES (2, '2.1.0');