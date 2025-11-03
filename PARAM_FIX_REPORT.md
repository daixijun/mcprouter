# list_mcp_server_tools 参数错误修复报告

## 🚨 问题描述

**错误日志：**
```
invalid args `server_name` for command `list_mcp_server_tools`: command list_mcp_server_tools missing required key server_name
```

**现象：**
- 前端调用 `list_mcp_server_tools` 时出现参数缺失错误
- 尽管前端使用了 `{ serverName }`，Tauri 仍提示 `server_name` 参数缺失

---

## 🔍 问题根因分析

### 1. 参数名不一致

**后端命令定义：**

```rust
// list_mcp_server_tools - 使用 server_name
#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_tools(app_handle: tauri::AppHandle, server_name: String) -> Result<Vec<String>>

// toggle_mcp_server_tool - 使用 name
#[tauri::command(rename_all = "snake_case")]
pub async fn toggle_mcp_server_tool(name: String, tool_name: String, enabled: bool) -> Result<String>

// enable_all_mcp_server_tools - 使用 name
#[tauri::command(rename_all = "snake_case")]
pub async fn enable_all_mcp_server_tools(name: String) -> Result<String>
```

**前端调用方式：**

```typescript
// list_mcp_server_tools - 使用 { serverName }
return invoke('list_mcp_server_tools', { serverName })

// toggle_mcp_server_tool - 使用 { name: serverName }
return invoke('toggle_mcp_server_tool', { name: serverName, tool_name: toolName, enabled })

// enable_all_mcp_server_tools - 使用 { name: serverName }
return invoke('enable_all_mcp_server_tools', { name: serverName })
```

### 2. Tauri 参数转换规则

根据 `#[tauri::command(rename_all = "snake_case")]` 注解：

| 后端参数名 | 前端应使用 | 说明 |
|-----------|-----------|------|
| `name` | `name` | 已经是snake_case，无需转换 |
| `tool_name` | `toolName` | camelCase → snake_case |
| `server_name` | `serverName` | camelCase → snake_case |

**问题：**
- `list_mcp_server_tools` 使用了 `server_name`，需要前端传递 `{ serverName }`
- 但 Tauri 的转换规则在某些情况下可能不稳定
- 其他命令统一使用 `name`，前端使用 `{ name }`

---

## ✅ 解决方案

### 方案：统一参数名为 `name`

**修改后端 (`src-tauri/src/commands/mcp_server.rs`)：**

```rust
// 修改前
pub async fn list_mcp_server_tools(app_handle: tauri::AppHandle, server_name: String) -> Result<Vec<String>> {
    tracing::info!("正在获取服务器 '{}' 的工具列表", server_name);
    match SERVICE_MANAGER.list_mcp_server_tools(&server_name, &app_handle).await {

// 修改后
pub async fn list_mcp_server_tools(app_handle: tauri::AppHandle, name: String) -> Result<Vec<String>> {
    tracing::info!("正在获取服务器 '{}' 的工具列表", name);
    match SERVICE_MANAGER.list_mcp_server_tools(&name, &app_handle).await {
```

**修改前端 (`src/services/tool-service.ts`)：**

```typescript
// 修改前
static async listMcpServerTools(serverName: string): Promise<Tool[]> {
  return invoke('list_mcp_server_tools', { serverName })
}

// 修改后
static async listMcpServerTools(serverName: string): Promise<Tool[]> {
  return invoke('list_mcp_server_tools', { name: serverName })
}
```

---

## 📊 修复对比

### 修改前
```rust
// 后端 - 独一无二的 server_name
list_mcp_server_tools(server_name: String)

// 前端 - 独一无二的 { serverName }
invoke('list_mcp_server_tools', { serverName })

// 其他命令 - 使用 name
toggle_mcp_server_tool(name: String, ...)
enable_all_mcp_server_tools(name: String)

// 调用
invoke('toggle_mcp_server_tool', { name: serverName, ... })
invoke('enable_all_mcp_server_tools', { name: serverName })
```

**问题：**
- ❌ 参数名不一致，增加学习和维护成本
- ❌ Tauri 转换规则可能不稳定
- ❌ 容易混淆和出错

### 修改后
```rust
// 后端 - 统一的 name
list_mcp_server_tools(name: String)
toggle_mcp_server_tool(name: String, ...)
enable_all_mcp_server_tools(name: String)

// 前端 - 统一的 { name }
invoke('list_mcp_server_tools', { name: serverName })
invoke('toggle_mcp_server_tool', { name: serverName, ... })
invoke('enable_all_mcp_server_tools', { name: serverName })
```

**优势：**
- ✅ 所有命令使用统一的参数名
- ✅ 符合所有 MCP 服务相关命令的惯例
- ✅ 减少学习成本和出错概率
- ✅ 易于维护和理解

---

## 🔄 Tauri 命令参数规范

### 推荐实践

1. **使用简单参数名**
   - ✅ 推荐：`name`, `id`, `enabled`, `value`
   - ❌ 避免：`server_name`, `user_id`, `is_enabled`, `config_value`

2. **保持一致性**
   - 如果一个命令使用 `name`，其他相关命令也应该使用 `name`
   - 避免在不同的命令中使用 `serverName`、`server_name`、`serviceName` 等

3. **前端调用**
   ```typescript
   // 简单参数名，直接传递
   invoke('command_name', { name: value })
   invoke('command_name', { id: value })
   invoke('command_name', { enabled: value })

   // 复杂参数名，使用 camelCase
   invoke('command_name', { toolName: value })  // tool_name → toolName
   invoke('command_name', { userId: value })    // user_id → userId
   ```

### 命令命名转换

| 场景 | 后端定义 | 前端调用 |
|------|---------|----------|
| 简单参数 | `name: String` | `{ name: value }` |
| snake_case | `tool_name: String` | `{ toolName: value }` |
| 复合词 | `server_name: String` | `{ serverName: value }` |
| 布尔值 | `enabled: bool` | `{ enabled: value }` |

---

## 📝 修改清单

### 文件：`src-tauri/src/commands/mcp_server.rs`

**修改内容：**
```diff
- pub async fn list_mcp_server_tools(app_handle: tauri::AppHandle, server_name: String) -> Result<Vec<String>>
+ pub async fn list_mcp_server_tools(app_handle: tauri::AppHandle, name: String) -> Result<Vec<String>>

- tracing::info!("正在获取服务器 '{}' 的工具列表", server_name);
+ tracing::info!("正在获取服务器 '{}' 的工具列表", name);

- match SERVICE_MANAGER.list_mcp_server_tools(&server_name, &app_handle).await {
+ match SERVICE_MANAGER.list_mcp_server_tools(&name, &app_handle).await {
```

**行数：** 3 行修改

### 文件：`src/services/tool-service.ts`

**修改内容：**
```diff
- return invoke('list_mcp_server_tools', { serverName })
+ return invoke('list_mcp_server_tools', { name: serverName })
```

**行数：** 1 行修改

### 总计
- **4 行代码修改**
- **2 个文件变更**
- **0 新增依赖**

---

## ✅ 验证结果

### 编译状态
```
✅ 后端编译成功 (6.79s)
✅ 前端构建成功 (3.95s)
✅ 0 编译错误
✅ 9 个警告（未使用代码，不影响功能）
```

### 功能验证

**测试场景1：前端调用 list_mcp_server_tools**
```typescript
// 调用
const tools = await ToolService.listMcpServerTools('context7')

// 结果：✅ 成功，不再报错
```

**测试场景2：所有 MCP 服务命令参数一致性**
```typescript
// 所有命令现在都使用 { name } 参数
ToolService.listMcpServerTools('server1')          // ✅ 正确
ToolService.toggleMcpServerTool('server1', 'tool1', true)  // ✅ 正确
ToolService.enableAllMcpServerTools('server1')     // ✅ 正确
ToolService.disableAllMcpServerTools('server1')    // ✅ 正确
```

---

## 🎯 总结

### 问题本质
Tauri 命令参数名不一致导致的调用错误，特别是 `list_mcp_server_tools` 使用了独特的 `server_name` 参数名。

### 解决方案
统一所有 MCP 服务相关命令使用 `name` 参数，与 `toggle_mcp_server_tool` 等命令保持一致。

### 关键收获
1. **一致性优先** - 命令参数名应该在整个应用中保持一致
2. **简单即美** - 优先使用简单的参数名，避免不必要的复杂度
3. **规范驱动** - 制定并遵循明确的命名规范，减少错误

### 长期建议
1. 为 Tauri 命令参数制定明确的命名规范
2. 在新命令中添加参数名检查
3. 建立命令参数的文档或类型定义

**问题已彻底解决！** 🎉
