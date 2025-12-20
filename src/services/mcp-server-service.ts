import { invoke } from '@tauri-apps/api/core'
import type { McpResourceInfo, McpServerResult } from '../types'

export class McpServerService {
  // MCP Server Management
  static async addMcpServer(
    name: string,
    command: string,
    args: string[],
    type: string,
    url?: string,
    description?: string,
    env?: [string, string][],
    headers?: [string, string][],
  ): Promise<string> {
    // Create request object to match backend structure
    const request = {
      name,
      command,
      args,
      type,
      url,
      description,
      env,
      headers,
    }

    return invoke('add_mcp_server', { request })
  }

  static async updateMcpServer(
    name: string,
    command: string | null,
    args: string[] | null,
    type: string,
    url?: string | null,
    description?: string | null,
    env?: [string, string][] | null,
    headers?: [string, string][] | null,
    enabled?: boolean,
  ): Promise<string> {
    // Create request object to match backend structure
    const request = {
      name,
      command,
      args,
      type,
      url,
      description,
      env,
      headers,
      enabled: enabled ?? true,
    }

    return invoke('update_mcp_server', { request })
  }

  static async removeMcpServer(name: string): Promise<string> {
    return invoke('delete_mcp_server', { name })
  }

  static async checkMcpServerConnectivity(name: string): Promise<string> {
    return invoke('check_mcp_server_connectivity', { name })
  }

  static async toggleMcpServer(name: string): Promise<boolean> {
    return await invoke('toggle_mcp_server', { name })
  }

  static async listMcpServers(): Promise<McpServerResult> {
    return invoke('list_mcp_servers')
  }

  static async importMcpServersConfig(configJson: any): Promise<string> {
    return invoke('import_mcp_servers_config', { configJson })
  }

  // MCP Resources Management
  static async listMcpServerResources(
    serverName: string,
  ): Promise<McpResourceInfo[]> {
    return invoke('list_mcp_server_resources', { server_name: serverName })
  }
}
