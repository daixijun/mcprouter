import { invoke } from '@tauri-apps/api/core'
import type { ToolInfo, PythonRuntimeInfo, ToolStartupStatus } from '../types'

export class ToolManagerService {
  /**
   * 获取所有工具信息
   */
  static async getToolsInfo(): Promise<ToolInfo[]> {
    return await invoke('get_tools_info')
  }

  /**
   * 获取特定工具信息
   */
  static async getToolInfo(toolName: string): Promise<ToolInfo | null> {
    return await invoke('get_tool_info', { tool_name: toolName })
  }

  /**
   * 安装所有必需的工具
   */
  static async installAllTools(): Promise<void> {
    return await invoke('install_all_tools')
  }

  /**
   * 安装特定工具
   */
  static async installTool(toolName: string): Promise<void> {
    return await invoke('install_tool', { tool_name: toolName })
  }

  /**
   * 检查 Python 运行时兼容性
   */
  static async checkPythonRuntime(): Promise<PythonRuntimeInfo> {
    const [available, version] = await invoke<[boolean, string | null]>('check_python_runtime')
    return { available, version: version || undefined }
  }

  
  /**
   * 获取工具启动状态用于应用启动检查
   */
  static async getToolStartupStatus(): Promise<ToolStartupStatus> {
    return await invoke('get_tool_startup_status')
  }
}