# Implementation Tasks

## Phase 1: Core Data Model Extension

### 1.1 Extend Token Structure

- [x] Add `allowed_tools: Option<Vec<String>>` field to Token struct
- [x] Add `allowed_resources: Option<Vec<String>>` field to Token struct
- [x] Add `allowed_prompts: Option<Vec<String>>` field to Token struct
- [x] Add serde attributes for backward compatibility (`#[serde(default, skip_serializing_if = "Option::is_none")]`)

**Dependencies**: None
**Validation**: Unit tests for Token serialization/deserialization with and without permission fields

### 1.2 Update Storage Version

- [x] Increment TokenStorage version from 1 to 2
- [x] Add migration logic for existing tokens without permission fields
- [x] Test backward compatibility loading of version 1 tokens

**Dependencies**: 1.1
**Validation**: Load existing token files and ensure they work with new structure

## Phase 2: Permission Validation Logic

### 2.1 Implement Pattern Matching

- [x] Create `PermissionMatcher` utility with wildcard support
- [x] Implement exact match: `server/tool` → specific tool
- [x] Implement server wildcard: `server/*` → all tools under server
- [x] Implement global wildcard: `*` → all tools
- [x] Implement resource path wildcard: `server/path/*` → all resources under path

**Dependencies**: 1.1
**Validation**: Comprehensive unit tests for pattern matching edge cases

### 2.2 Add Permission Methods to Token

- [x] Implement `has_tool_permission(&self, tool_name: &str) -> bool`
- [x] Implement `has_resource_permission(&self, resource_uri: &str) -> bool`
- [x] Implement `has_prompt_permission(&self, prompt_name: &str) -> bool`
- [x] Add permission validation logging for security auditing

**Dependencies**: 2.1
**Validation**: Unit tests covering all permission scenarios (allowed, denied, edge cases)

## Phase 3: Authentication Middleware Enhancement

### 3.1 Extend Authentication Middleware

- [x] Modify `dynamic_bearer_auth_middleware` to include permission validation
- [x] Extract MCP operation details from HTTP requests
- [x] Add permission check after token validation
- [x] Return 403 Forbidden for permission denied scenarios

**Dependencies**: 2.2
**Validation**: Integration tests with actual HTTP requests and various permission configurations

### 3.2 MCP Operation Extraction

- [x] Define `McpOperation` enum for different operation types
- [x] Implement request parsing to extract operation details
- [x] Handle tool names, resource URIs, and prompt names from request bodies
- [x] Add error handling for malformed requests
- [x] Implement SessionManager for connection-level permission caching
- [x] Implement AuthContext wrapper for RequestContext
- [x] Add ConnectionMapper for HTTP to MCP session passing

**Dependencies**: 3.1
**Validation**: Unit tests for request parsing with various MCP operation types

## Phase 4: Token Management API Extensions

### 4.1 Update Create Token API

- [x] Extend `CreateTokenRequest` with permission fields
- [x] Validate permission patterns during token creation
- [x] Add input validation for permission field formats
- [ ] Update frontend token creation form

**Dependencies**: 1.1
**Validation**: API tests creating tokens with various permission configurations

### 4.2 Update List/Get Token APIs

- [x] Include permission fields in `TokenInfo` responses
- [x] Add permission summary to token management UI
- [x] Implement get_available_permissions command for permission discovery
- [ ] Implement permission editing functionality

**Dependencies**: 4.1
**Validation**: UI tests for displaying and editing token permissions

## Phase 5: Aggregator Integration

### 5.1 Enhanced Aggregator Methods

- [x] Add permission checking helper methods to McpAggregator
- [x] Create token status validation functionality
- [x] Implement tool/resource/prompt permission validation methods
- [x] Modify `list_tools` to respect token permissions
- [x] Modify `call_tool` to validate permissions before execution
- [x] Modify `list_resources` and `read_resource` with permission checks
- [x] Modify `list_prompts` and `get_prompt` with permission checks

**Dependencies**: 3.2
**Validation**: End-to-end tests for all MCP operations with permission restrictions

### 5.2 Error Handling Enhancement

- [x] Define permission-specific error responses (PermissionError enum)
- [x] Add detailed error messages for permission denied scenarios
- [x] Implement structured error types for audit logging
- [ ] Implement audit logging for permission violations
- [ ] Add permission validation metrics

**Dependencies**: 5.1
**Validation**: Error handling tests and audit log verification

## Phase 6: Frontend Integration

### 6.1 Token Management UI

- [ ] Add permission input fields to token creation form
- [ ] Implement permission pattern validation in UI
- [ ] Add permission display to token list and detail views
- [ ] Create permission editing interface

**Dependencies**: 4.2
**Validation**: UI component tests and user interaction flows

### 6.2 Permission Help and Documentation

- [x] Remove permission help documentation page
- [x] Clean up permission-related imports from TokenManagement.tsx
- [x] Remove permission help button from token management interface
- [ ] Provide examples of common permission configurations
- [ ] Implement real-time permission validation feedback
- [ ] Add permission usage statistics display

**Dependencies**: 6.1
**Validation**: User acceptance testing and documentation review

## Phase 7: Testing and Validation

### 7.1 Unit Test Suite

- [ ] Token structure and serialization tests
- [ ] Permission pattern matching tests
- [ ] Permission validation logic tests
- [ ] Error handling and edge case tests

**Dependencies**: All previous phases
**Validation**: Code coverage >90% for new permission logic

### 7.2 Integration Test Suite

- [ ] End-to-end API tests with permission scenarios
- [ ] Middleware integration tests
- [ ] Aggregator permission validation tests
- [ ] Storage migration and backward compatibility tests

**Dependencies**: All previous phases
**Validation**: All integration tests passing with various permission configurations

### 7.3 Performance and Security Testing

- [ ] Permission validation performance benchmarks
- [ ] Memory usage impact assessment
- [ ] Security penetration testing for bypass attempts
- [ ] Audit log verification for permission violations

**Dependencies**: 7.2
**Validation**: Performance within acceptable limits and no security vulnerabilities

## Phase 8: Documentation and Deployment

### 8.1 API Documentation Updates

- [ ] Update OpenAPI/Swagger documentation with new fields
- [ ] Add permission configuration examples
- [ ] Document error response formats
- [ ] Create migration guide for existing deployments

**Dependencies**: 7.3
**Validation**: Documentation review and accuracy verification

### 8.2 Deployment Preparation

- [ ] Create database migration scripts
- [ ] Prepare deployment rollback procedures
- [ ] Update monitoring and alerting for permission violations
- [ ] Create operational runbooks for permission management

**Dependencies**: 8.1
**Validation**: Dry-run deployment testing and rollback procedure verification

## Risk Mitigation

### High Risk Items

1. **Backward Compatibility**: Ensure existing tokens continue working
2. **Performance Impact**: Monitor permission validation overhead
3. **Security Bypass**: Comprehensive security testing required

### Mitigation Strategies

1. **Gradual Rollout**: Deploy with feature flags for gradual enablement
2. **Comprehensive Testing**: Extensive test coverage before production
3. **Monitoring**: Detailed alerting for permission-related errors

### Rollback Plan

1. Feature flag to disable permission validation
2. Database migration rollback procedures
3. Code deployment with previous version compatibility

## Success Criteria

1. **Functional**: All permission scenarios work correctly
2. **Performance**: <10ms additional latency for permission validation
3. **Backward Compatibility**: Existing tokens work without modification
4. **Security**: No unauthorized access possible with permissions enabled
5. **Usability**: Clear UI for permission configuration and management
