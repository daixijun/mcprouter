-- MCP Router SQLite Database Schema
-- 包含 tokens、permissions 和 MCP server 管理的完整表结构

-- ============================================================================
-- Core Tables
-- ============================================================================

-- Tokens table for storing bearer tokens
CREATE TABLE IF NOT EXISTS tokens (
    id TEXT PRIMARY KEY,                    -- UUID v7 for natural ordering
    name TEXT NOT NULL,                     -- Human-readable token name
    token TEXT NOT NULL UNIQUE,             -- The actual token value
    description TEXT,                       -- Optional description
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for tokens
CREATE UNIQUE INDEX IF NOT EXISTS idx_tokens_token ON tokens(token);
CREATE INDEX IF NOT EXISTS idx_tokens_name ON tokens(name);

-- Permissions table for token-based access control
CREATE TABLE IF NOT EXISTS permissions (
    id TEXT PRIMARY KEY,                    -- UUID v7 for natural ordering
    token_id TEXT NOT NULL,                 -- Foreign key to tokens table
    resource_type TEXT NOT NULL,            -- 'tool', 'resource', or 'prompt'
    resource_id TEXT,                       -- UUID of the resource (for structured lookups)
    mcp_server_id TEXT,                     -- UUID of the associated MCP server
    resource_path TEXT,                     -- Legacy path format for backward compatibility
    allowed BOOLEAN NOT NULL DEFAULT 1,     -- Permission granted or denied
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Ensure either structured ID or legacy path is provided
    CONSTRAINT chk_resource_identifier CHECK (
        (resource_id IS NOT NULL AND mcp_server_id IS NOT NULL AND resource_path IS NULL) OR
        (resource_id IS NULL AND mcp_server_id IS NULL AND resource_path IS NOT NULL)
    ),

    -- Unique constraint for permission entries
    UNIQUE(token_id, resource_type, resource_id, mcp_server_id)
);

-- Performance indexes for permissions
CREATE INDEX IF NOT EXISTS idx_permissions_resource_id ON permissions(resource_id);
CREATE INDEX IF NOT EXISTS idx_permissions_mcp_server_id ON permissions(mcp_server_id);
CREATE INDEX IF NOT EXISTS idx_permissions_resource_type ON permissions(resource_type);
CREATE INDEX IF NOT EXISTS idx_permissions_token_resource_type ON permissions(token_id, resource_type);
CREATE INDEX IF NOT EXISTS idx_permissions_resource_lookup ON permissions(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_permissions_path ON permissions(resource_path) WHERE resource_path IS NOT NULL;

-- ============================================================================
-- MCP Server Management Tables
-- ============================================================================

-- MCP servers configuration table
CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,                    -- UUID v7
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    type TEXT NOT NULL CHECK (type IN ('stdio', 'http')),
    command TEXT,
    args TEXT,                              -- JSON array
    url TEXT,
    headers TEXT,                           -- JSON object
    env TEXT,                               -- JSON object
    enabled BOOLEAN NOT NULL DEFAULT 1,
    version TEXT,                           -- Server version information
    last_version_check DATETIME,            -- Last version check time
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Performance indexes for mcp_servers
CREATE UNIQUE INDEX IF NOT EXISTS idx_mcp_servers_name ON mcp_servers(name);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_type ON mcp_servers(type);

-- MCP server tools cache table
CREATE TABLE IF NOT EXISTS mcp_server_tools (
    id TEXT PRIMARY KEY,                    -- UUID v7
    server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    title TEXT,                             -- Display title for the tool
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    input_schema TEXT,                      -- JSON schema for tool input
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(server_id, name)
);

-- Performance indexes for mcp_server_tools
CREATE INDEX IF NOT EXISTS idx_mcp_server_tools_server_id ON mcp_server_tools(server_id);
CREATE INDEX IF NOT EXISTS idx_mcp_server_tools_enabled ON mcp_server_tools(enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_server_tools_name ON mcp_server_tools(name);

-- MCP server resources cache table
CREATE TABLE IF NOT EXISTS mcp_server_resources (
    id TEXT PRIMARY KEY,                    -- UUID v7
    server_id TEXT NOT NULL,
    uri TEXT NOT NULL,
    name TEXT NOT NULL,
    title TEXT,                             -- Display title for the resource
    description TEXT,
    mime_type TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    text_content TEXT,                      -- Resource content (if applicable)
    size INTEGER,                           -- Resource size in bytes
    is_template BOOLEAN NOT NULL DEFAULT 0, -- Whether this is a resource template
    uri_template TEXT,                      -- URI template for template resources
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(server_id, uri)
);

-- Performance indexes for mcp_server_resources
CREATE INDEX IF NOT EXISTS idx_mcp_server_resources_server_id ON mcp_server_resources(server_id);
CREATE INDEX IF NOT EXISTS idx_mcp_server_resources_enabled ON mcp_server_resources(enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_server_resources_uri ON mcp_server_resources(uri);
CREATE INDEX IF NOT EXISTS idx_mcp_server_resources_template ON mcp_server_resources(is_template);

-- MCP server prompts cache table
CREATE TABLE IF NOT EXISTS mcp_server_prompts (
    id TEXT PRIMARY KEY,                    -- UUID v7
    server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    title TEXT,                             -- Display title for the prompt
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    arguments TEXT,                         -- JSON array of prompt arguments
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(server_id, name)
);

-- Performance indexes for mcp_server_prompts
CREATE INDEX IF NOT EXISTS idx_mcp_server_prompts_server_id ON mcp_server_prompts(server_id);
CREATE INDEX IF NOT EXISTS idx_mcp_server_prompts_enabled ON mcp_server_prompts(enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_server_prompts_name ON mcp_server_prompts(name);

-- ============================================================================
-- Schema Version Table
-- ============================================================================

-- Database schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    id INTEGER PRIMARY KEY,
    version TEXT NOT NULL UNIQUE,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Insert initial schema version
INSERT OR IGNORE INTO schema_version (id, version) VALUES (1, '2.0.0');

-- ============================================================================
-- SQLite Performance Optimizations
-- ============================================================================

-- Enable WAL mode for better concurrency
PRAGMA journal_mode = WAL;

-- Optimize for better performance
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = 10000;
PRAGMA temp_store = memory;
PRAGMA mmap_size = 268435456;  -- 256MB

-- ============================================================================
-- Application-Level Data Integrity Notes
-- ============================================================================

-- This schema does not use database-level foreign key constraints.
-- Data integrity is maintained at the application level:

-- 1. Referential Integrity:
--    - mcp_server_tools.server_id references mcp_servers.id
--    - mcp_server_resources.server_id references mcp_servers.id
--    - mcp_server_prompts.server_id references mcp_servers.id
--    - permissions.mcp_server_id references mcp_servers.id (when not null)
--    - permissions.resource_id references appropriate resource tables (when not null)

-- 2. Cascade Delete Implementation:
--    - Deleting a server must delete all associated tools, resources, prompts, and permissions
--    - Deleting a resource should remove associated permissions
--    - Application code must handle these cascades in transactions

-- 3. Validation Rules:
--    - All ID fields must be valid UUIDs
--    - server_id fields must reference existing servers
--    - resource_id fields must reference existing resources
--    - Permission records must have either structured IDs or legacy paths, not both

-- 4. Cleanup Procedures:
--    - Periodic cleanup of orphaned records
--    - Data integrity validation on startup
--    - UUID format validation for all ID fields