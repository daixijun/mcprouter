# Token Permission Management Design

## Architecture Overview

The permission system extends the existing Token architecture with fine-grained access control while maintaining backward compatibility.

## Permission Model

### Permission Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    // ... existing fields ...
    pub allowed_tools: Option<Vec<String>>,
    pub allowed_resources: Option<Vec<String>>,
    pub allowed_prompts: Option<Vec<String>>,
}
```

### Permission Format

Permissions follow the `server/name` pattern to match MCP aggregation:

1. **Specific Item**: `filesystem/read_file` - allows only that specific tool
2. **Server Wildcard**: `filesystem/*` - allows all tools under filesystem server
3. **Global Wildcard**: `*` - allows all tools/resources/prompts
4. **Resource Path**: `filesystem/logs/*` - allows all resources under logs path

## Implementation Details

### 1. Token Structure Extension

The Token struct is extended with optional permission fields:

- `Option<Vec<String>>` for backward compatibility
- `None` means unrestricted access (existing behavior)
- `Some(vec)` means restricted access to only the listed items

### 2. Permission Validation Logic

```rust
impl Token {
    pub fn has_tool_permission(&self, tool_name: &str) -> bool {
        match &self.allowed_tools {
            None => true, // No restrictions
            Some(allowed) => {
                allowed.iter().any(|pattern| self.matches_pattern(pattern, tool_name))
            }
        }
    }

    fn matches_pattern(&self, pattern: &str, item: &str) -> bool {
        match pattern {
            "*" => true,
            _ if pattern.ends_with("/*") => {
                let server = &pattern[..pattern.len()-2];
                item.starts_with(&format!("{}/", server))
            }
            _ => pattern == item
        }
    }
}
```

### 3. Enhanced Authentication Middleware

The existing `dynamic_bearer_auth_middleware` will be extended with permission validation:

```rust
async fn enhanced_auth_middleware(
    req: Request,
    next: Next,
    token_manager: Arc<TokenManager>,
) -> Result<Response, StatusCode> {
    // 1. Validate token (existing logic)
    let (token_id, token) = validate_and_get_token(req, &token_manager).await?;

    // 2. Extract MCP operation from request
    let operation = extract_mcp_operation(&req)?;

    // 3. Validate permissions
    if !token.has_permission(&operation) {
        tracing::warn!("Permission denied for token {} on {}", token_id, operation);
        return Err(StatusCode::FORBIDDEN);
    }

    // 4. Proceed with request
    Ok(next.run(req).await)
}
```

### 4. Permission Extraction from MCP Requests

```rust
enum McpOperation {
    ListTools,
    CallTool(String),           // tool name
    ListResources,
    ReadResource(String),       // resource URI
    ListPrompts,
    GetPrompt(String),          // prompt name
}

fn extract_mcp_operation(req: &Request) -> Result<McpOperation, StatusCode> {
    // Parse MCP request body to extract operation details
    // This integrates with existing request parsing logic
}
```

## Integration Points

### 1. TokenManager Extensions

```rust
impl TokenManager {
    pub async fn validate_token_with_permissions(
        &self,
        token_value: &str,
        operation: &McpOperation
    ) -> Option<(String, Token)> {
        if let Some(token_id) = self.validate_token(token_value).await {
            if let Ok(token) = self.get_token_by_id(&token_id).await {
                if token.has_permission(operation) {
                    return Some((token_id, token));
                }
            }
        }
        None
    }
}
```

### 2. Aggregator Integration

The `McpAggregator` will be enhanced to use permission validation:

```rust
impl McpAggregator {
    async fn call_tool_with_permissions(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, RmcpErrorData> {
        // Extract token from context (added by middleware)
        let token = extract_token_from_context(&context)?;

        // Validate permission
        if !token.has_tool_permission(&request.name) {
            return Err(RmcpErrorData::new(
                ErrorCode(403),
                "Tool access denied".to_string(),
                None,
            ));
        }

        // Proceed with existing tool call logic
        self.call_tool_internal(request, context).await
    }
}
```

## Migration Strategy

### Phase 1: Data Structure Extension

1. Add optional permission fields to Token struct
2. Update storage version to 2
3. Implement backward compatibility migration

### Phase 2: Permission Validation

1. Implement permission matching logic
2. Extend authentication middleware
3. Add operation extraction from MCP requests

### Phase 3: UI Integration

1. Add permission configuration to token creation/editing
2. Implement permission validation feedback
3. Add permission usage statistics

## Security Considerations

### 1. Default Behavior

- Tokens without permission fields maintain full access (backward compatibility)
- New tokens with permission fields are restricted by default

### 2. Permission Inheritance

- Permissions are explicit whitelist approach
- No inheritance or implicit permissions
- Wildcard permissions only apply to specified level

### 3. Performance

- Permission validation is O(n) where n is number of permission patterns
- Early rejection for clearly unauthorized requests
- Minimal overhead for tokens without restrictions

## Error Handling

### Permission Denied Response

```rust
pub struct PermissionError {
    pub token_id: String,
    pub operation: McpOperation,
    pub reason: String,
}
```

### Logging and Monitoring

- Log permission denials for security auditing
- Track permission validation performance
- Monitor patterns of access attempts
