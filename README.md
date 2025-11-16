# MCPRouter - MCP Router

A modern MCP (Model Context Protocol) Router built with Tauri, React and TypeScript, providing comprehensive MCP server management, marketplace integration, and intelligent request routing.

## Features

### 🚀 **High Performance**

- Multi-transport protocol support (stdio, SSE, HTTP)
- Asynchronous server lifecycle management
- Real-time connection monitoring and health checks
- Automatic service reconnection and recovery

### 🔍 **Marketplace Integration**

- Browse and search MCP services from multiple providers
- One-click installation with automatic configuration
- Service details view with documentation and requirements
- Support for ModelScope and other MCP registries

### 📊 **Intelligent Dashboard**

- Real-time system statistics and health monitoring
- Active connections and server status tracking
- Startup time and performance metrics
- Visual service status indicators

### 🎯 **Comprehensive Management**

- Server lifecycle management (start/stop/restart)
- Tool, resource, and prompt discovery and management
- Configuration import/export and migration
- Bulk operations for service management

### 🛡️ **System Integration**

- Native system tray with quick access menu
- Auto-start and background service support
- Multi-theme support (Auto/Light/Dark)
- Platform-specific optimizations (macOS, Windows, Linux)

### 🔐 **Authentication & Security**

- Optional Bearer token authentication for aggregator endpoints
- Configurable authentication via `server.auth` setting
- Dynamic Token Management system with creation, deletion, and usage statistics
- **Fine-grained Token Permissions**: Control access to specific tools, resources, and prompts
- **Permission Pattern Matching**: Support for wildcard patterns (`*`, `server/*`, `server/tool`)
- **Session-level Permission Caching**: High-performance permission validation
- Constant-time token comparison to prevent timing attacks
- Secure configuration with validation and warnings for weak tokens
- Full backward compatibility (authentication disabled by default)

### 📝 **Rich Configuration**

- Flexible configuration management
- Environment variable support
- Network interface and IP address management
- Logging and debugging support

## Quick Start

1. **Install Dependencies**: `pnpm install`
2. **Development Mode**: `pnpm tauri dev`
3. **Build**: `pnpm tauri build`

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
│   ├── marketplace.rs   # Marketplace integration
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
├── marketplace/         # Marketplace providers
│   ├── mod.rs
│   └── providers/
│       ├── mod.rs
│       └── modelscope.rs  # ModelScope provider
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
│   ├── InstallConfirmModal.tsx # Installation confirmation
│   └── AboutModal.tsx         # About dialog
├── pages/               # Main application pages
│   ├── Dashboard.tsx          # System dashboard and statistics
│   ├── McpServerManager.tsx   # MCP server management
│   ├── Marketplace.tsx        # Marketplace browser
│   ├── Settings.tsx           # Application settings
│   └── TokenManagement.tsx    # Token management
├── services/            # API service layer
│   ├── api.ts                  # Tauri API client
│   ├── config-service.ts       # Configuration management
│   ├── dashboard-service.ts    # Dashboard statistics
│   ├── marketplace-service.ts  # Marketplace operations
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
- **Marketplace Integration**: Browse, search, and install MCP services from multiple providers
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
