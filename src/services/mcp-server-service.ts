import { invoke } from '@tauri-apps/api/core'
import type {
  McpServerInfo,
} from '../types'

export class McpServerService {
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
    // Create request object to match backend structure
    const request = {
      name,
      command,
      args,
      transport,
      url,
      description,
      env_vars,
      headers,
    };

    return invoke('add_mcp_server', { request });
  }

  static async updateMcpServer(
    name: string,
    command: string | null,
    args: string[] | null,
    transport: string,
    url?: string | null,
    description?: string | null,
    env_vars?: [string, string][] | null,
    headers?: [string, string][] | null,
    enabled?: boolean,
  ): Promise<string> {
    // Create request object to match backend structure
    const request = {
      name,
      command,
      args,
      transport,
      url,
      description,
      env_vars,
      headers,
      enabled: enabled ?? true,
    };

    return invoke('update_mcp_server', { request });
  }

  static async removeMcpServer(name: string): Promise<string> {
    return invoke('remove_mcp_server', { name })
  }

  static async checkMcpServerConnectivity(name: string): Promise<string> {
    return invoke('check_mcp_server_connectivity', { name })
  }

  static async toggleMcpServer(name: string): Promise<boolean> {
    try {
      return await invoke('toggle_mcp_server', { name })
    } catch (error) {
      // Re-throw the error to ensure it propagates to the caller
      throw error
    }
  }

  static async listMcpServers(): Promise<McpServerInfo[]> {
    return invoke('list_mcp_servers')
  }
}
