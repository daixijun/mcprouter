## 1. 数据库架构变更

- [x] 1.1 创建新的迁移脚本 `002_tool_level_auth.sql`
- [x] 1.2 实现表重命名逻辑 `tools` → `mcp_tools`
- [x] 1.3 创建 `api_key_tool_relations` 表
- [x] 1.4 实现数据迁移逻辑：从 `api_key_server_relations` 生成工具级授权记录
- [x] 1.5 更新 `connection.rs` 中的表创建逻辑

## 2. 数据模型更新

- [x] 2.1 在 `models.rs` 中添加 `ApiKeyToolRelationRow` 模型
- [x] 2.2 确保所有模型引用使用新的表名 `mcp_tools`

## 3. Repository 层重构

- [x] 3.1 更新 `tool_repository.rs` 中所有 SQL 查询，使用新表名 `mcp_tools`
- [x] 3.2 创建 `api_key_tool_repository.rs` 替代 `api_key_server_repository.rs`
- [x] 3.3 实现工具级别的权限检查方法 `has_tool_permission(api_key_id, tool_id)`
- [x] 3.4 实现批量授权方法 `grant_server_tools(api_key_id, server_id)` - 授权某个 Server 的所有工具
- [x] 3.5 实现获取 API Key 授权的工具列表 `get_tools_by_api_key(api_key_id)`

## 4. 业务逻辑更新

- [x] 4.1 查找所有调用 `ApiKeyServerRepository::has_permission` 的地方
- [x] 4.2 将授权检查改为基于 Tool ID 而非 Server ID
- [x] 4.3 更新 API Key 管理相关的 Tauri 命令
- [x] 4.4 确保 MCP 请求处理时正确检查工具级别权限

## 5. 测试与验证

- [x] 5.1 测试数据迁移脚本：从现有数据库升级到新架构
- [x] 5.2 测试工具级别的授权检查是否正常工作
- [x] 5.3 测试批量授权功能
- [x] 5.4 验证 API Key 无权限访问的工具会被正确拒绝
- [x] 5.5 测试向后兼容性：升级后的用户权限应与升级前一致

## 6. 文档更新

- [x] 6.1 更新 README.md 中的数据库架构说明
- [x] 6.2 更新 API 文档（如果有）
- [x] 6.3 添加迁移指南说明（changelog 或 migration notes）
