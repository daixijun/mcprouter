import { invoke } from '@tauri-apps/api/core'
import type { Tool } from '../types'

export class ToolService {
  static async listTools(): Promise<Tool[]> {
    return invoke('list_tools')
  }

  static async toggleTool(id: string, enabled: boolean): Promise<string> {
    return invoke('toggle_tool', { id, enabled })
  }

  static async getToolsByServer(serverName: string): Promise<Tool[]> {
    return invoke('list_mcp_server_tools', { connection_id: serverName })
  }

  static async enableAllTools(serverName: string): Promise<string> {
    return invoke('enable_all_mcp_server_tools', { name: serverName })
  }

  static async disableAllTools(serverName: string): Promise<string> {
    return invoke('disable_all_mcp_server_tools', { name: serverName })
  }
}
