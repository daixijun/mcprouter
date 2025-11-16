# aggregator-auth Specification

## Purpose
TBD - created by archiving change add-bearer-auth. Update Purpose after archive.
## Requirements
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

### Requirement: Migration Support

系统 SHALL 支持从旧的静态配置迁移到动态 Token 管理，提供灵活的迁移选项。

#### Scenario: Legacy Configuration Detection

**Given** 用户的配置文件包含旧的 `server.auth` 和 `server.bearer_token` 字段
**When** 应用程序启动
**Then** 系统 SHALL 检测到遗留配置
**And** 记录 INFO 级别的日志提示存在旧配置
**And** 在日志中显示迁移指导链接
**And** 系统 SHALL 正常启动并使用动态认证

#### Scenario: Manual Migration Guidance

**Given** 系统检测到遗留的静态配置
**When** 用户查看日志或管理界面
**Then** 系统 SHALL 提供清晰的迁移步骤:
  1. 打开 Token 管理界面
  2. 创建新的动态 token
  3. 更新客户端配置使用新 token
  4. 删除配置文件中的 `server.auth` 和 `server.bearer_token` 字段
**And** 提供 curl 示例展示新旧配置的差异

#### Scenario: Optional Automatic Migration

**Given** 系统检测到遗留配置且 TokenManager 为空
**When** 应用程序启动
**Then** 系统 MAY 提供自动迁移选项
**And** 如果启用自动迁移:
  - 创建名称为"从配置迁移的 Token"的动态 token
  - 继承原有的 bearer_token 值
  - 记录迁移完成日志
**And** 系统 SHALL 始终允许用户禁用自动迁移功能

### Requirement: Dynamic Token-Based Authentication

认证系统 SHALL 使用动态管理的 Token 进行验证，不再依赖静态配置文件。

#### Scenario: Dynamic Token Validation

**Given** 系统通过 TokenManager 管理所有有效的 token
**When** 客户端发送带有 `Authorization: Bearer <token>` 的请求
**Then** 系统 SHALL 在 TokenManager 中验证该 token
**And** 验证成功 SHALL 记录使用统计并处理请求
**And** 验证失败 SHALL 返回 401 Unauthorized
**And** 所有验证结果 SHALL 记录到审计日志

#### Scenario: Always-Enabled Authentication

**Given** 应用程序正在运行
**When** 任何客户端请求聚合接口端点
**Then** 认证 SHALL 始终启用
**And** 系统 SHALL 不支持无认证模式
**And** 如果 TokenManager 中没有 token，所有请求 SHALL 被拒绝
**And** 系统 SHALL 在启动时记录 "Authentication always enabled with dynamic tokens"

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

### Requirement: Enhanced Security Logging

认证系统 SHALL 提供更详细的安全审计日志。

#### Scenario: Detailed Authentication Logging

**When** 任何认证事件发生
**Then** 系统 SHALL 记录：
  - 认证尝试时间戳
  - 客户端 IP 地址（如果可用）
  - Token ID 或哈希值
  - 认证结果（成功/失败）
  - 失败原因（缺失、格式错误、无效 token、过期等）
**And** 日志 SHALL 不包含完整的 token 值
**And** 失败认证 SHALL 使用 WARN 级别
**And** 成功认证 SHALL 使用 DEBUG 级别

#### Scenario: Token Usage Statistics

**When** 认证成功时
**Then** 系统 SHALL 立即更新使用统计
**And** 增加该 token 的 usage_count
**And** 更新 last_used_at 时间戳
**And** 如果更新失败，系统 SHALL 记录错误但不拒绝请求

### Requirement: Performance and Scalability

认证系统 SHALL 优化性能以支持高并发请求。

#### Scenario: Efficient Token Lookup

**When** 验证 token 请求时
**Then** 查找操作 SHALL 在 O(1) 时间内完成
**And** 使用 HashMap 进行 token ID 到 Token 的映射
**And** 认证中间件 SHALL 不阻塞其他并发请求
**And** 使用统计更新 SHALL 异步进行

#### Scenario: Memory Management

**When** 系统运行时
**Then** TokenManager SHALL 使用高效的内存结构
**And** 定期清理过期的 token 以释放内存
**And** 大量 token 情况下仍保持良好性能

### Requirement: Error Handling and Resilience

系统 SHALL 优雅处理各种错误情况。

#### Scenario: Storage Unavailable

**Given** TokenManager 无法访问或更新 tokens.json 文件
**When** 认证请求到达
**Then** 系统 SHALL 使用内存中的 token 缓存继续验证
**And** 记录 ERROR 级别日志提示存储问题
**And** 在后台持续尝试恢复存储访问

#### Scenario: Token File Corruption

**Given** tokens.json 文件损坏或格式无效
**When** 应用程序启动时
**Then** 系统 SHALL 检测到损坏并记录错误
**And** 备份损坏的文件
**And** 创建新的空 token 存储
**And** 记录警告提示用户检查文件系统状态

---

