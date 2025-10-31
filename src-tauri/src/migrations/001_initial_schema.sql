-- 初始数据库架构
-- 创建 MCP 服务器表
CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    command TEXT,
    args TEXT,  -- JSON array as TEXT
    transport TEXT NOT NULL,
    url TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    env_vars TEXT,  -- JSON object as TEXT
    headers TEXT,   -- JSON object as TEXT
    version TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建 mcp_tools 表
CREATE TABLE IF NOT EXISTS mcp_tools (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    server_id TEXT NOT NULL,
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建 api_keys 表
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL, -- 存储哈希而非明文
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME DEFAULT CURRENT_TIMESTAMP  -- 添加最后使用时间字段
);

-- 创建 api_key_server_relations 表（用于服务器级别授权）
CREATE TABLE IF NOT EXISTS api_key_server_relations (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    server_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建 api_key_tool_relations 表（用于工具级别授权）
CREATE TABLE IF NOT EXISTS api_key_tool_relations (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    tool_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(api_key_id, tool_id)  -- 防止重复授权
);

-- 创建索引以优化查询性能
-- MCP 服务器表索引
CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_name ON mcp_servers(name);

-- MCP 工具表索引
CREATE INDEX IF NOT EXISTS idx_mcp_tools_server_id ON mcp_tools(server_id);
CREATE INDEX IF NOT EXISTS idx_mcp_tools_enabled ON mcp_tools(enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_tools_server_enabled ON mcp_tools(server_id, enabled);

-- API 密钥表索引
CREATE INDEX IF NOT EXISTS idx_api_keys_enabled ON api_keys(enabled);
CREATE INDEX IF NOT EXISTS idx_api_keys_last_used_at ON api_keys(last_used_at);

-- 服务器级授权关系表索引
CREATE INDEX IF NOT EXISTS idx_api_key_relations_key_id ON api_key_server_relations(api_key_id);
CREATE INDEX IF NOT EXISTS idx_api_key_relations_server_id ON api_key_server_relations(server_id);

-- 工具级授权关系表索引
CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_api_key_id ON api_key_tool_relations(api_key_id);
CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_tool_id ON api_key_tool_relations(tool_id);
CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_composite ON api_key_tool_relations(api_key_id, tool_id);