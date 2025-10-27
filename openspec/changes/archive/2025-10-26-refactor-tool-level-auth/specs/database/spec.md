## ADDED Requirements

### Requirement: mcp_tools 表结构

数据库 SHALL 包含名为 `mcp_tools` 的表（不再使用 `tools` 表名），用于存储 MCP 工具的元数据和状态。

#### Scenario: 创建 mcp_tools 表

- **WHEN** 应用首次初始化数据库
- **THEN** 系统 SHALL 创建 `mcp_tools` 表，包含以下字段：
  - `id` (TEXT PRIMARY KEY): 工具唯一标识
  - `name` (TEXT NOT NULL): 工具名称
  - `server_id` (TEXT NOT NULL): 所属 MCP 服务器 ID
  - `description` (TEXT): 工具描述
  - `enabled` (INTEGER NOT NULL DEFAULT 1): 启用状态
  - `created_at` (DATETIME): 创建时间
  - `updated_at` (DATETIME): 更新时间

#### Scenario: 查询 mcp_tools 表

- **WHEN** 应用查询某个 Server 的工具列表
- **THEN** 系统 SHALL 从 `mcp_tools` 表而非 `tools` 表读取数据

### Requirement: api_key_tool_relations 表结构

数据库 SHALL 包含 `api_key_tool_relations` 表，用于存储 API Key 到工具级别的授权关系。

#### Scenario: 创建 api_key_tool_relations 表

- **WHEN** 应用首次初始化数据库或执行迁移
- **THEN** 系统 SHALL 创建 `api_key_tool_relations` 表，包含以下字段：
  - `id` (TEXT PRIMARY KEY): 关系记录唯一标识
  - `api_key_id` (TEXT NOT NULL): API Key ID
  - `tool_id` (TEXT NOT NULL): 工具 ID（外键关联 `mcp_tools.id`）
  - `created_at` (DATETIME): 创建时间

#### Scenario: 建立唯一约束

- **WHEN** 创建 `api_key_tool_relations` 表
- **THEN** 系统 SHALL 在 `(api_key_id, tool_id)` 上建立唯一约束，防止重复授权

### Requirement: 数据库迁移脚本

系统 SHALL 提供从旧架构（`tools` 表 + `api_key_server_relations` 表）迁移到新架构的自动化脚本。

#### Scenario: 检测并执行迁移

- **WHEN** 应用启动时检测到数据库中存在 `tools` 表而不存在 `mcp_tools` 表
- **THEN** 系统 SHALL 自动执行迁移脚本 `002_tool_level_auth.sql`

#### Scenario: 表重命名

- **WHEN** 迁移脚本执行
- **THEN** 系统 SHALL 执行 `ALTER TABLE tools RENAME TO mcp_tools`

#### Scenario: 从 Server 权限生成 Tool 权限

- **WHEN** 迁移脚本执行且 `api_key_server_relations` 表存在数据
- **THEN** 系统 SHALL：
  1. 读取每条 `api_key_server_relations` 记录
  2. 查询该 `server_id` 下所有的工具（从 `mcp_tools` 表）
  3. 为每个工具创建一条 `api_key_tool_relations` 记录，关联同一个 `api_key_id`
  4. 确保迁移后的权限覆盖范围与迁移前一致（即：之前能访问 Server 的所有工具，迁移后仍能访问）

#### Scenario: 迁移事务性

- **WHEN** 迁移过程中任何步骤失败
- **THEN** 系统 SHALL 回滚所有已执行的迁移操作，保持数据库处于迁移前的状态

## MODIFIED Requirements

无（这是新增功能，不修改现有需求）

## REMOVED Requirements

无（暂时保留 `api_key_server_relations` 表以支持迁移和回滚）

## RENAMED Requirements

无
