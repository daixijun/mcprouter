# MCPRouter - MCP 路由器

基于 Tauri、React 和 TypeScript 构建的现代化 MCP (Model Context Protocol) 路由器，为 MCP 服务器提供高性能路由和管理功能。

## 特性

- 🚀 **高性能**: 优化的 SQLite 数据库查询和索引，实现快速的 MCP 请求路由
- 🔐 **安全**: SHA-256 哈希 API 密钥认证，支持细粒度的工具级访问控制
- 🔧 **精细控制**: 工具级授权，支持精确的 API 密钥权限管理
- 📊 **可靠**: 符合 ACID 标准的数据库事务，确保数据一致性和可靠性
- 🛡️ **可扩展**: 清晰的架构，支持大规模 MCP 服务器部署
- 🎯 **用户友好**: 现代化的 React 界面，简化服务器和 API 密钥管理

## 快速开始

1. **安装依赖**: `pnpm install`
2. **开发模式**: `pnpm tauri dev`
3. **构建**: `pnpm tauri build`

## 推荐的 IDE 设置

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 架构

### 后端 (Rust/Tauri)

```text
src-tauri/src/
├── main.rs              # 应用程序入口点
├── lib.rs               # Tauri 命令注册
├── db/                  # 数据库层
│   ├── connection.rs    # SQLite 连接和迁移
│   ├── models.rs        # 数据模型和架构
│   └── repositories/    # 数据库仓储
│       ├── mod.rs
│       ├── server_repository.rs
│       ├── tool_repository.rs
│       └── api_key_tool_repository.rs
├── mcp_manager.rs       # MCP 服务器生命周期管理
├── aggregator.rs        # 请求路由和授权
└── migrations/          # 数据库迁移脚本
    └── 002_tool_level_auth.sql
```

### 前端 (React/TypeScript)

```text
src/
├── components/          # 可重用的 UI 组件
│   └── ApiKeyPermissionSelector.tsx    # 工具级权限选择器
├── pages/              # 主要应用页面
│   ├── ApiKeys.tsx     # API 密钥管理
│   ├── Servers.tsx     # MCP 服务器管理
│   └── Settings.tsx    # 应用设置
├── services/           # API 服务层
│   └── api.ts          # Tauri 命令封装
└── types/              # TypeScript 类型定义
    └── index.ts
```

### 核心功能

- **MCP 服务器管理**: 连接、配置和管理多个 MCP 服务器
- **工具级授权**: 为单个 MCP 工具提供细粒度的 API 密钥权限
- **请求路由**: 高效的请求聚合和授权检查
- **数据库存储**: 基于 SQLite 的持久化存储，支持 ACID 事务
- **现代化 UI**: 基于 React 的界面，支持实时状态更新

### 数据库架构

MCPRouter 使用 SQLite 和清晰的架构来管理 MCP 服务器和 API 密钥：

- **mcp_servers**: 服务器配置和元数据
- **mcp_tools**: 每个服务器的独立工具定义
- **api_keys**: 安全的 API 密钥存储，使用 SHA-256 哈希
- **api_key_tool_relations**: 细粒度的工具级授权映射

## 开发

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 生产构建
pnpm tauri build

# 运行测试（如可用）
pnpm test
```
