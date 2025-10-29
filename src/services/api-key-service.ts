import { invoke } from '@tauri-apps/api/core'
import type { ApiKey, ApiKeyPermissions } from '../types'

export class ApiKeyService {
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
    return invoke('add_tool_permission', {
      api_key_id: apiKeyId,
      tool_id: toolId,
    })
  }

  static async removeToolPermission(
    apiKeyId: string,
    toolId: string,
  ): Promise<string> {
    return invoke('remove_tool_permission', {
      api_key_id: apiKeyId,
      tool_id: toolId,
    })
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
}
