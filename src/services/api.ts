import { invoke } from '@tauri-apps/api/core'
import type {
  ApiKey,
  ApiKeyPermissions,
  AppConfig,
  MarketplaceService,
  MarketplaceServiceResult,
  McpServerConfig,
  McpServerInfo,
  McpTool,
  SystemSettings,
  Tool,
} from '../types'

export class ApiService {
  // MCP Server Management
  static async addMcpServer(
    name: string,
    command: string,
    args: string[],
    transport: string,
    url?: string,
    description?: string,
    env_vars?: [string, string][],
    headers?: [string, string][],
  ): Promise<string> {
    return invoke('add_mcp_server', {
      name,
      command,
      args,
      transport,
      url,
      description,
      env_vars: env_vars,
      headers,
    })
  }

  static async removeMcpServer(name: string): Promise<string> {
    return invoke('remove_mcp_server', { name })
  }

  static async checkMcpServerConnectivity(name: string): Promise<string> {
    return invoke('check_mcp_server_connectivity', { name })
  }

  static async toggleMcpServer(name: string): Promise<boolean> {
    return invoke('toggle_mcp_server', { name })
  }

  static async listMcpServers(): Promise<McpServerInfo[]> {
    return invoke('list_mcp_servers')
  }

  // Marketplace - Unified API
  static async listMarketplaceServices(
    query: string,
    page: number = 1,
    pageSize: number = 20,
  ): Promise<MarketplaceServiceResult> {
    return invoke('list_marketplace_services', {
      query,
      page,
      pageSize,
    })
  }

  static async getMcpServerDetails(
    serviceId: string,
  ): Promise<MarketplaceService> {
    return invoke('get_mcp_server_details', { service_id: serviceId })
  }

  static async installMarketplaceService(
    serviceId: string,
    env_vars?: [string, string][],
  ): Promise<McpServerConfig> {
    return invoke('install_marketplace_service', {
      service_id: serviceId,
      env_vars,
    })
  }

  // Server
  static async getDashboardStats(): Promise<any> {
    return invoke('get_dashboard_stats')
  }

  // Configuration
  static async getConfig(): Promise<AppConfig> {
    return invoke('get_config')
  }

  static async updateConfig(config: AppConfig): Promise<string> {
    return invoke('update_config', { config })
  }

  // MCP Tool Management
  static async listMcpServerTools(serverName: string): Promise<McpTool[]> {
    return invoke('list_mcp_server_tools', { connection_id: serverName })
  }

  static async connectToMcpServer(name: string): Promise<string> {
    return invoke('connect_to_mcp_server', { name })
  }

  static async disconnectFromMcpServer(name: string): Promise<string> {
    return invoke('disconnect_from_mcp_server', { connection_id: name })
  }

  static async toggleMcpServerTool(
    name: string,
    toolName: string,
    enabled: boolean,
  ): Promise<string> {
    return invoke('toggle_mcp_server_tool', {
      name,
      tool_name: toolName,
      enabled,
    })
  }

  static async callMcpServerTool(
    name: string,
    toolName: string,
    args?: any,
  ): Promise<any> {
    return invoke('call_mcp_tool', {
      connection_id: name,
      tool_name: toolName,
      arguments: args,
    })
  }

  static async enableAllMcpServerTools(name: string): Promise<string> {
    return invoke('enable_all_mcp_server_tools', { name })
  }

  static async disableAllMcpServerTools(name: string): Promise<string> {
    return invoke('disable_all_mcp_server_tools', { name })
  }

  // System Settings
  static async getSystemSettings(): Promise<SystemSettings> {
    return invoke('get_settings')
  }

  static async saveSystemSettings(settings: SystemSettings): Promise<string> {
    return invoke('save_settings', { settings })
  }

  // Network Interface
  static async getLocalIpAddresses(): Promise<string[]> {
    return invoke('get_local_ip_addresses')
  }

  // Autostart Management
  static async isAutostartEnabled(): Promise<boolean> {
    return invoke('is_autostart_enabled')
  }

  static async toggleAutostart(): Promise<string> {
    return invoke('toggle_autostart')
  }

  // API Key Management
  static async createApiKey(
    name: string,
    permissions?: ApiKeyPermissions,
  ): Promise<ApiKey> {
    return invoke('create_api_key', { name, permissions })
  }

  static async listApiKeys(): Promise<ApiKey[]> {
    return invoke('list_api_keys')
  }

  static async getApiKeyDetails(id: string): Promise<ApiKey> {
    return invoke('get_api_key_details', { id })
  }

  static async deleteApiKey(id: string): Promise<string> {
    return invoke('delete_api_key', { id })
  }

  static async toggleApiKey(id: string): Promise<boolean> {
    return invoke('toggle_api_key', { id })
  }

  static async updateApiKeyPermissions(
    id: string,
    permissions: ApiKeyPermissions,
  ): Promise<string> {
    return invoke('update_api_key_permissions', { id, permissions })
  }

  // Tool-level Permission Management
  static async getApiKeyTools(apiKeyId: string): Promise<string[]> {
    return invoke('get_api_key_tools', { api_key_id: apiKeyId })
  }


  static async addToolPermission(
    apiKeyId: string,
    toolId: string,
  ): Promise<string> {
    return invoke('add_tool_permission', { api_key_id: apiKeyId, tool_id: toolId })
  }

  static async removeToolPermission(
    apiKeyId: string,
    toolId: string,
  ): Promise<string> {
    return invoke('remove_tool_permission', { api_key_id: apiKeyId, tool_id: toolId })
  }

  static async grantServerToolsToApiKey(
    apiKeyId: string,
    serverName: string,
  ): Promise<string> {
    return invoke('grant_server_tools_to_api_key', {
      api_key_id: apiKeyId,
      server_name: serverName,
    })
  }

  static async revokeServerToolsFromApiKey(
    apiKeyId: string,
    serverName: string,
  ): Promise<string> {
    return invoke('revoke_server_tools_from_api_key', {
      api_key_id: apiKeyId,
      server_name: serverName,
    })
  }

  // Removed: getApiKeyPermissions(id) — use getApiKeyDetails(id) and read `.permissions`.



  // Tool Management
  static async listTools(): Promise<Tool[]> {
    return invoke('list_tools')
  }

  static async toggleTool(id: string, enabled: boolean): Promise<string> {
    return invoke('toggle_tool', { id, enabled })
  }

  static async getToolsByServer(serverName: string): Promise<Tool[]> {
    return invoke('get_tools_by_server', { name: serverName })
  }

}
