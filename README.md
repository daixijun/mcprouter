# MCPRouter - MCP Router

A modern MCP (Model Context Protocol) Router built with Tauri, React and Typescript, now powered by SQLite database for enhanced performance and reliability.

## Features

- 🚀 **High Performance**: SQLite database with optimized queries and indexing
- 🔐 **Secure**: API key storage with SHA-256 hashing
- 🔧 **Fine-grained Control**: Tool-level enable/disable management
- 📊 **Reliable**: ACID-compliant transactions and data consistency
- 🛡️ **Scalable**: Clean architecture supporting large-scale deployments

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Configuration

### Database Architecture

MCPRouter now uses SQLite for data persistence instead of configuration files:

- **mcp_servers**: MCP server configurations with metadata
- **api_keys**: Secure API key storage with hash encoding
- **tools**: Tool-level status management per server
- **api_key_server_relations**: Many-to-many relationships for permissions

### Migration

Existing configuration files are automatically migrated to the new database structure on first run.

## Quick Start

1. **Install Dependencies**: `npm install` (or `pnpm install`)
2. **Development Mode**: `npm run tauri dev`
3. **Build**: `npm run tauri build`

## Project Structure

```
src-tauri/
├── src/
│   ├── main.rs              # Application entry point
│   ├── db/                  # Database layer
│   │   ├── connection.rs      # Database connection management
│   │   ├── models.rs          # Data models
│   │   ├── repositories/     # CRUD operations
│   │   └── migration.rs      # Schema migrations
│   ├── mcp_manager.rs       # MCP server management
│   ├── api_key_manager.rs   # API key management
│   └── aggregator.rs        # Request aggregation
│   └── ...
src/
├── components/              # React components
│   └── ...
└── pages/                  # React pages
    └── ...
```

## Development

### Database Schema

MCPRouter uses SQLite with the following schema:

#### mcp_servers table
```sql
CREATE TABLE mcp_servers (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    command TEXT,
    args TEXT, -- JSON array
    transport TEXT NOT NULL,
    url TEXT,
    enabled BOOLEAN DEFAULT TRUE,
    env_vars TEXT, -- JSON object
    headers TEXT, -- JSON object
    version TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### tools table
```sql
CREATE TABLE tools (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    server_id TEXT NOT NULL,
    description TEXT,
    enabled BOOLEAN DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### api_keys table
```sql
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL, -- SHA-256 hash
    enabled BOOLEAN DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### api_key_server_relations table
```sql
CREATE TABLE api_key_server_relations (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    server_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

See `src-tauri/src/db/models.rs` for complete database schema documentation.

### API Endpoints

All MCP and API key management endpoints are available through Tauri commands. See `src-tauri/src/lib.rs` for the complete API surface.

### Testing

```bash
# Run the application
npm run tauri dev

# Run tests (when available)
npm test
```

## Security

- API keys are stored as SHA-256 hashes
- Database transactions ensure ACID compliance
- Input validation on all API endpoints
- Secure defaults for all configurations

## Performance

- Indexed database queries for optimal lookup speed
- Connection pooling for efficient resource usage
- Removed caching layer for reduced memory footprint
- Optimized startup time with direct database access
