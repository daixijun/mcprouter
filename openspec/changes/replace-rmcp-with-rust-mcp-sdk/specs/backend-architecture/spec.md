## MODIFIED Requirements

### Requirement: HTTP 传输层支持完整 Header 传递

**原要求**：系统 SHALL 支持 HTTP 传输方式连接 MCP 服务器，但 Header 支持受限（仅支持 Authorization）

**修改后要求**：系统 SHALL 支持 HTTP 传输方式连接 MCP 服务器，必须支持完整的自定义 Header 传递机制

#### Scenario: HTTP 传输使用自定义 API Key Header

- **WHEN** 用户配置 MCP 服务器使用 HTTP 传输并设置自定义 API Key Header
- **THEN** 系统 SHALL 正确传递该 Header 到 MCP 服务器
- **AND** 系统 SHALL 能够成功建立连接并获取工具列表

#### Scenario: HTTP 传输使用 Content-Type Header

- **WHEN** 用户配置 MCP 服务器使用 HTTP 传输并设置 Content-Type Header
- **THEN** 系统 SHALL 正确传递该 Header 到 MCP 服务器
- **AND** 系统 SHALL 能够在请求中使用正确的 Content-Type

#### Scenario: HTTP 传输使用多个自定义 Headers

- **WHEN** 用户配置 MCP 服务器使用 HTTP 传输并设置多个自定义 Headers
- **THEN** 系统 SHALL 正确传递所有自定义 Headers 到 MCP 服务器
- **AND** 系统 SHALL 保持所有 Header 值的完整性

### Requirement: SSE 传输层支持自定义 Header 传递

**原要求**：系统 SHALL 支持 SSE 传输方式连接 MCP 服务器，但 Header 支持缺失

**修改后要求**：系统 SHALL 支持 SSE 传输方式连接 MCP 服务器，必须支持完整的自定义 Header 传递机制

#### Scenario: SSE 传输使用 Authorization Header

- **WHEN** 用户配置 MCP 服务器使用 SSE 传输并设置 Authorization Header
- **THEN** 系统 SHALL 正确传递该 Header 到 MCP 服务器
- **AND** 系统 SHALL 能够在 SSE 连接中携带认证信息

#### Scenario: SSE 传输使用自定义 Headers

- **WHEN** 用户配置 MCP 服务器使用 SSE 传输并设置自定义 Headers
- **THEN** 系统 SHALL 正确传递所有自定义 Headers 到 MCP 服务器
- **AND** 系统 SHALL 能够通过 Headers 传递必要的认证或配置信息

### Requirement: STDIO 传输层保持兼容性

**原要求**：系统 SHALL 支持 STDIO 传输方式连接 MCP 服务器

**修改后要求**：系统 SHALL 继续支持 STDIO 传输方式连接 MCP 服务器，保持现有功能不变

#### Scenario: STDIO 传输连接本地 MCP 服务器

- **WHEN** 用户配置 MCP 服务器使用 STDIO 传输
- **THEN** 系统 SHALL 成功启动本地进程并建立连接
- **AND** 系统 SHALL 能够正常列出和调用工具

## REMOVED Requirements

### Requirement: 旧版 RMCP Header 限制

**Reason**: 随着 rmcp 替换为 rust-mcp-sdk，旧版 RMCP 的 Header 支持限制不再适用

**Migration**: 所有 Header 相关功能现在由 rust-mcp-sdk 提供更完整的支持
