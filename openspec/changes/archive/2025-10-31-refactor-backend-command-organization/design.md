# 技术设计: 后端命令组织结构重构

## Context

MCPRouter 后端基于 Tauri 框架,使用 Rust 实现。当前所有 Tauri 命令函数都定义在 `lib.rs` 文件中,随着功能增加,该文件已经包含 40 个命令函数,代码行数接近 2000 行,严重影响了代码的可读性和可维护性。

项目约定要求:

- 代码整洁,不使用 `#[allow(dead_code)]` 等属性
- 后端变更后使用 `cargo check` 进行检查
- 如果连接了 IDE,每次任务结束前检查并修复 ISSUE
- 优先选择简单、直接的实现

## Goals / Non-Goals

### Goals

- 按照功能领域将 Tauri 命令组织到独立模块中
- 保持 `lib.rs` 简洁,只包含核心初始化逻辑和全局状态
- 确保所有命令在模块间清晰分类,易于查找和维护
- 清理未使用的代码和类型定义
- 保持与前端的 API 兼容性

### Non-Goals

- 不修改命令函数的签名或行为
- 不改变前端调用方式
- 不引入新的依赖或框架
- 不优化命令的实现逻辑(仅关注组织结构)

## Decisions

### 1. 命令模块划分策略

**决策**: 按照业务领域将命令划分为 8 个模块

**理由**:

- **config.rs**: 配置管理是独立的关注点,包括主题、应用配置等
- **mcp_server.rs**: MCP 服务器的 CRUD 操作是核心功能模块
- **mcp_client.rs**: 客户端连接管理与服务器管理是不同的生命周期
- **tool.rs**: 工具级别的管理有独立的数据库表和业务逻辑
- **api_key.rs**: API 密钥管理涉及安全认证,应独立模块
- **marketplace.rs**: 市场服务是独立的功能域
- **settings.rs**: 系统设置(自动启动、托盘等)与应用配置不同
- **dashboard.rs**: 仪表板数据聚合是独立的查询逻辑

**备选方案**:

- 方案 A: 按照前端页面划分(dashboard, servers, marketplace, settings, api-keys)
  - 缺点: 会导致 mcp_server 和 mcp_client 混在一起,不符合后端的生命周期管理
- 方案 B: 只分为 3-4 个大模块(server, client, admin)
  - 缺点: 模块过大,仍然会有数百行代码,不够细粒度

### 2. 全局状态保留在 lib.rs

**决策**: 保留 `SERVICE_MANAGER`、`MCP_CLIENT_MANAGER`、`AGGREGATOR` 等全局状态在 `lib.rs` 中

**理由**:

- 这些是应用级别的单例,需要在启动时初始化
- 各个命令模块都需要访问这些全局状态
- Rust 的 `LazyLock` 需要在 crate 根级别定义

### 3. 类型定义处理

**决策**: 创建专门的 `types.rs` 模块,统一管理所有共享类型定义

**理由**:

- 这些类型被多个模块使用,应该有统一的定义位置
- 将类型定义从 `lib.rs`、`config.rs`、`mcp_manager.rs`、`mcp_client.rs` 等模块中分离出来
- 通过 `pub use types::*` 重新导出,保持向后兼容性
- 符合"高内聚、低耦合"的设计原则,类型定义与业务逻辑分离

**实施结果**:

**第一阶段** - 从 lib.rs 迁移 (已完成):

- `McpTool` type alias
- `MarketplaceService` 及相关结构体(`InstallCommand`, `EnvSchema`, `EnvProperty`)
- `MarketplaceServiceListItem`
- 从 lib.rs 中移除了约 80 行类型定义代码

**第二阶段** - 从 config.rs 迁移:

- `McpServerConfig` - MCP 服务器配置
- `ServiceTransport` - 传输类型枚举
- `AppConfig` - 应用配置
- `LoggingSettings` - 日志设置
- `SystemTraySettings` - 系统托盘设置
- `Settings` - 应用设置
- `ApiKeyPermissions` - API 密钥权限
- `ApiKey` - API 密钥
- `SecuritySettings` - 安全设置
- `ServerConfig` - 服务器配置
- config.rs 保留配置管理方法 (`load()`, `save()`)

**第三阶段** - 从 mcp_manager.rs 迁移:

- `ServiceStatus` - 服务状态
- `McpServerInfo` - 服务器信息
- `ServiceVersionCache` - 版本缓存
- 保留 `McpServerManager` 类及其所有方法

**第四阶段** - 从 mcp_client.rs 迁移:

- `McpService` - MCP 服务枚举
- `ConnectionStatus` - 连接状态
- `McpConnection` - 连接信息
- 保留 `McpClientManager` 类及其所有方法

**第五阶段** - 清理未使用代码:

- 删除 `state_manager.rs` (856 行) - 未被使用的统一状态管理器
- 删除 `task_manager.rs` (476 行) - 未被使用的任务管理器

**最终结果**:

- types.rs: 从 87 行扩展到约 350 行
- config.rs: 从 284 行减少到约 150 行 (47.2% 减少)
- mcp_manager.rs: 从 826 行减少到约 790 行 (4.4% 减少)
- mcp_client.rs: 从 666 行减少到约 640 行 (3.9% 减少)
- 删除未使用代码: 1332 行

### 4. mod.rs 的角色

**决策**: 创建 `commands/mod.rs` 统一导出所有命令函数

**理由**:

- 简化 `lib.rs` 中的导入语句
- 提供清晰的模块边界
- 便于后续添加新命令模块

## Implementation Plan

### 阶段 1: 创建命令模块结构

1. 创建 `src-tauri/src/commands/` 目录
2. 创建各个模块文件: `config.rs`, `mcp_server.rs`, `mcp_client.rs`, `tool.rs`, `api_key.rs`, `marketplace.rs`, `settings.rs`, `dashboard.rs`
3. 创建 `commands/mod.rs` 进行模块导出

### 阶段 2: 迁移命令函数

1. 将命令函数从 `lib.rs` 复制到对应的模块文件
2. 调整导入语句,确保每个命令函数可以访问需要的类型和全局状态
3. 在 `commands/mod.rs` 中导出所有命令函数

### 阶段 3: 迁移类型定义

1. 创建 `src-tauri/src/types.rs` 模块
2. 将类型定义从 `lib.rs` 迁移到 `types.rs`
3. 在 `lib.rs` 中添加 `mod types;` 声明并通过 `pub use types::*` 重新导出
4. 更新使用这些类型的模块的导入语句(marketplace, commands/marketplace)

### 阶段 4: 更新 lib.rs

1. 在 `lib.rs` 中添加 `mod commands;` 声明
2. 更新 `invoke_handler` 宏,使用 `commands::*` 导入所有命令
3. 清理 `lib.rs` 中已迁移的命令函数
4. 清理未使用的辅助函数(如 `get_allowed_servers_from_tools`)
5. 清理未使用的导入(如 `HashMap`)

### 阶段 5: 验证和测试

1. 运行 `cargo check` 确保编译通过
2. 使用 IDE 诊断检查是否有未使用的代码或错误
3. (可选)启动应用测试前端功能

## Risks / Trade-offs

### 风险 1: 导入路径复杂度增加

- **风险**: 命令函数需要导入更多的类型和全局状态
- **缓解**: 在各个模块顶部统一导入常用类型,使用 `use crate::*` 简化

### 风险 2: 重构过程中可能遗漏命令

- **风险**: 40 个命令函数,可能在迁移时遗漏某些函数
- **缓解**: 使用 `grep` 工具确认所有 `#[tauri::command]` 都被迁移,使用编译器检查 `invoke_handler` 的完整性

### 风险 3: 全局状态访问的作用域问题

- **风险**: 命令模块可能无法访问 `lib.rs` 中的全局状态
- **缓解**: 将全局状态声明为 `pub(crate)` 或 `pub`,确保模块可见性

### 权衡: 模块数量 vs 模块大小

- **权衡**: 8 个模块可能显得过多,但每个模块会更加聚焦
- **决策**: 优先选择细粒度模块,便于后续扩展和维护

## Migration Plan

### 步骤 1: 准备工作(不影响现有功能)

- 创建 `commands/` 目录和模块文件
- 编写模块骨架代码

### 步骤 2: 逐模块迁移

- 按模块优先级迁移: config → mcp_server → mcp_client → tool → api_key → marketplace → settings → dashboard
- 每迁移一个模块,立即运行 `cargo check` 验证

### 步骤 3: 清理

- 删除 `lib.rs` 中已迁移的命令函数
- 清理未使用的辅助函数和 import
- 运行最终的 `cargo check` 和 IDE 诊断

### 回滚计划

- 如果重构失败,可以简单地恢复 `lib.rs` 文件
- 由于不涉及 API 变更,前端无需修改

## Open Questions

无
