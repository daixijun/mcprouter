import { ChevronDown, ChevronRight } from 'lucide-react'
import React, { useEffect, useState } from 'react'
import { App } from 'antd'
import { ApiKeyService } from '../services/api-key-service'
import { McpServerService } from '../services/mcp-server-service'
import { ToolService } from '../services/tool-service'
import type { ApiKeyPermissions, McpServerInfo, Tool } from '../types'

interface Props {
  permissions: ApiKeyPermissions
  onChange: (permissions: ApiKeyPermissions) => void
  apiKeyId?: string // Optional: if provided, will load and sync tool-level permissions
}

const ApiKeyPermissionSelector: React.FC<Props> = ({
  permissions,
  onChange,
  apiKeyId,
}) => {
  const { message } = App.useApp()
  const [servers, setServers] = useState<McpServerInfo[]>([])
  const [serverTools, setServerTools] = useState<Record<string, Tool[]>>({})
  const [authorizedToolIds, setAuthorizedToolIds] = useState<Set<string>>(
    new Set(),
  )
  const [expandedServers, setExpandedServers] = useState<Set<string>>(new Set())
  const [loading, setLoading] = useState(true)
  const [loadingTools, setLoadingTools] = useState<Set<string>>(new Set())

  useEffect(() => {
    loadServers()
    if (apiKeyId) {
      loadAuthorizedTools()
    }
  }, [apiKeyId])

  // 同步已授权工具到父级权限状态（用于保存时传递 allowed_tools）
  useEffect(() => {
    onChange({
      ...permissions,
      allowed_tools: Array.from(authorizedToolIds),
    })
  }, [authorizedToolIds])

  const loadServers = async () => {
    try {
      const serverList = await McpServerService.listMcpServers()
      setServers(serverList)
    } catch (error) {
      console.error('Failed to load servers:', error)
      message.error('加载服务器列表失败')
    } finally {
      setLoading(false)
    }
  }

  const loadAuthorizedTools = async () => {
    if (!apiKeyId) return

    try {
      const toolIds = await ApiKeyService.getApiKeyTools(apiKeyId)
      setAuthorizedToolIds(new Set(toolIds))
    } catch (error) {
      console.error('Failed to load authorized tools:', error)
      message.error('加载授权工具失败')
    }
  }

  const loadServerTools = async (serverName: string) => {
    if (serverTools[serverName]) return // Already loaded

    setLoadingTools((prev) => new Set(prev).add(serverName))
    try {
      const tools = await ToolService.listMcpServerTools(serverName)
      setServerTools((prev) => ({ ...prev, [serverName]: tools }))
    } catch (error) {
      console.error(`Failed to load tools for ${serverName}:`, error)
      message.error(`加载 ${serverName} 的工具失败`)
    } finally {
      setLoadingTools((prev) => {
        const newSet = new Set(prev)
        newSet.delete(serverName)
        return newSet
      })
    }
  }

  const toggleServerExpansion = async (serverName: string) => {
    const newExpanded = new Set(expandedServers)
    if (newExpanded.has(serverName)) {
      newExpanded.delete(serverName)
    } else {
      newExpanded.add(serverName)
      await loadServerTools(serverName)
    }
    setExpandedServers(newExpanded)
  }

  const isServerSelected = (serverName: string) => {
    return permissions.allowed_servers.includes(serverName)
  }

  const isToolAuthorized = (toolId: string) => {
    return authorizedToolIds.has(toolId)
  }

  const areAllServerToolsAuthorized = (serverName: string): boolean => {
    const tools = serverTools[serverName]
    if (!tools || tools.length === 0) return false
    return tools.every((tool) => authorizedToolIds.has(tool.id))
  }

  const toggleServerSelection = async (serverName: string) => {
    const isSelected = isServerSelected(serverName)

    const newAllowedServers = isSelected
      ? permissions.allowed_servers.filter((s) => s !== serverName)
      : [...permissions.allowed_servers, serverName]

    onChange({
      ...permissions,
      allowed_servers: newAllowedServers,
      allowed_tools: Array.from(authorizedToolIds),
    })

    // If we have apiKeyId, also grant/revoke tools on the backend
    if (apiKeyId) {
      try {
        if (isSelected) {
          // Revoke all tools from this server
          await ApiKeyService.revokeServerToolsFromApiKey(apiKeyId, serverName)
          // Remove tools from authorized set
          const tools = serverTools[serverName] || []
          const newAuthorized = new Set(authorizedToolIds)
          tools.forEach((tool) => newAuthorized.delete(tool.id))
          setAuthorizedToolIds(newAuthorized)
          message.success(`已取消 ${serverName} 的所有工具授权`)
        } else {
          // Grant all tools from this server
          await ApiKeyService.grantServerToolsToApiKey(apiKeyId, serverName)
          // Add tools to authorized set
          const tools =
            serverTools[serverName] ||
            (await ToolService.listMcpServerTools(serverName))
          const newAuthorized = new Set(authorizedToolIds)
          tools.forEach((tool) => newAuthorized.add(tool.id))
          setAuthorizedToolIds(newAuthorized)
          message.success(`已授权 ${serverName} 的所有工具`)
        }
      } catch (error) {
        console.error('Failed to toggle server tools:', error)
        message.error('更新服务器工具权限失败')
      }
    }
  }

  const toggleToolAuthorization = async (serverName: string, tool: Tool) => {
    if (!apiKeyId) {
      message.error('无法更新工具权限：缺少 API Key ID')
      return
    }

    const isAuthorized = authorizedToolIds.has(tool.id)

    try {
      if (isAuthorized) {
        await ApiKeyService.removeToolPermission(apiKeyId, tool.id)
        const newAuthorized = new Set(authorizedToolIds)
        newAuthorized.delete(tool.id)
        setAuthorizedToolIds(newAuthorized)
        message.success(`工具 "${tool.name}" 权限已移除`)

        // Check if we should remove server from allowed_servers
        const remainingTools = (serverTools[serverName] || []).filter(
          (t) => t.id !== tool.id && newAuthorized.has(t.id),
        )
        if (remainingTools.length === 0 && isServerSelected(serverName)) {
          onChange({
            ...permissions,
            allowed_servers: permissions.allowed_servers.filter(
              (s) => s !== serverName,
            ),
            allowed_tools: Array.from(newAuthorized),
          })
        }
      } else {
        await ApiKeyService.addToolPermission(apiKeyId, tool.id)
        const newAuthorized = new Set(authorizedToolIds)
        newAuthorized.add(tool.id)
        setAuthorizedToolIds(newAuthorized)
        message.success(`工具 "${tool.name}" 权限已添加`)

        // Ensure server is in allowed_servers
        if (!isServerSelected(serverName)) {
          onChange({
            ...permissions,
            allowed_servers: [...permissions.allowed_servers, serverName],
            allowed_tools: Array.from(newAuthorized),
          })
        }
      }
    } catch (error) {
      console.error('Failed to toggle tool permission:', error)
      message.error('更新工具权限失败')
    }
  }

  const handleSelectAll = async () => {
    if (!apiKeyId) {
      message.error('无法批量授权：缺少 API Key ID')
      return
    }

    try {
      // Grant all tools from all servers
      const grantPromises = servers.map((server) =>
        ApiKeyService.grantServerToolsToApiKey(apiKeyId, server.name),
      )
      await Promise.all(grantPromises)

      // Load all tools and update authorized set
      const allServerNames = servers.map((s) => s.name)
      const toolPromises = allServerNames.map((name) =>
        serverTools[name]
          ? Promise.resolve(serverTools[name])
          : ToolService.listMcpServerTools(name),
      )
      const allToolsArrays = await Promise.all(toolPromises)

      // Update server tools cache
      const newServerTools = { ...serverTools }
      allServerNames.forEach((name, idx) => {
        newServerTools[name] = allToolsArrays[idx]
      })
      setServerTools(newServerTools)

      // Update authorized tools set
      const allToolIds = new Set<string>()
      allToolsArrays.forEach((tools) => {
        tools.forEach((tool) => allToolIds.add(tool.id))
      })
      setAuthorizedToolIds(allToolIds)

      // Update allowed servers
      onChange({
        ...permissions,
        allowed_servers: allServerNames,
        allowed_tools: Array.from(allToolIds),
      })

      message.success('已授权所有工具')
    } catch (error) {
      console.error('Failed to select all:', error)
      message.error('全选失败')
    }
  }

  const handleDeselectAll = async () => {
    if (!apiKeyId) {
      message.error('无法批量撤销：缺少 API Key ID')
      return
    }

    try {
      // Revoke all tools from all servers
      const revokePromises = servers.map((server) =>
        ApiKeyService.revokeServerToolsFromApiKey(apiKeyId, server.name),
      )
      await Promise.all(revokePromises)

      // Clear authorized tools
      setAuthorizedToolIds(new Set())

      // Clear allowed servers
      onChange({
        ...permissions,
        allowed_servers: [],
        allowed_tools: [],
      })

      message.success('已取消所有授权')
    } catch (error) {
      console.error('Failed to deselect all:', error)
      message.error('取消全选失败')
    }
  }

  const handleInvertSelection = async () => {
    if (!apiKeyId) {
      message.error('无法反选：缺少 API Key ID')
      return
    }

    try {
      // Load all server tools if not already loaded
      const allServerNames = servers.map((s) => s.name)
      const toolPromises = allServerNames.map((name) =>
        serverTools[name]
          ? Promise.resolve(serverTools[name])
          : ToolService.listMcpServerTools(name),
      )
      const allToolsArrays = await Promise.all(toolPromises)

      // Update server tools cache
      const newServerTools = { ...serverTools }
      allServerNames.forEach((name, idx) => {
        newServerTools[name] = allToolsArrays[idx]
      })
      setServerTools(newServerTools)

      // Collect all tools
      const allTools: Tool[] = []
      allToolsArrays.forEach((tools) => allTools.push(...tools))

      // Determine which tools to add and which to remove
      const toolsToAdd = allTools.filter(
        (tool) => !authorizedToolIds.has(tool.id),
      )
      const toolsToRemove = allTools.filter((tool) =>
        authorizedToolIds.has(tool.id),
      )

      // Execute all changes in parallel
      const addPromises = toolsToAdd.map((tool) =>
        ApiKeyService.addToolPermission(apiKeyId, tool.id),
      )
      const removePromises = toolsToRemove.map((tool) =>
        ApiKeyService.removeToolPermission(apiKeyId, tool.id),
      )

      await Promise.all([...addPromises, ...removePromises])

      // Update authorized tools set
      const newAuthorized = new Set<string>()
      toolsToAdd.forEach((tool) => newAuthorized.add(tool.id))
      setAuthorizedToolIds(newAuthorized)

      // Update allowed servers (servers with at least one authorized tool)
      const serversWithTools = new Set<string>()
      toolsToAdd.forEach((tool) => {
        // Find which server this tool belongs to
        for (const [serverName, tools] of Object.entries(newServerTools)) {
          if (tools.some((t) => t.id === tool.id)) {
            serversWithTools.add(serverName)
            break
          }
        }
      })

      onChange({
        ...permissions,
        allowed_servers: Array.from(serversWithTools),
        allowed_tools: Array.from(newAuthorized),
      })

      message.success('已反选所有工具')
    } catch (error) {
      console.error('Failed to invert selection:', error)
      message.error('反选失败')
    }
  }

  // Server-level tool selection functions
  const handleSelectAllServerTools = async (serverName: string) => {
    if (!apiKeyId) {
      message.error('无法批量授权：缺少 API Key ID')
      return
    }

    try {
      // Grant all tools from this server
      await ApiKeyService.grantServerToolsToApiKey(apiKeyId, serverName)

      // Load tools if not already loaded
      const tools =
        serverTools[serverName] ||
        (await ToolService.listMcpServerTools(serverName))

      // Update server tools cache
      if (!serverTools[serverName]) {
        setServerTools((prev) => ({ ...prev, [serverName]: tools }))
      }

      // Add all tools to authorized set
      const newAuthorized = new Set(authorizedToolIds)
      tools.forEach((tool) => newAuthorized.add(tool.id))
      setAuthorizedToolIds(newAuthorized)

      // Ensure server is in allowed_servers
      if (!isServerSelected(serverName)) {
        onChange({
          ...permissions,
          allowed_servers: [...permissions.allowed_servers, serverName],
          allowed_tools: Array.from(newAuthorized),
        })
      }

      message.success(`已授权 ${serverName} 的所有工具`)
    } catch (error) {
      console.error('Failed to select all server tools:', error)
      message.error('全选失败')
    }
  }

  const handleInvertServerTools = async (serverName: string) => {
    if (!apiKeyId) {
      message.error('无法反选：缺少 API Key ID')
      return
    }

    try {
      // Load tools if not already loaded
      const tools =
        serverTools[serverName] ||
        (await ToolService.listMcpServerTools(serverName))

      // Update server tools cache
      if (!serverTools[serverName]) {
        setServerTools((prev) => ({ ...prev, [serverName]: tools }))
      }

      // Determine which tools to add and which to remove
      const toolsToAdd = tools.filter((tool) => !authorizedToolIds.has(tool.id))
      const toolsToRemove = tools.filter((tool) =>
        authorizedToolIds.has(tool.id),
      )

      // Execute all changes in parallel
      const addPromises = toolsToAdd.map((tool) =>
        ApiKeyService.addToolPermission(apiKeyId, tool.id),
      )
      const removePromises = toolsToRemove.map((tool) =>
        ApiKeyService.removeToolPermission(apiKeyId, tool.id),
      )

      await Promise.all([...addPromises, ...removePromises])

      // Update authorized tools set
      const newAuthorized = new Set(authorizedToolIds)
      toolsToAdd.forEach((tool) => newAuthorized.add(tool.id))
      toolsToRemove.forEach((tool) => newAuthorized.delete(tool.id))
      setAuthorizedToolIds(newAuthorized)

      // Update allowed servers
      const hasAuthorizedTools = toolsToAdd.length > 0
      if (hasAuthorizedTools && !isServerSelected(serverName)) {
        onChange({
          ...permissions,
          allowed_servers: [...permissions.allowed_servers, serverName],
          allowed_tools: Array.from(newAuthorized),
        })
      } else if (!hasAuthorizedTools && isServerSelected(serverName)) {
        // Check if there are any other authorized tools from this server
        const remainingTools = tools.filter((t) => newAuthorized.has(t.id))
        if (remainingTools.length === 0) {
          onChange({
            ...permissions,
            allowed_servers: permissions.allowed_servers.filter(
              (s) => s !== serverName,
            ),
            allowed_tools: Array.from(newAuthorized),
          })
        }
      }

      message.success(`已反选 ${serverName} 的工具`)
    } catch (error) {
      console.error('Failed to invert server tools:', error)
      message.error('反选失败')
    }
  }

  const handleDeselectAllServerTools = async (serverName: string) => {
    if (!apiKeyId) {
      message.error('无法批量撤销：缺少 API Key ID')
      return
    }

    try {
      // Revoke all tools from this server
      await ApiKeyService.revokeServerToolsFromApiKey(apiKeyId, serverName)

      // Load tools if not already loaded
      const tools =
        serverTools[serverName] ||
        (await ToolService.listMcpServerTools(serverName))

      // Update server tools cache
      if (!serverTools[serverName]) {
        setServerTools((prev) => ({ ...prev, [serverName]: tools }))
      }

      // Remove all tools from authorized set
      const newAuthorized = new Set(authorizedToolIds)
      tools.forEach((tool) => newAuthorized.delete(tool.id))
      setAuthorizedToolIds(newAuthorized)

      // Remove server from allowed_servers if no more authorized tools
      if (isServerSelected(serverName)) {
        onChange({
          ...permissions,
          allowed_servers: permissions.allowed_servers.filter(
            (s) => s !== serverName,
          ),
          allowed_tools: Array.from(newAuthorized),
        })
      }

      message.success(`已取消 ${serverName} 的所有工具授权`)
    } catch (error) {
      console.error('Failed to deselect all server tools:', error)
      message.error('取消全选失败')
    }
  }

  if (loading) {
    return <div className='text-sm'>加载中...</div>
  }

  return (
    <div className='space-y-3'>
      {/* Action Buttons */}
      {apiKeyId && servers.length > 0 && (
        <div className='flex items-center justify-between border-b  pb-3'>
          <div className='text-sm'>
            已授权 {permissions.allowed_servers.length} 个服务和{' '}
            {authorizedToolIds.size} 个工具
          </div>
          <div className='flex space-x-2'>
            <button
              onClick={handleSelectAll}
              className='px-3 py-1.5 text-xs font-medium rounded-md bg-primary-100 text-primary-700 hover:bg-primary-200 dark:bg-primary-900/30  dark:hover:bg-primary-900/50 transition-colors'>
              全选
            </button>
            <button
              onClick={handleDeselectAll}
              className='px-3 py-1.5 text-xs font-medium rounded-md bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-700   transition-colors'>
              取消全选
            </button>
            <button
              onClick={handleInvertSelection}
              className='px-3 py-1.5 text-xs font-medium rounded-md bg-purple-100 text-purple-700 hover:bg-purple-200 dark:bg-purple-900/30  dark:hover:bg-purple-900/50 transition-colors'>
              反选
            </button>
          </div>
        </div>
      )}

      {/* Server and Tool List */}
      <div className='space-y-2 max-h-96 overflow-y-auto border border-gray-200  rounded-lg p-4'>
        {servers.length === 0 ? (
          <div className='text-sm'>暂无可用的MCP服务器</div>
        ) : (
          servers.map((server) => {
            const isExpanded = expandedServers.has(server.name)
            const tools = serverTools[server.name] || []
            const isLoadingTools = loadingTools.has(server.name)
            const allToolsAuthorized = apiKeyId
              ? areAllServerToolsAuthorized(server.name)
              : false

            return (
              <div
                key={server.name}
                className='border border-gray-200  rounded-lg'>
                {/* Server Level */}
                <div className='flex items-center p-2'>
                  <button
                    onClick={() => toggleServerExpansion(server.name)}
                    className='mr-2 '>
                    {isExpanded ? (
                      <ChevronDown size={16} />
                    ) : (
                      <ChevronRight size={16} />
                    )}
                  </button>
                  <input
                    type='checkbox'
                    checked={
                      isServerSelected(server.name) || allToolsAuthorized
                    }
                    onChange={() => toggleServerSelection(server.name)}
                    className='mr-2'
                    disabled={!apiKeyId && allToolsAuthorized}
                  />
                  <div className='flex-1 flex items-center justify-between'>
                    <div>
                      <div className='text-sm font-medium  '>{server.name}</div>
                      {server.description && (
                        <div className='text-xs  '>{server.description}</div>
                      )}
                    </div>
                    <div className='text-xs text-gray-500 '>
                      {server.tool_count ? `${server.tool_count} 个工具` : ''}
                    </div>
                  </div>
                </div>

                {/* Tool Level */}
                {isExpanded && (
                  <div className='border-t border-gray-200   '>
                    {isLoadingTools ? (
                      <div className='p-4 text-sm '>加载工具中...</div>
                    ) : tools.length === 0 ? (
                      <div className='p-4 text-sm'>该服务器暂无工具</div>
                    ) : (
                      <>
                        {/* Server-level tool actions */}
                        {apiKeyId && (
                          <div className='flex items-center justify-between px-3 py-2 border-b '>
                            <div className='text-xs '>
                              已授权{' '}
                              {
                                tools.filter((t) => authorizedToolIds.has(t.id))
                                  .length
                              }
                              /{tools.length} 个工具
                            </div>
                            <div className='flex space-x-1'>
                              <button
                                onClick={() =>
                                  handleSelectAllServerTools(server.name)
                                }
                                className='px-2 py-1 text-xs font-medium rounded bg-primary-100 text-primary-700 hover:bg-primary-200 dark:bg-primary-900/40  dark:hover:bg-primary-900/60 transition-colors'>
                                全选
                              </button>
                              <button
                                onClick={() =>
                                  handleDeselectAllServerTools(server.name)
                                }
                                className='px-2 py-1 text-xs font-medium rounded bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-700   transition-colors'>
                                取消全选
                              </button>
                              <button
                                onClick={() =>
                                  handleInvertServerTools(server.name)
                                }
                                className='px-2 py-1 text-xs font-medium rounded bg-purple-100 text-purple-700 hover:bg-purple-200 dark:bg-purple-900/40  dark:hover:bg-purple-900/60 transition-colors'>
                                反选
                              </button>
                            </div>
                          </div>
                        )}
                        {/* Tool list */}
                        <div className='p-2 space-y-1'>
                          {tools.map((tool) => {
                            const isAuthorized = isToolAuthorized(tool.id)
                            return (
                              <div
                                key={tool.id}
                                className='flex items-center p-2 rounded'>
                                <input
                                  type='checkbox'
                                  checked={isAuthorized}
                                  onChange={() =>
                                    toggleToolAuthorization(server.name, tool)
                                  }
                                  className='mr-2'
                                  disabled={!apiKeyId}
                                />
                                <div className='flex-1'>
                                  <div className='text-sm'>{tool.name}</div>
                                  {tool.description && (
                                    <div className='text-xs '>
                                      {tool.description}
                                    </div>
                                  )}
                                </div>
                                {!tool.enabled && (
                                  <span className='text-xs text-gray-500 '>
                                    (已禁用)
                                  </span>
                                )}
                              </div>
                            )
                          })}
                        </div>
                      </>
                    )}
                  </div>
                )}
              </div>
            )
          })
        )}
      </div>
    </div>
  )
}

export default ApiKeyPermissionSelector
