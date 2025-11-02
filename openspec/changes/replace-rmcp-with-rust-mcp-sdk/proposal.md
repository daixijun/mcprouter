# 变更提案：将 rmcp 替换为 rust-mcp-sdk

## Why

当前的 rmcp 库在调用 streamablehttp/sse MCP server 时无法正确支持 Header 信息，具体表现为：

1. **SSE 传输**：SseClientTransport 不支持自定义 headers，导致无法传递认证信息
2. **HTTP 传输**：StreamableHttpClientTransport 对 headers 的支持有限，主要只支持 Authorization header
3. **功能限制**：现有的 HttpTransportConfig 只能设置 Authorization header，其他自定义 headers 会被忽略并记录警告

这导致了一些 MCP 服务器需要传递自定义 headers（如 API Key、Content-Type 等）时无法正常工作，影响了系统的兼容性和功能完整性。

rust-mcp-sdk 是一个更现代、更完整的 MCP SDK 实现，支持完整的 header 传递机制，能够解决上述问题。

## 什么发生了变化

### 核心变化

- **替换依赖**：将 `rmcp = "0.8.1"` 替换为 `rust-mcp-sdk` 相关依赖
- **客户端更新**：重写 `mcp_client.rs` 中的所有传输方式实现（STDIO、SSE、HTTP）
- **HTTP 客户端重构**：评估并重构 `http_client.rs`，如果 rust-mcp-sdk 已内置完整 header 支持，则可简化或移除该模块
- **聚合接口更新**：更新 `aggregator.rs` 中的服务器实现以使用新的 SDK
- **类型适配**：更新 `types.rs` 中相关的类型定义

### 影响的功能模块

- **聚合接口（server）**：`aggregator.rs` 中的 MCP 聚合服务器功能
- **MCP 服务管理（client）**：`mcp_client.rs` 中的客户端连接管理
- **HTTP 客户端工具**：`http_client.rs` 中的自定义 HTTP 传输配置

### 兼容性保证

- 保持所有现有 API 的兼容性
- 保持数据库模式不变
- 保持前端接口不变
- 保持配置格式不变

## 影响范围

### 受影响规格

- **backend-architecture**：后端架构需要适配新的 SDK
- **ui-service-management**：UI 服务管理功能可能需要更新（如果涉及传输层）

### 受影响代码文件

- `src-tauri/Cargo.toml`：依赖更新
- `src-tauri/src/mcp_client.rs`：客户端实现重写
- `src-tauri/src/http_client.rs`：评估是否需要，如 rust-mcp-sdk 已内置完整 header 支持则移除
- `src-tauri/src/aggregator.rs`：聚合服务器实现更新
- `src-tauri/src/types.rs`：类型定义更新（如需要）

### 测试需求

- 所有传输方式（STDIO、SSE、HTTP）的连接测试
- 自定义 headers 传递测试
- API Key 认证测试
- 聚合接口功能测试
- 工具列表和调用测试

## 风险评估

### 低风险

- STDIO 传输方式：实现相对稳定，变化较小
- 基本功能保持不变：工具列表、调用等核心功能保持 API 兼容

### 中等风险

- SSE 传输：API 变化可能较大，需要仔细测试
- HTTP 传输：headers 支持的变化需要验证

### 缓解措施

- 渐进式替换：先实现新 SDK 的支持，再移除旧代码
- 完整测试覆盖：包括所有传输方式和认证方式
- 回滚计划：如果出现问题可以快速回滚到 rmcp 版本
- 并行验证：在开发过程中同时测试新旧实现的兼容性

## 实施策略

### 阶段 1：准备工作

1. 研究 rust-mcp-sdk 的 API 和特性
2. 创建兼容性适配层
3. 设置测试环境

### 阶段 2：逐步替换

1. 更新依赖和 Cargo.toml
2. 重写 HTTP 客户端配置（支持完整 headers）
3. 更新客户端传输实现
4. 更新聚合服务器实现
5. 更新类型定义

### 阶段 3：测试验证

1. 单元测试
2. 集成测试
3. 端到端测试
4. 性能测试

### 阶段 4：部署

1. 代码审查
2. 发布准备
3. 监控和回滚准备
