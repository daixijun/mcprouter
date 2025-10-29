import { invoke } from '@tauri-apps/api/core'
import type {
  McpServerInfo,
  McpTool,
  ToolCallArguments,
  ToolCallResult,
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

  // MCP Server Connection Management
  static async connectToMcpServer(name: string): Promise<string> {
    return invoke('connect_to_mcp_server', { name })
  }

  static async disconnectFromMcpServer(name: string): Promise<string> {
    return invoke('disconnect_from_mcp_server', { connection_id: name })
  }

  // MCP Tool Management
  static async listMcpServerTools(serverName: string): Promise<McpTool[]> {
    return invoke('list_mcp_server_tools', { connection_id: serverName })
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
    args?: ToolCallArguments,
  ): Promise<ToolCallResult> {
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
}
