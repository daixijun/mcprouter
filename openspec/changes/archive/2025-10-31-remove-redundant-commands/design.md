# 技术设计: 移除冗余命令

## Context

MCPRouter 后端在之前的开发过程中积累了一些冗余或未使用的 Tauri 命令。经过分析发现：

1. **MCP 客户端管理命令**（connect_to_mcp_server 等）虽然存在，但前端实际没有使用
2. **工具查询命令**存在功能重叠：list_mcp_server_tools 和 get_tools_by_server
3. 这些冗余代码增加了维护成本，容易造成 API 混淆

## Goals / Non-Goals

### Goals

- 移除未使用的 MCP 客户端管理命令
- 合并功能重叠的工具查询命令
- 保持 API 简洁和清晰
- 减少代码维护负担

### Non-Goals

- 不修改核心业务逻辑
- 不移除正在使用的命令
- 不改变现有的工具管理功能

## Decisions

### 1. 移除未使用的客户端命令

**决策**: 删除以下 4 个命令

- `connect_to_mcp_server` - 连接由 MCP_CLIENT_MANAGER 自动管理
- `disconnect_from_mcp_server` - 前端没有手动断开的需求
- `call_mcp_tool` - 工具调用通过聚合器的 HTTP API 处理
- `list_mcp_connections` - 连接状态在服务列表中体现

**理由**:

- 这些命令在前端代码中没有被实际调用
- 现代架构中，连接管理应该自动化，不应暴露给前端
- 工具调用通过统一的 HTTP 接口，更符合 RESTful 设计

### 2. 保留功能完善的工具查询命令

**决策**: 保留 `get_tools_by_server`，移除 `list_mcp_server_tools`

**理由**:

- `get_tools_by_server` 支持通过 server_id 和 name 两种方式查询
- `list_mcp_server_tools` 只支持 connection_id，功能较弱
- 统一使用一个接口，避免 API 冗余

### 3. 前端服务层清理

**决策**: 从 mcp-server-service.ts 移除对应的前端方法

**理由**:

- 保持前后端 API 的一致性
- 移除未使用的前端代码
- 避免误导开发者的 API 文档

## Implementation Plan

### 阶段 1: 移除后端命令

1. 删除 `src-tauri/src/commands/mcp_client.rs` 中的 5 个命令函数
2. 更新 `src-tauri/src/lib.rs` 中的 invoke_handler，移除这些命令注册

### 阶段 2: 更新前端代码

1. 从 `src/services/mcp-server-service.ts` 移除对应的方法
2. 检查是否有组件使用这些方法并更新

### 阶段 3: 验证

1. 运行 `cargo check` 验证后端编译
2. 运行前端构建验证前端编译
3. 测试核心功能（工具管理、服务管理）正常工作

## Implementation Results

**实施完成时间**: 2025-10-30

### 删除的命令

**已移除的 5 个 Tauri 命令**:

1. `connect_to_mcp_server` - 由 `McpClientManager.ensure_connection` 自动管理
2. `disconnect_from_mcp_server` - 连接由系统自动管理
3. `list_mcp_server_tools` - 功能重叠，被 `get_tools_by_server` 替代
4. `call_mcp_tool` - 工具调用通过聚合器 HTTP API 处理
5. `list_mcp_connections` - 连接状态在服务列表中体现

### 保留的核心组件

**保留 `McpClientManager` 类**:

- 核心连接管理功能仍然需要（用于聚合器）
- `connect_mcp_server` 方法用于内部连接建立
- `call_tool` 方法用于聚合器的工具调用
- `ensure_connection` 方法用于自动连接管理

### 更新的文件

**后端文件**:

- `src-tauri/src/commands/mcp_client.rs` - 重写为只包含 McpClientManager 类（移除 118 行命令函数）
- `src-tauri/src/commands/mod.rs` - 移除 mcp_client 模块导入
- `src-tauri/src/lib.rs` - 从 invoke_handler 移除 5 个命令注册

**前端文件**:

- `src/services/mcp-server-service.ts` - 移除 4 个未使用的方法（约 40 行）
- 更新 TypeScript 类型导入，移除未使用的类型

### 最终结果

- **代码减少**: 约 158 行冗余代码
- **命令数量**: 从 41 个减少到 36 个命令
- **API 简化**: 清晰的命令职责，无功能重叠
- **向后兼容**: 100% 兼容，移除的命令未被前端使用
- **编译状态**: ✅ 后端编译通过，只有少量未使用方法警告
