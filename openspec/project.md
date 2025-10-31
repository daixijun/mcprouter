# 项目上下文

## 目的

MCPRouter 是一个现代化的 MCP (Model Context Protocol) 路由器，提供桌面应用程序来管理 MCP 服务器、API 密钥和工具。该应用程序能够对 MCP 服务器配置进行细粒度控制，具有安全的 API 密钥存储和工具级管理功能。

## 技术栈

- **前端**：Ant Design v5、React 19、TypeScript、Vite、TailwindCSS
- **后端**：Tauri 2、Rust
- **数据库**：SQLite，支持 ACID 兼容事务
- **UI 库**：Lucide React（图标）、React Router、React Markdown
- **构建工具**：TypeScript 5.8+、Vite 7+

## 项目约定

### AI 助手工作规范

- **语言规范**：所有的回复、任务清单、代码注释都应该使用中文
- **服务器启动限制**：如果没有明确指定，不要运行 `pnpm tauri dev`、`pnpm tauri build`、`pnpm dev`、`pnpm preview` 等命令启动开发服务器、预览环境或编译
- **前端检查**：前端变更后应该使用 `npx tsc --noEmit` 命令进行类型检查
- **后端检查**：后端变更后应该使用 `cargo clippy` 命令进行检查
- **IDE 诊断**：如果连接了 IDE，每次任务结束之前都应该检查 IDE 中是否还存在 ISSUE 并修复
- **代码整洁**：后端代码中不要使用 `#[allow(dead_code)]`、`#[deprecated]` 等属性抑制警告，应该清理未使用的代码块，保持代码整洁
- **任务细分**：如果改动较多不要一次性操作，应该细分任务一步一步实现，使用 TodoWrite 工具跟踪进度
- **代码哲学**：除非明确要求，否则不要编写兼容性代码或防御性编程，优先选择简单、直接的实现
- **前端样式**：使用 TailwindCSS 进行样式设计，保持组件化和可维护性，需要考虑响应式设计和主题适配
- **任务清单更新**: 每次任务完成后，应该更新任务清单 tasks.md，标记为已完成

### 代码风格

- TypeScript 使用严格类型检查
- ES 模块（`"type": "module"`）
- React 函数式组件与 Hooks
- Markdown 渲染使用清理器（rehype-sanitize）

### React 19 兼容性规范

**重要**：由于 antd v5 与 React 19 版本兼容问题，必须使用 hooks 方式使用 Notification、Message、Modal 等组件，不要直接导入。

#### 正确使用方式

```typescript
import { App } from 'antd'

function MyComponent() {
  const { notification } = App.useApp()

  const showNotification = () => {
    notification.success({
      message: '成功',
      description: '操作完成',
    })
  }

  return <Button onClick={showNotification}>显示通知</Button>
}
```

#### 错误使用方式

```typescript
// ❌ 错误：直接导入组件
import { notification, message, modal } from 'antd'

function MyComponent() {
  const showNotification = () => {
    notification.success({
      // 这会导致错误
      message: '成功',
      description: '操作完成',
    })
  }

  return <Button onClick={showNotification}>显示通知</Button>
}
```

#### 应用配置

确保在应用根部使用 `<App>` 组件包装：

```typescript
import { App as AntdApp } from 'antd'

function App() {
  return (
    <AntdApp>
      <YourMainApp />
    </AntdApp>
  )
}
```

### 架构模式

- **数据库优先架构**：SQLite 作为所有配置的真相来源
- **仓储模式**：数据库操作分离到仓储模块（`src-tauri/src/db/repositories/`）
- **Tauri 命令**：后端 API 作为 Tauri 命令暴露给前端
- **数据模型**：Rust 中的强类型模型（`src-tauri/src/db/models.rs`）

### 数据库架构

- `mcp_servers`：MCP 服务器配置及元数据
- `api_keys`：安全的 API 密钥存储（SHA-256 哈希）
- `mcp_tools`：每个服务器的工具级状态管理
- `api_key_server_relations`：权限的多对多关系

### Git 工作流

- 主分支：`main`
- 鼓励使用常规提交
- 新能力使用功能分支

## 领域上下文

### MCP (Model Context Protocol)

- MCP 服务器为 AI 模型提供工具和资源
- 每个服务器可以有多个可以独立启用/禁用的工具
- 服务器支持使用 StreamableHTTP 协议提供聚合接口
- API 密钥控制对特定服务器的访问

### 关键实体

- **MCP 服务器**：提供工具/资源的服务，通过命令/参数或 URL 配置
- **工具**：MCP 服务器暴露的单个能力
- **API 密钥**：具有服务器级权限的认证凭证
- **传输方式**：通信方法（stdio 或 HTTP）

## 重要约束

- API 密钥必须以 SHA-256 哈希形式存储以确保安全
- 数据库操作必须维持 ACID 合规性
- 工具和服务器状态必须在应用程序重启后持久化

## 外部依赖

- Tauri 插件：clipboard-manager、dialog、opener
- React 生态系统：antd、react-markdown、lucide-react
- 无外部 API 服务（独立桌面应用）

## 变更记录

### 2025-11-01

**增加响应式布局支持**

- 调整主容器最大宽度从 1280px 增加到 1600px，充分利用宽屏显示空间
- 优化 Dashboard 统计卡片响应式布局：
  - xl 及以上（≥1280px）：5 列布局
  - lg（1024px-1279px）：4 列布局
  - md（768px-1023px）：2 列布局
  - sm 及以下：单列布局
- 优化 Dashboard 信息卡片响应式布局：
  - lg 及以上：并排显示
  - md 及以下：垂直堆叠
- 优化 MCP 广场列表响应式布局：
  - xl 及以上：4 列布局
  - lg（1024px-1279px）：3 列布局
  - md（768px-1023px）：2 列布局
  - sm 及以下：单列布局
- 优化 Settings 页面表单响应式布局：
  - md 及以上：双列并排显示
  - sm 及以下：单列垂直堆叠
- 宽屏用户（1920px+）内容区域宽度提升约 25%
