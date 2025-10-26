import React, { useEffect, useState } from 'react'
// removed chevrons
import toastService from '../services/toastService'
import { ApiService } from '../services/api'
import type { ApiKeyPermissions, McpServerInfo } from '../types'

interface Props {
  permissions: ApiKeyPermissions
  onChange: (permissions: ApiKeyPermissions) => void
}

const ApiKeyPermissionSelector: React.FC<Props> = ({
  permissions,
  onChange,
}) => {
  const [servers, setServers] = useState<McpServerInfo[]>([])
  // tool-level permissions removed
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadServers()
  }, [])

  const loadServers = async () => {
    try {
      const serverList = await ApiService.listMcpServers()
      setServers(serverList)
    } catch (error) {
      console.error('Failed to load servers:', error)
      toastService.sendErrorNotification('加载服务器列表失败')
    } finally {
      setLoading(false)
    }
  }

  // tool-level permissions removed: no server tools loading

  // expansion removed: no server toggling

  const isServerSelected = (serverName: string) => {
    return permissions.allowed_servers.includes(serverName)
  }

  // tool-level selection removed

  // tool-level selection removed

  // tool-level selection removed

  // tool-level selection removed

  // tool-level selection removed

  const toggleServerSelection = (serverName: string) => {
    const newAllowedServers = isServerSelected(serverName)
      ? permissions.allowed_servers.filter((s) => s !== serverName)
      : [...permissions.allowed_servers, serverName]

    onChange({
      ...permissions,
      allowed_servers: newAllowedServers,
    })
  }

  if (loading) {
    return (
      <div className='text-sm text-gray-500 dark:text-gray-400'>加载中...</div>
    )
  }

  return (
    <div className='space-y-2 max-h-96 overflow-y-auto border border-gray-200 rounded-lg p-4'>
      {servers.length === 0 ? (
        <div className='text-sm text-gray-500 dark:text-gray-400'>
          暂无可用的MCP服务器
        </div>
      ) : (
        servers.map((server) => {

          return (
            <div
              key={server.name}
              className='border border-gray-200 dark:border-gray-600 rounded-lg'>
              {/* Server Level */}
              <div className='flex items-center p-2 hover:bg-gray-50 dark:hover:bg-gray-800'>
                {/* Expansion removed */}
                <input
                  type='checkbox'
                  checked={isServerSelected(server.name)}
                  onChange={() => toggleServerSelection(server.name)}
                  className='mr-2'
                />
                <div className='flex-1 flex items-center justify-between'>
                  <div>
                    <div className='text-sm font-medium text-gray-800 dark:text-gray-100'>
                      {server.name}
                    </div>
                    {server.description && (
                      <div className='text-xs text-gray-600 dark:text-gray-300'>
                        {server.description}
                      </div>
                    )}
                  </div>
                </div>
              </div>

              {/* Tool-level UI removed */}
            </div>
          )
        })
      )}
    </div>
  )
}

export default ApiKeyPermissionSelector
