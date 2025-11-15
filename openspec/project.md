# MCPRouter Project Specification

## Overview

MCPRouter is a modern MCP (Model Context Protocol) Router built with Tauri, React and TypeScript, providing comprehensive MCP server management, marketplace integration, and intelligent request routing.

## Core Capabilities

### MCP Server Management

- Multi-transport protocol support (stdio, SSE, HTTP)
- Asynchronous server lifecycle management
- Real-time connection monitoring and health checks
- Automatic service reconnection and recovery

### Aggregator Service

- Request routing and aggregation across multiple MCP servers
- Tool, resource, and prompt aggregation from multiple servers
- HTTP-based MCP protocol server implementation
- Stateless session management

### Configuration Management

- JSON-based configuration stored in `~/.mcprouter/config.json`
- Server configuration (host, port, timeout, max connections)
- MCP server configurations with transport type support
- Logging and settings configuration

## Technology Stack

- **Backend**: Rust 1.70+, Tauri 2.x, rmcp library
- **Frontend**: React 19, TypeScript 5, Vite 7
- **UI**: Ant Design 5, Tailwind CSS 3
- **Platforms**: macOS, Windows, Linux

## Current Architecture

### Backend Structure

```
src-tauri/src/
├── aggregator.rs       # MCP aggregator server (HTTP-based)
├── mcp_manager.rs      # MCP server lifecycle management
├── mcp_client.rs       # MCP client connection handling
├── config/             # Configuration layer
├── commands/           # Tauri command handlers
├── marketplace/        # Marketplace providers
├── types.rs            # Shared type definitions
└── error.rs            # Error handling
```

### Key Components

#### McpAggregator (aggregator.rs)

- Implements rmcp ServerHandler trait
- Provides MCP protocol endpoints at `/mcp`
- Aggregates tools, resources, and prompts from multiple servers
- Handles initialize, list_tools, call_tool, list_prompts, get_prompt, list_resources, read_resource

#### ServerConfig (types.rs)

```rust
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub timeout_seconds: u64,
}
```

#### AppConfig (types.rs)

```rust
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: Option<LoggingSettings>,
    pub settings: Option<Settings>,
    pub mcp_servers: Vec<McpServerConfig>,
}
```

## Current Behavior

### Aggregator Startup

1. Loads configuration from `~/.mcprouter/config.json`
2. Creates HTTP server binding to `{host}:{port}` from ServerConfig
3. Exposes MCP protocol endpoints at `/mcp` route
4. Currently has no authentication or authorization

### Request Flow

1. Client sends HTTP request to `/mcp` endpoint
2. Request is processed by StreamableHttpService
3. McpAggregator handles the MCP protocol request
4. Response is returned to client

## Known Limitations

- No authentication or authorization on aggregator endpoints
- All requests to `/mcp` are currently unauthenticated
- Server configuration does not include authentication options
