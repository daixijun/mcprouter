import React, { useEffect, useState } from 'react'
import toastService from '../services/toastService'
import { ApiService } from '../services/api'
import type { Tool, McpServerInfo } from '../types'

interface ToolManagerProps {
  mcpServer: McpServerInfo
}

const ToolManager: React.FC<ToolManagerProps> = ({ mcpServer }) => {
  const [tools, setTools] = useState<Tool[]>([])
  const [loading, setLoading] = useState(false)
  const [updating, setUpdating] = useState<string | null>(null)

  useEffect(() => {
    loadTools()
  }, [mcpServer.name])

  const loadTools = async () => {
    setLoading(true)
    try {
      // 直接从数据库获取工具列表（无需连接服务）
      const serverTools = await ApiService.getToolsByServer(mcpServer.name)
      setTools(serverTools)
    } catch (error) {
      console.error('Failed to load tools:', error)
      toastService.sendErrorNotification('加载工具列表失败')
    } finally {
      setLoading(false)
    }
  }

  const handleToggleTool = async (toolName: string, enabled: boolean) => {
    setUpdating(toolName)
    try {
      await ApiService.toggleMcpServerTool(mcpServer.name, toolName, enabled)
      toastService.sendSuccessNotification(`工具已${enabled ? '启用' : '禁用'}`)

      // 重新加载工具以获取最新状态
      await loadTools()
    } catch (error) {
      console.error('Failed to toggle tool:', error)
      toastService.sendErrorNotification('切换工具状态失败')
    } finally {
      setUpdating(null)
    }
  }

  if (loading) {
    return (
      <div className='card-glass p-4 text-center'>
        <div className='animate-spin rounded-full h-6 w-6 border-2 border-blue-500 border-t-transparent mx-auto mb-2'></div>
        <p className='text-sm text-gray-600 dark:text-gray-300'>加载工具中...</p>
      </div>
    )
  }

  if (tools.length === 0) {
    return (
      <div className='card-glass p-4 text-center'>
        <p className='text-sm text-gray-600 dark:text-gray-300'>该服务暂无可用工具</p>
      </div>
    )
  }

  return (
    <div className='card-glass p-4'>
      <h4 className='font-medium text-sm text-gray-800 dark:text-gray-100 mb-3'>
        工具管理 ({tools.length} 个工具)
      </h4>
      <div className='space-y-2'>
        {tools.map((tool) => (
          <div
            key={tool.name}
            className='flex items-center justify-between p-2 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors'
          >
            <div className='flex-1 min-w-0'>
              <div className='font-medium text-sm text-gray-900 dark:text-gray-100 truncate'>
                {tool.name}
              </div>
              {tool.description && (
                <div className='text-xs text-gray-600 dark:text-gray-400 truncate mt-1'>
                  {tool.description}
                </div>
              )}
            </div>
            <div className='flex items-center ml-3'>
              <button
                onClick={() => handleToggleTool(tool.name, !tool.enabled)}
                disabled={updating === tool.name}
                className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1 ${
                  tool.enabled ? 'bg-green-500' : 'bg-gray-300'
                } ${updating === tool.name ? 'opacity-50 cursor-not-allowed' : ''}`}
                aria-label={`Toggle ${tool.name}`}
                title={tool.enabled ? '点击禁用' : '点击启用'}
              >
                <span
                  className={`inline-block h-3 w-3 transform rounded-full bg-white transition-transform ${
                    tool.enabled ? 'translate-x-5' : 'translate-x-1'
                  } ${updating === tool.name ? 'animate-pulse' : ''}`}
                />
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

export default ToolManager