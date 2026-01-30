# MCP ListChanged 和 Notifications 功能设计

**日期**: 2026-01-31
**状态**: 设计阶段
**作者**: AI Assistant
**优先级**: 中等

## 概述

本设计文档描述了为 MCPRouter 的 MCP 聚合接口实现 `listChanged` 能力声明和通知推送功能。当后端 MCP 服务器的工具/资源/提示清单发生变化时,聚合器将主动通知客户端重新获取列表。

## 背景

根据 [MCP 规范 (2025-06-18)](https://modelcontextprotocol.io/specification/2025-06-18/),服务器可以通过 `listChanged` 能力声明并主动推送通知,告知客户端可用的资源列表已发生变化。

当前 MCPRouter 已经实现了基础的聚合功能,但缺少:
1. `listChanged` 能力声明
2. 主动推送通知的机制
3. 检测后端服务器清单变化的逻辑

## 目标

### 主要目标

1. **能力声明**: 在 MCP `initialize` 握手中声明支持 `listChanged` 能力
2. **通知推送**: 实现通过 SSE 向客户端推送 listChanged 通知
3. **变化检测**: 在后端服务器清单发生变化时自动触发通知

### 非目标

- 实现 SSE 连接管理(由 rmcp 库提供)
- 实现客户端重连逻辑
- 实现通知的持久化和重传

## 设计方案

### 1. 架构概览

```
┌─────────────────┐
│  MCP Client     │
│  (外部应用)      │
└────────┬────────┘
         │ SSE / HTTP
         ▼
┌─────────────────────────────────┐
│   McpAggregator                │
│   - ServerHandler 实现          │
│   - SSE 通知广播                │
│   - 权限过滤                    │
└────────┬────────────────────────┘
         │ 回调通知
         ▼
┌─────────────────────────────────┐
│   McpServerManager             │
│   - 清单同步                    │
│   - 变化检测                    │
│   - 回调触发                    │
└────────┬────────────────────────┘
         │ MCP 协议
         ▼
┌─────────────────────────────────┐
│  后端 MCP 服务器                 │
│  (stdio/http)                   │
└─────────────────────────────────┘
```

### 2. 能力声明

修改 `aggregator.rs` 中的 `initialize` 方法,启用 listChanged 能力:

```rust
pub async fn initialize(
    &self,
    _request: InitializeRequestParams,
    _context: RequestContext<RoleServer>,
) -> Result<InitializeResult, RmcpErrorData> {
    Ok(InitializeResult {
        protocol_version: ProtocolVersion::default(),
        capabilities: rmcp::model::ServerCapabilities {
            experimental: None,
            logging: None,
            completions: None,
            prompts: Some(rmcp::model::PromptsCapability {
                list_changed: Some(true),  // ✨ 新增
            }),
            resources: Some(rmcp::model::ResourcesCapability {
                subscribe: None,
                list_changed: Some(true),  // ✨ 新增
            }),
            tools: Some(rmcp::model::ToolsCapability {
                list_changed: Some(true),  // ✨ 新增
            }),
            tasks: None,
        },
        server_info: get_mcp_server_info(&self.app),
        instructions: None,
    })
}
```

### 3. 回调机制

#### 3.1 定义回调 Trait

创建新文件 `src-tauri/src/notification_callback.rs`:

```rust
use async_trait::async_trait;

/// 清单变化通知回调 Trait
#[async_trait]
pub trait ManifestChangeCallback: Send + Sync {
    /// 当工具列表发生变化时调用
    async fn tools_list_changed(&self, server_name: &str);

    /// 当资源列表发生变化时调用
    async fn resources_list_changed(&self, server_name: &str);

    /// 当提示词列表发生变化时调用
    async fn prompts_list_changed(&self, server_name: &str);
}

/// 用于测试的空实现
pub struct NullCallback;

#[async_trait]
impl ManifestChangeCallback for NullCallback {
    async fn tools_list_changed(&self, _server_name: &str) {}
    async fn resources_list_changed(&self, _server_name: &str) {}
    async fn prompts_list_changed(&self, _server_name: &str) {}
}
```

#### 3.2 在 Manager 中添加回调支持

修改 `mcp_manager.rs`,在 `McpServerManager` 结构体中添加回调字段:

```rust
#[derive(Clone)]
pub struct McpServerManager {
    orm_storage: Arc<Storage>,
    callbacks: Arc<std::sync::RwLock<Vec<Arc<dyn ManifestChangeCallback>>>>,
}

impl McpServerManager {
    pub fn new(orm_storage: Arc<Storage>) -> Self {
        Self {
            orm_storage,
            callbacks: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    /// 注册清单变化回调
    pub fn register_callback(&self, callback: Arc<dyn ManifestChangeCallback>) {
        let mut callbacks = self.callbacks.write().unwrap();
        callbacks.push(callback);
        tracing::info!("Registered manifest change callback, total callbacks: {}", callbacks.len());
    }

    /// 触发工具列表变化回调
    async fn notify_tools_changed(&self, server_name: &str) {
        let callbacks = self.callbacks.read().unwrap();
        for callback in callbacks.iter() {
            callback.tools_list_changed(server_name).await;
        }
    }

    /// 触发资源列表变化回调
    async fn notify_resources_changed(&self, server_name: &str) {
        let callbacks = self.callbacks.read().unwrap();
        for callback in callbacks.iter() {
            callback.resources_list_changed(server_name).await;
        }
    }

    /// 触发提示词列表变化回调
    async fn notify_prompts_changed(&self, server_name: &str) {
        let callbacks = self.callbacks.read().unwrap();
        for callback in callbacks.iter() {
            callback.prompts_list_changed(server_name).await;
        }
    }
}
```

#### 3.3 在同步清单时触发回调

修改 `sync_server_manifests` 方法,添加变化检测和回调触发:

```rust
pub async fn sync_server_manifests(&self, server_name: &str) -> Result<()> {
    tracing::info!("Syncing manifests for server: {}", server_name);

    // 获取服务器信息
    let raw_server = self.get_raw_server_by_name(server_name).await?
        .ok_or_else(|| McpError::NotFound(format!("Server '{}' not found", server_name)))?;

    // 记录旧数据数量用于变化检测
    let old_tool_count = self.orm_storage.list_server_tools(&raw_server.id).await
        .map_err(|e| McpError::DatabaseError(format!("Failed to get tools: {}", e)))?
        .len();
    let old_resource_count = self.orm_storage.list_server_resources(&raw_server.id).await
        .map_err(|e| McpError::DatabaseError(format!("Failed to get resources: {}", e)))?
        .len();
    let old_prompt_count = self.orm_storage.list_server_prompts(&raw_server.id).await
        .map_err(|e| McpError::DatabaseError(format!("Failed to get prompts: {}", e)))?
        .len();

    // 同步工具...
    match crate::MCP_CLIENT_MANAGER.list_tools(server_name).await {
        Ok(tools) => {
            // ... 现有的 upsert 逻辑 ...
            let tool_count = tool_models.len();
            self.orm_storage.upsert_server_tools(&raw_server.id, tool_models).await?;

            // ✨ 检测变化并触发回调
            if tool_count != old_tool_count {
                tracing::info!("Tools list changed for server '{}': {} -> {}",
                    server_name, old_tool_count, tool_count);
                self.notify_tools_changed(server_name).await;
            }
        }
        // ...
    }

    // 同步资源...
    match crate::MCP_CLIENT_MANAGER.list_resources(server_name).await {
        Ok(resources) => {
            // ... 现有的 upsert 逻辑 ...
            let resource_count = resource_models.len();
            self.orm_storage.upsert_server_resources(&raw_server.id, resource_models).await?;

            // ✨ 检测变化并触发回调
            if resource_count != old_resource_count {
                tracing::info!("Resources list changed for server '{}': {} -> {}",
                    server_name, old_resource_count, resource_count);
                self.notify_resources_changed(server_name).await;
            }
        }
        // ...
    }

    // 同步提示词...
    match crate::MCP_CLIENT_MANAGER.list_prompts(server_name).await {
        Ok(prompts) => {
            // ... 现有的 upsert 逻辑 ...
            let prompt_count = prompt_models.len();
            self.orm_storage.upsert_server_prompts(&raw_server.id, prompt_models).await?;

            // ✨ 检测变化并触发回调
            if prompt_count != old_prompt_count {
                tracing::info!("Prompts list changed for server '{}': {} -> {}",
                    server_name, old_prompt_count, prompt_count);
                self.notify_prompts_changed(server_name).await;
            }
        }
        // ...
    }

    tracing::info!("Completed manifest sync for server: {}", server_name);
    Ok(())
}
```

### 4. SSE 通知发送

#### 4.1 实现 Aggregator 的回调

在 `aggregator.rs` 中为 `McpAggregator` 实现 `ManifestChangeCallback` trait:

```rust
#[async_trait]
impl ManifestChangeCallback for McpAggregator {
    async fn tools_list_changed(&self, server_name: &str) {
        tracing::info!("📢 Broadcasting tools/list_changed for server: {}", server_name);
        // TODO: 实现 SSE 广播
        // 需要研究 rmcp 的 API 来确定正确的调用方式
    }

    async fn resources_list_changed(&self, server_name: &str) {
        tracing::info!("📢 Broadcasting resources/list_changed for server: {}", server_name);
        // TODO: 实现 SSE 广播
    }

    async fn prompts_list_changed(&self, server_name: &str) {
        tracing::info!("📢 Broadcasting prompts/list_changed for server: {}", server_name);
        // TODO: 实现 SSE 广播
    }
}
```

#### 4.2 SSE 通知实现

根据 MCP 规范,通知消息格式为:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/tools/list_changed"
}
```

rmcp 库应该提供了发送通知的 API。实现细节需要查阅 rmcp 文档或源码来确定正确的调用方式。

可能的实现方式:

```rust
use rmcp::model::ClientNotification;

pub async fn broadcast_notification(&self, notification: ClientNotification) {
    // 通过 session_manager 广播到所有 SSE 连接
    // 具体实现取决于 rmcp 的 API
}
```

### 5. 注册回调

在应用初始化时 (可能在 `lib.rs` 或 `main.rs` 中),将 aggregator 注册为回调:

```rust
// 创建聚合器和管理器
let aggregator = Arc::new(McpAggregator::new(
    mcp_server_manager.clone(),
    mcp_client_manager.clone(),
    config,
    token_manager,
    app_handle,
));

// 注册回调
mcp_server_manager.register_callback(aggregator.clone());
```

## 实现步骤

### 阶段 1: 基础设施 (1-2 天)

1. ✅ 创建 `notification_callback.rs` 文件
2. ✅ 定义 `ManifestChangeCallback` trait
3. ✅ 在 `McpServerManager` 中添加回调字段和方法
4. ✅ 实现 `NullCallback` 用于测试

### 阶段 2: 集成 Manager (2-3 天)

5. ✅ 修改 `sync_server_manifests` 添加变化检测
6. ✅ 在变化时触发相应的回调
7. ✅ 编写单元测试验证回调机制

### 阶段 3: 能力声明 (0.5 天)

8. ✅ 修改 `aggregator.rs` 的 `initialize` 方法
9. ✅ 验证客户端能正确接收 listChanged 能力

### 阶段 4: SSE 通知 (2-3 天)

10. 🔬 研究 rmcp 的 SSE 通知 API
11. 🔬 为 `McpAggregator` 实现 `ManifestChangeCallback`
12. 🔬 实现三种 listChanged 通知的发送
13. 🔬 集成测试验证通知功能

### 阶段 5: 集成与测试 (2-3 天)

14. 🔬 在应用启动时注册回调
15. 🔬 端到端测试: 启动服务器并观察通知
16. 🔬 手动测试: 使用 MCP 客户端验证
17. 🔬 性能测试: 多服务器、多并发场景

**总计估计**: 7.5-11.5 天

## 测试策略

### 单元测试

**回调机制测试**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct TestCallback {
        tools_changed_count: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl ManifestChangeCallback for TestCallback {
        async fn tools_list_changed(&self, server_name: &str) {
            self.tools_changed_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            println!("Tools changed for: {}", server_name);
        }
        // ...
    }

    #[tokio::test]
    async fn test_callback_registration() {
        let manager = McpServerManager::new(...);
        let callback = Arc::new(TestCallback { ... });

        manager.register_callback(callback.clone());
        // 触发变化并验证回调被调用
    }
}
```

### 集成测试

1. **多服务器场景**:
   - 启动 3 个不同的 MCP 服务器
   - 动态添加/删除服务器
   - 验证通知是否正确发送

2. **清单变化场景**:
   - 模拟服务器清单增加
   - 模拟服务器清单减少
   - 模拟服务器清单更新

### 手动测试

使用 MCP Inspector 或其他 MCP 客户端:

```bash
# 连接到 MCPRouter
mcp-inspector connect http://localhost:8000/mcp

# 观察 initialize 响应中的 capabilities
# 动态添加/删除后端服务器
# 观察是否收到 listChanged 通知
# 调用 list_tools 验证列表已更新
```

## 风险与挑战

### 技术风险

1. **rmcp API 不确定**: SSE 通知发送的具体 API 可能需要深入研究
   - **缓解**: 预留研究时间,准备备用方案

2. **回调异步性**: 回调在异步上下文中调用,需要确保线程安全
   - **缓解**: 使用 `Arc` 和 `async_trait` 确保安全

3. **性能影响**: 频繁的清单变化可能导致大量通知
   - **缓解**: 实现节流机制,合并短时间内的多次变化

### 兼容性风险

1. **客户端支持**: 不是所有客户端都支持 SSE 或 listChanged
   - **缓解**: 保持向后兼容,不影响现有功能

## 依赖项

### 新增依赖

- 无 (使用现有的 async_trait)

### 现有依赖

- `rmcp`: 0.8.3 (用于 MCP 协议实现)
- `tokio`: 异步运行时
- `async_trait`: 异步 trait 支持

## 相关文档

- [MCP 规范 - Transports](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
- [MCP 规范 - Tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
- [MCP 规范 - Resources](https://modelcontextprotocol.io/specification/2025-06-18/server/resources)
- MCPRouter 代码库:
  - `src-tauri/src/aggregator.rs`
  - `src-tauri/src/mcp_manager.rs`
  - `src-tauri/src/mcp_client.rs`

## 附录

### MCP 通知格式参考

```json
// tools/list_changed
{
  "jsonrpc": "2.0",
  "method": "notifications/tools/list_changed"
}

// resources/list_changed
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/list_changed"
}

// prompts/list_changed
{
  "jsonrpc": "2.0",
  "method": "notifications/prompts/list_changed"
}
```

### 变更日志

- **2026-01-31**: 初始设计文档创建
