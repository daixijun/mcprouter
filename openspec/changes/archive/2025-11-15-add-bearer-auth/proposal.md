# Proposal: Add Bearer Token Authentication to Aggregator

## Metadata

- **Change ID**: add-bearer-auth
- **Status**: Proposed
- **Created**: 2025-11-15
- **Priority**: High

## Summary

Add Bearer token authentication to the MCP aggregator HTTP endpoints to secure access to the aggregated tools, resources, and prompts. Authentication can be enabled/disabled via the `server.auth` configuration option.

## Motivation

Currently, the MCP aggregator exposes its HTTP endpoints (`/mcp`) without any authentication mechanism. This means anyone who can reach the server's network address can access all aggregated MCP tools, resources, and prompts. This presents a security risk in scenarios where:

1. The aggregator is exposed to untrusted networks
2. Multiple users or applications share the same network
3. Sensitive tools or resources need access control
4. Production deployments require authenticated access

## Goals

1. **Security**: Add Bearer token authentication to protect aggregator endpoints
2. **Configuration**: Allow enabling/disabling auth via `server.auth` config option
3. **Compatibility**: Maintain backward compatibility by defaulting to `auth = false`
4. **Standards**: Follow HTTP Bearer authentication standards (RFC 6750)
5. **Simplicity**: Keep implementation simple with static token validation

## Non-Goals

1. Complex authentication schemes (OAuth, JWT validation, etc.)
2. User management or multi-user support
3. Token rotation or expiration
4. Fine-grained authorization (all authenticated users have full access)
5. API key management UI

## Proposed Solution

### Configuration Changes

Extend `ServerConfig` to include authentication options:

```rust
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub timeout_seconds: u64,
    // New fields
    #[serde(default)]
    pub auth: bool,  // Enable/disable authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,  // Bearer token value
}
```

Default configuration (`config.json`):

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8000,
    "max_connections": 100,
    "timeout_seconds": 30,
    "auth": false,
    "bearer_token": null
  }
}
```

Enabled authentication example:

```json
{
  "server": {
    "auth": true,
    "bearer_token": "your-secret-token-here"
  }
}
```

### Implementation Strategy

1. **Middleware Approach**: Create an authentication middleware layer that intercepts requests before they reach the MCP handler
2. **Header Validation**: Extract and validate `Authorization: Bearer <token>` header
3. **Conditional Application**: Only apply middleware when `server.auth = true`
4. **Error Responses**: Return proper HTTP 401 Unauthorized on authentication failures

### Code Changes

#### 1. Update ServerConfig (types.rs)

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub auth: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,
}
```

#### 2. Create Authentication Middleware (aggregator.rs)

```rust
use axum::{
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
};

async fn auth_middleware<B>(
    req: Request<B>,
    next: Next<B>,
    token: Arc<Option<String>>,
) -> Result<Response, StatusCode> {
    // Skip auth if no token configured
    let Some(expected_token) = token.as_ref() else {
        return Ok(next.run(req).await);
    };

    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    // Validate Bearer token
    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let provided_token = &header[7..];
            if provided_token == expected_token {
                Ok(next.run(req).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
```

#### 3. Apply Middleware (aggregator.rs start method)

```rust
// Create router with conditional auth middleware
let router = if self.config.auth && self.config.bearer_token.is_some() {
    let token = Arc::new(self.config.bearer_token.clone());
    axum::Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn(move |req, next| {
            auth_middleware(req, next, token.clone())
        }))
} else {
    axum::Router::new().nest_service("/mcp", service)
};
```

### Testing Strategy

1. **Unit Tests**: Test auth middleware with valid/invalid tokens
2. **Integration Tests**: Test full aggregator with auth enabled/disabled
3. **Manual Testing**: Test with real MCP clients and curl commands

Example test cases:

- Request without auth when `auth = false` → Success
- Request without auth when `auth = true` → 401 Unauthorized
- Request with valid token when `auth = true` → Success
- Request with invalid token when `auth = true` → 401 Unauthorized
- Request with malformed header when `auth = true` → 401 Unauthorized

## Migration Path

1. **Backward Compatibility**: Existing configurations will work unchanged (defaults to `auth = false`)
2. **Gradual Adoption**: Users can enable auth by adding `auth: true` and `bearer_token` to their config
3. **Documentation**: Update README with authentication setup instructions

## Risks and Mitigations

| Risk                                | Mitigation                                                   |
| ----------------------------------- | ------------------------------------------------------------ |
| Token exposure in config file       | Document best practices for file permissions (chmod 600)     |
| Token transmitted in clear text     | Recommend using HTTPS or localhost-only bindings             |
| Breaking changes for existing users | Default to `auth = false` for backward compatibility         |
| Performance overhead                | Middleware is lightweight and only runs when auth is enabled |

## Alternatives Considered

1. **API Keys**: Similar to Bearer tokens but less standardized
2. **OAuth/OIDC**: Too complex for the use case
3. **mTLS**: Requires certificate management, overkill for simple auth
4. **Basic Auth**: Less secure than Bearer tokens for API access

## Implementation Phases

### Phase 1: Core Implementation (This Change)

- Add `auth` and `bearer_token` to ServerConfig
- Implement authentication middleware
- Apply middleware conditionally based on config
- Update default configuration

### Phase 2: Future Enhancements (Out of Scope)

- Token rotation support
- Multiple tokens for different access levels
- UI for token management
- Integration with external auth providers

## Dependencies

- `axum` crate (already in use)
- `serde` for configuration (already in use)
- No new external dependencies required

## Success Criteria

1. Authentication can be enabled via configuration
2. Valid Bearer tokens grant access to all endpoints
3. Invalid or missing tokens are rejected with 401
4. Existing users without auth config continue to work
5. All tests pass
6. Documentation is updated

## Related Changes

None. This is a standalone security enhancement.

## References

- [RFC 6750: The OAuth 2.0 Authorization Framework: Bearer Token Usage](https://datatracker.ietf.org/doc/html/rfc6750)
- [Axum Middleware Documentation](https://docs.rs/axum/latest/axum/middleware/index.html)
