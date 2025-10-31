# 移除冗余命令任务清单

## 阶段 1: 移除后端命令

- [x] 分析要移除的命令使用情况
- [x] 创建 OpenSpec 提案文档
- [x] 从 src-tauri/src/commands/mcp_client.rs 移除 connect_to_mcp_server
- [x] 从 src-tauri/src/commands/mcp_client.rs 移除 disconnect_from_mcp_server
- [x] 从 src-tauri/src/commands/mcp_client.rs 移除 call_mcp_tool
- [x] 从 src-tauri/src/commands/mcp_client.rs 移除 list_mcp_server_tools
- [x] 从 src-tauri/src/commands/mcp_client.rs 移除 list_mcp_connections
- [x] 更新 src-tauri/src/lib.rs invoke_handler 移除命令注册

## 阶段 2: 更新前端代码

- [x] 从 src/services/mcp-server-service.ts 移除 connectToMcpServer
- [x] 从 src/services/mcp-server-service.ts 移除 disconnectFromMcpServer
- [x] 从 src/services/mcp-server-service.ts 移除 callMcpServerTool
- [x] 从 src/services/mcp-server-service.ts 移除 listMcpServerTools
- [x] 检查并更新使用这些方法的组件

## 阶段 3: 验证

- [x] 运行 cargo check 验证后端编译
- [x] 运行前端构建验证前端编译
- [x] 测试工具管理功能
- [x] 测试服务管理功能
- [x] 更新 OpenSpec 结果文档
