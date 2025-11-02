## MODIFIED Requirements

### Requirement: MCP 服务器管理功能

**原要求**：系统 SHALL 提供 MCP 服务器配置管理功能，支持 STDIO、SSE、HTTP 三种传输方式

**修改后要求**：系统 SHALL 继续提供 MCP 服务器配置管理功能，必须完全支持 STDIO、SSE、HTTP 三种传输方式，且 SSE 和 HTTP 传输支持完整的自定义 Header 传递

#### Scenario: 配置 SSE 传输并设置自定义 Headers

- **WHEN** 用户在 UI 中配置 MCP 服务器使用 SSE 传输并添加自定义 Headers
- **THEN** 系统 SHALL 保存这些配置
- **AND** 系统 SHALL 在连接时正确传递所有自定义 Headers
- **AND** UI SHALL 显示连接成功状态

#### Scenario: 配置 HTTP 传输并设置 API Key Header

- **WHEN** 用户在 UI 中配置 MCP 服务器使用 HTTP 传输并添加 API Key Header
- **THEN** 系统 SHALL 保存这些配置
- **AND** 系统 SHALL 在连接时正确传递该 Header
- **AND** UI SHALL 能够成功获取工具列表

#### Scenario: 编辑现有服务器的自定义 Headers

- **WHEN** 用户编辑已配置的 MCP 服务器并修改自定义 Headers
- **THEN** 系统 SHALL 更新配置
- **AND** 系统 SHALL 使用新 Headers 建立连接
- **AND** 系统 SHALL 验证 Header 传递正确性

### Requirement: 聚合接口访问控制

**原要求**：系统 SHALL 提供聚合接口供外部客户端访问 MCP 服务器功能

**修改后要求**：系统 SHALL 继续提供聚合接口，必须支持通过自定义 Headers 传递认证信息，且能够正确转发到目标 MCP 服务器

#### Scenario: 聚合接口转发自定义认证 Header

- **WHEN** 外部客户端通过聚合接口访问 MCP 服务器并在请求中包含自定义认证 Header
- **THEN** 系统 SHALL 验证请求权限
- **AND** 系统 SHALL 将自定义 Header 正确转发到目标 MCP 服务器
- **AND** 系统 SHALL 返回 MCP 服务器的响应

#### Scenario: 聚合接口支持多种认证方式

- **WHEN** 外部客户端通过聚合接口访问 MCP 服务器并使用不同的认证方式（API Key、Bearer Token、自定义 Header）
- **THEN** 系统 SHALL 验证每种认证方式
- **AND** 系统 SHALL 将相应认证信息正确转发到目标 MCP 服务器
- **AND** 系统 SHALL 返回正确的响应或错误信息

### Requirement: 工具管理与调用

**原要求**：系统 SHALL 提供 MCP 工具的列出、启用/禁用和调用功能

**修改后要求**：系统 SHALL 继续提供 MCP 工具的列出、启用/禁用和调用功能，必须兼容通过不同传输方式（支持自定义 Headers）连接的 MCP 服务器

#### Scenario: 通过 SSE 连接调用工具

- **WHEN** 用户通过 UI 调用 MCP 工具，该工具来自通过 SSE 传输（带自定义 Headers）连接的服务器
- **THEN** 系统 SHALL 保持现有功能不变
- **AND** 调用 SHALL 成功执行并返回结果
- **AND** Header 传递 SHALL 不影响工具调用流程

#### Scenario: 通过 HTTP 连接调用工具

- **WHEN** 用户通过 UI 调用 MCP 工具，该工具来自通过 HTTP 传输（带自定义 Headers）连接的服务器
- **THEN** 系统 SHALL 保持现有功能不变
- **AND** 调用 SHALL 成功执行并返回结果
- **AND** Header 传递 SHALL 不影响工具调用流程
