# MCP ListChanged 和 Notifications 功能实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 为 MCPRouter 实现 MCP 协议的 listChanged 能力声明和通知推送功能,当后端服务器清单发生变化时主动通知客户端。

**架构:** 基于回调机制的通信方式 - McpServerManager 检测清单变化后通过回调通知 McpAggregator,Aggregator 通过 SSE 向客户端推送 listChanged 通知。

**技术栈:**
- Rust 1.70+ / Tauri 2.x
- rmcp 0.8.3 (MCP 协议库)
- tokio (异步运行时)
- async_trait (异步 trait 支持)
- SeaORM (数据库 ORM)

---

## 前置准备

### 了解关键概念

**MCP 协议通知格式:**
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

**设计文档:** 参考 `docs/plans/2026-01-31-mcp-listchanged-notifications-design.md`

**相关文件:**
- `src-tauri/src/aggregator.rs` - MCP 聚合器实现
- `src-tauri/src/mcp_manager.rs` - MCP 服务器管理
- `src-tauri/src/lib.rs` - 模块声明和初始化
- `src-tauri/Cargo.toml` - 依赖管理

---

## Task 1: 创建通知回调 Trait

**目标:** 定义清单变化通知的回调接口

**Files:**
- Create: `src-tauri/src/notification_callback.rs`

**Step 1: 在 lib.rs 中添加模块声明**

编辑 `src-tauri/src/lib.rs`,在模块声明部分添加:

```rust
pub mod notification_callback;
```

位置: 在 `pub mod aggregator;` 之后

**Step 2: 创建 notification_callback.rs 文件**

创建文件 `src-tauri/src/notification_callback.rs`:

```rust
//! 清单变化通知回调 Trait
//!
//! 定义了当 MCP 服务器的 tools/resources/prompts 清单发生变化时的回调接口

use async_trait::async_trait;

/// 清单变化通知回调 Trait
///
/// 当后端 MCP 服务器的工具/资源/提示清单发生变化时,
/// McpServerManager 会通过此 trait 通知注册的监听者
#[async_trait]
pub trait ManifestChangeCallback: Send + Sync {
    /// 当工具列表发生变化时调用
    ///
    /// # Arguments
    /// * `server_name` - 发生变化的服务器名称
    async fn tools_list_changed(&self, server_name: &str);

    /// 当资源列表发生变化时调用
    ///
    /// # Arguments
    /// * `server_name` - 发生变化的服务器名称
    async fn resources_list_changed(&self, server_name: &str);

    /// 当提示词列表发生变化时调用
    ///
    /// # Arguments
    /// * `server_name` - 发生变化的服务器名称
    async fn prompts_list_changed(&self, server_name: &str);
}

/// 用于测试的空实现
///
/// 不会执行任何操作的回调实现,可用于测试或作为占位符
pub struct NullCallback;

#[async_trait]
impl ManifestChangeCallback for NullCallback {
    async fn tools_list_changed(&self, _server_name: &str) {
        tracing::debug!("NullCallback: tools_list_changed called (no-op)");
    }

    async fn resources_list_changed(&self, _server_name: &str) {
        tracing::debug!("NullCallback: resources_list_changed called (no-op)");
    }

    async fn prompts_list_changed(&self, _server_name: &str) {
        tracing::debug!("NullCallback: prompts_list_changed called (no-op)");
    }
}
```

**Step 3: 编译检查**

运行: `cd src-tauri && cargo check`

Expected: 编译成功,无错误

**Step 4: 提交**

```bash
git add src-tauri/src/notification_callback.rs src-tauri/src/lib.rs
git commit -m "feat(notification): add ManifestChangeCallback trait

定义清单变化通知的回调接口,支持 tools/resources/prompts 三种类型的变化通知。

- 添加 ManifestChangeCallback trait
- 提供 NullCallback 空实现用于测试
- 使用 async_trait 支持异步回调

参考设计文档: docs/plans/2026-01-31-mcp-listchanged-notifications-design.md

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 2: 在 McpServerManager 中添加回调支持

**目标:** 在服务器管理器中实现回调注册和触发机制

**Files:**
- Modify: `src-tauri/src/mcp_manager.rs`

**Step 1: 添加回调字段到 McpServerManager 结构体**

编辑 `src-tauri/src/mcp_manager.rs`,在文件顶部的 use 语句后添加:

```rust
use crate::notification_callback::ManifestChangeCallback;
```

找到 `McpServerManager` 结构体定义 (约第 18-21 行):

```rust
#[derive(Clone)]
pub struct McpServerManager {
    orm_storage: Arc<Storage>,
}
```

修改为:

```rust
#[derive(Clone)]
pub struct McpServerManager {
    orm_storage: Arc<Storage>,
    callbacks: Arc<std::sync::RwLock<Vec<Arc<dyn ManifestChangeCallback>>>>,
}
```

**Step 2: 更新 new 方法**

找到 `new` 方法 (约第 25-27 行):

```rust
pub fn new(orm_storage: Arc<Storage>) -> Self {
    Self {
        orm_storage,
    }
}
```

修改为:

```rust
pub fn new(orm_storage: Arc<Storage>) -> Self {
    Self {
        orm_storage,
        callbacks: Arc::new(std::sync::RwLock::new(Vec::new())),
    }
}
```

**Step 3: 更新 with_storage_manager 方法**

找到 `with_storage_manager` 方法 (约第 29-36 行),修改返回的 Self:

```rust
Ok(Self {
    orm_storage: storage_manager.orm_storage(),
    callbacks: Arc::new(std::sync::RwLock::new(Vec::new())),
})
```

**Step 4: 添加回调注册方法**

在 `new` 方法之后添加以下方法:

```rust
/// 注册清单变化回调
///
/// 允许外部监听器注册以接收清单变化通知
///
/// # Arguments
/// * `callback` - 实现了 ManifestChangeCallback trait 的对象
pub fn register_callback(&self, callback: Arc<dyn ManifestChangeCallback>) {
    let mut callbacks = self.callbacks.write().unwrap();
    callbacks.push(callback);
    tracing::info!(
        "Registered manifest change callback, total callbacks: {}",
        callbacks.len()
    );
}
```

**Step 5: 添加通知触发方法**

在 `register_callback` 方法之后添加以下三个方法:

```rust
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
```

**Step 6: 编译检查**

运行: `cd src-tauri && cargo check`

Expected: 编译成功,无错误

**Step 7: 提交**

```bash
git add src-tauri/src/mcp_manager.rs
git commit -m "feat(mcp_manager): add callback registration and notification

在 McpServerManager 中添加回调机制支持:

- 添加 callbacks 字段存储注册的回调
- 实现 register_callback 方法用于注册回调
- 实现 notify_*_changed 方法用于触发通知
- 更新 new 和 with_storage_manager 方法初始化 callbacks

下一步将在 sync_server_manifests 中集成变化检测和回调触发

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 3: 在清单同步时触发回调

**目标:** 检测清单变化并触发相应的回调

**Files:**
- Modify: `src-tauri/src/mcp_manager.rs`

**Step 1: 修改 sync_server_manifests 方法签名**

找到 `sync_server_manifests` 方法 (约第 1203 行),在方法开始处添加变化检测逻辑。

首先,在获取 server_info 后,记录旧数据数量:

```rust
pub async fn sync_server_manifests(&self, server_name: &str) -> Result<()> {
    tracing::info!("Syncing manifests for server: {}", server_name);

    // Get server info from database to find server_id
    let raw_server = self
        .get_raw_server_by_name(server_name)
        .await?
        .ok_or_else(|| {
            crate::error::McpError::NotFound(format!("Server '{}' not found", server_name))
        })?;

    // ✨ 新增: 记录旧数据数量用于变化检测
    let old_tool_count = self
        .orm_storage
        .list_server_tools(&raw_server.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get old tools count: {}", e);
            0 // 默认值,不影响后续流程
        })
        .unwrap_or(0);

    let old_resource_count = self
        .orm_storage
        .list_server_resources(&raw_server.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get old resources count: {}", e);
            0
        })
        .unwrap_or(0);

    let old_prompt_count = self
        .orm_storage
        .list_server_prompts(&raw_server.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get old prompts count: {}", e);
            0
        })
        .unwrap_or(0);

    tracing::debug!(
        "Old counts - tools: {}, resources: {}, prompts: {}",
        old_tool_count,
        old_resource_count,
        old_prompt_count
    );
```

**Step 2: 在 tools 同步后检测变化**

找到 tools 同步代码块 (约第 1215-1291 行),在 upsert 成功后添加变化检测:

```rust
match crate::MCP_CLIENT_MANAGER.list_tools(server_name).await {
    Ok(tools) => {
        tracing::info!(
            "Retrieved {} tools from server '{}'",
            tools.len(),
            server_name
        );

        // ... (现有的 tool_models 构建代码,保持不变) ...

        let tool_count = tool_models.len();
        if let Err(e) = self
            .orm_storage
            .upsert_server_tools(&raw_server.id, tool_models)
            .await
        {
            tracing::error!("Failed to save tools for server '{}': {}", server_name, e);
        } else {
            tracing::info!(
                "Successfully saved {} tools for server '{}'",
                tool_count,
                server_name
            );

            // ✨ 新增: 检测变化并触发回调
            if tool_count != old_tool_count {
                tracing::info!(
                    "✅ Tools list changed for server '{}': {} -> {}",
                    server_name,
                    old_tool_count,
                    tool_count
                );
                self.notify_tools_changed(server_name).await;
            } else {
                tracing::debug!(
                    "Tools list unchanged for server '{}' (count: {})",
                    server_name,
                    tool_count
                );
            }
        }
    }
    Err(e) => {
        if should_ignore_mcp_error(&e) {
            tracing::debug!(
                "Server '{}' does not support tools method (ignoring): {}",
                server_name,
                e
            );
        } else {
            tracing::error!(
                "Failed to retrieve tools from server '{}': {}",
                server_name,
                e
            );
        }
    }
}
```

**Step 3: 在 resources 同步后检测变化**

找到 resources 同步代码块 (约第 1293-1363 行),在 upsert 成功后添加变化检测:

```rust
match crate::MCP_CLIENT_MANAGER.list_resources(server_name).await {
    Ok(resources) => {
        tracing::info!(
            "Retrieved {} resources from server '{}'",
            resources.len(),
            server_name
        );

        // ... (现有的 resource_models 构建代码,保持不变) ...

        let resource_count = resource_models.len();
        if let Err(e) = self
            .orm_storage
            .upsert_server_resources(&raw_server.id, resource_models)
            .await
        {
            tracing::error!(
                "Failed to save resources for server '{}': {}",
                server_name,
                e
            );
        } else {
            tracing::info!(
                "Successfully saved {} resources for server '{}'",
                resource_count,
                server_name
            );

            // ✨ 新增: 检测变化并触发回调
            if resource_count != old_resource_count {
                tracing::info!(
                    "✅ Resources list changed for server '{}': {} -> {}",
                    server_name,
                    old_resource_count,
                    resource_count
                );
                self.notify_resources_changed(server_name).await;
            } else {
                tracing::debug!(
                    "Resources list unchanged for server '{}' (count: {})",
                    server_name,
                    resource_count
                );
            }
        }
    }
    Err(e) => {
        if should_ignore_mcp_error(&e) {
            tracing::debug!(
                "Server '{}' does not support resources method (ignoring): {}",
                server_name,
                e
            );
        } else {
            tracing::error!(
                "Failed to retrieve resources from server '{}': {}",
                server_name,
                e
            );
        }
    }
}
```

**Step 4: 在 prompts 同步后检测变化**

找到 prompts 同步代码块 (约第 1365-1430 行),在 upsert 成功后添加变化检测:

```rust
match crate::MCP_CLIENT_MANAGER.list_prompts(server_name).await {
    Ok(prompts) => {
        tracing::info!(
            "Retrieved {} prompts from server '{}'",
            prompts.len(),
            server_name
        );

        // ... (现有的 prompt_models 构建代码,保持不变) ...

        let prompt_count = prompt_models.len();
        if let Err(e) = self
            .orm_storage
            .upsert_server_prompts(&raw_server.id, prompt_models)
            .await
        {
            tracing::error!("Failed to save prompts for server '{}': {}", server_name, e);
        } else {
            tracing::info!(
                "Successfully saved {} prompts for server '{}'",
                prompt_count,
                server_name
            );

            // ✨ 新增: 检测变化并触发回调
            if prompt_count != old_prompt_count {
                tracing::info!(
                    "✅ Prompts list changed for server '{}': {} -> {}",
                    server_name,
                    old_prompt_count,
                    prompt_count
                );
                self.notify_prompts_changed(server_name).await;
            } else {
                tracing::debug!(
                    "Prompts list unchanged for server '{}' (count: {})",
                    server_name,
                    prompt_count
                );
            }
        }
    }
    Err(e) => {
        if should_ignore_mcp_error(&e) {
            tracing::debug!(
                "Server '{}' does not support prompts method (ignoring): {}",
                server_name,
                e
            );
        } else {
            tracing::error!(
                "Failed to retrieve prompts from server '{}': {}",
                server_name,
                e
            );
        }
    }
}
```

**Step 5: 编译检查**

运行: `cd src-tauri && cargo check`

Expected: 编译成功,无错误

**Step 6: 提交**

```bash
git add src-tauri/src/mcp_manager.rs
git commit -m "feat(mcp_manager): add manifest change detection and callback triggering

在 sync_server_manifests 方法中实现清单变化检测:

- 记录同步前的 tools/resources/prompts 数量
- 在每次 upsert 后对比新旧数量
- 当数量变化时触发相应的回调通知
- 添加详细的日志输出便于调试

注意: 当前使用简单的数量对比,后续可以优化为内容级别的变化检测

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 4: 修改能力声明

**目标:** 在 initialize 响应中声明支持 listChanged 能力

**Files:**
- Modify: `src-tauri/src/aggregator.rs`

**Step 1: 找到 initialize 方法**

在 `aggregator.rs` 中找到 `initialize` 方法实现 (约第 830-854 行)

**Step 2: 修改 capabilities 声明**

找到以下代码块:

```rust
capabilities: rmcp::model::ServerCapabilities {
    experimental: None,
    logging: None,
    completions: None,
    prompts: Some(rmcp::model::PromptsCapability { list_changed: None }),
    resources: Some(rmcp::model::ResourcesCapability {
        subscribe: None,
        list_changed: None,
    }),
    tools: Some(rmcp::model::ToolsCapability { list_changed: None }),
    tasks: None,
},
```

修改为:

```rust
capabilities: rmcp::model::ServerCapabilities {
    experimental: None,
    logging: None,
    completions: None,
    prompts: Some(rmcp::model::PromptsCapability {
        list_changed: Some(true),
    }),
    resources: Some(rmcp::model::ResourcesCapability {
        subscribe: None,
        list_changed: Some(true),
    }),
    tools: Some(rmcp::model::ToolsCapability {
        list_changed: Some(true),
    }),
    tasks: None,
},
```

**Step 3: 编译检查**

运行: `cd src-tauri && cargo check`

Expected: 编译成功,无错误

**Step 4: 提交**

```bash
git add src-tauri/src/aggregator.rs
git commit -m "feat(aggregator): declare listChanged capability in initialize

修改 initialize 方法的 ServerCapabilities 声明:

- tools.list_changed: Some(true)
- resources.list_changed: Some(true)
- prompts.list_changed: Some(true)

客户端现在可以在初始化时看到聚合器支持 listChanged 通知

参考: https://modelcontextprotocol.io/specification/2025-06-18/server/tools

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 5: 实现 Aggregator 的回调

**目标:** 为 McpAggregator 实现 ManifestChangeCallback trait

**Files:**
- Modify: `src-tauri/src/aggregator.rs`

**Step 1: 添加必要的导入**

在 `aggregator.rs` 文件顶部的 use 语句区域添加:

```rust
use crate::notification_callback::ManifestChangeCallback;
```

找到 `use async_trait::async_trait;` 并添加:

```rust
use async_trait::async_trait;
```

**Step 2: 实现 ManifestChangeCallback trait**

在 `McpAggregator` impl 块的最后 (约第 825 行,`}` 之前)添加以下代码:

```rust
/// ManifestChangeCallback trait 实现
///
/// 当后端服务器的清单发生变化时,通过 SSE 向客户端推送 listChanged 通知
#[async_trait]
impl ManifestChangeCallback for McpAggregator {
    async fn tools_list_changed(&self, server_name: &str) {
        tracing::info!(
            "📢 Broadcasting tools/list_changed for server: {}",
            server_name
        );
        // TODO: 实现 SSE 广播
        // 需要研究 rmcp 的 API 来确定正确的调用方式
        // 预期流程:
        // 1. 构建 ClientNotification::ToolsListChanged
        // 2. 通过 session_manager 向所有 SSE 连接广播
        tracing::warn!("SSE broadcasting not yet implemented - notification logged only");
    }

    async fn resources_list_changed(&self, server_name: &str) {
        tracing::info!(
            "📢 Broadcasting resources/list_changed for server: {}",
            server_name
        );
        // TODO: 实现 SSE 广播
        tracing::warn!("SSE broadcasting not yet implemented - notification logged only");
    }

    async fn prompts_list_changed(&self, server_name: &str) {
        tracing::info!(
            "📢 Broadcasting prompts/list_changed for server: {}",
            server_name
        );
        // TODO: 实现 SSE 广播
        tracing::warn!("SSE broadcasting not yet implemented - notification logged only");
    }
}
```

**Step 3: 编译检查**

运行: `cd src-tauri && cargo check`

Expected: 编译成功,无错误

**Step 4: 提交**

```bash
git add src-tauri/src/aggregator.rs
git-tauri/src/aggregator.rs
git commit -m "feat(aggregator): implement ManifestChangeCallback trait

为 McpAggregator 实现 ManifestChangeCallback trait:

- 实现 tools_list_changed 方法
- 实现 resources_list_changed 方法
- 实现 prompts_list_changed 方法
- 添加日志记录当前只记录通知,SSE 广播待实现

下一步需要研究 rmcp 库的 SSE 通知 API 来完成实际的广播功能

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 6: 注册回调

**目标:** 在应用初始化时注册 aggregator 的回调

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: 找到 aggregator 创建位置**

在 `lib.rs` 中搜索 aggregator 的创建代码。可能在 `setup_services` 或类似的初始化函数中。

使用 grep 查找: `grep -n "McpAggregator::new" src-tauri/src/lib.rs`

如果找不到,可能在 `src-tauri/src/main.rs` 中,也需要检查: `grep -n "McpAggregator::new" src-tauri/src/main.rs`

**Step 2: 根据找到的位置添加回调注册**

假设在 lib.rs 中找到类似这样的代码:

```rust
let aggregator = Arc::new(McpAggregator::new(
    mcp_server_manager.clone(),
    mcp_client_manager.clone(),
    config,
    token_manager,
    app_handle,
));
```

在这之后添加:

```rust
// 注册清单变化回调
mcp_server_manager.register_callback(aggregator.clone());
tracing::info!("✅ Registered aggregator as manifest change callback");
```

**Step 3: 编译检查**

运行: `cd src-tauri && cargo check`

Expected: 编译成功,无错误

**Step 4: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(init): register aggregator callback in initialization

在应用初始化时注册 aggregator 作为 manifest change callback:

- 在 McpAggregator 创建后立即注册回调
- 添加成功注册的日志

现在当后端服务器清单变化时,aggregator 会收到通知并记录日志

SSE 广播功能待后续实现

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 7: 研究并实现 SSE 通知广播

**目标:** 完成 SSE 通知的发送功能

**前置条件:** 这个任务需要研究 rmcp 库的 API,可能需要查阅文档或源码

**研究步骤:**

**Step 1: 查看 rmcp 文档和示例**

```bash
# 查看 rmcp 的依赖版本
grep rmcp src-tauri/Cargo.toml

# 查找 rmcp 的文档或示例
# 可能需要在线查看: https://docs.rs/rmcp/
# 或查看本地缓存: ~/.cargo/registry/src/
```

**Step 2: 查找 SSE 通知相关的类型和方法**

在 aggregator.rs 中,查找 StreamableHttpService 和 session_manager 的使用:

```bash
# 查看 aggregator 中如何创建 session_manager
grep -n "session_manager\|SessionManager" src-tauri/src/aggregator.rs
```

**Step 3: 确定通知发送方式**

根据研究,可能需要:

1. 在 `McpAggregator` 中保存 `session_manager` 的引用
2. 使用 session_manager 向所有连接的客户端发送通知
3. 或者通过 `RequestContext` 的方法发送通知

**可能的实现方案:**

```rust
// 在 McpAggregator 结构体中添加
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

#[derive(Clone)]
pub struct McpAggregator {
    // ... 现有字段 ...
    session_manager: Arc<LocalSessionManager>,
}

// 在 new 方法中初始化
pub fn new(...) -> Self {
    Self {
        // ... 现有字段 ...
        session_manager: Arc::new(LocalSessionManager::default()),
    }
}

// 在通知方法中使用
async fn broadcast_notification(&self, notification: ClientNotification) {
    // 使用 session_manager 广播
    // 具体实现取决于 rmcp API
}
```

**Step 4: 根据研究结果实现**

由于这个任务需要先研究 rmcp API,具体实现步骤会在研究后补充。

**可能的实现示例 (待验证):**

```rust
use rmcp::model::{ClientNotification, Notification};

async fn tools_list_changed(&self, server_name: &str) {
    tracing::info!(
        "📢 Broadcasting tools/list_changed for server: {}",
        server_name
    );

    // 创建通知
    let notification = ClientNotification::ToolsListChanged;

    // 广播到所有 SSE 连接
    // TODO: 确定正确的 API 调用方式
    // 可能是:
    // self.session_manager.broadcast(notification).await;
    // 或其他方式
}
```

**Step 5: 测试 SSE 通知**

使用 MCP 客户端 (如 MCP Inspector) 连接到聚合器,然后:
1. 动态添加一个后端 MCP 服务器
2. 观察客户端是否收到 listChanged 通知

**Step 6: 编译和提交**

```bash
cd src-tauri && cargo check
git add src-tauri/src/aggregator.rs
git commit -m "feat(aggregator): implement SSE notification broadcasting

完成 SSE 通知广播功能:

- 在 McpAggregator 中保存 session_manager 引用
- 实现 tools_list_changed SSE 广播
- 实现 resources_list_changed SSE 广播
- 实现 prompts_list_changed SSE 广播

客户端现在可以实时收到清单变化通知

需要研究 rmcp 库的具体 API 来完成此任务

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 8: 端到端测试

**目标:** 验证完整的 listChanged 通知流程

**测试步骤:**

**Step 1: 启动 MCPRouter**

```bash
cd /Users/xijun/Projects/devtools/mcprouter
pnpm tauri dev
```

**Step 2: 使用 MCP Inspector 连接**

```bash
# 在另一个终端
mcp-inspector connect http://localhost:8000/mcp
```

**Step 3: 观察 initialize 响应**

确认 capabilities 中包含:
```json
{
  "tools": { "listChanged": true },
  "resources": { "listChanged": true },
  "prompts": { "listChanged": true }
}
```

**Step 4: 动态添加后端服务器**

通过 MCPRouter 的 Web UI 或 API 添加一个新的 MCP 服务器

**Step 5: 观察通知**

在 MCP Inspector 中观察是否收到:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/tools/list_changed"
}
```

**Step 6: 验证列表更新**

调用 `tools/list` 验证新服务器的工具已出现在聚合列表中

**Step 7: 记录测试结果**

创建测试报告文档: `docs/plans/2026-01-31-testing-report.md`

**Step 8: 修复发现的问题**

根据测试结果修复任何 bug 或问题

**Step 9: 提交测试结果和修复**

```bash
git add docs/plans/2026-01-31-testing-report.md
git commit -m "test: add listChanged notifications E2E test report

记录端到端测试结果:

- 测试环境配置
- 测试步骤和观察结果
- 发现的问题和修复方案
- 功能验证结论

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 验收标准

完成所有任务后,应该满足:

1. ✅ **能力声明**: 客户端在 initialize 时能看到 listChanged: true
2. ✅ **变化检测**: 后端服务器清单变化时能被检测到
3. ✅ **回调触发**: Manager 能正确触发 Aggregator 的回调
4. ✅ **SSE 广播**: 客户端能通过 SSE 收到 listChanged 通知
5. ✅ **日志完整**: 所有关键操作都有详细日志
6. ✅ **编译通过**: `cargo check` 无错误
7. ✅ **测试通过**: 端到端测试验证功能正常

---

## 故障排查

### 常见问题

**Q: 编译错误 "cannot find async_trait in this scope"**
A: 在 `Cargo.toml` 中确保有 `async-trait = "0.1"` 依赖

**Q: 回调没有被触发**
A: 检查:
1. 是否调用了 `register_callback`
2. 清单是否真的发生了变化
3. 查看日志确认回调路径

**Q: SSE 通知未收到**
A: 检查:
1. 客户端是否支持 SSE
2. 是否有活跃的 SSE 连接
3. 查看 rmcp session_manager 的状态

### 调试技巧

**启用详细日志:**

在 `src-tauri/tauri.conf.json` 中设置:

```json
{
  "logging": {
    "level": "debug"
  }
}
```

**查看日志:**

```bash
# macOS
tail -f ~/.local/share/mcprouter/logs/mcprouter.log

# 或在终端直接查看 pnpm tauri dev 的输出
```

---

## 参考资源

### 设计文档
- `docs/plans/2026-01-31-mcp-listchanged-notifications-design.md`

### MCP 规范
- [Transports - Streamable HTTP](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
- [Tools - List Changed Notification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
- [Resources - List Changed Notification](https://modelcontextprotocol.io/specification/2025-06-18/server/resources)

### 代码文件
- `src-tauri/src/notification_callback.rs` - 回调 trait 定义
- `src-tauri/src/mcp_manager.rs` - 服务器管理器
- `src-tauri/src/aggregator.rs` - MCP 聚合器
- `src-tauri/src/lib.rs` - 模块声明和初始化

### Rust 相关
- [async_trait 文档](https://docs.rs/async-trait/)
- [tokio 文档](https://tokio.rs/)
- [Arc<RwLock> 模式](https://doc.rust-lang.org/std/sync/struct.RwLock.html)

---

**计划创建日期:** 2026-01-31
**预计完成时间:** 7.5-11.5 天
**当前状态:** 准备实施
