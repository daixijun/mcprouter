<div align="center">

# MCPRouter - MCP 路由器

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/daixijun/mcprouter)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)](https://github.com/daixijun/mcprouter)

</div>

🚀 **现代 MCP 协议路由器** - 无缝桥接 stdio 与 HTTP，提供企业级权限管理和智能路由功能

## ✨ 核心亮点

### 🔄 **MCP 协议转换**

- **stdio 到 HTTP 转换**：将标准 MCP 服务器通过 HTTP 接口暴露
- **多传输协议支持**：stdio 和 HTTP 两种传输方式
- **透明的协议适配**：无需修改现有 MCP 服务器

### 🔐 **细粒度权限管理**

- **基于 Token 的访问控制**：每个 Token 独立权限配置
- **工具级授权**：对单个工具访问权限的精确控制
- **安全 Token 验证**：常量时间比较增强安全性

### ⚡ **高性能路由聚合**

- **智能请求路由**：自动分发到对应 MCP 服务器
- **异步并发处理**：支持高并发请求
- **自动故障恢复**：服务异常时自动重连
- **实时监控仪表板**：可视化系统状态

## 🖥️ 界面预览

### 实时监控仪表板

![仪表板界面](docs/screenshots/dashboard.png)
_实时系统状态、活跃连接监控、性能指标展示_

### 服务器管理界面

![服务器管理](docs/screenshots/server-management.png)
_MCP 服务器生命周期管理、批量操作、状态监控_

### Token 权限管理

![Token管理](docs/screenshots/token-management.png)
_创建和管理 Token、配置细粒度权限、使用统计_

### 系统设置页面

![设置页面](docs/screenshots/settings.png)
_网络配置、主题设置、系统偏好配置_

## 🚀 功能特性

### 🔄 协议转换与路由

- **多传输协议支持**：stdio 和 HTTP
- **协议转换**：MCP stdio 到 HTTP 接口
- **智能请求路由**：自动识别请求目标并路由到对应服务器
- **聚合服务**：将多个 MCP 服务器聚合为统一接口

### 🔐 权限管理系统

- **Token 管理**：动态创建、删除、更新 Token
- **工具级授权**：对单个工具访问权限的精确控制
- **访问控制**：
  - `allowed_tools`：精确控制可访问的工具
  - `allowed_resources`：控制资源访问权限
  - `allowed_prompts`：控制提示模板访问权限
- **安全特性**：
  - 常量时间 Token 比较增强安全性
  - 详细审计日志
  - 向后兼容（默认禁用认证）

### 🎯 服务管理与监控

- **生命周期管理**：启动、停止、重启、配置 MCP 服务器
- **自动发现**：自动识别服务器提供的工具、资源和提示
- **健康检查**：实时监控服务状态
- **性能指标**：请求统计、响应时间、错误率

### 🛡️ 系统集成

- **原生系统托盘**：快速访问菜单
- **自启动**：后台服务支持
- **多主题**：自动/亮色/暗色主题支持
- **平台优化**：macOS、Windows、Linux 特定优化
- **灵活配置**：基于 JSON 的配置，支持导入/导出
- **网络管理**：本地 IP 地址发现和网络接口配置

## 🚀 快速开始

1. **安装依赖**：`pnpm install`
2. **开发模式**：`pnpm tauri dev`
3. **构建生产版**：`pnpm tauri build`

启动后通过直观的 Web 界面配置您的 MCP 服务器和管理权限。

## 推荐 IDE 配置

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 架构

### 后端（Rust/Tauri）

```text
src-tauri/src/
├── main.rs              # 应用程序入口
├── lib.rs               # Tauri 命令注册和全局状态
├── commands/            # Tauri 命令处理器
│   ├── mod.rs
│   ├── config.rs        # 配置管理
│   ├── dashboard.rs     # 仪表板统计
│   ├── mcp_server.rs    # MCP 服务器操作
│   ├── settings.rs      # 系统设置
│   ├── token_management.rs  # Token 管理命令
│   └── tool.rs          # 工具管理
├── config/              # 配置层
│   ├── mod.rs
│   ├── file_manager.rs  # 配置文件 I/O
│   └── mcp_server_config.rs  # 服务器配置模型
├── mcp_manager.rs       # MCP 服务器生命周期管理
├── mcp_client.rs        # MCP 客户端连接处理
├── aggregator.rs        # 请求路由和聚合
├── token_manager.rs     # Token 管理系统
├── session_manager.rs   # 会话级权限缓存
├── auth_context.rs      # 认证上下文和权限验证
├── connection_mapper.rs # HTTP 到 MCP 连接映射
├── types.rs             # 共享类型定义
└── error.rs             # 错误处理
```

### 前端（React/TypeScript）

```text
src/
├── components/          # 可复用 UI 组件
│   ├── ErrorBoundary.tsx      # 错误边界包装器
│   ├── Layout.tsx             # 主布局包装器
│   ├── ServiceDetail.tsx      # 服务器详情视图
│   ├── ToolManager.tsx        # 工具管理界面
│   └── AboutModal.tsx         # 关于对话框
├── pages/               # 主应用页面
│   ├── Dashboard.tsx          # 系统仪表板和统计
│   ├── McpServerManager.tsx   # MCP 服务器管理
│   ├── Settings.tsx           # 应用设置
│   └── TokenManagement.tsx    # Token 管理
├── services/            # API 服务层
│   ├── api.ts                  # Tauri API 客户端
│   ├── config-service.ts       # 配置管理
│   ├── dashboard-service.ts    # 仪表板统计
│   ├── mcp-server-service.ts   # 服务器管理
│   └── tool-service.ts         # 工具操作
├── types/               # TypeScript 类型定义
│   └── index.ts
├── theme/               # 主题配置
│   └── antd-config.ts
└── vite-env.d.ts        # Vite 环境类型
```

### 核心功能

- **仪表板分析**：实时系统统计、活跃连接和健康监控
- **服务器管理**：完整生命周期管理（创建、启动、停止、重启、配置）
- **自动发现**：自动发现已连接服务器的工具、资源和提示
- **配置管理**：基于 JSON 的灵活配置，支持导入/导出
- **系统集成**：原生系统托盘、自启动和多主题支持
- **网络管理**：本地 IP 地址发现和网络接口配置
- **传输协议**：支持 stdio、服务器发送事件（SSE）和 HTTP 传输

### 配置架构

MCPRouter 使用灵活的基于 JSON 的配置系统，存储在 `~/.mcprouter/config.json`：

- **服务器配置**：主机、端口、超时、连接限制和可选认证
- **MCP 服务器**：配置的服务器列表，包含传输类型、命令和环境变量
- **设置**：主题、自启动、系统托盘和注册表首选项
- **日志记录**：可配置的日志级别和文件输出

#### 认证配置

为聚合器端点启用 Bearer token 认证：

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8000,
    "max_connections": 100,
    "timeout_seconds": 30,
    "auth": true,
    "bearer_token": "your-secret-token-here"
  }
}
```

**安全最佳实践：**

- 使用加密随机 token（推荐 32 个以上字符）
- 设置文件权限 `chmod 600 ~/.mcprouter/config.json` 保护 token
- 启用认证时使用 HTTPS 或仅绑定到 localhost (`127.0.0.1`)
- Token 区分大小写，使用常量时间比较验证
- 默认禁用认证，保持向后兼容

**客户端使用：**

```bash
# 不启用认证（默认）
curl http://127.0.0.1:8000/mcp

# 启用认证
curl -H "Authorization: Bearer your-secret-token-here" \
  http://127.0.0.1:8000/mcp
```

#### Token 权限管理

MCPRouter 支持细粒度的权限控制，允许您限制对特定工具、资源和提示的访问：

```json
{
  "tokens": [
    {
      "name": "只读token",
      "token": "ro-secret-token-here",
      "allowed_tools": ["server/list_tools", "server/read_resource"],
      "allowed_resources": ["server/data/*"],
      "allowed_prompts": ["server/summary"]
    },
    {
      "name": "管理员token",
      "token": "admin-secret-token-here",
      "allowed_tools": ["*"],
      "allowed_resources": ["*"],
      "allowed_prompts": ["*"]
    }
  ]
}
```

**权限模式：**

- `*` - 允许访问所有工具/资源/提示
- `server/*` - 允许访问 `server` 命名空间下的所有工具
- `server/tool` - 仅允许访问特定工具
- `server/path/*` - 允许访问特定路径下的所有资源

**权限验证：**

- 权限在 HTTP 和 MCP 协议层均进行验证
- 会话级缓存提供高性能验证
- 详细的审计日志用于安全监控
- 未指定的权限自动拒绝访问

## 开发

### 前置要求

- [Rust](https://www.rust-lang.org/)（最新稳定版）
- [Node.js](https://nodejs.org/)（v18 或更高版本）
- [pnpm](https://pnpm.io/)（推荐包管理器）
- [Tauri CLI](https://tauri.app/v1/guides/building/setup)

### 设置

```bash
# 克隆仓库
git clone https://github.com/your-org/mcprouter.git
cd mcprouter

# 安装依赖
pnpm install

# 开发模式（同时运行 Rust 和 Web 开发服务器）
pnpm tauri dev

# 构建生产版本
pnpm tauri build

# 预览生产构建
pnpm tauri build && pnpm tauri build --debug
```

### 可用脚本

```bash
# 启动开发服务器
pnpm dev                    # 仅启动 Vite 开发服务器
pnpm tauri dev             # 启动完整的 Tauri 应用和开发服务器

# 构建命令
pnpm build                 # TypeScript 构建 + Vite 构建
pnpm tauri build           # 完整生产构建（创建安装程序）

# 实用命令
pnpm preview               # 预览 Vite 构建
```

### 项目结构

项目遵循模块化架构：

- **`src-tauri/`**：使用 Tauri 框架的 Rust 后端
- **`src/`**：React + TypeScript 前端
- **`src/components/`**：可复用 UI 组件
- **`src/pages/`**：主应用视图
- **`src/services/`**：后端通信的 API 服务层
- **`src/types/`**：TypeScript 类型定义

### 调试

```bash
# 启用调试日志
# 编辑 src-tauri/tauri.conf.json 并设置：
# "logging": { "level": "debug" }

# 查看日志（macOS/Linux）
tail -f ~/.local/share/mcprouter/logs/mcprouter.log

# 查看日志（Windows）
type %LOCALAPPDATA%\mcprouter\logs\mcprouter.log
```

### 贡献指南

1. Fork 仓库
2. 创建功能分支：`git checkout -b feature-name`
3. 进行更改并充分测试
4. 使用清晰的消息提交：`git commit -m "feat: add new feature"`
5. 推送到您的 fork 并提交拉取请求

### 技术栈

- **后端**：Rust 1.70+、Tauri 2.x
- **前端**：React 19、TypeScript 5、Vite 7
- **UI**：Ant Design 5、Tailwind CSS 3
- **图标**：Lucide React
- **平台**：macOS、Windows、Linux
