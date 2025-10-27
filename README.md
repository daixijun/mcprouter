# MCPRouter - MCP Router

A modern MCP (Model Context Protocol) Router built with Tauri, React and Typescript, providing high-performance routing and management for MCP servers.

## Features

- 🚀 **High Performance**: SQLite database with optimized queries and indexing for fast MCP request routing
- 🔐 **Secure**: SHA-256 hashed API key authentication with fine-grained tool-level access control
- 🔧 **Fine-grained Control**: Tool-level authorization allowing precise API key permission management
- 📊 **Reliable**: ACID-compliant database transactions ensuring data consistency and reliability
- 🛡️ **Scalable**: Clean architecture supporting large-scale MCP server deployments
- 🎯 **User-Friendly**: Modern React-based UI for easy server and API key management

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
├── lib.rs               # Tauri command registry
├── db/                  # Database layer
│   ├── connection.rs    # SQLite connection and migrations
│   ├── models.rs        # Data models and schemas
│   └── repositories/    # Database repositories
│       ├── mod.rs
│       ├── server_repository.rs
│       ├── tool_repository.rs
│       └── api_key_tool_repository.rs
├── mcp_manager.rs       # MCP server lifecycle management
├── aggregator.rs        # Request routing and authorization
└── migrations/          # Database migration scripts
    └── 002_tool_level_auth.sql
```

### Frontend (React/TypeScript)

```text
src/
├── components/          # Reusable UI components
│   └── ApiKeyPermissionSelector.tsx    # Tool-level permission selector
├── pages/              # Main application pages
│   ├── ApiKeys.tsx     # API key management
│   ├── Servers.tsx     # MCP server management
│   └── Settings.tsx    # Application settings
├── services/           # API service layer
│   └── api.ts          # Tauri command wrappers
└── types/              # TypeScript type definitions
    └── index.ts
```

### Core Features

- **MCP Server Management**: Connect, configure, and manage multiple MCP servers
- **Tool-Level Authorization**: Fine-grained API key permissions for individual MCP tools
- **Request Routing**: Efficient request aggregation and authorization checking
- **Database Storage**: SQLite-based persistence with ACID transactions
- **Modern UI**: React-based interface with real-time status updates

### Database Architecture

MCPRouter uses SQLite with a clean schema for MCP server and API key management:

- **mcp_servers**: Server configurations and metadata
- **mcp_tools**: Individual tool definitions per server
- **api_keys**: Secure API key storage with SHA-256 hashing
- **api_key_tool_relations**: Fine-grained tool-level authorization mapping

## Development

```bash
# Install dependencies
pnpm install

# Development mode
pnpm tauri dev

# Build for production
pnpm tauri build

# Run tests (when available)
pnpm test
```
