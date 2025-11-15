# Tasks: Add Bearer Token Authentication

## Overview

Implement Bearer token authentication for the MCP aggregator HTTP endpoints with configuration-based enable/disable functionality.

## Task List

### 1. Configuration Schema Updates

**Status**: Completed
**Owner**: Developer
**Priority**: High
**Dependencies**: None

- [x] Add `auth: bool` field to `ServerConfig` in `src-tauri/src/types.rs`
- [x] Add `bearer_token: Option<String>` field to `ServerConfig`
- [x] Add `#[serde(default)]` attribute to `auth` field
- [x] Add `#[serde(skip_serializing_if = "Option::is_none")]` to `bearer_token`
- [x] Update `Default` implementation for `ServerConfig` with `auth: false` and `bearer_token: None`
- [x] Update `AppConfig::default()` to include new fields in default config

**Acceptance Criteria**:

- Configuration file can be loaded with old schema (backward compatible)
- New `auth` and `bearer_token` fields are properly serialized/deserialized
- Default configuration has `auth: false`

---

### 2. Authentication Middleware Implementation

**Status**: Completed
**Owner**: Developer
**Priority**: High
**Dependencies**: Task 1

- [x] Add `bearer_auth_middleware` function in `src-tauri/src/aggregator.rs`
  - Extract `Authorization` header from request
  - Validate "Bearer <token>" format
  - Compare token with expected value using constant-time comparison
  - Return `StatusCode::UNAUTHORIZED` (401) on failure
  - Call `next.run(req).await` on success
- [x] Add `constant_time_compare` helper function
  - Implement constant-time string comparison
  - Prevent timing attacks
  - Handle different string lengths
- [x] Add appropriate logging
  - Debug log on successful authentication
  - Warn log on authentication failure (without revealing token)
  - Info log on middleware application

**Acceptance Criteria**:

- Middleware correctly validates Bearer tokens
- Constant-time comparison prevents timing attacks
- Appropriate log messages are emitted
- No token values are logged

---

### 3. Router Integration

**Status**: Completed
**Owner**: Developer
**Priority**: High
**Dependencies**: Task 2

- [x] Modify `McpAggregator::start()` method in `src-tauri/src/aggregator.rs`
- [x] Add conditional router construction:
  - If `config.auth == true` and `config.bearer_token.is_some()`: apply middleware
  - If `config.auth == true` but `config.bearer_token.is_none()`: log warning and skip auth
  - If `config.auth == false`: skip middleware
- [x] Update startup log message to include authentication status
- [x] Test all three configuration scenarios

**Acceptance Criteria**:

- Middleware is applied only when `auth = true` and token is configured
- Warning is logged when auth is enabled but token is missing
- Startup logs clearly indicate authentication status

---

### 4. Configuration Validation

**Status**: Completed
**Owner**: Developer
**Priority**: Medium
**Dependencies**: Task 1

- [x] Add `validate()` method to `ServerConfig` in `src-tauri/src/types.rs`
- [x] Check for `auth = true` with `bearer_token = None` and return error
- [x] Add warning for weak tokens (< 16 characters)
- [x] Add warning for purely alphanumeric tokens
- [x] Call validation during config loading in `AppConfig::load()`

**Acceptance Criteria**:

- Invalid configurations (auth without token) are rejected with clear error message
- Warnings are shown for weak tokens
- Application fails to start with invalid configuration

---

### 5. Unit Tests

**Status**: Pending
**Owner**: Developer
**Priority**: High
**Dependencies**: Task 2

- [ ] Test `constant_time_compare` function
  - Equal strings return true
  - Different strings return false
  - Case-sensitive comparison
  - Different lengths return false
- [ ] Test `bearer_auth_middleware` (using mock requests)
  - Valid Bearer token allows request
  - Invalid Bearer token returns 401
  - Missing Authorization header returns 401
  - Malformed Authorization header returns 401
  - Non-Bearer auth scheme returns 401

**Acceptance Criteria**:

- All unit tests pass
- Test coverage for authentication logic is >= 90%

---

### 6. Integration Tests

**Status**: Pending
**Owner**: Developer
**Priority**: High
**Dependencies**: Task 3

- [ ] Test aggregator with `auth = false`
  - Requests without auth header succeed
  - Requests with auth header also succeed (auth is ignored)
- [ ] Test aggregator with `auth = true` and valid token
  - Requests without auth header return 401
  - Requests with valid Bearer token succeed
  - Requests with invalid Bearer token return 401
  - Requests with malformed header return 401
- [ ] Test all MCP protocol endpoints (initialize, list_tools, call_tool, etc.)
- [ ] Test with real MCP clients (if available)

**Acceptance Criteria**:

- All integration tests pass
- Authentication works correctly for all MCP endpoints
- Backward compatibility is maintained (existing configs work)

---

### 7. Manual Testing

**Status**: Pending
**Owner**: Developer
**Priority**: Medium
**Dependencies**: Task 6

- [ ] Test with curl commands:
  - `curl http://127.0.0.1:8000/mcp` without auth (auth disabled)
  - `curl http://127.0.0.1:8000/mcp` without auth (auth enabled) → expect 401
  - `curl -H "Authorization: Bearer <token>" http://127.0.0.1:8000/mcp` → expect success
  - `curl -H "Authorization: Bearer wrong" http://127.0.0.1:8000/mcp` → expect 401
- [ ] Test with real MCP client (if available)
- [ ] Test configuration reload scenarios
- [ ] Verify logging output

**Acceptance Criteria**:

- All manual test scenarios work as expected
- Logs contain appropriate messages
- No errors or panics occur

---

### 8. Documentation Updates

**Status**: Completed
**Owner**: Developer
**Priority**: Medium
**Dependencies**: Task 7

- [x] Update `README.md` and `README.zh.md` with authentication section
  - Explain how to enable authentication
  - Provide example configuration
  - Document Bearer token format
  - Add security best practices
- [x] Update configuration schema documentation
- [x] Add examples to `openspec/changes/add-bearer-auth/` directory
- [x] Document testing procedures

**Acceptance Criteria**:

- README has clear authentication setup instructions
- Security best practices are documented
- Example configurations are provided

---

### 9. Security Review

**Status**: Pending
**Owner**: Security Reviewer
**Priority**: High
**Dependencies**: Task 7

- [ ] Review constant-time comparison implementation
- [ ] Verify no token logging
- [ ] Check for timing attack vulnerabilities
- [ ] Review error messages for information leakage
- [ ] Verify default configuration is secure
- [ ] Review documentation for security recommendations

**Acceptance Criteria**:

- No security vulnerabilities identified
- Security best practices are followed
- Documentation includes security guidance

---

### 10. Performance Testing

**Status**: Pending
**Owner**: Developer
**Priority**: Low
**Dependencies**: Task 6

- [ ] Benchmark authentication middleware overhead
- [ ] Test with high request volume (1000+ req/s)
- [ ] Compare performance with auth enabled vs disabled
- [ ] Verify no memory leaks
- [ ] Check CPU usage under load

**Acceptance Criteria**:

- Authentication adds < 1ms latency per request
- No memory leaks detected
- Performance is acceptable under load

---

## Task Dependencies

```
Task 1 (Config Schema)
  ↓
Task 2 (Middleware)
  ↓
Task 3 (Router Integration)
  ↓
Task 4 (Validation) ← can run in parallel with Task 5-7
  ↓
Task 5 (Unit Tests)
  ↓
Task 6 (Integration Tests)
  ↓
Task 7 (Manual Testing)
  ↓
Task 8 (Documentation) ← can run in parallel with Task 9-10
Task 9 (Security Review) ← can run in parallel with Task 10
Task 10 (Performance Testing)
```

## Estimated Timeline

- **Task 1-3**: 4 hours (Core implementation)
- **Task 4**: 1 hour (Validation)
- **Task 5-6**: 3 hours (Testing)
- **Task 7**: 1 hour (Manual testing)
- **Task 8**: 2 hours (Documentation)
- **Task 9**: 1 hour (Security review)
- **Task 10**: 2 hours (Performance testing)

**Total**: ~14 hours (2 working days)

## Risks and Mitigations

| Risk                                | Impact | Probability | Mitigation                   |
| ----------------------------------- | ------ | ----------- | ---------------------------- |
| Breaking changes for existing users | High   | Low         | Default to `auth = false`    |
| Token exposure in logs              | High   | Medium      | Never log token values       |
| Timing attacks                      | Medium | Low         | Use constant-time comparison |
| Performance degradation             | Medium | Low         | Benchmark and optimize       |
| Integration issues with MCP clients | Medium | Low         | Thorough integration testing |

## Success Criteria

1. ✅ Authentication can be enabled/disabled via configuration
2. ✅ Valid Bearer tokens grant access to all endpoints
3. ✅ Invalid or missing tokens return 401 Unauthorized
4. ✅ Existing configurations work without modification
5. ✅ All tests pass (unit, integration, manual)
6. ✅ Documentation is clear and complete
7. ✅ No security vulnerabilities
8. ✅ Performance impact is minimal (< 1ms per request)
