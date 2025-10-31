# 移除冗余和未使用的后端命令

## Why

当前后端存在多个冗余或未使用的 Tauri 命令：

1. **未使用的 MCP 客户端命令**：前端没有实际使用这些命令，因为连接管理已经自动化

   - `connect_to_mcp_server`
   - `disconnect_from_mcp_server`
   - `call_mcp_tool`
   - `list_mcp_connections`

2. **功能重叠的工具查询命令**：
   - `list_mcp_server_tools` (mcp_client.rs) - 通过 connection_id 查询
   - `get_tools_by_server` (tool.rs) - 通过 server_id 或 name 查询，功能更完善

这些冗余命令增加了维护成本，且容易造成混淆。

## What Changes

### 移除未使用的命令

从后端移除以下 4 个命令（`src-tauri/src/commands/mcp_client.rs`）：

- `connect_to_mcp_server` - 连接由 `ensure_connection` 自动管理
- `disconnect_from_mcp_server` - 连接由系统自动管理
- `call_mcp_tool` - 工具调用通过聚合器 HTTP API 处理
- `list_mcp_connections` - 连接状态在服务列表中体现

### 移除重复的工具查询命令

保留 `get_tools_by_server`（功能更完善），移除 `list_mcp_server_tools`：

- `list_mcp_server_tools` - **移除**（功能较弱，只支持 connection_id）
- `get_tools_by_server` - **保留**（支持 server_id 和 name 两种方式查询）

### 更新前端代码

从 `src/services/mcp-server-service.ts` 移除未使用的方法：

- `connectToMcpServer()`
- `disconnectFromMcpServer()`
- `callMcpServerTool()`
- `listMcpServerTools()` - 改用 `get_tools_by_server` 后端命令

### 更新命令注册

从 `src-tauri/src/lib.rs` 的 `invoke_handler` 中移除这 5 个命令的注册。

## Impact

- **受影响的规范**: 无（纯清理工作）
- **受影响的代码**:
  - `src-tauri/src/commands/mcp_client.rs` - 移除约 110 行未使用代码
  - `src-tauri/src/lib.rs` - 从 invoke_handler 移除 5 个命令注册
  - `src/services/mcp-server-service.ts` - 移除 4 个未使用的方法（约 30 行）
  - `src/components/ToolManager.tsx` - 可能需要更新 API 调用方式
- **破坏性变更**: 无，前端实际未使用这些方法
- **风险**: 低，这些命令前端未实际调用
- **测试需求**:
  - 编译检查: 使用 `cargo check` 验证后端编译通过
  - 前端编译: 使用 `npm run build` 验证前端编译通过
  - 功能测试: 验证工具管理功能仍然正常工作

## Results

重构完成后的效果：

- **代码量**: 减少约 140 行冗余代码
- **命令数**: 从 41 个命令减少到 36 个命令
- **可维护性**: 清晰的命令职责，无重复功能
- **代码质量**: 无未使用代码，无功能重叠
- **向后兼容**: 100% 兼容，移除的命令未被使用
