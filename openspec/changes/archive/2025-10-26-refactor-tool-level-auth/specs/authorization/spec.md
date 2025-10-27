## ADDED Requirements

### Requirement: 工具级别的权限检查

系统 SHALL 支持基于工具（Tool）级别的权限检查，而非仅检查 Server 级别权限。

#### Scenario: 检查 API Key 是否有权限访问特定工具

- **WHEN** 收到一个 MCP 工具调用请求，包含 API Key 和工具名称
- **THEN** 系统 SHALL：
  1. 根据工具名称和 Server ID 查询 `mcp_tools` 表获取 `tool_id`
  2. 在 `api_key_tool_relations` 表中检查是否存在 `(api_key_id, tool_id)` 的授权记录
  3. 如果存在且该工具的 `enabled` 状态为 true，则允许访问
  4. 否则返回权限拒绝错误

#### Scenario: 未授权的工具访问被拒绝

- **WHEN** API Key 尝试调用未授权的工具
- **THEN** 系统 SHALL 返回 HTTP 403 错误，并在响应中说明"API Key 无权限访问该工具"

### Requirement: 批量授权接口

系统 SHALL 提供批量授权接口，简化权限管理操作。

#### Scenario: 授权 API Key 访问某个 Server 的所有工具

- **WHEN** 管理员调用 `grant_server_tools(api_key_id, server_id)` 接口
- **THEN** 系统 SHALL：
  1. 查询 `mcp_tools` 表获取该 `server_id` 下的所有工具
  2. 为每个工具创建一条 `api_key_tool_relations` 记录（如果不存在）
  3. 返回成功授权的工具数量

#### Scenario: 批量撤销授权

- **WHEN** 管理员调用 `revoke_server_tools(api_key_id, server_id)` 接口
- **THEN** 系统 SHALL：
  1. 查询 `mcp_tools` 表获取该 `server_id` 下的所有工具 ID
  2. 删除所有匹配的 `api_key_tool_relations` 记录
  3. 返回撤销的授权数量

### Requirement: 获取 API Key 的授权工具列表

系统 SHALL 提供接口查询某个 API Key 被授权访问的所有工具。

#### Scenario: 查询 API Key 的工具权限列表

- **WHEN** 调用 `get_tools_by_api_key(api_key_id)` 接口
- **THEN** 系统 SHALL：
  1. 从 `api_key_tool_relations` 表查询该 API Key 的所有授权记录
  2. 关联 `mcp_tools` 表获取工具的详细信息（名称、描述、Server ID 等）
  3. 返回工具列表，按 Server 分组

## MODIFIED Requirements

### Requirement: API Key 授权模型从 Server 级别改为 Tool 级别

API Key 的授权 SHALL 通过 `api_key_tool_relations` 表管理，授权粒度为 Tool 级别（即：授权后仅能访问指定的工具），而不再使用 Server 级别的 `api_key_server_relations` 表。

#### Scenario: 权限检查使用新的授权表

- **WHEN** 系统执行权限检查
- **THEN** 系统 SHALL 查询 `api_key_tool_relations` 表而非 `api_key_server_relations` 表

#### Scenario: 升级后的权限一致性

- **WHEN** 用户从旧版本升级到支持工具级授权的版本
- **THEN** 系统 SHALL 确保升级前后的权限等价：
  - 升级前：API Key 能访问 Server A 的所有工具
  - 升级后：API Key 能访问 Server A 的所有工具（通过迁移脚本自动生成工具级授权）

## REMOVED Requirements

无（`api_key_server_relations` 表暂时保留，计划在后续版本中移除）

## RENAMED Requirements

无
