# Token Permission Management

## Why

The current token system provides basic authentication but lacks fine-grained access control. As MCPRouter aggregates multiple MCP servers, there's a growing need to restrict token access to specific tools, resources, and prompts for security and multi-tenant scenarios. This enhancement will enable administrators to create tokens with limited scope, improving security posture and enabling new use cases like per-client access control.

## What Changes

### Core Implementation
- **Session Management**: Add SessionManager and SessionInfo for connection-level permission caching
- **Auth Context**: Implement AuthContext wrapper for RequestContext to provide permission validation
- **Permission Validation**: Add fine-grained permission checking in MCP aggregator methods
- **Token Enhancement**: Extend Token structure with allowed_tools, allowed_resources, allowed_prompts fields
- **UI Updates**: Remove permission help documentation, streamline token management interface

### New Components
- `session_manager.rs`: Session-based permission caching system
- `auth_context.rs`: Permission validation wrapper for MCP requests
- `connection_mapper.rs`: HTTP to MCP connection mapping for session passing
- Enhanced `aggregator.rs`: Permission filtering in all MCP protocol methods

### API Changes
- Add `get_available_permissions` command for permission discovery
- Update `list_tools`, `list_resources`, `list_prompts` to filter by permissions
- Enhanced token CRUD operations with permission fields

### Files Modified/Created
- **New**: `src-tauri/src/session_manager.rs`, `src-tauri/src/auth_context.rs`, `src-tauri/src/connection_mapper.rs`
- **Modified**: `src-tauri/src/aggregator.rs`, `src-tauri/src/token_manager.rs`, `src/pages/TokenManagement.tsx`
- **Removed**: `src/pages/PermissionDocumentation.tsx`

### Migration Impact
- **Backward Compatible**: Existing tokens without permission fields retain full access
- **Default Behavior**: New tokens default to unrestricted access unless permissions specified
- **Database**: JSON token storage extended with optional permission arrays

## Overview

Add fine-grained permission management capabilities to the existing Token system, enabling access control at the tools/resources/prompts level with support for server-specific permissions using the `server/tool` format.

## Scope

### In Scope

- Extend Token structure with permission fields for tools, resources, and prompts
- Implement permission validation middleware in the MCP aggregator
- Add token management UI for configuring permissions
- Support both wildcard and specific permission patterns
- Backward compatibility with existing tokens

### Out of Scope

- Role-based access control (RBAC) system
- User management and authentication
- Permission inheritance or grouping
- Audit logging for permission violations

## Architecture

### Permission Format

Permissions use the `server/name` format to match the existing MCP aggregation pattern:

- Tools: `filesystem/read_file`, `database/query_users`
- Resources: `filesystem/logs`, `database/users`
- Prompts: `codegen/generate`, `analysis/summarize`

### Permission Fields

```rust
pub struct Token {
    // ... existing fields ...
    pub allowed_tools: Option<Vec<String>>,      // e.g., ["filesystem/*", "database/query"]
    pub allowed_resources: Option<Vec<String>>,  // e.g., ["filesystem/logs/*"]
    pub allowed_prompts: Option<Vec<String>>,    // e.g., ["codegen/*"]
}
```

### Wildcard Support

- `*` matches all servers/tools/resources/prompts
- `server/*` matches all items under a specific server
- `server/tool` matches a specific item

## Implementation Approach

1. **Data Model Extension**: Add permission fields to Token struct with default None for backward compatibility
2. **Permission Validation**: Extend the existing authentication middleware to check permissions after token validation
3. **Permission Matching**: Implement pattern matching with wildcard support
4. **UI Integration**: Add permission configuration to the existing token management interface

## Dependencies

- Existing TokenManager and authentication middleware
- Current MCP aggregator architecture
- Existing token storage format (JSON)

## Migration Strategy

- Existing tokens without permission fields will have full access (backward compatibility)
- New tokens can optionally specify permission restrictions
- Database schema version increment for the new fields
