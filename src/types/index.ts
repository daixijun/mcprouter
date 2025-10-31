export interface McpServerConfig {
  name: string
  description?: string
  command?: string
  args?: string[]
  transport: 'stdio' | 'sse' | 'http'
  url?: string
  enabled: boolean
  env_vars?: Record<string, string>
  headers?: Record<string, string>
  version?: string
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
  transport: 'stdio' | 'sse' | 'http'
  url?: string
  description?: string
  env_vars?: Record<string, string>
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
    max_connections: number
    timeout_seconds: number
  }
  logging?: {
    level: 'debug' | 'info' | 'warn' | 'error'
    file_name?: string
  }
  security?: {
    auth: boolean
    allowed_hosts: string[]
  }
  // 新的嵌套设置结构
  settings?: {
    theme?: string | null
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

export interface ApiKeyPermissions {
  allowed_servers: string[]
  allowed_tools: string[]
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

export interface ApiKey {
  id: string
  name: string
  key: string // Will be masked in list view (sk-xxx...xxx)
  enabled: boolean
  created_at: string
  updated_at: string
  permissions?: ApiKeyPermissions
  tool_count?: number // Number of authorized tools
}

export interface ApiKeyListItem {
  id: string
  name: string
  key: string
  enabled: boolean
  created_at: string
  updated_at: string
  authorized_server_count: number
  authorized_tool_count: number
}

export interface SystemSettings {
  server: {
    host: string
    port: number
    max_connections: number
    timeout_seconds: number
  }
  logging: {
    level: 'debug' | 'info' | 'warn' | 'error'
    file_name?: string
  }
  security: {
    auth: boolean
    allowed_hosts: string[]
  }
  // 新的嵌套设置结构（应用层设置）
  settings?: {
    theme?: string | null
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
  disabled_servers: number
  connected_services: number
  total_tools: number
  active_clients: number
  startup_time: string
  aggregator?: {
    endpoint: string
    max_connections?: number
    is_running?: boolean
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
