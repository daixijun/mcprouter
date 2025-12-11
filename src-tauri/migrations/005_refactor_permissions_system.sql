-- MCP 权限系统重构迁移脚本
-- 彻底重建表结构，使用 resource_path 字段和唯一索引
-- 只在 permissions 表中使用 resource_path 字段，其他 MCP 表保持原有结构

-- 1. 删除旧表（数据可以清理，不需要备份）
DROP TABLE IF EXISTS permissions;
DROP TABLE IF EXISTS mcp_server_tools;
DROP TABLE IF EXISTS mcp_server_resources;
DROP TABLE IF EXISTS mcp_server_prompts;

-- 2. 重建 MCP 工具表（移除 resource_path 字段，保持原有结构）
CREATE TABLE mcp_server_tools (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,           -- 保留到 mcp_servers 表的关联
    name TEXT NOT NULL,                -- 原始工具名称
    description TEXT,
    input_schema TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 3. 重建 MCP 资源表（移除 resource_path 字段，保持原有结构）
CREATE TABLE mcp_server_resources (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,           -- 保留到 mcp_servers 表的关联
    uri TEXT NOT NULL,                 -- 原始 URI
    name TEXT,                         -- 显示名称
    description TEXT,
    mime_type TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 4. 重建 MCP 提示词表（移除 resource_path 字段，保持原有结构）
CREATE TABLE mcp_server_prompts (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,           -- 保留到 mcp_servers 表的关联
    name TEXT NOT NULL,                -- 原始提示词名称
    description TEXT,
    arguments TEXT,                    -- JSON 格式存储参数
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 5. 重建权限表（使用 resource_path 字段存储完整路径）
CREATE TABLE permissions (
    id TEXT PRIMARY KEY,
    token_id TEXT NOT NULL,
    resource_type TEXT NOT NULL,       -- 'tool' | 'resource' | 'prompt'
    resource_path TEXT NOT NULL,       -- 格式：server__resource
    allowed BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 6. 创建唯一索引确保数据唯一性和性能（只对 permissions 表创建 resource_path 索引）
CREATE UNIQUE INDEX idx_permissions_unique ON permissions(token_id, resource_type, resource_path);
CREATE INDEX idx_permissions_resource_path ON permissions(resource_path);

-- 7. 创建查询性能索引
CREATE INDEX idx_permissions_token_id ON permissions(token_id);
CREATE INDEX idx_permissions_resource_type ON permissions(resource_type);
CREATE INDEX idx_tools_server_id ON mcp_server_tools(server_id);
CREATE INDEX idx_resources_server_id ON mcp_server_resources(server_id);
CREATE INDEX idx_prompts_server_id ON mcp_server_prompts(server_id);