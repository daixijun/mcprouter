# backend-architecture Spec Delta

## ADDED Requirements

### Requirement: 命令模块化组织

后端代码 MUST 按照功能领域将 Tauri 命令组织到独立的模块中,而不是将所有命令集中在单一文件中。命令模块应该按照以下领域划分:

- **配置管理** (`commands/config.rs`): 应用配置、主题管理等
- **MCP 服务器管理** (`commands/mcp_server.rs`): 服务器的增删改查操作
- **MCP 客户端管理** (`commands/mcp_client.rs`): 客户端连接生命周期管理
- **工具管理** (`commands/tool.rs`): 工具级别的启用/禁用操作
- **API 密钥管理** (`commands/api_key.rs`): API 密钥的 CRUD 和权限管理
- **市场服务** (`commands/marketplace.rs`): 市场服务的查询和安装
- **系统设置** (`commands/settings.rs`): 系统级设置(自动启动、托盘等)
- **仪表板数据** (`commands/dashboard.rs`): 统计数据聚合

#### Scenario: 配置管理命令的组织

- **GIVEN** 应用需要提供配置管理相关的 Tauri 命令
- **WHEN** 开发者查看后端代码结构
- **THEN** 所有配置相关的命令函数(get_config, get_theme, set_theme, update_config)应该位于 `src-tauri/src/commands/config.rs` 模块中

#### Scenario: MCP 服务器管理命令的组织

- **GIVEN** 应用需要提供 MCP 服务器管理的 Tauri 命令
- **WHEN** 开发者查看后端代码结构
- **THEN** 所有 MCP 服务器管理相关的命令函数(add_mcp_server, remove_mcp_server, toggle_mcp_server, list_mcp_servers, check_mcp_server_connectivity, delete_mcp_server)应该位于 `src-tauri/src/commands/mcp_server.rs` 模块中

#### Scenario: 新增命令时的模块选择

- **GIVEN** 开发者需要添加新的 Tauri 命令
- **WHEN** 该命令属于已有的功能领域(如 API 密钥管理)
- **THEN** 开发者应该将新命令添加到对应的模块文件中(如 `commands/api_key.rs`)
- **AND** 在 `commands/mod.rs` 中导出该命令
- **AND** 在 `lib.rs` 的 `invoke_handler` 中注册该命令

### Requirement: lib.rs 的职责边界

`src-tauri/src/lib.rs` 文件 MUST 只包含以下内容:

1. 模块声明(`mod` statements)
2. 全局状态定义(如 `SERVICE_MANAGER`, `MCP_CLIENT_MANAGER`, `AGGREGATOR`)
3. 全局类型定义(跨模块共享的类型)
4. 系统托盘初始化函数(`build_main_tray`)
5. 应用启动函数(`pub async fn run()`)

`lib.rs` 不应包含具体的 Tauri 命令函数实现。

#### Scenario: lib.rs 的内容限制

- **GIVEN** 后端代码组织遵循模块化原则
- **WHEN** 开发者查看 `lib.rs` 文件
- **THEN** 该文件不应包含任何 `#[tauri::command]` 标注的命令函数
- **AND** 该文件应该只包含全局状态、类型定义、模块声明和应用启动逻辑
- **AND** 文件代码行数应该在合理范围内(建议不超过 500 行)

#### Scenario: 全局状态的保留

- **GIVEN** 应用需要全局的单例状态管理
- **WHEN** 开发者查看 `lib.rs` 文件
- **THEN** `SERVICE_MANAGER`, `MCP_CLIENT_MANAGER`, `AGGREGATOR` 等全局状态应该定义在 `lib.rs` 中
- **AND** 这些全局状态应该声明为 `pub` 或 `pub(crate)` 以便命令模块访问

### Requirement: 命令模块的导入和导出

命令模块 MUST 通过 `commands/mod.rs` 统一导出,`lib.rs` 应该通过 `use commands::*` 导入所有命令函数并在 `invoke_handler` 宏中注册。

#### Scenario: 命令模块的统一导出

- **GIVEN** 后端有多个命令模块
- **WHEN** 开发者查看 `src-tauri/src/commands/mod.rs` 文件
- **THEN** 该文件应该声明所有的命令子模块(如 `pub mod config;`, `pub mod mcp_server;` 等)
- **AND** 该文件应该使用 `pub use` 重新导出所有命令函数,以便 `lib.rs` 统一导入

#### Scenario: lib.rs 中的命令注册

- **GIVEN** 所有命令函数已经在各自的模块中实现
- **WHEN** 开发者查看 `lib.rs` 中的 `invoke_handler` 宏
- **THEN** 所有命令函数应该通过 `commands::*` 导入并注册
- **AND** 不应存在未注册的命令函数

### Requirement: 代码整洁性

后端代码 MUST 保持整洁,不应包含未使用的代码块、函数或类型定义。禁止使用 `#[allow(dead_code)]` 等属性来抑制警告,应该直接清理未使用的代码。

#### Scenario: 清理未使用的辅助函数

- **GIVEN** 后端代码中存在未被命令函数使用的辅助函数
- **WHEN** 开发者运行 `cargo check` 或使用 IDE 诊断
- **THEN** 应该删除这些未使用的函数,而不是使用 `#[allow(dead_code)]` 抑制警告

#### Scenario: 清理未使用的类型定义

- **GIVEN** 后端代码中存在未被使用的结构体或类型定义
- **WHEN** 开发者运行 `cargo check` 或使用 IDE 诊断
- **THEN** 应该删除这些未使用的类型,或将其移动到实际使用它们的模块中

#### Scenario: 编译检查的通过

- **GIVEN** 后端代码完成重构
- **WHEN** 开发者运行 `cargo check`
- **THEN** 命令应该成功通过,不应有任何未使用代码的警告
- **AND** IDE 诊断不应显示任何错误或警告
