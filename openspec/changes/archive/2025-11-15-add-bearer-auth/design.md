# Design: Bearer Token Authentication

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                       HTTP Client Request                    │
│              GET /mcp                         │
│              Authorization: Bearer <token>                   │
└────────────────────────┬────────────────────────────────────┘
                         │
                         v
┌─────────────────────────────────────────────────────────────┐
│                    Axum HTTP Server                          │
│  ┌───────────────────────────────────────────────────────┐  │
│  │           Authentication Middleware Layer             │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │  1. Extract Authorization header                 │  │  │
│  │  │  2. Validate "Bearer <token>" format             │  │  │
│  │  │  3. Compare token with configured value          │  │  │
│  │  │  4. Return 401 if invalid, continue if valid     │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────┬───────────────────────────────┘  │
│                          │ (if auth enabled)                │
│                          v                                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │         StreamableHttpService (MCP Protocol)          │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │            McpAggregator Handler                 │  │  │
│  │  │  - initialize()                                  │  │  │
│  │  │  - list_tools()                                  │  │  │
│  │  │  - call_tool()                                   │  │  │
│  │  │  - list_resources()                              │  │  │
│  │  │  - read_resource()                               │  │  │
│  │  │  - list_prompts()                                │  │  │
│  │  │  - get_prompt()                                  │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Configuration Layer

**File**: `src-tauri/src/types.rs`

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub timeout_seconds: u64,

    /// Enable authentication for aggregator endpoints
    /// Default: false (for backward compatibility)
    #[serde(default)]
    pub auth: bool,

    /// Bearer token for authentication
    /// Only used when auth = true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8000,
            max_connections: 100,
            timeout_seconds: 30,
            auth: false,
            bearer_token: None,
        }
    }
}
```

**Configuration File Example** (`~/.mcprouter/config.json`):

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8000,
    "max_connections": 100,
    "timeout_seconds": 30,
    "auth": true,
    "bearer_token": "mcp-secret-token-12345"
  },
  "logging": {
    "level": "info",
    "file_name": "mcprouter.log"
  },
  "mcp_servers": []
}
```

### 2. Authentication Middleware

**File**: `src-tauri/src/aggregator.rs`

```rust
use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Authentication middleware for Bearer token validation
async fn bearer_auth_middleware(
    req: Request,
    next: Next,
    expected_token: Arc<String>,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    // Validate Bearer token format and value
    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let provided_token = &header[7..]; // Skip "Bearer "

            // Constant-time comparison to prevent timing attacks
            if constant_time_compare(provided_token, expected_token.as_str()) {
                tracing::debug!("Authentication successful");
                Ok(next.run(req).await)
            } else {
                tracing::warn!("Authentication failed: invalid token");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        Some(_) => {
            tracing::warn!("Authentication failed: invalid Authorization header format");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            tracing::warn!("Authentication failed: missing Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }
    result == 0
}
```

### 3. Router Integration

**File**: `src-tauri/src/aggregator.rs` (in `start` method)

```rust
pub async fn start(
    self: &Arc<Self>,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("MCP Aggregator server starting...");

    // Build listen address from config
    let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    tracing::info!("Starting HTTP server on {}", addr);

    // Clone the Arc to pass to the service factory
    let aggregator_for_service = self.clone();

    // Create session manager
    let session_manager = Arc::new(LocalSessionManager::default());

    // Create service factory
    let service_factory = move || Ok(aggregator_for_service.as_ref().clone());

    // Create server config
    let server_config = StreamableHttpServerConfig {
        sse_keep_alive: Some(std::time::Duration::from_secs(self.config.timeout_seconds)),
        stateful_mode: false,
    };

    // Create StreamableHttpService
    let service = StreamableHttpService::new(service_factory, session_manager, server_config);

    // Build router with conditional authentication
    let router = if self.config.auth {
        if let Some(ref token) = self.config.bearer_token {
            tracing::info!("Bearer token authentication enabled");
            let token_arc = Arc::new(token.clone());

            axum::Router::new()
                .nest_service("/mcp", service)
                .layer(middleware::from_fn(move |req, next| {
                    bearer_auth_middleware(req, next, token_arc.clone())
                }))
        } else {
            tracing::warn!("Auth enabled but no bearer_token configured, authentication disabled");
            axum::Router::new().nest_service("/mcp", service)
        }
    } else {
        tracing::info!("Authentication disabled");
        axum::Router::new().nest_service("/mcp", service)
    };

    // Bind TCP listener and start server
    let tcp_listener = tokio::net::TcpListener::bind(addr).await?;
    let ct = CancellationToken::new();

    // Store cancellation token
    {
        let mut shutdown_guard = self.shutdown_signal.lock().unwrap();
        *shutdown_guard = Some(ct.clone());
    }

    // Spawn server task
    tokio::spawn({
        let ct = ct.clone();
        async move {
            tracing::info!("MCP Aggregator HTTP server running on {}", addr);
            let result = axum::serve(tcp_listener, router)
                .with_graceful_shutdown(async move {
                    ct.cancelled_owned().await;
                    tracing::info!("MCP Aggregator server shutting down...");
                })
                .await;

            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
            }
        }
    });

    tracing::info!(
        "MCP Aggregator started successfully on {} (auth: {}, timeout: {}s, max_connections: {})",
        addr,
        self.config.auth,
        self.config.timeout_seconds,
        self.config.max_connections
    );

    Ok(())
}
```

## Security Considerations

### 1. Token Storage

- **File Permissions**: Config file should be readable only by the user (chmod 600)
- **No Logging**: Never log the actual token value
- **Environment Variables**: Consider supporting token from environment variable

### 2. Token Validation

- **Constant-Time Comparison**: Prevents timing attacks
- **Length Check**: Quick rejection of obviously wrong tokens
- **No Error Details**: Don't reveal why authentication failed

### 3. Transport Security

- **HTTPS Recommended**: Bearer tokens should be transmitted over HTTPS
- **Localhost Binding**: Default to 127.0.0.1 to avoid network exposure
- **Network Isolation**: Deploy behind reverse proxy with TLS

### 4. Token Management

- **No Defaults**: Don't ship with default tokens
- **Strong Tokens**: Recommend cryptographically random tokens (32+ characters)
- **Manual Generation**: User must explicitly set token

## Performance Impact

1. **Minimal Overhead**: Single header lookup and string comparison
2. **Early Rejection**: Authentication happens before MCP protocol processing
3. **No Database Lookups**: Token is in-memory from config
4. **Constant-Time Comparison**: Prevents timing attacks without significant overhead

## Error Handling

```rust
// Authentication failure response
HTTP/1.1 401 Unauthorized
Content-Length: 0

// No additional error details to prevent information leakage
```

Log messages for debugging:

```
WARN  Authentication failed: missing Authorization header
WARN  Authentication failed: invalid Authorization header format
WARN  Authentication failed: invalid token
DEBUG Authentication successful
```

## Configuration Validation

Add validation in config loading:

```rust
impl ServerConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate auth configuration
        if self.auth && self.bearer_token.is_none() {
            return Err(ConfigError::Invalid(
                "Authentication enabled but no bearer_token configured".to_string()
            ));
        }

        // Validate token strength
        if let Some(ref token) = self.bearer_token {
            if token.len() < 16 {
                tracing::warn!(
                    "Bearer token is weak (length: {}). Recommend at least 32 characters.",
                    token.len()
                );
            }
            if token.chars().all(|c| c.is_ascii_alphanumeric()) {
                tracing::warn!(
                    "Bearer token contains only alphanumeric characters. \
                    Recommend using cryptographically random tokens."
                );
            }
        }

        Ok(())
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("secret", "secret"));
        assert!(!constant_time_compare("secret", "Secret"));
        assert!(!constant_time_compare("secret", "wrong"));
        assert!(!constant_time_compare("short", "longer"));
    }

    #[tokio::test]
    async fn test_auth_middleware_valid_token() {
        // Test with valid Bearer token
    }

    #[tokio::test]
    async fn test_auth_middleware_invalid_token() {
        // Test with invalid Bearer token
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_header() {
        // Test without Authorization header
    }

    #[tokio::test]
    async fn test_auth_middleware_malformed_header() {
        // Test with malformed Authorization header
    }
}
```

### Integration Tests

```bash
# Test 1: Request without auth (auth disabled)
curl -v http://127.0.0.1:8000/mcp/initialize

# Test 2: Request without auth (auth enabled)
curl -v http://127.0.0.1:8000/mcp/initialize
# Expected: 401 Unauthorized

# Test 3: Request with valid token (auth enabled)
curl -v -H "Authorization: Bearer mcp-secret-token-12345" \
  http://127.0.0.1:8000/mcp/initialize
# Expected: 200 OK

# Test 4: Request with invalid token (auth enabled)
curl -v -H "Authorization: Bearer wrong-token" \
  http://127.0.0.1:8000/mcp/initialize
# Expected: 401 Unauthorized

# Test 5: Request with malformed header (auth enabled)
curl -v -H "Authorization: NotBearer token" \
  http://127.0.0.1:8000/mcp/initialize
# Expected: 401 Unauthorized
```

## Migration and Deployment

### Step 1: Update Configuration Schema

- Add `auth` and `bearer_token` fields to ServerConfig
- Provide defaults for backward compatibility

### Step 2: Implement Middleware

- Add authentication middleware function
- Add constant-time comparison helper

### Step 3: Integrate Middleware

- Conditionally apply middleware based on config
- Add logging for auth status

### Step 4: Documentation

- Update README with authentication setup
- Add security best practices guide
- Provide example configurations

### Step 5: Testing

- Run unit tests
- Run integration tests
- Manual testing with real MCP clients

## Future Enhancements (Out of Scope)

1. **Multiple Tokens**: Support array of valid tokens for different clients
2. **Token Rotation**: Support token expiration and rotation
3. **Scoped Access**: Different tokens for different permission levels
4. **Audit Logging**: Track which tokens are used and when
5. **Environment Variable**: Support `MCPROUTER_BEARER_TOKEN` env var
6. **UI Integration**: Token management in settings page
