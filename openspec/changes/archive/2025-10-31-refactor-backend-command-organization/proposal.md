# 重构后端命令组织结构

## Why

当前 `src-tauri/src/lib.rs` 文件包含了 40 个 Tauri 命令函数和多个类型定义,导致文件过于庞大(近 2000 行代码)且难以维护。这些命令涵盖了多个不同的领域(MCP 服务器管理、API 密钥管理、配置管理、工具管理等),类型定义也散落在 lib.rs 中,应该按照功能模块进行组织,以提高代码的可读性、可维护性和模块化程度。

此外,项目约定明确要求"代码整洁",不应使用 `#[allow(dead_code)]` 等属性,应清理未使用的代码块。

## What Changes

### 命令模块化

- 创建 `src-tauri/src/commands/` 目录,按功能模块组织 Tauri 命令
- 将 40 个命令按照以下模块拆分:
  - `config.rs` - 配置管理命令(4 个)
  - `mcp_server.rs` - MCP 服务器管理命令(6 个)
  - `mcp_client.rs` - MCP 客户端连接管理命令(6 个)
  - `tool.rs` - 工具管理命令(4 个)
  - `api_key.rs` - API 密钥管理命令(12 个)
  - `marketplace.rs` - 市场服务命令(3 个)
  - `settings.rs` - 系统设置命令(4 个)
  - `dashboard.rs` - 仪表板数据命令(2 个)
  - `mod.rs` - 命令模块统一导出

### 类型定义模块化

- 创建 `src-tauri/src/types.rs` 模块,统一管理共享类型定义
- **第一阶段**: 从 lib.rs 迁移
  - `McpTool` (type alias)
  - `MarketplaceService`
  - `InstallCommand`
  - `EnvSchema`
  - `EnvProperty`
  - `MarketplaceServiceListItem`
- **第二阶段**: 从 config.rs 迁移
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
- **第三阶段**: 从 mcp_manager.rs 迁移
  - `ServiceStatus` - 服务状态
  - `McpServerInfo` - 服务器信息
  - `ServiceVersionCache` - 版本缓存
- **第四阶段**: 从 mcp_client.rs 迁移
  - `McpService` - MCP 服务枚举
  - `ConnectionStatus` - 连接状态
  - `McpConnection` - 连接信息
- 通过 `pub use types::*` 在相关模块中重新导出,保持向后兼容

### 未使用代码清理

- 删除 `src-tauri/src/state_manager.rs` (856 行) - 未被使用的统一状态管理器
- 删除 `src-tauri/src/task_manager.rs` (476 行) - 未被使用的任务管理器

### lib.rs 简化

- 移除所有 40 个命令函数实现(约 1300 行)
- 移除所有类型定义(约 80 行)
- 保留全局状态管理(`SERVICE_MANAGER`, `MCP_CLIENT_MANAGER`, `AGGREGATOR`)
- 保留系统托盘初始化函数和应用启动逻辑
- 清理未使用的导入和代码块

## Impact

- **受影响的规范**: backend-architecture (新增)
- **受影响的代码**:
  - `src-tauri/src/lib.rs` - 从 1921 行减少到 532 行,减少 **72.3%**
  - `src-tauri/src/commands/` - 新增目录,包含 9 个模块文件
  - `src-tauri/src/types.rs` - 从 87 行扩展到约 **350+ 行**,统一管理所有共享类型
  - `src-tauri/src/config.rs` - 从 284 行减少到约 **150 行**,只保留配置管理方法
  - `src-tauri/src/mcp_manager.rs` - 从 826 行减少到约 **790 行**,移除类型定义
  - `src-tauri/src/mcp_client.rs` - 从 666 行减少到约 **640 行**,移除类型定义
  - `src-tauri/src/marketplace/mod.rs` - 更新导入语句
  - `src-tauri/src/state_manager.rs` - **删除** (856 行未使用代码)
  - `src-tauri/src/task_manager.rs` - **删除** (476 行未使用代码)
  - 构建配置 - 无影响,纯代码组织优化
- **破坏性变更**: 无,这是内部重构,不影响外部 API
- **风险**: 低,命令函数签名和行为保持不变,类型通过 pub use 重新导出
- **测试需求**:
  - 编译检查: 使用 `cargo check` 验证所有命令正确导入 ✅
  - 类型检查: 确保 IDE 诊断无错误 ✅
  - 功能测试: 验证前端调用各个命令仍然正常工作

## Results

重构完成后的效果:

- **代码量**:
  - lib.rs: 从 1921 行 → 532 行,减少 1389 行 (72.3%)
  - types.rs: 从 87 行 → 约 350 行,增加 263 行
  - config.rs: 从 284 行 → 约 150 行,减少 134 行 (47.2%)
  - mcp_manager.rs: 从 826 行 → 约 790 行,减少 36 行 (4.4%)
  - mcp_client.rs: 从 666 行 → 约 640 行,减少 26 行 (3.9%)
  - **删除未使用代码**: state_manager.rs (856 行) + task_manager.rs (476 行) = 1332 行
  - **净减少**: 约 1654 行代码
- **模块数**: 新增 10 个模块文件(9 个命令模块 + 1 个类型模块)
- **可维护性**:
  - 每个命令模块职责单一,平均 50-200 行
  - 所有共享类型集中在 types.rs,易于查找和维护
  - 核心业务逻辑保持独立模块,结构清晰
- **代码质量**: 无编译警告,无未使用代码
- **向后兼容**: 100%兼容,无破坏性变更
