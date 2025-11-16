# aggregator-auth Specification (Modified)

## Purpose

将聚合接口认证从静态配置模式迁移到动态 Token 管理模式，移除配置文件中的静态字段，始终启用认证。

## Requirements

## REMOVED Requirements

### Requirement: Configuration-Based Authentication Control

移除原有的基于配置文件的认证控制功能。

#### Scenario: Authentication Disabled by Default

移除默认禁用认证的选项。

#### Scenario: Enable Authentication via Configuration

移除通过配置文件启用认证的功能。

#### Scenario: Authentication Enabled Without Token

移除配置启用但缺少 token 的错误处理场景。

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Configuration Validation

移除静态配置验证要求，包括：
- 检查 `auth: true` 与缺少 `bearer_token` 的错误
- 弱 token 警告
- 环境变量支持

#### Scenario: Static Configuration Validation

移除静态配置验证的场景。

### Requirement: Backward Compatibility

移除与不存在认证字段的向后兼容性支持，因为此变更是破坏性变更。

#### Scenario: Backward Compatibility

移除向后兼容性场景。

## ADDED Requirements

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

## Note: Breaking Changes

此规格更新包含破坏性变更：

1. **移除配置字段**: `server.auth` 和 `server.bearer_token` 字段不再支持
2. **始终启用认证**: 不再支持无认证模式
3. **配置文件迁移**: 需要迁移现有的静态配置到动态 token
4. **向后兼容性**: 旧配置文件需要更新

迁移路径已在上述需求中详细说明。