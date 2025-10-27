## Why

当前的授权系统仅支持 API Key 对 Server 级别的访问控制，无法实现对单个工具的精细化权限管理。为了提供更灵活的权限控制，需要将授权粒度从 Server 级别降低到 Tool 级别，让管理员能够精确控制每个 API Key 可以访问哪些具体的工具。

同时，tools 表名不符合项目命名规范（其他表都使用 `mcp_` 前缀），需要重命名为 `mcp_tools` 以保持一致性。

## What Changes

- **数据库架构变更**：

  - 将 `tools` 表重命名为 `mcp_tools`
  - 创建新表 `api_key_tool_relations` 用于存储 API Key 到 Tool 的授权关系
  - 创建数据迁移脚本，从现有的 `api_key_server_relations` 自动生成工具级别的授权记录

- **授权逻辑重构**：

  - 修改授权检查逻辑，从检查 API Key → Server 权限改为检查 API Key → Tool 权限
  - 更新相关的 Repository 接口以支持工具级别的授权管理
  - 添加批量授权接口（例如：授权某个 API Key 访问某个 Server 的所有工具）

- **向后兼容性**：
  - **BREAKING**: `api_key_server_relations` 表将被弃用，但在迁移阶段保留以支持数据转换
  - 现有用户在升级时会自动执行迁移脚本，将 Server 级别的授权扩展为该 Server 下所有工具的授权

## Impact

- **受影响的规范**：

  - `database`: 表结构变更（表重命名、新增关系表）
  - `authorization`: 授权检查逻辑从 Server 级别改为 Tool 级别

- **受影响的代码**：

  - `src-tauri/src/db/connection.rs`: 表创建逻辑
  - `src-tauri/src/db/models.rs`: 数据模型定义
  - `src-tauri/src/db/repositories/tool_repository.rs`: 表名从 `tools` 改为 `mcp_tools`
  - `src-tauri/src/db/repositories/api_key_server_repository.rs`: 需要重构为 `api_key_tool_repository.rs`
  - `src-tauri/src/migrations/`: 添加新的迁移脚本
  - 所有调用授权检查的业务逻辑代码

- **用户影响**：
  - 升级时会自动执行数据迁移，现有的 Server 级别权限会转换为该 Server 下所有工具的权限
  - UI 需要更新以支持工具级别的权限配置
  - API 响应格式可能需要调整以返回工具级别的权限信息
