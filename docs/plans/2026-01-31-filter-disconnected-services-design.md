# 过滤未连接 MCP 服务的资源设计方案

**日期**: 2026-01-31
**状态**: 设计完成
**作者**: AI Assistant

## 目标

当 MCP 服务连接失败时，不向客户端返回该服务提供的 tools/resources/prompts。

## 核心设计

### 架构变更

**当前流程**：
```
数据库 → 所有服务资源 → 权限过滤 → 客户端
```

**新流程**：
```
数据库 → 已连接服务资源 → 权限过滤 → 客户端
         (第一层过滤)    (第二层过滤)
```

### 双重过滤策略

1. **McpServerManager 层（第一层）**：在数据源过滤，减少不必要的数据处理
2. **McpAggregator 层（第二层）**：在上层二次验证，确保即使底层数据变化也能正确过滤

## 实现细节

### McpServerManager 层面

#### 1. 新增辅助方法

```rust
pub async fn get_connected_server_names(&self) -> std::collections::HashSet<String> {
    let (servers, _) = self.list_servers(None, None).await.unwrap_or_default();
    servers
        .into_iter()
        .filter(|s| s.status == "connected")
        .map(|s| s.name.to_string())
        .collect()
}
```

#### 2. 修改聚合方法

在以下方法中添加连接状态过滤：
- `get_all_tools_for_aggregation()`
- `get_all_resources_for_aggregation()`
- `get_all_prompts_for_aggregation()`

过滤逻辑：
```rust
let connected_servers = self.get_connected_server_names().await;

for server_info in server_infos {
    let server_name = server_info.name.clone();

    if !connected_servers.contains(&server_name) {
        tracing::debug!("Skipping resources from disconnected server: {}", server_name);
        continue;
    }

    // ... 处理已连接服务的资源
}
```

### McpAggregator 层面

#### 1. 批量查询已连接服务

在 fetch 方法开始时获取已连接服务列表：
```rust
let connected_servers = self
    .mcp_server_manager
    .get_connected_server_names()
    .await;
```

#### 2. 修改 fetch 方法

在以下方法中添加二次验证：
- `fetch_tools_from_database()`
- `fetch_resources_from_database()`
- `fetch_prompts_from_database()`

过滤逻辑：
```rust
for (_tool_id, tool_name, description, input_schema_json, server_name) in tools_data {
    if !connected_servers.contains(&server_name) {
        tracing::debug!(
            "Filtering out tool '{}' from disconnected server '{}'",
            tool_name,
            server_name
        );
        continue;
    }

    // ... 处理已连接服务的工具
}
```

## 错误处理和边界情况

### 1. 连接状态查询失败

- **McpServerManager 层**：如果 `list_servers()` 失败，返回空集合（安全默认值）
- **McpAggregator 层**：如果状态查询失败，不返回该服务的资源（保守策略）

### 2. 性能优化

- 避免对每个 resource 单独查询连接状态
- 在方法开始时批量获取已连接服务列表，然后在遍历时使用 HashSet 查找

### 3. 日志记录

- **Debug 级别**：记录过滤掉的资源详情
- **Info 级别**：记录过滤统计（如"过滤了 X 个未连接服务的资源"）

### 4. 现有连接检查保留

`call_tool`、`read_resource`、`get_prompt` 方法中已有的连接状态检查应保留，作为调用时的最后防线。

## 测试场景

需要验证以下场景：

- **场景 A**：服务正常连接 → 应返回其 resources
- **场景 B**：服务连接失败 → 不应返回其 resources
- **场景 C**：服务从连接变为断开 → 应实时过滤掉
- **场景 D**：服务从断开变为连接 → 应自动出现在列表中（下次查询时）

## 向后兼容性

- 不影响 API 接口
- 现有认证和权限逻辑保持不变
- 仅改变返回的 resource 数量（减少）

## 实施步骤

1. 在 `McpServerManager` 实现 `get_connected_server_names()` 方法
2. 修改 `get_all_tools_for_aggregation()` 添加连接状态过滤
3. 修改 `get_all_resources_for_aggregation()` 添加连接状态过滤
4. 修改 `get_all_prompts_for_aggregation()` 添加连接状态过滤
5. 修改 `fetch_tools_from_database()` 添加二次验证
6. 修改 `fetch_resources_from_database()` 添加二次验证
7. 修改 `fetch_prompts_from_database()` 添加二次验证
8. 添加日志记录过滤统计
9. 编写测试验证各种连接状态场景

## 过滤状态定义

过滤规则：只返回 `status == "connected"` 的服务资源
- ✅ **included**: `connected`
- ❌ **filtered**: `failed`, `disconnected`, `connecting`, `disabled`
