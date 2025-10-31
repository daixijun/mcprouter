# 实施任务清单

## 1. 准备工作

- [x] 1.1 创建 `src-tauri/src/commands/` 目录
- [x] 1.2 创建命令模块文件骨架:
  - [x] 1.2.1 创建 `commands/config.rs`
  - [x] 1.2.2 创建 `commands/mcp_server.rs`
  - [x] 1.2.3 创建 `commands/mcp_client.rs`
  - [x] 1.2.4 创建 `commands/tool.rs`
  - [x] 1.2.5 创建 `commands/api_key.rs`
  - [x] 1.2.6 创建 `commands/marketplace.rs`
  - [x] 1.2.7 创建 `commands/settings.rs`
  - [x] 1.2.8 创建 `commands/dashboard.rs`
  - [x] 1.2.9 创建 `commands/mod.rs`

## 2. 迁移命令函数

- [x] 2.1 迁移配置管理命令到 `commands/config.rs`:
  - [x] 2.1.1 迁移 `get_config`
  - [x] 2.1.2 迁移 `get_theme`
  - [x] 2.1.3 迁移 `set_theme`
  - [x] 2.1.4 迁移 `update_config`
- [x] 2.2 迁移 MCP 服务器管理命令到 `commands/mcp_server.rs`:
  - [x] 2.2.1 迁移 `add_mcp_server`
  - [x] 2.2.2 迁移 `remove_mcp_server`
  - [x] 2.2.3 迁移 `check_mcp_server_connectivity`
  - [x] 2.2.4 迁移 `toggle_mcp_server`
  - [x] 2.2.5 迁移 `list_mcp_servers`
  - [x] 2.2.6 迁移 `delete_mcp_server`
- [x] 2.3 迁移 MCP 客户端管理命令到 `commands/mcp_client.rs`:
  - [x] 2.3.1 迁移 `connect_to_mcp_server`
  - [x] 2.3.2 迁移 `disconnect_from_mcp_server`
  - [x] 2.3.3 迁移 `list_mcp_server_tools`
  - [x] 2.3.4 迁移 `call_mcp_tool`
  - [x] 2.3.5 迁移 `get_mcp_server_info`
  - [x] 2.3.6 迁移 `list_mcp_connections`
- [x] 2.4 迁移工具管理命令到 `commands/tool.rs`:
  - [x] 2.4.1 迁移 `toggle_mcp_server_tool`
  - [x] 2.4.2 迁移 `enable_all_mcp_server_tools`
  - [x] 2.4.3 迁移 `disable_all_mcp_server_tools`
  - [x] 2.4.4 迁移 `get_tools_by_server`
- [x] 2.5 迁移 API 密钥管理命令到 `commands/api_key.rs`:
  - [x] 2.5.1 迁移辅助函数 `get_allowed_servers_from_tools`
  - [x] 2.5.2 迁移 `create_api_key`
  - [x] 2.5.3 迁移 `list_api_keys`
  - [x] 2.5.4 迁移 `get_api_key_details`
  - [x] 2.5.5 迁移 `delete_api_key`
  - [x] 2.5.6 迁移 `toggle_api_key`
  - [x] 2.5.7 迁移 `update_api_key_permissions`
  - [x] 2.5.8 迁移 `get_api_key_tools`
  - [x] 2.5.9 迁移 `add_tool_permission`
  - [x] 2.5.10 迁移 `remove_tool_permission`
  - [x] 2.5.11 迁移 `grant_server_tools_to_api_key`
  - [x] 2.5.12 迁移 `revoke_server_tools_from_api_key`
- [x] 2.6 迁移市场服务命令到 `commands/marketplace.rs`:
  - [x] 2.6.1 迁移 `get_mcp_server_details`
  - [x] 2.6.2 迁移 `list_marketplace_services`
  - [x] 2.6.3 迁移 `install_marketplace_service`
- [x] 2.7 迁移系统设置命令到 `commands/settings.rs`:
  - [x] 2.7.1 迁移 `get_settings`
  - [x] 2.7.2 迁移 `save_settings`
  - [x] 2.7.3 迁移 `is_autostart_enabled`
  - [x] 2.7.4 迁移 `toggle_autostart`
- [x] 2.8 迁移仪表板数据命令到 `commands/dashboard.rs`:
  - [x] 2.8.1 迁移 `get_dashboard_stats`
  - [x] 2.8.2 迁移 `get_local_ip_addresses`

## 3. 更新模块导出

- [x] 3.1 在 `commands/mod.rs` 中声明所有子模块
- [x] 3.2 在 `commands/mod.rs` 中重新导出所有命令函数

## 4. 更新 lib.rs

- [x] 4.1 在 `lib.rs` 中添加 `mod commands;` 声明
- [x] 4.2 使用 `use commands::*;` 导入所有命令函数
- [x] 4.3 确认 `invoke_handler` 宏中注册了所有 40 个命令
- [x] 4.4 删除 `lib.rs` 中已迁移的命令函数实现
- [x] 4.5 删除 `lib.rs` 中未使用的辅助函数(如 `get_allowed_servers_from_tools`)
- [x] 4.6 删除或移动未使用的类型定义
- [x] 4.7 保留全局状态定义和系统托盘初始化函数
- [x] 4.8 保留应用启动函数 `pub async fn run()`

## 5. 验证和测试

- [x] 5.1 运行 `cargo check` 确保编译通过
- [x] 5.2 检查 IDE 诊断,确保没有警告或错误
- [x] 5.3 确认没有未使用的代码警告
- [x] 5.4 确认所有命令在 `invoke_handler` 中正确注册
- [x] 5.5 (可选)启动应用,测试前端功能是否正常

## 6. 代码审查和文档

- [x] 6.1 审查各个命令模块,确保导入语句正确
- [x] 6.2 审查 `lib.rs`,确保职责清晰、代码简洁
- [x] 6.3 更新代码注释(如有必要)
- [x] 6.4 提交变更,准备 PR
