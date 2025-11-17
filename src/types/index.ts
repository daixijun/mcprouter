export interface McpServerConfig {
  name: string
  description?: string
  command?: string
  args?: string[]
  env?: Record<string, string>
  type: 'stdio' | 'sse' | 'http'
  url?: string
  headers?: Record<string, string>
  enabled: boolean
}

export interface ServiceStatus {
  name: string
  enabled: boolean
  status: 'connecting' | 'connected' | 'disconnected' | 'failed'
  version?: string
  start_time?: string
  last_error?: string
  error_message?: string
}

export interface McpServerInfo {
  name: string
  enabled: boolean
  status: 'connecting' | 'connected' | 'disconnected' | 'failed'
  version?: string
  last_error?: string
  error_message?: string
  type: 'stdio' | 'sse' | 'http'
  url?: string
  description?: string
  env?: Record<string, string>
  headers?: Record<string, string>
  command?: string
  args?: string[]
  tool_count?: number
}

export interface McpTool {
  name: string
  description: string
  enabled: boolean
  // 后端始终返回 input_schema 字段，但可能为 null
  // 为兼容 rmcp 的灵活 JSON Schema，这里不限制具体形状
  input_schema?: Record<string, any> | null
  // 后端当前返回为 null，占位保留
  parameters?: Record<string, any> | null
}

export interface McpServer {
  name: string
  description?: string
  command: string
  args: string[]
  transport: 'stdio' | 'sse' | 'http'
  url?: string
  status: 'running' | 'stopped' | 'starting' | 'stopping' | 'error'
  enabled: boolean
  is_active: boolean
  env: Record<string, string>
  version?: string
  created_at: string
  tools: McpTool[]
  tool_count?: number
}

export interface MarketplaceServiceListItem {
  id: string
  name: string
  description: string
  author: string
  is_hosted?: boolean
  is_verified?: boolean
  tags: string[]
  downloads: number
  github_stars?: number
  transport: 'stdio' | 'sse' | 'http'
  category: string
  last_updated: string
  platform: string
  logo_url?: string
  license?: string
}

export interface EnvProperty {
  description?: string
  type: string
  default?: any
  enum?: any[]
}

export interface EnvSchema {
  properties?: Record<string, EnvProperty>
  required?: string[]
  type: string
}

export interface MarketplaceService extends MarketplaceServiceListItem {
  install_command?: {
    command: string
    args: string[]
    package_manager:
      | 'npm'
      | 'npx'
      | 'yarn'
      | 'pnpm'
      | 'cargo'
      | 'pip'
      | 'uvx'
      | 'uv'
      | 'custom'
  }
  requirements: string[]
  readme?: string
  server_config?: Array<Record<string, any>>
  repository?: string
  homepage?: string
  env_schema?: EnvSchema
}

// Unified marketplace result type
export interface MarketplaceServiceResult {
  services: MarketplaceServiceListItem[]
  total_count: number
  has_more: boolean
}

// 便于单独引用安装命令类型
export type InstallCommand = MarketplaceService['install_command']

export interface AppConfig {
  server: {
    host: string
    port: number
    maxConnections: number
    timeoutSeconds: number
  }
  logging?: {
    level: 'trace' | 'debug' | 'info' | 'warn' | 'error'
    fileName?: string
  }
  mcpServers: McpServerConfig[]
  settings?: {
    theme?: string | null
    language?: string
    autostart?: boolean
    systemTray?: {
      enabled?: boolean
      closeToTray?: boolean
      startToTray?: boolean
    }
    uvIndexUrl?: string
    npmRegistry?: string
  }
}

export interface Tool {
  id: string
  name: string
  server_id: string
  description?: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface SystemSettings {
  server: {
    host: string
    port: number
    max_connections: number
    timeout_seconds: number
    auth?: boolean
  }
  logging: {
    level: 'trace' | 'debug' | 'info' | 'warn' | 'error'
    file_name?: string
  }
  settings?: {
    theme?: string | null
    language?: string
    autostart?: boolean
    system_tray?: {
      enabled?: boolean
      close_to_tray?: boolean
      start_to_tray?: boolean
    }
    uv_index_url?: string
    npm_registry?: string
  }
}

export interface DashboardStats {
  total_servers: number
  enabled_servers: number
  failed_servers: number
  healthy_services: number
  connected_services: number
  total_tools: number
  active_clients: number
  startup_time: string
  aggregator?: {
    endpoint: string
    max_connections?: number
    status?: 'running' | 'stopped' | 'error'
    connected_services?: number
  }
  os_info?: {
    type: string
    version: string
    arch: string
    platform?: string
  }
  connections?: {
    active_clients: number
    active_services: number
    total_connections?: number
  }
}

export type ToolCallArguments = Record<string, any> | null
export type ToolCallResult = Record<string, any> | null

// Token Management Types
export interface Token {
  id: string
  name: string
  description?: string
  created_at: number
  expires_at?: number
  last_used_at?: number
  usage_count: number
  is_expired: boolean
  enabled: boolean
  // Permission fields
  allowed_tools?: string[]
  allowed_resources?: string[]
  allowed_prompts?: string[]
}

export interface CreateTokenRequest {
  name: string
  description?: string
  expires_in?: number // seconds
  // Permission fields
  allowed_tools?: string[]
  allowed_resources?: string[]
  allowed_prompts?: string[]
}

export interface CreateTokenResponse {
  token: {
    id: string
    value: string // only returned on creation
    name: string
    description?: string
    created_at: number
    expires_at?: number
  }
}

export interface UpdateTokenRequest {
  id: string
  name?: string
  description?: string
  // Permission fields - use optional nested types to distinguish between "don't update" and "set to undefined"
  allowed_tools?: string[] | undefined
  allowed_resources?: string[] | undefined
  allowed_prompts?: string[] | undefined
}

export interface UpdateTokenResponse {
  token: Token
}

export interface PermissionItem {
  id: string
  description: string
}

export interface AvailablePermissions {
  tools: PermissionItem[]
  resources: PermissionItem[]
  prompts: PermissionItem[]
}

export interface TokenStats {
  total_count: number
  active_count: number
  expired_count: number
  total_usage: number
  last_used?: number
}

export interface CleanupResult {
  removed_count: number
  message: string
}

export interface ValidationResult {
  valid: boolean
  token_info?: Token
  message: string
}
