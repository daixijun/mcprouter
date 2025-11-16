# token-management Specification

## Purpose
TBD - created by archiving change migrate-dynamic-token-management. Update Purpose after archive.
## Requirements
### Requirement: Token Creation

系统 SHALL 允许用户创建新的 Bearer Token 并配置相关元数据。

#### Scenario: Create Token with Name and Description

**Given** 用户打开 Token 创建界面
**When** 用户输入 token 名称 "Production API" 和描述 "用于生产环境的访问凭证"
**And** 用户设置过期时间为 30 天
**And** 点击创建按钮
**Then** 系统 SHALL 生成一个新的 token
**And** token SHALL 包含加密安全的随机值
**And** token SHALL 包含指定的名称、描述和过期时间
**And** 系统 SHALL 在界面中显示完整的 token 值 (仅此一次)

#### Scenario: Create Token without Expiration

**Given** 用户打开 Token 创建界面
**When** 用户输入 token 名称 "Development" 但不设置过期时间
**And** 点击创建按钮
**Then** 系统 SHALL 创建一个永不过期的 token
**And** token 的 expires_at 字段 SHALL 为 null
**And** 系统 SHALL 在日志中记录永不过期的警告

#### Scenario: Create Token with Duplicate Name

**Given** 已存在名为 "Production API" 的 token
**When** 用户尝试创建另一个名为 "Production API" 的 token
**Then** 系统 SHALL 显示错误消息 "Token 名称已存在"
**And** 系统 SHALL 不创建新 token

### Requirement: Token Listing and Display

系统 SHALL 提供界面显示所有已创建的 token 及其使用统计信息。

#### Scenario: List Active Tokens

**Given** 用户访问 Token 管理页面
**And** 系统中有 3 个有效 token
**When** 页面加载完成
**Then** 系统 SHALL 显示所有 3 个 token 的列表
**And** 每个 token SHALL 显示: 名称、创建时间、最后使用时间、使用次数
**And** 系统 SHALL 隐藏实际的 token 值 (仅显示前 8 位)
**And** 过期时间 SHALL 以相对时间格式显示 (如 "15 天后过期")

#### Scenario: Show Token Usage Statistics

**Given** 有一个 token 已被使用 156 次,最后使用时间为 2 小时前
**When** 用户在列表中查看该 token
**Then** 使用次数 SHALL 显示为 "156"
**And** 最后使用时间 SHALL 显示为 "2 小时前"
**And** 创建时间 SHALL 显示为具体的日期和时间

#### Scenario: Empty Token List

**Given** 系统中没有创建任何 token
**When** 用户访问 Token 管理页面
**Then** 系统 SHALL 显示空状态提示
**And** 提供 "创建第一个 Token" 的引导按钮
**And** 说明没有 token 时无法访问聚合接口

### Requirement: Token Deletion

系统 SHALL 允许用户删除不再需要的 token 并提供安全确认机制。

#### Scenario: Delete Unused Token

**Given** 用户有一个名为 "Test" 的 token,使用次数为 0
**When** 用户点击删除该 token
**Then** 系统 SHALL 显示确认对话框
**And** 对话框 SHALL 显示 token 名称和删除警告
**When** 用户确认删除
**Then** 系统 SHALL 从存储中永久删除该 token
**And** 使用该 token 的后续请求 SHALL 被拒绝

#### Scenario: Delete Active Token Warning

**Given** 用户有一个名为 "Production" 的 token,使用次数为 1234,最后使用时间为 1 小时前
**When** 用户点击删除该 token
**Then** 系统 SHALL 显示增强的警告对话框
**And** 警告 SHALL 包含: "该 token 在过去 24 小时内被使用"
**And** 警告 SHALL 显示使用统计信息
**And** 用户需要二次确认才能删除

#### Scenario: Cancel Deletion

**Given** 用户点击删除 token
**When** 系统显示确认对话框
**And** 用户点击取消或关闭对话框
**Then** 系统 SHALL 不删除 token
**And** token SHALL 保持原有状态
**And** 用户界面 SHALL 不发生变化

### Requirement: Token Validation and Authentication

系统 SHALL 使用动态存储的 token 验证所有聚合接口请求。

#### Scenario: Valid Token Authentication

**Given** 系统中有有效的 token "mcp_abc123def456..."
**When** 客户端发送带有 `Authorization: Bearer mcp_abc123def456...` 的请求
**Then** 系统 SHALL 成功验证该 token
**And** 记录使用统计 (使用次数 +1, 更新最后使用时间)
**And** 返回正常的聚合接口响应
**And** 系统 SHALL 在日志中记录认证成功的 DEBUG 消息

#### Scenario: Invalid Token Rejection

**Given** 系统中没有 token "mcp_invalid_token"
**When** 客户端发送带有 `Authorization: Bearer mcp_invalid_token` 的请求
**Then** 系统 SHALL 拒绝该请求
**And** 返回 HTTP 401 Unauthorized 状态码
**And** 系统 SHALL 在日志中记录认证失败的 WARN 消息
**And** 系统 SHALL 不记录任何使用统计

#### Scenario: Expired Token Handling

**Given** 有一个 token 过期时间为昨天
**When** 客户端使用该过期的 token 发送请求
**Then** 系统 SHALL 拒绝该请求
**And** 返回 HTTP 401 Unauthorized 状态码
**And** 日志 SHALL 记录 "Token 已过期" 的详细信息
**And** 系统 SHOULD 自动清理过期 token (后台任务)

#### Scenario: No Tokens Available

**Given** 系统中没有创建任何 token
**When** 客户端发送任何请求到聚合接口
**Then** 系统 SHALL 拒绝所有请求
**And** 返回 HTTP 401 Unauthorized 状态码
**And** 错误信息 SHOULD 提示用户先创建有效的 token

### Requirement: Configuration Migration

系统 SHALL 自动将现有的静态配置迁移到新的动态管理系统。

#### Scenario: Migrate Existing Static Token

**Given** 用户的 config.json 包含:
```json
{
  "server": {
    "auth": true,
    "bearer_token": "legacy-secret-token"
  }
}
```
**And** tokens.json 文件不存在或为空
**When** 应用程序启动
**Then** 系统 SHALL 检测到旧的静态配置
**And** 自动创建一个新的 token:
  - name: "从配置迁移的 Token"
  - description: "从 server.bearer_token 自动迁移的访问凭证"
  - value: "legacy-secret-token"
  - expires_at: null (永不过期)
**And** 系统 SHALL 在日志中记录迁移警告
**And** 系统 SHALL 提示用户移除旧的配置字段

#### Scenario: Skip Migration for Existing Tokens

**Given** 用户的 tokens.json 已包含 2 个 token
**And** config.json 包含旧的静态配置
**When** 应用程序启动
**Then** 系统 SHALL 跳过迁移
**And** 保留现有的 tokens 不变
**And** 忽略旧的静态配置

#### Scenario: No Migration Needed

**Given** 用户的 config.json 不包含 auth 或 bearer_token 字段
**When** 应用程序启动
**Then** 系统 SHALL 正常启动
**And** 不执行任何迁移逻辑
**And** 日志 SHALL 不包含迁移相关消息

### Requirement: Persistent Storage Management

系统 SHALL 可靠地管理 token 的持久化存储。

#### Scenario: Save New Token to Storage

**Given** 用户成功创建了一个新 token
**Then** 系统 SHALL 立即将 token 保存到 ~/.mcprouter/tokens.json
**And** 文件 SHALL 设置正确的权限 (600)
**And** 存储 SHALL 包含完整的 token 元数据
**And** 保存失败 SHALL 导致创建操作失败

#### Scenario: Load Tokens from Storage

**Given** ~/.mcprouter/tokens.json 存在并包含有效的 token 数据
**When** 应用程序启动
**Then** 系统 SHALL 加载所有 token 到内存
**And** 验证文件格式的完整性
**And** 如果文件损坏,系统 SHOULD 记录错误并启动时清空文件

#### Scenario: Handle Storage Corruption

**Given** tokens.json 文件被损坏或格式无效
**When** 应用程序启动并尝试加载 token
**Then** 系统 SHALL 检测到文件损坏
**And** 记录错误日志
**And** 重命名损坏的文件为 tokens.json.backup.YYYYMMDDHHMMSS
**And** 创建新的空 tokens.json 文件
**And** 记录警告提示用户手动恢复

### Requirement: Security and Privacy

系统 SHALL 实施适当的安全措施保护 token 信息。

#### Scenario: Secure Token Generation

**When** 系统生成新的 token 值
**Then** token SHALL 使用密码学安全的随机数生成器
**And** token 长度 SHALL 至少为 64 字符
**And** token 格式 SHALL 为 "mcp_" + base64 编码的随机字节
**And** 生成过程 SHALL 不依赖可预测的种子

#### Scenario: Prevent Token Information Leakage

**When** 系统记录认证相关日志
**Then** 日志 SHALL 不包含完整的 token 值
**And** 系统 SHALL 仅记录 token 的前 8 位或 token ID
**And** 失败的认证尝试 SHALL 不泄露错误的 token 信息
**And** 日志 SHALL 包含时间戳和客户端 IP (如果可用)

#### Scenario: File Permission Protection

**When** 系统创建或更新 tokens.json 文件
**Then** 文件权限 SHALL 设置为 600 (仅所有者可读写)
**And** 如果权限设置失败,系统 SHALL 记录安全警告
**And** 应用 SHOULD 验证文件权限的安全性

### Requirement: Frontend Integration

系统 SHALL 提供完整的前端界面支持 token 管理操作。

#### Scenario: Responsive Token Management UI

**Given** 用户在不同屏幕尺寸的设备上
**When** 用户访问 Token 管理页面
**Then** 界面 SHALL 在桌面端显示完整的列表布局
**And** 界面 SHALL 在移动端调整为卡片式布局
**And** 所有操作按钮 SHALL 保持可用性
**And** 文本 SHALL 不会在窄屏上溢出

#### Scenario: Token Creation Form Validation

**Given** 用户在创建 token 表单中
**When** 用户提交空名称
**Then** 系统 SHALL 显示错误 "Token 名称不能为空"
**And** 阻止表单提交
**When** 用户输入超过 100 字符的名称
**Then** 系统 SHALL 显示错误 "名称长度不能超过 100 字符"
**And** 显示当前字符计数

#### Scenario: Copy Token to Clipboard

**Given** 用户刚创建了一个新 token
**When** 用户点击"复制 Token"按钮
**Then** 系统 SHALL 将完整的 token 值复制到剪贴板
**And** 显示"已复制到剪贴板"的成功提示
**And** 提示在 3 秒后自动消失
**And** 复制操作 SHALL 支持 HTTPS 和 HTTP 环境

