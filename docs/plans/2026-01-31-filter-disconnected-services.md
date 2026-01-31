# 过滤未连接 MCP 服务的资源实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 当 MCP 服务连接失败时，不向客户端返回该服务提供的 tools/resources/prompts。

**架构:** 双重过滤策略 - 在 McpServerManager 层数据源过滤 + 在 McpAggregator 层二次验证，确保未连接服务的资源不会返回给客户端。

**技术栈:** Rust, Tauri, SeaORM, tokio, rmcp

---

## Task 1: 在 McpServerManager 中添加辅助方法

**Files:**
- Modify: `src-tauri/src/mcp_manager.rs` (在文件末尾，impl McpServerManager 块内)

**Step 1: 添加 `get_connected_server_names()` 辅助方法**

在 `src-tauri/src/mcp_manager.rs` 文件的 `McpServerManager` impl 块末尾（约第 1740 行之前）添加以下方法：

```rust
/// 获取所有已连接的服务名称列表
///
/// 返回当前连接状态为 "connected" 的所有 MCP 服务名称的集合
/// 用于过滤聚合接口中的 tools/resources/prompts
pub async fn get_connected_server_names(&self) -> std::collections::HashSet<String> {
    let (servers, _) = self.list_servers(None, None).await.unwrap_or_default();
    servers
        .into_iter()
        .filter(|s| s.status == "connected")
        .map(|s| s.name.to_string())
        .collect()
}
```

**Step 2: 编译验证**

```bash
cd src-tauri
cargo check
```

Expected: 编译成功，无错误

**Step 3: 提交**

```bash
git add src-tauri/src/mcp_manager.rs
git commit -m "feat(mcp_manager): add get_connected_server_names helper method

This method returns a HashSet of connected server names for filtering
resources in aggregation methods."
```

---

## Task 2: 修改 get_all_tools_for_aggregation 添加连接状态过滤

**Files:**
- Modify: `src-tauri/src/mcp_manager.rs:300-338`

**Step 1: 修改 `get_all_tools_for_aggregation()` 方法**

在 `src-tauri/src/mcp_manager.rs` 中找到 `get_all_tools_for_aggregation()` 方法（约第 300 行），修改如下：

```rust
pub async fn get_all_tools_for_aggregation(
    &self,
) -> Result<Vec<(String, String, String, Option<String>, String)>> {
    // 获取已连接的服务列表（第一层过滤）
    let connected_servers = self.get_connected_server_names().await;

    // Get all tools from database and return with server information
    let (server_infos, _) = self
        .orm_storage
        .list_mcp_servers(None, None)
        .await
        .map_err(|e| {
            crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
        })?;
    let mut all_tools = Vec::new();

    for server_info in server_infos {
        let server_name = server_info.name.clone();

        // 检查服务是否已连接，跳过未连接的服务
        if !connected_servers.contains(&server_name) {
            tracing::debug!(
                "Skipping tools from disconnected server: {}",
                server_name
            );
            continue;
        }

        let tools = self
            .orm_storage
            .list_server_tools(&server_info.id)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!(
                    "Failed to get tools for server {}: {}",
                    server_info.name, e
                ))
            })?;

        for tool in tools {
            all_tools.push((
                tool.id,
                tool.name,
                tool.description.unwrap_or_default(),
                tool.input_schema,
                server_name.clone(),
            ));
        }
    }

    Ok(all_tools)
}
```

**Step 2: 编译验证**

```bash
cd src-tauri
cargo check
```

Expected: 编译成功

**Step 3: 提交**

```bash
git add src-tauri/src/mcp_manager.rs
git commit -m "feat(mcp_manager): filter tools by connection status in get_all_tools_for_aggregation

Skip tools from servers that are not in 'connected' status."
```

---

## Task 3: 修改 get_all_resources_for_aggregation 添加连接状态过滤

**Files:**
- Modify: `src-tauri/src/mcp_manager.rs:340-379`

**Step 1: 修改 `get_all_resources_for_aggregation()` 方法**

在 `src-tauri/src/mcp_manager.rs` 中找到 `get_all_resources_for_aggregation()` 方法（约第 340 行），修改如下：

```rust
pub async fn get_all_resources_for_aggregation(
    &self,
) -> Result<Vec<(String, String, String, String, Option<String>, String)>> {
    // 获取已连接的服务列表（第一层过滤）
    let connected_servers = self.get_connected_server_names().await;

    // Get all resources from database and return with server information
    let (server_infos, _) = self
        .orm_storage
        .list_mcp_servers(None, None)
        .await
        .map_err(|e| {
            crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
        })?;
    let mut all_resources = Vec::new();

    for server_info in server_infos {
        let server_name = server_info.name.clone();

        // 检查服务是否已连接，跳过未连接的服务
        if !connected_servers.contains(&server_name) {
            tracing::debug!(
                "Skipping resources from disconnected server: {}",
                server_name
            );
            continue;
        }

        let resources = self
            .orm_storage
            .list_server_resources(&server_info.id)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!(
                    "Failed to get resources for server {}: {}",
                    server_info.name, e
                ))
            })?;

        for resource in resources {
            all_resources.push((
                resource.id,
                resource.uri,
                resource.name.unwrap_or_default(),
                resource.description.unwrap_or_default(),
                resource.mime_type,
                server_name.clone(),
            ));
        }
    }

    Ok(all_resources)
}
```

**Step 2: 编译验证**

```bash
cd src-tauri
cargo check
```

Expected: 编译成功

**Step 3: 提交**

```bash
git add src-tauri/src/mcp_manager.rs
git commit -m "feat(mcp_manager): filter resources by connection status in get_all_resources_for_aggregation

Skip resources from servers that are not in 'connected' status."
```

---

## Task 4: 修改 get_all_prompts_for_aggregation 添加连接状态过滤

**Files:**
- Modify: `src-tauri/src/mcp_manager.rs:381-418`

**Step 1: 修改 `get_all_prompts_for_aggregation()` 方法**

在 `src-tauri/src/mcp_manager.rs` 中找到 `get_all_prompts_for_aggregation()` 方法（约第 381 行），修改如下：

```rust
pub async fn get_all_prompts_for_aggregation(
    &self,
) -> Result<Vec<(String, String, Option<String>, String)>> {
    // 获取已连接的服务列表（第一层过滤）
    let connected_servers = self.get_connected_server_names().await;

    // Get all prompts from database and return with server information
    let (server_infos, _) = self
        .orm_storage
        .list_mcp_servers(None, None)
        .await
        .map_err(|e| {
            crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
        })?;
    let mut all_prompts = Vec::new();

    for server_info in server_infos {
        let server_name = server_info.name.clone();

        // 检查服务是否已连接，跳过未连接的服务
        if !connected_servers.contains(&server_name) {
            tracing::debug!(
                "Skipping prompts from disconnected server: {}",
                server_name
            );
            continue;
        }

        let prompts = self
            .orm_storage
            .list_server_prompts(&server_info.id)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!(
                    "Failed to get prompts for server {}: {}",
                    server_info.name, e
                ))
            })?;

        for prompt in prompts {
            all_prompts.push((
                prompt.id,
                prompt.name,
                prompt.description,
                server_name.clone(),
            ));
        }
    }

    Ok(all_prompts)
}
```

**Step 2: 编译验证**

```bash
cd src-tauri
cargo check
```

Expected: 编译成功

**Step 3: 提交**

```bash
git add src-tauri/src/mcp_manager.rs
git commit -m "feat(mcp_manager): filter prompts by connection status in get_all_prompts_for_aggregation

Skip prompts from servers that are not in 'connected' status."
```

---

## Task 5: 修改 fetch_tools_from_database 添加二次验证

**Files:**
- Modify: `src-tauri/src/aggregator.rs:621-726`

**Step 1: 修改 `fetch_tools_from_database()` 方法**

在 `src-tauri/src/aggregator.rs` 中找到 `fetch_tools_from_database()` 方法（约第 621 行），在方法开始处添加连接状态检查：

```rust
async fn fetch_tools_from_database(&self) -> Result<Vec<McpTool>, RmcpErrorData> {
    tracing::info!("🔍 Getting tools directly from database");

    // 获取已连接的服务列表（第二层过滤）
    let connected_servers = self
        .mcp_server_manager
        .get_connected_server_names()
        .await;

    // 通过 McpServerManager 的公共方法获取完整的工具信息，包含 input_schema
    let tools_data = self
        .mcp_server_manager
        .get_all_tools_for_aggregation()
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to fetch tools from manager: {}", e);
            RmcpErrorData::internal_error(format!("Failed to fetch tools: {}", e), None)
        })?;

    tracing::info!("📊 Retrieved {} tools from database", tools_data.len());

    // 优化：预分配 Vec 容量，避免多次重分配
    let mut mcp_tools = Vec::with_capacity(tools_data.len());
    let mut filtered_count = 0;

    for (_tool_id, tool_name, description, input_schema_json, server_name) in tools_data {
        // 二次验证：检查服务是否仍然连接
        if !connected_servers.contains(&server_name) {
            filtered_count += 1;
            tracing::debug!(
                "🚫 Filtering out tool '{}' from disconnected server '{}'",
                tool_name,
                server_name
            );
            continue;
        }

        // 记录原始数据
        tracing::debug!(
            "🔧 Processing tool: {} from server: {}",
            tool_name,
            server_name
        );
        tracing::debug!(
            "📝 Raw input_schema from DB: {}",
            input_schema_json
                .as_ref()
                .map_or("NULL".to_string(), |s| s.clone())
        );

        let server_name_str = server_name.clone(); // server_name 已经是 String 类型

        // 生成 resource_path
        let resource_path = format!("{}__{}", server_name_str, tool_name);

        // 处理 input_schema，使用数据库中存储的真实数据或创建默认的空 schema
        let input_schema: std::sync::Arc<serde_json::Map<String, serde_json::Value>> =
            if let Some(schema_str) = &input_schema_json {
                // 尝试解析 JSON Schema
                match serde_json::from_str::<serde_json::Value>(schema_str) {
                    Ok(schema) => {
                        tracing::debug!(
                            "✅ Successfully parsed JSON Schema for tool: {}",
                            tool_name
                        );
                        tracing::debug!("📋 Schema content: {}", schema);

                        if let serde_json::Value::Object(mut map) = schema {
                            // 确保至少有 type 字段
                            if !map.contains_key("type") {
                                map.insert(
                                    "type".to_string(),
                                    serde_json::Value::String("object".to_string()),
                                );
                                tracing::debug!(
                                    "➕ Added default 'type: object' field to schema"
                                );
                            }
                            std::sync::Arc::new(map)
                        } else {
                            tracing::warn!(
                                "⚠️ Schema for tool {} is not an object, using default",
                                tool_name
                            );
                            Self::create_default_schema()
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ Failed to parse JSON Schema for tool {}: {}",
                            tool_name,
                            e
                        );
                        tracing::error!("🔍 Original schema string: {}", schema_str);
                        Self::create_default_schema()
                    }
                }
            } else {
                tracing::debug!("⚠️ Tool {} has NULL input_schema, using default", tool_name);
                Self::create_default_schema()
            };

        mcp_tools.push(McpTool {
            name: resource_path.clone().into(), // 克隆 resource_path 并转换为 Cow
            description: Some(description.clone().into()),
            input_schema,
            // Default values for other fields
            title: None,
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        });

        tracing::debug!("✅ Processed tool: {} -> {}", tool_name, resource_path);
    }

    if filtered_count > 0 {
        tracing::info!(
            "🔍 Filtered {} tools from disconnected servers",
            filtered_count
        );
    }

    tracing::info!(
        "🎉 Successfully processed {} McpTool objects",
        mcp_tools.len()
    );
    Ok(mcp_tools)
}
```

**Step 2: 编译验证**

```bash
cd src-tauri
cargo check
```

Expected: 编译成功

**Step 3: 提交**

```bash
git add src-tauri/src/aggregator.rs
git commit -m "feat(aggregator): add connection status validation in fetch_tools_from_database

Second layer filtering - verify server connection status before
including tools in the response."
```

---

## Task 6: 修改 fetch_resources_from_database 添加二次验证

**Files:**
- Modify: `src-tauri/src/aggregator.rs:728-787`

**Step 1: 修改 `fetch_resources_from_database()` 方法**

在 `src-tauri/src/aggregator.rs` 中找到 `fetch_resources_from_database()` 方法（约第 728 行），在方法开始处添加连接状态检查：

```rust
async fn fetch_resources_from_database(&self) -> Result<Vec<Resource>, RmcpErrorData> {
    tracing::info!("🔍 Getting resources directly from database");

    // 获取已连接的服务列表（第二层过滤）
    let connected_servers = self
        .mcp_server_manager
        .get_connected_server_names()
        .await;

    // 通过 McpServerManager 的公共方法获取完整的资源信息
    let resources_data = self
        .mcp_server_manager
        .get_all_resources_for_aggregation()
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to fetch resources from manager: {}", e);
            RmcpErrorData::internal_error(format!("Failed to fetch resources: {}", e), None)
        })?;

    tracing::info!(
        "📊 Retrieved {} resources from database",
        resources_data.len()
    );

    let mut mcp_resources = Vec::new();
    let mut filtered_count = 0;

    for (_resource_id, uri, name, description, mime_type, server_name) in resources_data {
        // 二次验证：检查服务是否仍然连接
        if !connected_servers.contains(&server_name) {
            filtered_count += 1;
            tracing::debug!(
                "🚫 Filtering out resource '{}' from disconnected server '{}'",
                uri,
                server_name
            );
            continue;
        }

        // 记录原始数据
        tracing::debug!(
            "🔧 Processing resource: {} from server: {}",
            uri,
            server_name
        );

        let server_name_str = server_name.clone(); // server_name 已经是 String 类型

        // 构建完整的 resource_path (server_name__uri)
        let resource_path = format!("{}__{}", server_name_str, uri);

        // 创建 Resource 结构体
        let raw_resource = rmcp::model::RawResource {
            uri: resource_path.clone(),
            name: name.clone(),
            title: None,
            description: Some(description),
            mime_type,
            size: None,
            icons: None,
            meta: None,
        };

        let resource = Resource {
            raw: raw_resource,
            annotations: None,
        };

        mcp_resources.push(resource);
    }

    if filtered_count > 0 {
        tracing::info!(
            "🔍 Filtered {} resources from disconnected servers",
            filtered_count
        );
    }

    tracing::info!(
        "✅ Successfully processed {} resources",
        mcp_resources.len()
    );
    Ok(mcp_resources)
}
```

**Step 2: 编译验证**

```bash
cd src-tauri
cargo check
```

Expected: 编译成功

**Step 3: 提交**

```bash
git add src-tauri/src/aggregator.rs
git commit -m "feat(aggregator): add connection status validation in fetch_resources_from_database

Second layer filtering - verify server connection status before
including resources in the response."
```

---

## Task 7: 修改 fetch_prompts_from_database 添加二次验证

**Files:**
- Modify: `src-tauri/src/aggregator.rs:789-835`

**Step 1: 修改 `fetch_prompts_from_database()` 方法**

在 `src-tauri/src/aggregator.rs` 中找到 `fetch_prompts_from_database()` 方法（约第 789 行），在方法开始处添加连接状态检查：

```rust
async fn fetch_prompts_from_database(&self) -> Result<Vec<rmcp::model::Prompt>, RmcpErrorData> {
    tracing::info!("🔍 Getting prompts directly from database");

    // 获取已连接的服务列表（第二层过滤）
    let connected_servers = self
        .mcp_server_manager
        .get_connected_server_names()
        .await;

    // 通过 McpServerManager 的公共方法获取完整的提示词信息
    let prompts_data = self
        .mcp_server_manager
        .get_all_prompts_for_aggregation()
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to fetch prompts from manager: {}", e);
            RmcpErrorData::internal_error(format!("Failed to fetch prompts: {}", e), None)
        })?;

    tracing::info!("📊 Retrieved {} prompts from database", prompts_data.len());

    let mut mcp_prompts = Vec::new();
    let mut filtered_count = 0;

    for (_prompt_id, name, description, server_name) in prompts_data {
        // 二次验证：检查服务是否仍然连接
        if !connected_servers.contains(&server_name) {
            filtered_count += 1;
            tracing::debug!(
                "🚫 Filtering out prompt '{}' from disconnected server '{}'",
                name,
                server_name
            );
            continue;
        }

        // 记录原始数据
        tracing::debug!(
            "🔧 Processing prompt: {} from server: {}",
            name,
            server_name
        );

        let server_name_str = server_name.clone(); // server_name 已经是 String 类型

        // 生成 resource_path (server_name__prompt_name)
        let resource_path = format!("{}__{}", server_name_str, name);

        // 创建 Prompt 结构体
        let prompt = rmcp::model::Prompt {
            name: resource_path.clone(),
            description: description.clone(),
            arguments: None, // TODO: 根据需要实现参数
            icons: None,
            meta: None,
            title: None,
        };

        mcp_prompts.push(prompt);
    }

    if filtered_count > 0 {
        tracing::info!(
            "🔍 Filtered {} prompts from disconnected servers",
            filtered_count
        );
    }

    tracing::info!("✅ Successfully processed {} prompts", mcp_prompts.len());
    Ok(mcp_prompts)
}
```

**Step 2: 编译验证**

```bash
cd src-tauri
cargo check
```

Expected: 编译成功

**Step 3: 提交**

```bash
git add src-tauri/src/aggregator.rs
git commit -m "feat(aggregator): add connection status validation in fetch_prompts_from_database

Second layer filtering - verify server connection status before
including prompts in the response."
```

---

## Task 8: 完整编译和测试

**Files:**
- Build: All modified files

**Step 1: 完整编译**

```bash
cd src-tauri
cargo build --release
```

Expected: 编译成功，无警告和错误

**Step 2: 运行应用测试**

启动应用并测试以下场景：
- **场景 A**: 服务正常连接时，能够看到其 tools/resources/prompts
- **场景 B**: 服务连接失败时，不应该看到其 tools/resources/prompts
- **场景 C**: 服务从连接变为断开时，列表应该自动更新（刷新后）

**Step 3: 检查日志**

查看日志确认过滤逻辑正常工作：
```bash
# 查找过滤日志
grep "Skipping.*disconnected server" logs/
grep "Filtering out.*disconnected server" logs/
grep "Filtered.*from disconnected servers" logs/
```

Expected: 能够看到过滤日志输出

**Step 4: 最终提交**

如果测试通过：
```bash
git add docs/plans/2026-01-31-filter-disconnected-services.md
git commit -m "docs: add implementation plan for filtering disconnected services

Complete implementation plan for dual-layer filtering of MCP service
resources based on connection status."
```

---

## 验收标准

完成所有任务后，应该满足以下标准：

1. ✅ 代码编译通过，无警告
2. ✅ 未连接的服务的 tools/resources/prompts 不会出现在聚合接口中
3. ✅ 日志中有清晰的过滤记录
4. ✅ 现有的认证和权限功能不受影响
5. ✅ 服务重新连接后，其资源会自动出现在列表中

## 参考文档

- 设计文档: `docs/plans/2026-01-31-filter-disconnected-services-design.md`
- 相关文件:
  - `src-tauri/src/mcp_manager.rs` - MCP 服务管理器
  - `src-tauri/src/aggregator.rs` - MCP 聚合器
