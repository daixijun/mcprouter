import React, { useEffect, useState } from 'react'
import ConfirmModal from '../components/ConfirmModal'
import ToolManager from '../components/ToolManager'
import { ApiService } from '../services/api'
import toastService from '../services/toastService'
import type { McpServer, McpServerInfo } from '../types'

interface McpServerManagerProps {
  onServiceChange?: () => void
}

const McpServerManager: React.FC<McpServerManagerProps> = ({
  onServiceChange,
}) => {
  const [mcpServers, setMcpServers] = useState<McpServer[]>([])
  const [filter, setFilter] = useState<'all' | 'running' | 'stopped'>('all')
  const [searchQuery, setSearchQuery] = useState('')
  const [showAddService, setShowAddService] = useState(false)
  const [showEditService, setShowEditService] = useState(false)
  const [editingService, setEditingService] = useState<McpServer | null>(null)
  const [showToolsModal, setShowToolsModal] = useState(false)
  // @ts-ignore - Used in tools modal
  const [selectedServiceForTools, setSelectedServiceForTools] =
    useState<McpServer | null>(null)

  // Confirm modal state
  const [showConfirmModal, setShowConfirmModal] = useState(false)
  const [confirmModalData, setConfirmModalData] = useState<{
    title: string
    message: string
    onConfirm: () => void
  } | null>(null)
  const [newServiceConfig, setNewServiceConfig] = useState({
    name: '',
    description: '',
    transport: 'stdio' as 'stdio' | 'sse' | 'streamablehttp',
    command: '',
    args: '',
    url: '',
    env: '',
    headers: '',
  })
  const [loading, setLoading] = useState(false)
  const [expandedServices, setExpandedServices] = useState<Set<string>>(
    new Set(),
  )

  // Add service mode: 'form' or 'json'
  const [addServiceMode, setAddServiceMode] = useState<'form' | 'json'>('form')
  const [jsonConfig, setJsonConfig] = useState('')
  const [jsonError, setJsonError] = useState('')

  useEffect(() => {
    fetchMcpServers()
  }, [])

  // Handle ESC key to close modals
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (showAddService) {
          setShowAddService(false)
          setAddServiceMode('form')
          setJsonConfig('')
          setJsonError('')
          setNewServiceConfig({
            name: '',
            description: '',
            transport: 'stdio',
            command: '',
            args: '',
            url: '',
            env: '',
            headers: '',
          })
        } else if (showEditService) {
          setShowEditService(false)
          setEditingService(null)
          setNewServiceConfig({
            name: '',
            description: '',
            transport: 'stdio',
            command: '',
            args: '',
            url: '',
            env: '',
            headers: '',
          })
        } else if (showToolsModal) {
          setShowToolsModal(false)
        } else if (showConfirmModal) {
          setShowConfirmModal(false)
          setConfirmModalData(null)
        }
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [showAddService, showEditService, showToolsModal, showConfirmModal])

  const fetchMcpServers = async () => {
    setLoading(true)
    try {
      console.log('Starting to fetch MCP servers...')
      const serverInfos = await ApiService.listMcpServers()

      console.log('Loaded server infos:', serverInfos)

      // Debug version information
      serverInfos.forEach((mcpServer) => {
        if (mcpServer.version) {
          console.log(
            `Service ${mcpServer.name} (${mcpServer.name}) version:`,
            mcpServer.version,
          )
        } else {
          console.log(
            `Service ${mcpServer.name} (${mcpServer.name}) version: None`,
          )
        }
      })

      // Convert server infos to McpServer format
      const mcpServices: McpServer[] = serverInfos.map(
        (serverInfo: McpServerInfo) => {
          // Debug logging for HTTP services
          if (
            serverInfo.transport &&
            (serverInfo.transport === 'sse' ||
              serverInfo.transport === 'streamablehttp')
          ) {
            console.log('HTTP Service Config:', {
              name: serverInfo.name,
              transport: serverInfo.transport,
              url: serverInfo.url,
            })
          }

          return {
            name: serverInfo.name,
            description: serverInfo.description,
            command: serverInfo.command || 'unknown',
            args: serverInfo.args || [],
            transport: serverInfo.transport as
              | 'stdio'
              | 'sse'
              | 'streamablehttp',
            url: serverInfo.url,
            status: serverInfo.is_active ? 'running' : 'stopped',
            enabled: serverInfo.enabled,
            is_active: serverInfo.is_active,
            env: serverInfo.env_vars || {},
            version: serverInfo.version || undefined,
            created_at: new Date().toISOString(),
            tools: [], // Empty tools array for compatibility
            tool_count: serverInfo.tool_count,
          }
        },
      )

      console.log('Setting MCP servers:', mcpServices)
      setMcpServers(mcpServices)


      console.log('Successfully loaded', mcpServices.length, 'services')
      onServiceChange?.()
    } catch (error) {
      console.error('Failed to load services:', error)
      toastService.sendErrorNotification('加载服务失败，请检查配置文件是否正确')
    } finally {
      console.log('Setting loading to false')
      setLoading(false)
    }
  }


  const filteredMcpServers = mcpServers.filter((mcpServer) => {
    const matchesFilter =
      filter === 'all' ||
      (filter === 'running' && mcpServer.enabled && !false) ||
      (filter === 'stopped' && (!mcpServer.enabled || false))

    const matchesSearch =
      mcpServer.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      mcpServer.command.toLowerCase().includes(searchQuery.toLowerCase())

    return matchesFilter && matchesSearch
  })

  const handleCheckConnectivity = async (serviceName: string) => {
    try {
      await ApiService.checkMcpServerConnectivity(serviceName)
      await fetchMcpServers()
      const service = mcpServers.find((s) => s.name === serviceName)
      if (service) {
        toastService.sendSuccessNotification(`服务 "${service.name}" 连接成功`)
      }
    } catch (error) {
      console.error('Failed to check service connectivity:', error)
      // Reload services to update the service status
      await fetchMcpServers()
      // Show notification with generic message
      toastService.sendErrorNotification(
        '连接服务失败，请查看服务卡片中的错误详情。',
      )
    }
  }

  const toggleServiceExpanded = async (serviceName: string) => {
    const newExpanded = new Set(expandedServices)
    if (newExpanded.has(serviceName)) {
      newExpanded.delete(serviceName)
    } else {
      newExpanded.add(serviceName)
    }
    setExpandedServices(newExpanded)
  }


  const handleRemove = async (
    serviceName: string,
    serviceDisplayName?: string,
  ) => {
    const confirmMessage = `确定要删除服务"${
      serviceDisplayName || serviceName || '未知服务'
    }"吗？此操作不可撤销。`

    // Show confirm modal instead of native confirm
    setConfirmModalData({
      title: '确认删除',
      message: confirmMessage,
      onConfirm: async () => {
        try {
          console.log('正在删除服务:', serviceName)
          const result = await ApiService.removeMcpServer(serviceName)
          console.log('删除服务结果:', result)
          await fetchMcpServers()
          toastService.sendSuccessNotification('服务删除成功')
        } catch (error) {
          console.error('Failed to remove service:', error)
          toastService.sendErrorNotification(
            `删除服务失败: ${
              error instanceof Error ? error.message : '未知错误'
            }`,
          )
        } finally {
          setShowConfirmModal(false)
          setConfirmModalData(null)
        }
      },
    })
    setShowConfirmModal(true)
  }

  // Import configuration from JSON
  const handleImportFromJson = () => {
    setJsonError('')
    try {
      const config = JSON.parse(jsonConfig)

      // Validate required fields
      if (!config.name) {
        setJsonError('JSON配置缺少必需字段: name')
        return
      }
      if (!config.transport) {
        setJsonError('JSON配置缺少必需字段: transport')
        return
      }

      // Validate transport value
      if (!['stdio', 'sse', 'streamablehttp'].includes(config.transport)) {
        setJsonError('transport 字段必须是: stdio, sse 或 streamablehttp')
        return
      }

      // Convert env object to string format for textarea
      let envString = ''
      if (config.env && typeof config.env === 'object') {
        envString = Object.entries(config.env)
          .map(([key, value]) => `${key}=${value}`)
          .join('\n')
      }

      // Convert headers object to string format for textarea
      let headersString = ''
      if (config.headers && typeof config.headers === 'object') {
        headersString = Object.entries(config.headers)
          .map(([key, value]) => `${key}: ${value}`)
          .join('\n')
      }

      // Set the configuration
      setNewServiceConfig({
        name: config.name || '',
        description: config.description || '',
        transport: config.transport || 'stdio',
        command: config.command || '',
        args: Array.isArray(config.args)
          ? config.args.join(' ')
          : config.args || '',
        url: config.url || '',
        env: envString,
        headers: headersString,
      })

      // Switch to form mode to show the imported config
      setAddServiceMode('form')
      toastService.sendSuccessNotification('JSON配置导入成功')
    } catch (error) {
      setJsonError(
        `JSON解析失败: ${error instanceof Error ? error.message : '未知错误'}`,
      )
    }
  }

  // Export current form configuration to JSON
  const handleExportToJson = () => {
    const config: any = {
      name: newServiceConfig.name,
      transport: newServiceConfig.transport,
    }

    if (newServiceConfig.description) {
      config.description = newServiceConfig.description
    }

    if (newServiceConfig.transport === 'stdio') {
      config.command = newServiceConfig.command
      if (newServiceConfig.args) {
        config.args = newServiceConfig.args
          .trim()
          .split(/\s+/)
          .filter((arg) => arg.length > 0)
      }

      // Parse environment variables
      if (newServiceConfig.env) {
        const envObj: Record<string, string> = {}
        newServiceConfig.env.split('\n').forEach((line) => {
          const [key, value] = line.split('=')
          if (key && value) {
            envObj[key.trim()] = value.trim()
          }
        })
        if (Object.keys(envObj).length > 0) {
          config.env = envObj
        }
      }
    } else {
      config.url = newServiceConfig.url

      // Parse headers
      if (newServiceConfig.headers) {
        const headersObj: Record<string, string> = {}
        newServiceConfig.headers.split('\n').forEach((line) => {
          const [key, value] = line.split(':')
          if (key && value) {
            headersObj[key.trim()] = value.trim()
          }
        })
        if (Object.keys(headersObj).length > 0) {
          config.headers = headersObj
        }
      }
    }

    const jsonString = JSON.stringify(config, null, 2)
    setJsonConfig(jsonString)
    setAddServiceMode('json')
    toastService.sendSuccessNotification('配置已导出为JSON')
  }

  const handleAddService = async () => {
    try {
      // Process environment variables or headers based on transport type
      const envVars: [string, string][] = []
      const headers: [string, string][] = []

      if (newServiceConfig.transport === 'stdio') {
        // Process environment variables for STDIO
        newServiceConfig.env.split('\n').forEach((line) => {
          const [key, value] = line.split('=')
          if (key && value) {
            envVars.push([key.trim(), value.trim()])
          }
        })
      } else {
        // Process headers for SSE and StreamableHTTP
        newServiceConfig.headers.split('\n').forEach((line) => {
          const [key, value] = line.split(':')
          if (key && value) {
            headers.push([key.trim(), value.trim()])
          }
        })
      }

      // For STDIO transport, use command and args
      // For SSE/StreamableHTTP transport, use URL
      if (newServiceConfig.transport === 'stdio') {
        console.log('Adding STDIO service:', newServiceConfig.name)

        // Parse command line arguments by splitting on spaces
        // This is a simple approach and might need improvement for complex cases
        const argsArray = newServiceConfig.args
          .trim()
          .split(/\s+/)
          .filter((arg) => arg.length > 0)

        await ApiService.addMcpServer(
          newServiceConfig.name,
          newServiceConfig.command,
          argsArray,
          newServiceConfig.transport,
          undefined, // No URL for STDIO
          newServiceConfig.description || undefined, // description
          envVars.length > 0 ? envVars : undefined,
          undefined, // No headers for STDIO
        )
      } else {
        // For URL-based transports, pass the URL and headers
        console.log(
          'Adding HTTP service:',
          newServiceConfig.name,
          'URL:',
          newServiceConfig.url,
        )
        await ApiService.addMcpServer(
          newServiceConfig.name,
          '', // No command for URL-based transports
          [], // No args for URL-based transports
          newServiceConfig.transport,
          newServiceConfig.url || undefined,
          newServiceConfig.description || undefined, // description
          undefined, // No env vars for HTTP transports
          headers.length > 0 ? headers : undefined, // Pass headers for HTTP transports
        )
      }

      setShowAddService(false)
      setAddServiceMode('form')
      setJsonConfig('')
      setJsonError('')
      setNewServiceConfig({
        name: '',
        description: '',
        transport: 'stdio',
        command: '',
        args: '',
        url: '',
        env: '',
        headers: '',
      })
      await fetchMcpServers()
      toastService.sendSuccessNotification('服务添加成功')
    } catch (error) {
      console.error('Failed to add service:', error)
      toastService.sendErrorNotification(
        '添加服务失败，请检查日志获取详细信息。',
      )
    }
  }

  const handleEditService = async () => {
    if (!editingService) return

    try {
      // First, remove the old service
      await ApiService.removeMcpServer(editingService.name)

      // Process environment variables or headers based on transport type
      const envVars: [string, string][] = []
      const headers: [string, string][] = []

      if (newServiceConfig.transport === 'stdio') {
        // Process environment variables for STDIO
        newServiceConfig.env.split('\n').forEach((line) => {
          const [key, value] = line.split('=')
          if (key && value) {
            envVars.push([key.trim(), value.trim()])
          }
        })
      } else {
        // Process headers for SSE and StreamableHTTP
        newServiceConfig.headers.split('\n').forEach((line) => {
          const [key, value] = line.split(':')
          if (key && value) {
            headers.push([key.trim(), value.trim()])
          }
        })
      }

      // Add the updated service
      if (newServiceConfig.transport === 'stdio') {
        const argsArray = newServiceConfig.args
          .trim()
          .split(/\s+/)
          .filter((arg) => arg.length > 0)

        await ApiService.addMcpServer(
          newServiceConfig.name,
          newServiceConfig.command,
          argsArray,
          newServiceConfig.transport,
          undefined,
          newServiceConfig.description || undefined,
          envVars.length > 0 ? envVars : undefined,
          undefined,
        )
      } else {
        await ApiService.addMcpServer(
          newServiceConfig.name,
          '',
          [],
          newServiceConfig.transport,
          newServiceConfig.url || undefined,
          newServiceConfig.description || undefined,
          undefined,
          headers.length > 0 ? headers : undefined,
        )
      }

      setShowEditService(false)
      setEditingService(null)
      setNewServiceConfig({
        name: '',
        description: '',
        transport: 'stdio',
        command: '',
        args: '',
        url: '',
        env: '',
        headers: '',
      })
      await fetchMcpServers()
      toastService.sendSuccessNotification('服务更新成功')
    } catch (error) {
      console.error('Failed to edit service:', error)
      toastService.sendErrorNotification(
        '编辑服务失败，请检查日志获取详细信息。',
      )
    }
  }

  const handleToggleService = async (serviceName: string) => {
    try {
      const newState = await ApiService.toggleMcpServer(serviceName)
      await fetchMcpServers()

      const service = mcpServers.find((s) => s.name === serviceName)
      const displayName = service?.name || serviceName

      if (newState) {
        toastService.sendSuccessNotification(`服务 "${displayName}" 已启用`)
      } else {
        toastService.sendSuccessNotification(`服务 "${displayName}" 已禁用`)
      }
    } catch (error) {
      console.error('Failed to toggle service:', error)
      toastService.sendErrorNotification(
        '切换服务状态失败，请检查日志获取详细信息。',
      )
    }
  }

  const getStatusIcon = (service: any) => {
    if (!service.enabled) {
      return '⚫' // Disabled
    }
    return '🟢' // Connected
  }

  const getStatusText = (service: any) => {
    if (!service.enabled) {
      return '已禁用'
    }
    return '已连接'
  }


  return (
    <div className='h-full flex flex-col'>
      {/* Controls - Fixed at top */}
      <div className='flex-shrink-0 mb-4'>
        <div className='flex flex-col md:flex-row gap-3 justify-between items-center'>
          <div className='flex flex-col md:flex-row gap-3 items-center w-full md:w-auto'>
            <div className='flex-1 md:flex-initial min-w-[400px]'>
              <input
                type='text'
                placeholder='🔍 搜索服务...'
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className='input-modern w-full'
              />
            </div>

            <div className='inline-flex rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 shadow-sm'>
              <button
                onClick={() => setFilter('all')}
                className={`px-3 py-1.5 text-sm font-medium border-r border-gray-200 dark:border-gray-600 rounded-l-lg transition-colors duration-200 ${
                  filter === 'all'
                    ? 'bg-blue-600 text-white'
                    : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600'
                }`}>
                全部
              </button>
              <button
                onClick={() => setFilter('running')}
                className={`px-3 py-1.5 text-sm font-medium border-r border-gray-200 dark:border-gray-600 transition-colors duration-200 ${
                  filter === 'running'
                    ? 'bg-green-600 text-white'
                    : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600'
                }`}>
                🟢 已连接
              </button>
              <button
                onClick={() => setFilter('stopped')}
                className={`px-3 py-1.5 text-sm font-medium rounded-r-lg transition-colors duration-200 ${
                  filter === 'stopped'
                    ? 'bg-red-600 text-white'
                    : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600'
                }`}>
                ⚫ 未连接
              </button>
            </div>
          </div>

          <button
            onClick={() => setShowAddService(true)}
            className='btn-modern btn-primary-modern w-full md:w-auto'>
            ➕ 添加服务
          </button>
        </div>
      </div>

      {/* Services List - Scrollable area */}
      <div className='flex-1 min-h-0'>
        {loading ? (
          <div className='card-glass p-6 text-center'>
            <div className='animate-spin rounded-full h-8 w-8 border-4 border-blue-500 border-t-transparent mx-auto mb-3'></div>
            <p className='text-sm text-gray-600 dark:text-gray-300'>
              正在加载服务列表...
            </p>
          </div>
        ) : filteredMcpServers.length > 0 ? (
          <div className='h-full overflow-y-auto pr-1 scrollbar-custom'>
            <div className='space-y-3'>
              {filteredMcpServers.map((mcpServer) => (
                <div
                  key={mcpServer.name}
                  className='card-glass p-4 compact-card'>
                  <div className='flex flex-col gap-3'>
                    {/* Service Info and Controls Row */}
                    <div className='flex flex-col md:flex-row md:items-start md:justify-between gap-3'>
                      {/* Service Information */}
                      <div className='flex-1 min-w-0'>
                        <div className='flex items-center space-x-2 mb-2'>
                          <h3 className='font-bold text-base text-gray-800 dark:text-gray-100 compact-title'>
                            {mcpServer.name}
                          </h3>
                          {mcpServer.version && (
                            <span className='badge-modern bg-purple-100 dark:bg-purple-900/30 text-purple-800 dark:text-purple-300 text-xs'>
                              v{mcpServer.version}
                            </span>
                          )}
                          <span
                            className={`badge-modern ${
                              mcpServer.enabled && !false
                                ? 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300'
                                : false
                                ? 'bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-300'
                                : 'bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200'
                            }`}>
                            {getStatusIcon(mcpServer)}{' '}
                            {getStatusText(mcpServer)}
                          </span>
                        </div>

                        <div className='space-y-1 text-xs text-gray-600 dark:text-gray-300 compact-list'>
                          <div className='flex items-center space-x-2'>
                            <span className='font-medium'>传输协议:</span>
                            <span className='badge-modern bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300 text-xs'>
                              {mcpServer.transport === 'stdio'
                                ? '🔌 STDIO'
                                : mcpServer.transport === 'sse'
                                ? '🌊 SSE'
                                : '📡 StreamableHTTP'}
                            </span>
                          </div>

                          {mcpServer.transport === 'stdio' ? (
                            <>
                              <div className='flex items-center space-x-2'>
                                <span className='font-medium'>命令:</span>
                                <code className='bg-gray-100 dark:bg-gray-700 px-2 py-0.5 rounded text-xs'>
                                  {mcpServer.command}
                                </code>
                                {mcpServer.args &&
                                  mcpServer.args.length > 0 && (
                                    <>
                                      <span className='font-medium'>参数:</span>
                                      <code className='bg-gray-100 dark:bg-gray-700 px-2 py-0.5 rounded text-xs'>
                                        {mcpServer.args.join(' ')}
                                      </code>
                                    </>
                                  )}
                              </div>
                              {mcpServer.description && (
                                <div className='text-xs text-gray-600 dark:text-gray-300 mt-1'>
                                  <span className='font-medium'>描述:</span>{' '}
                                  {mcpServer.description}
                                </div>
                              )}
                            </>
                          ) : (
                            <>
                              {mcpServer.url && (
                                <div className='flex items-center space-x-2'>
                                  <span className='font-medium'>地址:</span>
                                  <code className='bg-gray-100 dark:bg-gray-700 px-2 py-0.5 rounded text-xs'>
                                    {mcpServer.url}
                                  </code>
                                </div>
                              )}
                              {mcpServer.description && (
                                <div className='text-xs text-gray-600 dark:text-gray-300 mt-1'>
                                  <span className='font-medium'>描述:</span>{' '}
                                  {mcpServer.description}
                                </div>
                              )}
                            </>
                          )}
                        </div>

                        {/* Connection Error Display */}
                        {false && (
                          <div className='mt-2 p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg'>
                            <div className='flex items-start justify-between'>
                              <div className='flex items-start space-x-2'>
                                <span className='text-red-500 text-xs'>⚠️</span>
                                <div>
                                  <p className='text-xs font-medium text-red-800'>
                                    连接失败
                                  </p>
                                  <p className='text-xs text-red-600 mt-1 font-mono'>
                                    {false}
                                  </p>
                                </div>
                              </div>
                            </div>
                          </div>
                        )}
                      </div>

                      {/* Controls - Fixed position */}
                      <div className='flex items-center gap-2 flex-shrink-0'>
                        {/* Enable/Disable Switch */}
                        <div className='flex items-center space-x-2'>
                          <span className='text-xs text-gray-700 dark:text-gray-300 font-medium'>
                            {mcpServer.enabled ? '已启用' : '已禁用'}
                          </span>
                          <button
                            onClick={() => handleToggleService(mcpServer.name)}
                            className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
                              mcpServer.enabled ? 'bg-green-500' : 'bg-gray-300'
                            }`}
                            aria-label='Toggle service'>
                            <span
                              className={`inline-block h-3 w-3 transform rounded-full bg-white transition-transform ${
                                mcpServer.enabled
                                  ? 'translate-x-5'
                                  : 'translate-x-1'
                              }`}
                            />
                          </button>
                        </div>

                        {/* Edit and Delete buttons */}
                        <div className='flex items-center gap-1'>
                          {/* Reconnect button - only show for enabled services with errors */}
                          {mcpServer.enabled && false && (
                            <button
                              onClick={() =>
                                handleCheckConnectivity(mcpServer.name)
                              }
                              className='btn-modern bg-orange-500 hover:bg-orange-600 text-white text-xs px-2 py-1'>
                              🔄 重新连接
                            </button>
                          )}

                          {/* Edit button */}
                          <button
                            onClick={() => {
                              // Pre-populate the form with current service data
                              const envString = mcpServer.env
                                ? Object.entries(mcpServer.env)
                                    .map(([key, value]) => `${key}=${value}`)
                                    .join('\n')
                                : ''

                              setNewServiceConfig({
                                name: mcpServer.name,
                                description: mcpServer.description || '',
                                transport: mcpServer.transport,
                                command: mcpServer.command || '',
                                args: mcpServer.args
                                  ? mcpServer.args.join(' ')
                                  : '',
                                url: mcpServer.url || '',
                                env: envString,
                                headers: '',
                              })
                              setEditingService(mcpServer)
                              setShowEditService(true)
                            }}
                            className='p-1.5 rounded-lg hover:bg-blue-50 transition-colors group'
                            title='编辑服务'>
                            <svg
                              xmlns='http://www.w3.org/2000/svg'
                              className='h-4 w-4 text-blue-500 group-hover:text-blue-600'
                              fill='none'
                              viewBox='0 0 24 24'
                              stroke='currentColor'>
                              <path
                                strokeLinecap='round'
                                strokeLinejoin='round'
                                strokeWidth={2}
                                d='M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z'
                              />
                            </svg>
                          </button>

                          {/* Delete button */}
                          <button
                            onClick={() =>
                              handleRemove(mcpServer.name, mcpServer.name)
                            }
                            className='p-1.5 rounded-lg hover:bg-red-50 transition-colors group'
                            title='删除服务'>
                            <svg
                              xmlns='http://www.w3.org/2000/svg'
                              className='h-4 w-4 text-red-500 group-hover:text-red-600'
                              fill='none'
                              viewBox='0 0 24 24'
                              stroke='currentColor'>
                              <path
                                strokeLinecap='round'
                                strokeLinejoin='round'
                                strokeWidth={2}
                                d='M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16'
                              />
                            </svg>
                          </button>
                        </div>
                      </div>
                    </div>

                    {/* Tools List Display - Separate section */}
                    {mcpServer.enabled && !false && (
                      <div className='border-t pt-3'>
                        <button
                          onClick={() => toggleServiceExpanded(mcpServer.name)}
                          className='flex items-center space-x-2 text-xs font-medium text-gray-700 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400 transition-colors'>
                          <span>
                            {expandedServices.has(mcpServer.name) ? '▼' : '▶'}
                          </span>
                          <span>
                            工具管理
                            {mcpServer.tool_count !== undefined &&
                              mcpServer.tool_count !== null ? (
                              <span className='ml-1 text-gray-500 dark:text-gray-400'>
                                ({mcpServer.tool_count})
                              </span>
                            ) : (
                              <span className='ml-1 text-gray-400 dark:text-gray-500'>
                                (未加载)
                              </span>
                            )}
                          </span>
                        </button>

                        {expandedServices.has(mcpServer.name) && (
                          <div className='mt-2'>
                            <ToolManager mcpServer={mcpServer} />
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className='card-glass p-8 text-center'>
            <div className='text-4xl mb-3'>📭</div>
            <h3 className='text-lg font-semibold text-gray-700 mb-2'>
              暂无服务
            </h3>
            <p className='text-sm text-gray-500 dark:text-gray-400 mb-4'>
              {searchQuery || filter !== 'all'
                ? '没有找到匹配的服务，请尝试调整搜索条件。'
                : '还没有配置任何 MCP 服务。点击"添加服务"开始配置。'}
            </p>
            {!searchQuery && filter === 'all' && (
              <button
                onClick={() => setShowAddService(true)}
                className='btn-modern btn-primary-modern'>
                ➕ 添加第一个服务
              </button>
            )}
          </div>
        )}
      </div>

      {/* Edit Service Modal */}
      {showEditService && (
        <div className='modal-modern'>
          <div className='modal-content-modern max-w-2xl max-h-[90vh] overflow-y-auto compact-modal'>
            <div className='flex justify-between items-start mb-4'>
              <div>
                <h3 className='text-lg font-bold text-gray-800 dark:text-gray-100 compact-title'>
                  ✏️ 编辑服务
                </h3>
                <p className='text-sm text-gray-600 dark:text-gray-300'>
                  修改 MCP 服务配置
                </p>
              </div>
              <button
                onClick={() => {
                  setShowEditService(false)
                  setEditingService(null)
                }}
                className='btn-modern bg-gray-300 hover:bg-gray-400 text-gray-700 p-1.5'>
                ❌
              </button>
            </div>

            <div className='space-y-4'>
              <div>
                <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  📛 服务名称
                </label>
                <input
                  type='text'
                  value={newServiceConfig.name}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      name: e.target.value,
                    })
                  }
                  className='input-modern'
                  placeholder='输入服务名称'
                />
              </div>

              <div>
                <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  📝 服务描述
                </label>
                <textarea
                  value={newServiceConfig.description}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      description: e.target.value,
                    })
                  }
                  className='input-modern min-h-[60px]'
                  placeholder='服务描述（可选）'
                />
              </div>

              <div>
                <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  🔗 传输协议
                </label>
                <select
                  value={newServiceConfig.transport}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      transport: e.target.value as
                        | 'stdio'
                        | 'sse'
                        | 'streamablehttp',
                    })
                  }
                  className='input-modern'>
                  <option value='stdio'>STDIO (标准输入输出)</option>
                  <option value='sse'>SSE (Server-Sent Events)</option>
                  <option value='streamablehttp'>
                    StreamableHTTP (流式HTTP)
                  </option>
                </select>
              </div>

              {newServiceConfig.transport === 'stdio' ? (
                <>
                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      💻 执行命令
                    </label>
                    <input
                      type='text'
                      value={newServiceConfig.command}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          command: e.target.value,
                        })
                      }
                      className='input-modern'
                      placeholder='如: node, python, docker run ...'
                    />
                  </div>

                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      ⚙️ 命令参数
                    </label>
                    <input
                      type='text'
                      value={newServiceConfig.args}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          args: e.target.value,
                        })
                      }
                      className='input-modern'
                      placeholder='命令参数，用空格分隔'
                    />
                  </div>

                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      🔧 环境变量
                    </label>
                    <textarea
                      value={newServiceConfig.env}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          env: e.target.value,
                        })
                      }
                      className='input-modern min-h-[80px]'
                      placeholder='每行一个，格式: KEY=VALUE'
                    />
                  </div>
                </>
              ) : (
                <>
                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      🌐 服务地址
                    </label>
                    <input
                      type='text'
                      value={newServiceConfig.url}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          url: e.target.value,
                        })
                      }
                      className='input-modern'
                      placeholder='http://localhost:3000'
                    />
                  </div>

                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      📋 HTTP 请求头
                    </label>
                    <textarea
                      value={newServiceConfig.headers}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          headers: e.target.value,
                        })
                      }
                      className='input-modern min-h-[80px]'
                      placeholder='每行一个，格式: Header-Name: value'
                    />
                  </div>
                </>
              )}

              <div className='flex justify-end space-x-3 pt-4 border-t'>
                <button
                  onClick={() => {
                    setShowEditService(false)
                    setEditingService(null)
                  }}
                  className='btn-modern bg-gray-300 hover:bg-gray-400 text-gray-700 text-sm px-3 py-1.5'>
                  取消
                </button>
                <button
                  onClick={handleEditService}
                  className='btn-modern btn-primary-modern text-sm px-3 py-1.5'
                  disabled={
                    !newServiceConfig.name ||
                    !newServiceConfig.transport ||
                    (newServiceConfig.transport === 'stdio' &&
                      !newServiceConfig.command) ||
                    (newServiceConfig.transport !== 'stdio' &&
                      !newServiceConfig.url)
                  }>
                  ✏️ 保存修改
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Add Service Modal */}
      {showAddService && (
        <div className='modal-modern'>
          <div className='modal-content-modern max-w-2xl max-h-[90vh] overflow-y-auto compact-modal'>
            <div className='flex justify-between items-start mb-4'>
              <div>
                <h3 className='text-lg font-bold text-gray-800 dark:text-gray-100 compact-title'>
                  ➕ 添加新服务
                </h3>
                <p className='text-sm text-gray-600 dark:text-gray-300'>
                  配置新的 MCP 服务
                </p>
              </div>
              <button
                onClick={() => {
                  setShowAddService(false)
                  setAddServiceMode('form')
                  setJsonConfig('')
                  setJsonError('')
                  setNewServiceConfig({
                    name: '',
                    description: '',
                    transport: 'stdio',
                    command: '',
                    args: '',
                    url: '',
                    env: '',
                    headers: '',
                  })
                }}
                className='btn-modern bg-gray-300 hover:bg-gray-400 text-gray-700 p-1.5'>
                ❌
              </button>
            </div>

            {/* Mode Switcher */}
            <div className='flex gap-2 mb-4 border-b pb-2'>
              <button
                onClick={() => setAddServiceMode('form')}
                className={`px-4 py-2 text-sm font-medium rounded-t-lg transition-colors ${
                  addServiceMode === 'form'
                    ? 'bg-blue-500 text-white'
                    : 'bg-gray-100 dark:bg-gray-700 text-gray-700 hover:bg-gray-200'
                }`}>
                📝 表单模式
              </button>
              <button
                onClick={() => setAddServiceMode('json')}
                className={`px-4 py-2 text-sm font-medium rounded-t-lg transition-colors ${
                  addServiceMode === 'json'
                    ? 'bg-blue-500 text-white'
                    : 'bg-gray-100 dark:bg-gray-700 text-gray-700 hover:bg-gray-200'
                }`}>
                📄 JSON模式
              </button>
            </div>

            {/* Form Mode */}
            {addServiceMode === 'form' && (
              <div className='space-y-4'>
                <div>
                  <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    📛 服务名称
                  </label>
                  <input
                    type='text'
                    value={newServiceConfig.name}
                    onChange={(e) =>
                      setNewServiceConfig({
                        ...newServiceConfig,
                        name: e.target.value,
                      })
                    }
                    className='input-modern'
                    placeholder='输入服务名称'
                  />
                </div>

                <div>
                  <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    📝 服务描述
                  </label>
                  <textarea
                    value={newServiceConfig.description}
                    onChange={(e) =>
                      setNewServiceConfig({
                        ...newServiceConfig,
                        description: e.target.value,
                      })
                    }
                    className='input-modern min-h-[60px]'
                    placeholder='输入服务描述（可选）'
                  />
                </div>

                <div>
                  <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    🔗 协议类型
                  </label>
                  <div className='flex flex-wrap gap-2'>
                    <label className='flex items-center p-1.5 border rounded-lg cursor-pointer hover:bg-gray-50 dark:bg-gray-700 transition-colors'>
                      <input
                        type='radio'
                        name='transport'
                        value='stdio'
                        checked={newServiceConfig.transport === 'stdio'}
                        onChange={(e) =>
                          setNewServiceConfig({
                            ...newServiceConfig,
                            transport: e.target.value as
                              | 'stdio'
                              | 'sse'
                              | 'streamablehttp',
                          })
                        }
                        className='h-3 w-3 text-blue-600 border-gray-300 focus:ring-blue-500'
                      />
                      <div className='ml-1.5'>
                        <div className='text-xs font-medium text-gray-900 dark:text-gray-100'>
                          🔌 STDIO
                        </div>
                      </div>
                    </label>

                    <label className='flex items-center p-1.5 border rounded-lg cursor-pointer hover:bg-gray-50 dark:bg-gray-700 transition-colors'>
                      <input
                        type='radio'
                        name='transport'
                        value='sse'
                        checked={newServiceConfig.transport === 'sse'}
                        onChange={(e) =>
                          setNewServiceConfig({
                            ...newServiceConfig,
                            transport: e.target.value as
                              | 'stdio'
                              | 'sse'
                              | 'streamablehttp',
                          })
                        }
                        className='h-3 w-3 text-blue-600 border-gray-300 focus:ring-blue-500'
                      />
                      <div className='ml-1.5'>
                        <div className='text-xs font-medium text-gray-900 dark:text-gray-100'>
                          🌊 SSE
                        </div>
                      </div>
                    </label>

                    <label className='flex items-center p-1.5 border rounded-lg cursor-pointer hover:bg-gray-50 dark:bg-gray-700 transition-colors'>
                      <input
                        type='radio'
                        name='transport'
                        value='streamablehttp'
                        checked={
                          newServiceConfig.transport === 'streamablehttp'
                        }
                        onChange={(e) =>
                          setNewServiceConfig({
                            ...newServiceConfig,
                            transport: e.target.value as
                              | 'stdio'
                              | 'sse'
                              | 'streamablehttp',
                          })
                        }
                        className='h-3 w-3 text-blue-600 border-gray-300 focus:ring-blue-500'
                      />
                      <div className='ml-1.5'>
                        <div className='text-xs font-medium text-gray-900 dark:text-gray-100'>
                          📡 StreamableHTTP
                        </div>
                      </div>
                    </label>
                  </div>
                </div>

                {/* STDIO 协议配置 */}
                {newServiceConfig.transport === 'stdio' && (
                  <>
                    <div>
                      <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                        💻 执行命令
                      </label>
                      <input
                        type='text'
                        value={newServiceConfig.command}
                        onChange={(e) =>
                          setNewServiceConfig({
                            ...newServiceConfig,
                            command: e.target.value,
                          })
                        }
                        className='input-modern'
                        placeholder='uvx or npx'
                      />
                    </div>

                    <div>
                      <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                        ⚙️ 命令行参数
                      </label>
                      <input
                        type='text'
                        value={newServiceConfig.args}
                        onChange={(e) =>
                          setNewServiceConfig({
                            ...newServiceConfig,
                            args: e.target.value,
                          })
                        }
                        className='input-modern'
                        placeholder='例如: --port 8080 --host localhost'
                      />
                    </div>
                  </>
                )}

                {/* SSE 和 StreamableHTTP 协议配置 */}
                {(newServiceConfig.transport === 'sse' ||
                  newServiceConfig.transport === 'streamablehttp') && (
                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      🌐 服务URL
                    </label>
                    <input
                      type='url'
                      value={newServiceConfig.url}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          url: e.target.value,
                        })
                      }
                      className='input-modern'
                      placeholder={`例如: http://localhost:8080${
                        newServiceConfig.transport === 'sse'
                          ? '/events'
                          : '/stream'
                      }`}
                    />
                    <p className='text-xs text-gray-500 dark:text-gray-400 mt-1'>
                      提供{' '}
                      {newServiceConfig.transport === 'sse'
                        ? 'Server-Sent Events'
                        : 'HTTP 流式'}{' '}
                      服务的URL地址
                    </p>
                  </div>
                )}

                {/* STDIO 协议显示环境变量 */}
                {newServiceConfig.transport === 'stdio' && (
                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      🌍 环境变量 (每行一个 KEY=VALUE)
                    </label>
                    <textarea
                      value={newServiceConfig.env}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          env: e.target.value,
                        })
                      }
                      className='input-modern min-h-[80px]'
                      placeholder='例如:
PORT=8080
HOST=localhost'
                    />
                  </div>
                )}

                {/* SSE 和 StreamableHTTP 协议显示请求头 */}
                {(newServiceConfig.transport === 'sse' ||
                  newServiceConfig.transport === 'streamablehttp') && (
                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      📋 请求头 (每行一个 KEY:VALUE)
                    </label>
                    <textarea
                      value={newServiceConfig.headers}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          headers: e.target.value,
                        })
                      }
                      className='input-modern min-h-[80px]'
                      placeholder='例如:
Authorization: Bearer token123
Content-Type: application/json
X-Custom-Header: custom-value'
                    />
                  </div>
                )}

                <div className='flex justify-end space-x-3 pt-4 border-t'>
                  <button
                    onClick={() => setShowAddService(false)}
                    className='btn-modern bg-gray-300 hover:bg-gray-400 text-gray-700 text-sm px-3 py-1.5'>
                    取消
                  </button>
                  {addServiceMode === 'form' && (
                    <button
                      onClick={handleExportToJson}
                      className='btn-modern bg-purple-500 hover:bg-purple-600 text-white text-sm px-3 py-1.5'>
                      📤 导出JSON
                    </button>
                  )}
                  <button
                    onClick={handleAddService}
                    className='btn-modern btn-primary-modern text-sm px-3 py-1.5'
                    disabled={
                      !newServiceConfig.name ||
                      !newServiceConfig.transport ||
                      (newServiceConfig.transport === 'stdio' &&
                        !newServiceConfig.command) ||
                      ((newServiceConfig.transport === 'sse' ||
                        newServiceConfig.transport === 'streamablehttp') &&
                        !newServiceConfig.url)
                    }>
                    ➕ 添加服务
                  </button>
                </div>
              </div>
            )}

            {/* JSON Mode */}
            {addServiceMode === 'json' && (
              <div className='space-y-4'>
                {/* JSON Editor */}
                <div>
                  <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    📄 JSON配置
                  </label>
                  <textarea
                    value={jsonConfig}
                    onChange={(e) => {
                      setJsonConfig(e.target.value)
                      setJsonError('')
                    }}
                    className='input-modern font-mono text-xs min-h-[300px]'
                    placeholder='粘贴JSON配置或从示例开始...'
                  />
                  {jsonError && (
                    <p className='text-xs text-red-600 mt-1'>⚠️ {jsonError}</p>
                  )}
                </div>

                {/* JSON Example */}
                <details className='border border-gray-200 rounded-lg p-3'>
                  <summary className='cursor-pointer text-sm font-medium text-gray-700 hover:text-blue-600'>
                    💡 查看JSON配置示例
                  </summary>
                  <div className='mt-3 space-y-3'>
                    <div>
                      <p className='text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                        STDIO协议示例:
                      </p>
                      <pre className='bg-gray-100 dark:bg-gray-700 p-2 rounded text-xs overflow-x-auto'>
                        {`{
  "name": "weather-server",
  "description": "天气查询服务",
  "transport": "stdio",
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-weather"],
  "env": {
    "API_KEY": "your-api-key"
  }
}`}
                      </pre>
                    </div>
                    <div>
                      <p className='text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                        HTTP协议示例:
                      </p>
                      <pre className='bg-gray-100 dark:bg-gray-700 p-2 rounded text-xs overflow-x-auto'>
                        {`{
  "name": "context7",
  "description": "Context7文档服务",
  "transport": "streamablehttp",
  "url": "https://mcp.context7.com/mcp",
  "headers": {
    "Authorization": "Bearer token123",
    "Content-Type": "application/json"
  }
}`}
                      </pre>
                    </div>
                  </div>
                </details>

                {/* JSON Mode Buttons */}
                <div className='flex justify-end space-x-3 pt-4 border-t'>
                  <button
                    onClick={() => {
                      setShowAddService(false)
                      setJsonConfig('')
                      setJsonError('')
                    }}
                    className='btn-modern bg-gray-300 hover:bg-gray-400 text-gray-700 text-sm px-3 py-1.5'>
                    取消
                  </button>
                  <button
                    onClick={handleImportFromJson}
                    className='btn-modern btn-primary-modern text-sm px-3 py-1.5'
                    disabled={!jsonConfig.trim()}>
                    📥 导入配置
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Edit Service Modal */}
      {showEditService && editingService && (
        <div className='modal-modern'>
          <div className='modal-content-modern max-w-2xl max-h-[90vh] overflow-y-auto compact-modal'>
            <div className='flex justify-between items-start mb-4'>
              <div>
                <h3 className='text-lg font-bold text-gray-800 dark:text-gray-100 compact-title'>
                  ✏️ 编辑服务
                </h3>
                <p className='text-sm text-gray-600 dark:text-gray-300'>
                  修改 MCP 服务配置
                </p>
              </div>
              <button
                onClick={() => {
                  setShowEditService(false)
                  setEditingService(null)
                  setNewServiceConfig({
                    name: '',
                    description: '',
                    transport: 'stdio',
                    command: '',
                    args: '',
                    url: '',
                    env: '',
                    headers: '',
                  })
                }}
                className='btn-modern bg-gray-300 hover:bg-gray-400 text-gray-700 p-1.5'>
                ❌
              </button>
            </div>

            <div className='space-y-4'>
              <div>
                <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  📛 服务名称
                </label>
                <input
                  type='text'
                  value={newServiceConfig.name}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      name: e.target.value,
                    })
                  }
                  className='input-modern'
                  placeholder='输入服务名称'
                />
              </div>

              <div>
                <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  📝 服务描述
                </label>
                <textarea
                  value={newServiceConfig.description}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      description: e.target.value,
                    })
                  }
                  className='input-modern min-h-[60px]'
                  placeholder='输入服务描述（可选）'
                />
              </div>

              <div>
                <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  🔗 协议类型
                </label>
                <div className='flex flex-wrap gap-2'>
                  <label className='flex items-center p-1.5 border rounded-lg cursor-pointer hover:bg-gray-50 dark:bg-gray-700 transition-colors'>
                    <input
                      type='radio'
                      name='transport-edit'
                      value='stdio'
                      checked={newServiceConfig.transport === 'stdio'}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          transport: e.target.value as
                            | 'stdio'
                            | 'sse'
                            | 'streamablehttp',
                        })
                      }
                      className='h-3 w-3 text-blue-600 border-gray-300 focus:ring-blue-500'
                    />
                    <div className='ml-1.5'>
                      <div className='text-xs font-medium text-gray-900 dark:text-gray-100'>
                        🔌 STDIO
                      </div>
                    </div>
                  </label>

                  <label className='flex items-center p-1.5 border rounded-lg cursor-pointer hover:bg-gray-50 dark:bg-gray-700 transition-colors'>
                    <input
                      type='radio'
                      name='transport-edit'
                      value='sse'
                      checked={newServiceConfig.transport === 'sse'}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          transport: e.target.value as
                            | 'stdio'
                            | 'sse'
                            | 'streamablehttp',
                        })
                      }
                      className='h-3 w-3 text-blue-600 border-gray-300 focus:ring-blue-500'
                    />
                    <div className='ml-1.5'>
                      <div className='text-xs font-medium text-gray-900 dark:text-gray-100'>
                        🌊 SSE
                      </div>
                    </div>
                  </label>

                  <label className='flex items-center p-1.5 border rounded-lg cursor-pointer hover:bg-gray-50 dark:bg-gray-700 transition-colors'>
                    <input
                      type='radio'
                      name='transport-edit'
                      value='streamablehttp'
                      checked={newServiceConfig.transport === 'streamablehttp'}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          transport: e.target.value as
                            | 'stdio'
                            | 'sse'
                            | 'streamablehttp',
                        })
                      }
                      className='h-3 w-3 text-blue-600 border-gray-300 focus:ring-blue-500'
                    />
                    <div className='ml-1.5'>
                      <div className='text-xs font-medium text-gray-900 dark:text-gray-100'>
                        📡 StreamableHTTP
                      </div>
                    </div>
                  </label>
                </div>
              </div>

              {/* STDIO 协议配置 */}
              {newServiceConfig.transport === 'stdio' && (
                <>
                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      💻 执行命令
                    </label>
                    <input
                      type='text'
                      value={newServiceConfig.command}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          command: e.target.value,
                        })
                      }
                      className='input-modern'
                      placeholder='uvx or npx'
                    />
                  </div>

                  <div>
                    <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      ⚙️ 命令行参数
                    </label>
                    <input
                      type='text'
                      value={newServiceConfig.args}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          args: e.target.value,
                        })
                      }
                      className='input-modern'
                      placeholder='例如: --port 8080 --host localhost'
                    />
                  </div>
                </>
              )}

              {/* SSE 和 StreamableHTTP 协议配置 */}
              {(newServiceConfig.transport === 'sse' ||
                newServiceConfig.transport === 'streamablehttp') && (
                <div>
                  <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    🌐 服务URL
                  </label>
                  <input
                    type='url'
                    value={newServiceConfig.url}
                    onChange={(e) =>
                      setNewServiceConfig({
                        ...newServiceConfig,
                        url: e.target.value,
                      })
                    }
                    className='input-modern'
                    placeholder={`例如: http://localhost:8080${
                      newServiceConfig.transport === 'sse'
                        ? '/events'
                        : '/stream'
                    }`}
                  />
                  <p className='text-xs text-gray-500 dark:text-gray-400 mt-1'>
                    提供{' '}
                    {newServiceConfig.transport === 'sse'
                      ? 'Server-Sent Events'
                      : 'HTTP 流式'}{' '}
                    服务的URL地址
                  </p>
                </div>
              )}

              {/* STDIO 协议显示环境变量 */}
              {newServiceConfig.transport === 'stdio' && (
                <div>
                  <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    🌍 环境变量 (每行一个 KEY=VALUE)
                  </label>
                  <textarea
                    value={newServiceConfig.env}
                    onChange={(e) =>
                      setNewServiceConfig({
                        ...newServiceConfig,
                        env: e.target.value,
                      })
                    }
                    className='input-modern min-h-[80px]'
                    placeholder='例如:
PORT=8080
HOST=localhost'
                  />
                </div>
              )}

              {/* SSE 和 StreamableHTTP 协议显示请求头 */}
              {(newServiceConfig.transport === 'sse' ||
                newServiceConfig.transport === 'streamablehttp') && (
                <div>
                  <label className='block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    📋 请求头 (每行一个 KEY:VALUE)
                  </label>
                  <textarea
                    value={newServiceConfig.headers}
                    onChange={(e) =>
                      setNewServiceConfig({
                        ...newServiceConfig,
                        headers: e.target.value,
                      })
                    }
                    className='input-modern min-h-[80px]'
                    placeholder='例如:
Authorization: Bearer token123
Content-Type: application/json
X-Custom-Header: custom-value'
                  />
                </div>
              )}

              <div className='flex justify-end space-x-3 pt-4 border-t'>
                <button
                  onClick={() => {
                    setShowEditService(false)
                    setEditingService(null)
                    setNewServiceConfig({
                      name: '',
                      description: '',
                      transport: 'stdio',
                      command: '',
                      args: '',
                      url: '',
                      env: '',
                      headers: '',
                    })
                  }}
                  className='btn-modern bg-gray-300 hover:bg-gray-400 text-gray-700 text-sm px-3 py-1.5'>
                  取消
                </button>
                <button
                  onClick={handleEditService}
                  className='btn-modern btn-primary-modern text-sm px-3 py-1.5'
                  disabled={
                    !newServiceConfig.name ||
                    !newServiceConfig.transport ||
                    (newServiceConfig.transport === 'stdio' &&
                      !newServiceConfig.command) ||
                    ((newServiceConfig.transport === 'sse' ||
                      newServiceConfig.transport === 'streamablehttp') &&
                      !newServiceConfig.url)
                  }>
                  ✏️ 保存修改
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Tools Management Modal */}
      {showToolsModal && selectedServiceForTools && (
        <div className='modal-modern'>
          <div className='modal-content-modern max-w-4xl max-h-[90vh] overflow-y-auto compact-modal'>
            <div className='flex justify-between items-start mb-4'>
              <div>
                <h3 className='text-lg font-bold text-gray-800 dark:text-gray-100 flex items-center compact-title'>
                  <span className='mr-3'>🔧</span>
                  {selectedServiceForTools.name} - 工具管理
                </h3>
                <p className='text-sm text-gray-600 dark:text-gray-300'>
                  管理MCP服务的工具启用状态
                </p>
              </div>
              <button
                onClick={() => {
                  setShowToolsModal(false)
                }}
                className='btn-modern bg-gray-300 hover:bg-gray-400 text-gray-700 p-1.5'>
                ❌
              </button>
            </div>

            {/* Tools List */}
            <ToolManager mcpServer={selectedServiceForTools} />
          </div>
        </div>
      )}

      {/* Confirm Modal */}
      <ConfirmModal
        isOpen={showConfirmModal}
        title={confirmModalData?.title || ''}
        message={confirmModalData?.message || ''}
        confirmText='删除'
        cancelText='取消'
        type='danger'
        onConfirm={() => {
          if (confirmModalData?.onConfirm) {
            confirmModalData.onConfirm()
          }
        }}
        onCancel={() => {
          setShowConfirmModal(false)
          setConfirmModalData(null)
        }}
      />
    </div>
  )
}

export default McpServerManager
