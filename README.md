# MCPRouter - MCP Router

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/daixijun/mcprouter)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)](https://github.com/daixijun/mcprouter)

🚀 **Modern MCP Protocol Router** - Seamlessly bridge stdio and HTTP with enterprise-grade permission management and intelligent routing

## ✨ Core Highlights

### 🔄 **MCP Protocol Conversion**

- **stdio to HTTP Conversion**: Expose standard MCP servers via HTTP interface
- **Multi-transport Protocol Support**: Both stdio and HTTP transport methods
- **Transparent Protocol Adaptation**: No modifications required for existing MCP servers

### 🔐 **Fine-grained Permission Management**

- **Token-based Access Control**: Independent permission configuration for each token
- **Tool-level Authorization**: Granular control over individual tool access permissions
- **Secure Token Validation**: Constant-time comparison for enhanced security

### ⚡ **High-performance Routing Aggregation**

- **Intelligent Request Routing**: Automatic distribution to corresponding MCP servers
- **Asynchronous Concurrent Processing**: Support for high-concurrency requests
- **Automatic Failure Recovery**: Automatic reconnection on service exceptions
- **Real-time Monitoring Dashboard**: Visual system status display

## 🖥️ Interface Preview

### Real-time Monitoring Dashboard

![Dashboard Interface](docs/screenshots/dashboard.png)
*Real-time system status, active connection monitoring, and performance metrics display*

### Server Management Interface

![Server Management](docs/screenshots/server-management.png)
*MCP server lifecycle management, batch operations, and status monitoring*

### Token Permission Management

![Token Management](docs/screenshots/token-management.png)
*Create and manage tokens, configure fine-grained permissions, and view usage statistics*

### System Settings Page

![Settings Page](docs/screenshots/settings.png)
*Network configuration, theme settings, and system preferences*

## 🚀 Features

### 🔄 Protocol Conversion & Routing

- **Multi-transport Protocol Support**: Both stdio and HTTP
- **Protocol Conversion**: MCP stdio to HTTP interface
- **Intelligent Request Routing**: Automatic identification and routing to corresponding servers
- **Aggregated Services**: Combine multiple MCP servers into a unified interface

### 🔐 Permission Management System

- **Token Management**: Dynamic creation, deletion, and updating of tokens
- **Tool-level Authorization**: Granular control over individual tool access permissions
- **Access Control**:
  - `allowed_tools`: Precise control over which tools can be accessed
  - `allowed_resources`: Control resource access permissions
  - `allowed_prompts`: Control prompt template access permissions
- **Security Features**:
  - Constant-time token comparison for enhanced security
  - Detailed audit logging
  - Backward compatibility (authentication disabled by default)

### 🎯 Service Management & Monitoring

- **Lifecycle Management**: Start, stop, restart, and configure MCP servers
- **Auto-discovery**: Automatic identification of tools, resources, and prompts provided by servers
- **Health Checks**: Real-time monitoring of service status
- **Performance Metrics**: Request statistics, response times, error rates

### 🛡️ System Integration

- **Native System Tray**: Quick access menu
- **Auto-start**: Background service support
- **Multi-theme**: Auto/Light/Dark theme support
- **Platform Optimization**: macOS, Windows, Linux specific optimizations
- **Flexible Configuration**: JSON-based configuration with import/export support
- **Network Management**: Local IP address discovery and network interface configuration

## 🚀 Quick Start

1. **Install Dependencies**: `pnpm install`
2. **Development Mode**: `pnpm tauri dev`
3. **Build Production**: `pnpm tauri build`

Configure your MCP servers and manage permissions through the intuitive web interface after startup.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Architecture

### Backend (Rust/Tauri)

```text
src-tauri/src/
├── main.rs              # Application entry point
├── lib.rs               # Tauri command registry and global state
├── commands/            # Tauri command handlers
│   ├── mod.rs
│   ├── config.rs        # Configuration management
│   ├── dashboard.rs     # Dashboard statistics
│   ├── mcp_server.rs    # MCP server operations
│   ├── settings.rs      # System settings
│   ├── token_management.rs  # Token management commands
│   └── tool.rs          # Tool management
├── config/              # Configuration layer
│   ├── mod.rs
│   ├── file_manager.rs  # Config file I/O
│   └── mcp_server_config.rs  # Server configuration models
├── mcp_manager.rs       # MCP server lifecycle management
├── mcp_client.rs        # MCP client connection handling
├── aggregator.rs        # Request routing and aggregation
├── token_manager.rs     # Token management system
├── session_manager.rs   # Session-level permission caching
├── auth_context.rs      # Authentication context and permission validation
├── connection_mapper.rs # HTTP to MCP connection mapping
├── types.rs             # Shared type definitions
└── error.rs             # Error handling
```

### Frontend (React/TypeScript)

```text
src/
├── components/          # Reusable UI components
│   ├── ErrorBoundary.tsx      # Error boundary wrapper
│   ├── Layout.tsx             # Main layout wrapper
│   ├── ServiceDetail.tsx      # Server detail view
│   ├── ToolManager.tsx        # Tool management interface
│   └── AboutModal.tsx         # About dialog
├── pages/               # Main application pages
│   ├── Dashboard.tsx          # System dashboard and statistics
│   ├── McpServerManager.tsx   # MCP server management
│   ├── Settings.tsx           # Application settings
│   └── TokenManagement.tsx    # Token management
├── services/            # API service layer
│   ├── api.ts                  # Tauri API client
│   ├── config-service.ts       # Configuration management
│   ├── dashboard-service.ts    # Dashboard statistics
│   ├── mcp-server-service.ts   # Server management
│   └── tool-service.ts         # Tool operations
├── types/               # TypeScript type definitions
│   └── index.ts
├── theme/               # Theme configuration
│   └── antd-config.ts
└── vite-env.d.ts        # Vite environment types
```

### Core Features

- **Dashboard Analytics**: Real-time system statistics, active connections, and health monitoring
- **Server Management**: Full lifecycle management (create, start, stop, restart, configure)
- **Discovery & Discovery**: Automatic discovery of tools, resources, and prompts from connected servers
- **Configuration Management**: Flexible JSON-based configuration with import/export support
- **System Integration**: Native system tray, auto-start, and multi-theme support
- **Network Management**: Local IP address discovery and network interface configuration
- **Transport Protocols**: Support for stdio, Server-Sent Events (SSE), and HTTP transports

### Configuration Architecture

MCPRouter uses a flexible JSON-based configuration system stored in `~/.mcprouter/config.json`:

- **Server Configuration**: Host, port, timeout, connection limits, and optional authentication
- **MCP Servers**: List of configured servers with transport type, command, and environment
- **Settings**: Theme, auto-start, system tray, and registry preferences
- **Logging**: Configurable log levels and file output

#### Authentication Configuration

Enable Bearer token authentication for the aggregator endpoints:

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

**Security Best Practices:**

- Use cryptographically random tokens (32+ characters recommended)
- Set file permissions to `chmod 600 ~/.mcprouter/config.json` to protect the token
- Use HTTPS or bind to localhost only (`127.0.0.1`) when authentication is enabled
- Tokens are case-sensitive and validated using constant-time comparison
- Authentication is disabled by default for backward compatibility

**Client Usage:**

```bash
# Without authentication (default)
curl http://127.0.0.1:8000/mcp

# With authentication enabled
curl -H "Authorization: Bearer your-secret-token-here" \
  http://127.0.0.1:8000/mcp
```

#### Token Permission Management

MCPRouter supports fine-grained permission control for tokens, allowing you to restrict access to specific tools, resources, and prompts:

```json
{
  "tokens": [
    {
      "name": "read-only-token",
      "token": "ro-secret-token-here",
      "allowed_tools": ["server/list_tools", "server/read_resource"],
      "allowed_resources": ["server/data/*"],
      "allowed_prompts": ["server/summary"]
    },
    {
      "name": "admin-token",
      "token": "admin-secret-token-here",
      "allowed_tools": ["*"],
      "allowed_resources": ["*"],
      "allowed_prompts": ["*"]
    }
  ]
}
```

**Permission Patterns:**

- `*` - Allows access to all tools/resources/prompts
- `server/*` - Allows access to all tools under the `server` namespace
- `server/tool` - Allows access to a specific tool only
- `server/path/*` - Allows access to all resources under a specific path

**Permission Validation:**

- Permissions are validated at both HTTP and MCP protocol layers
- Session-level caching provides high-performance validation
- Detailed audit logging for security monitoring
- Automatic fallback to deny for unspecified permissions

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable)
- [Node.js](https://nodejs.org/) (v18 or higher)
- [pnpm](https://pnpm.io/) (recommended package manager)
- [Tauri CLI](https://tauri.app/v1/guides/building/setup)

### Setup

```bash
# Clone the repository
git clone https://github.com/your-org/mcprouter.git
cd mcprouter

# Install dependencies
pnpm install

# Development mode (runs both Rust and web dev server)
pnpm tauri dev

# Build for production
pnpm tauri build

# Preview production build
pnpm tauri build && pnpm tauri build --debug
```

### Available Scripts

```bash
# Start development server
pnpm dev                    # Start Vite dev server only
pnpm tauri dev             # Start full Tauri app with dev server

# Build commands
pnpm build                 # TypeScript build + Vite build
pnpm tauri build           # Full production build (creates installers)

# Utility commands
pnpm preview               # Preview Vite build
```

### Project Structure

The project follows a modular architecture:

- **`src-tauri/`**: Rust backend using Tauri framework
- **`src/`**: React + TypeScript frontend
- **`src/components/`**: Reusable UI components
- **`src/pages/`**: Main application views
- **`src/services/`**: API service layer for backend communication
- **`src/types/`**: TypeScript type definitions

### Debugging

```bash
# Enable debug logging
# Edit src-tauri/tauri.conf.json and set:
# "logging": { "level": "debug" }

# View logs (macOS/Linux)
tail -f ~/.local/share/mcprouter/logs/mcprouter.log

# View logs (Windows)
type %LOCALAPPDATA%\mcprouter\logs\mcprouter.log
```

### Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes and test thoroughly
4. Commit with a clear message: `git commit -m "feat: add new feature"`
5. Push to your fork and submit a pull request

### Technology Stack

- **Backend**: Rust 1.70+, Tauri 2.x
- **Frontend**: React 19, TypeScript 5, Vite 7
- **UI**: Ant Design 5, Tailwind CSS 3
- **Icons**: Lucide React
- **Platforms**: macOS, Windows, Linux
