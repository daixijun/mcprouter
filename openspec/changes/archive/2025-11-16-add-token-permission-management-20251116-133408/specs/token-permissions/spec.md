# Token Permissions Specification

## ADDED Requirements

### Requirement: Token Permission Structure Extension

The system SHALL extend the Token data structure to support fine-grained permission control for tools, resources, and prompts.

#### Scenario: Admin creates a token with restricted tool access

Given an administrator wants to create a token with limited access
When they create a new token with `allowed_tools: ["filesystem/read_file", "database/query"]`
Then the token should only allow access to those specific tools
And all other tools should return permission denied

#### Scenario: Existing token without permission fields

Given an existing token in the database without permission fields
When the system validates this token
Then it should have full access to all tools, resources, and prompts (backward compatibility)

### Requirement: Permission Format and Pattern Matching

The system SHALL support flexible permission patterns including wildcards and server-level access control.

#### Scenario: Token with server-level wildcard permission

Given a token with `allowed_tools: ["filesystem/*"]`
When the user tries to access `filesystem/read_file` or `filesystem/write_file`
Then both operations should be allowed
And access to `database/query` should be denied

#### Scenario: Token with global wildcard permission

Given a token with `allowed_resources: ["*"]`
When the user tries to access any resource
Then the access should be allowed
Regardless of the server or specific resource

#### Scenario: Token with specific resource path permission

Given a token with `allowed_resources: ["filesystem/logs/*"]`
When the user tries to access `filesystem/logs/app.log`
Then the access should be allowed
And access to `filesystem/config/settings.json` should be denied

### Requirement: Permission Validation Middleware

The system SHALL implement permission validation in the authentication middleware to enforce access control.

#### Scenario: Permission denied for unauthorized tool access

Given a token with `allowed_tools: ["filesystem/read_file"]`
When the user tries to call `database/query`
Then the system should return a 403 Forbidden error
With a clear message indicating permission denied

#### Scenario: Successful permission validation

Given a token with `allowed_prompts: ["codegen/generate"]`
When the user tries to get the `codegen/generate` prompt
Then the system should allow the operation
And return the prompt content normally

### Requirement: Token Management UI Extensions

The system SHALL extend the token management interface to support permission configuration.

#### Scenario: Admin creates token with permissions via UI

Given an administrator using the token management interface
When they create a new token
Then they should be able to specify allowed tools, resources, and prompts
Using pattern matching syntax with wildcards

#### Scenario: Admin edits existing token permissions

Given an existing token with permissions
When the administrator edits the token
Then they should be able to modify the permission lists
And the changes should be immediately applied

### Requirement: Token Storage Format

The system SHALL support storing permission fields while maintaining backward compatibility.

#### Scenario: Storage format migration

Given existing tokens in the storage without permission fields
When the system starts up
Then it should load all tokens successfully
Treat missing permission fields as None (unrestricted access)

#### Scenario: New token with permissions storage

Given a newly created token with specified permissions
When the token is saved to storage
Then all permission fields should be persisted correctly
And be available when the token is loaded

### Requirement: Token Validation Enhancement

The system SHALL extend the token validation process to include permission checks.

#### Scenario: Token validation with permission check

Given a valid token with restricted permissions
When the system validates the token for a specific MCP operation
Then it should return both the token information and permission validation result
Allowing the aggregator to make authorization decisions

## Technical Requirements

### Permission Pattern Matching Rules

1. **Exact Match**: `server/tool` matches only that specific tool
2. **Server Wildcard**: `server/*` matches all tools under the specified server
3. **Global Wildcard**: `*` matches all tools/resources/prompts
4. **Resource Path Wildcard**: `server/path/*` matches all resources under the specified path

### Error Handling

1. **Permission Denied**: Return HTTP 403 with descriptive error message
2. **Invalid Pattern**: Reject token creation with invalid permission patterns
3. **Malformed Request**: Return HTTP 400 for requests with invalid MCP operation format

### Performance Requirements

1. **Validation Speed**: Permission validation should complete within 10ms
2. **Memory Usage**: Permission patterns should not significantly increase memory footprint
3. **Backward Compatibility**: Tokens without permissions should not incur additional overhead

### Security Requirements

1. **Whitelist Approach**: Permissions are explicit whitelists, not blacklists
2. **No Implicit Access**: Absence of permission means denial
3. **Audit Trail**: All permission denials should be logged for security monitoring

### Data Structure Changes

```rust
// Modified Token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_resources: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_prompts: Option<Vec<String>>,
}
```

### API Extensions

```rust
// New token creation request with permissions
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub description: Option<String>,
    pub expires_in: Option<u64>,
    pub allowed_tools: Option<Vec<String>>,
    pub allowed_resources: Option<Vec<String>>,
    pub allowed_prompts: Option<Vec<String>>,
}
```

## Validation Criteria

### Functional Testing

1. Create token with specific permissions and verify access control
2. Test wildcard permission patterns work correctly
3. Verify backward compatibility with existing tokens
4. Test permission denial scenarios and error responses
5. Validate UI integration for permission management

### Performance Testing

1. Measure permission validation overhead
2. Test system performance with large numbers of permission patterns
3. Verify memory usage remains within acceptable limits

### Security Testing

1. Attempt to bypass permission restrictions
2. Test for permission escalation vulnerabilities
3. Verify audit logging captures all permission denials
4. Test edge cases of permission pattern matching
