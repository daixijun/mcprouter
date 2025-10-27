-- 工具级别授权迁移脚本
-- 此脚本将授权粒度从 Server 级别细化到 Tool 级别

-- 步骤 1: 检查 tools 表是否存在，如果存在则重命名为 mcp_tools
-- SQLite 不支持 IF EXISTS 在 ALTER TABLE 中，所以需要在代码层面处理

-- 步骤 2: 重命名 tools 表为 mcp_tools（此步骤在 Rust 代码中执行，带错误处理）
-- ALTER TABLE tools RENAME TO mcp_tools;

-- 步骤 3: 创建 api_key_tool_relations 表
CREATE TABLE IF NOT EXISTS api_key_tool_relations (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    tool_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(api_key_id, tool_id)  -- 防止重复授权
);

-- 步骤 4: 创建索引以优化查询性能
CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_api_key_id ON api_key_tool_relations(api_key_id);
CREATE INDEX IF NOT EXISTS idx_api_key_tool_relations_tool_id ON api_key_tool_relations(tool_id);

-- 步骤 5: 更新旧的 tools 索引名称为 mcp_tools（如果表已重命名）
-- SQLite 会自动保留索引，但需要重新创建以使用新表名
DROP INDEX IF EXISTS idx_tools_server_id;
DROP INDEX IF EXISTS idx_tools_enabled;
CREATE INDEX IF NOT EXISTS idx_mcp_tools_server_id ON mcp_tools(server_id);
CREATE INDEX IF NOT EXISTS idx_mcp_tools_enabled ON mcp_tools(enabled);

-- 步骤 6: 数据迁移（从 api_key_server_relations 生成工具级授权）
-- 此步骤在 Rust 代码中执行，因为需要查询和批量插入逻辑
-- 迁移逻辑：
-- 1. 读取 api_key_server_relations 的每条记录
-- 2. 查询该 server_id 下的所有 tool_id（从 mcp_tools 表）
-- 3. 为每个 (api_key_id, tool_id) 组合插入一条 api_key_tool_relations 记录
