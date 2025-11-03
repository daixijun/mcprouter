import { invoke } from '@tauri-apps/api/core'
import type { Tool } from '../types'

export class ToolService {
  static async listTools(): Promise<Tool[]> {
    return invoke('list_tools')
  }

  static async toggleMcpServerTool(
    serverName: string,
    toolName: string,
    enabled: boolean
  ): Promise<string> {
    return invoke('toggle_mcp_server_tool', { name: serverName, tool_name: toolName, enabled })
  }

  static async listMcpServerTools(serverName: string): Promise<Tool[]> {
    return invoke('list_mcp_server_tools', { serverName })
  }

  static async enableAllMcpServerTools(serverName: string): Promise<string> {
    return invoke('enable_all_mcp_server_tools', { name: serverName })
  }

  static async disableAllMcpServerTools(serverName: string): Promise<string> {
    return invoke('disable_all_mcp_server_tools', { name: serverName })
  }
}
