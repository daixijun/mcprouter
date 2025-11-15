# Spec: Aggregator Authentication

## Overview

This specification defines the authentication requirements for the MCP aggregator HTTP endpoints.

## ADDED Requirements

### Requirement: Configuration-Based Authentication Control

The system SHALL allow administrators to enable or disable authentication for aggregator endpoints via configuration.

#### Scenario: Authentication Disabled by Default

**Given** the application is using default configuration
**When** the aggregator server starts
**Then** authentication SHALL be disabled
**And** all HTTP requests to `/mcp` endpoints SHALL succeed without authentication headers

#### Scenario: Enable Authentication via Configuration

**Given** the configuration file contains `"server": { "auth": true, "bearer_token": "secret123" }`
**When** the aggregator server starts
**Then** authentication SHALL be enabled
**And** HTTP requests without valid Bearer token SHALL be rejected with 401 Unauthorized

#### Scenario: Authentication Enabled Without Token

**Given** the configuration file contains `"server": { "auth": true }`
**And** no `bearer_token` is specified
**When** the application attempts to load configuration
**Then** a warning SHALL be logged
**And** authentication SHALL be disabled
**Or** the application MAY refuse to start with a configuration error

---

### Requirement: Bearer Token Validation

The system SHALL validate Bearer tokens according to RFC 6750 standards when authentication is enabled.

#### Scenario: Valid Bearer Token Authentication

**Given** authentication is enabled with token "mcp-secret-token"
**When** a client sends request with header `Authorization: Bearer mcp-secret-token`
**Then** the request SHALL be processed normally
**And** the response SHALL have status code 200 (or appropriate MCP response)

#### Scenario: Invalid Bearer Token Rejection

**Given** authentication is enabled with token "mcp-secret-token"
**When** a client sends request with header `Authorization: Bearer wrong-token`
**Then** the request SHALL be rejected
**And** the response SHALL have status code 401 Unauthorized
**And** no further processing SHALL occur

#### Scenario: Missing Authorization Header

**Given** authentication is enabled
**When** a client sends request without Authorization header
**Then** the request SHALL be rejected
**And** the response SHALL have status code 401 Unauthorized

#### Scenario: Malformed Authorization Header

**Given** authentication is enabled with token "mcp-secret-token"
**When** a client sends request with header `Authorization: NotBearer mcp-secret-token`
**Then** the request SHALL be rejected
**And** the response SHALL have status code 401 Unauthorized

#### Scenario: Case-Sensitive Token Comparison

**Given** authentication is enabled with token "SecretToken"
**When** a client sends request with header `Authorization: Bearer secrettoken`
**Then** the request SHALL be rejected
**And** the response SHALL have status code 401 Unauthorized

---

### Requirement: Security Best Practices

The system SHALL implement authentication in a secure manner following industry best practices.

#### Scenario: Constant-Time Token Comparison

**Given** authentication is enabled
**When** the system compares a provided token with the configured token
**Then** the comparison SHALL use constant-time algorithm
**And** the comparison SHALL NOT reveal information about token correctness through timing

#### Scenario: No Token Logging

**Given** authentication is enabled with a bearer token
**When** any log message is emitted
**Then** the actual token value SHALL NOT appear in any log message
**And** only generic authentication status SHALL be logged (success/failure)

#### Scenario: Secure Configuration File Permissions

**Given** the configuration file contains a bearer token
**When** the application starts
**Then** a warning SHOULD be logged if file permissions are too permissive
**And** the documentation SHALL recommend chmod 600 for the config file

---

### Requirement: Backward Compatibility

The system SHALL maintain backward compatibility with existing configurations that do not specify authentication settings.

#### Scenario: Existing Configuration Without Auth Fields

**Given** a configuration file without `auth` or `bearer_token` fields
**When** the application loads the configuration
**Then** the configuration SHALL load successfully
**And** authentication SHALL be disabled
**And** no errors or warnings SHALL be emitted

#### Scenario: Configuration Migration

**Given** an existing configuration file from a previous version
**When** the application upgrades to a version with authentication support
**Then** the existing configuration SHALL continue to work
**And** the aggregator SHALL function identically to before the upgrade

---

### Requirement: All Endpoints Protected

The system SHALL apply authentication to all MCP aggregator endpoints when enabled.

#### Scenario: Initialize Endpoint Authentication

**Given** authentication is enabled
**When** a client requests the initialize endpoint (`POST /mcp`)
**Then** the request SHALL require valid Bearer token authentication

#### Scenario: List Tools Endpoint Authentication

**Given** authentication is enabled
**When** a client requests list_tools (`POST /mcp` with list_tools method)
**Then** the request SHALL require valid Bearer token authentication

#### Scenario: Call Tool Endpoint Authentication

**Given** authentication is enabled
**When** a client requests call_tool (`POST /mcp` with call_tool method)
**Then** the request SHALL require valid Bearer token authentication

#### Scenario: List Resources Endpoint Authentication

**Given** authentication is enabled
**When** a client requests list_resources (`POST /mcp` with list_resources method)
**Then** the request SHALL require valid Bearer token authentication

#### Scenario: Read Resource Endpoint Authentication

**Given** authentication is enabled
**When** a client requests read_resource (`POST /mcp` with read_resource method)
**Then** the request SHALL require valid Bearer token authentication

#### Scenario: List Prompts Endpoint Authentication

**Given** authentication is enabled
**When** a client requests list_prompts (`POST /mcp` with list_prompts method)
**Then** the request SHALL require valid Bearer token authentication

#### Scenario: Get Prompt Endpoint Authentication

**Given** authentication is enabled
**When** a client requests get_prompt (`POST /mcp` with get_prompt method)
**Then** the request SHALL require valid Bearer token authentication

---

### Requirement: Configuration Validation

The system SHALL validate authentication-related configuration at startup.

#### Scenario: Valid Authentication Configuration

**Given** configuration with `"auth": true` and `"bearer_token": "valid-token-here"`
**When** the application loads configuration
**Then** validation SHALL succeed
**And** the application SHALL start normally

#### Scenario: Weak Token Warning

**Given** configuration with `"bearer_token": "weak"` (less than 16 characters)
**When** the application loads configuration
**Then** a warning SHALL be logged recommending stronger tokens
**And** the application MAY still start with the weak token

#### Scenario: Token Strength Recommendation

**Given** configuration with a purely alphanumeric token
**When** the application loads configuration
**Then** a warning SHOULD be logged recommending cryptographically random tokens
**And** the documentation SHALL provide token generation examples

---

### Requirement: Logging and Monitoring

The system SHALL log authentication events for security monitoring and debugging.

#### Scenario: Successful Authentication Logging

**Given** authentication is enabled
**When** a client successfully authenticates
**Then** a DEBUG level log message SHALL be emitted indicating successful authentication
**And** the log SHALL NOT contain the token value

#### Scenario: Failed Authentication Logging

**Given** authentication is enabled
**When** a client fails authentication
**Then** a WARN level log message SHALL be emitted indicating authentication failure
**And** the log SHALL include the failure reason (missing header, invalid format, or wrong token)
**And** the log SHALL NOT contain the actual token values

#### Scenario: Startup Authentication Status Logging

**Given** the application is starting
**When** the aggregator server initializes
**Then** an INFO level log message SHALL indicate whether authentication is enabled or disabled
**And** if enabled, the log SHALL confirm that a token is configured (without revealing the token)

---

## MODIFIED Requirements

None. This is a new capability with no modifications to existing requirements.

---

## REMOVED Requirements

None. No existing requirements are being removed.

---

## Cross-References

- Related to: Aggregator HTTP Server (existing capability)
- Depends on: Configuration Management (existing capability)
- Impacts: All MCP protocol endpoints exposed by the aggregator

---

## Security Considerations

1. **Token Storage**: Configuration file containing tokens SHOULD have restrictive permissions (chmod 600)
2. **Transport Security**: Bearer tokens SHOULD be transmitted over HTTPS or localhost-only connections
3. **Token Strength**: Tokens SHOULD be cryptographically random with minimum 32 characters
4. **Timing Attacks**: Token comparison MUST use constant-time algorithms
5. **Information Disclosure**: Error responses MUST NOT reveal why authentication failed
6. **Logging**: Token values MUST NEVER appear in log files

---

## Performance Considerations

1. **Minimal Overhead**: Authentication check adds < 1ms latency per request
2. **In-Memory Validation**: No database lookups or external service calls
3. **Early Rejection**: Invalid requests rejected before MCP protocol processing
4. **No Caching**: Token is read from config once at startup

---

## Future Enhancements (Out of Scope)

1. Multiple tokens for different access levels
2. Token rotation and expiration
3. Integration with external authentication providers
4. Rate limiting per token
5. Audit logging of token usage
