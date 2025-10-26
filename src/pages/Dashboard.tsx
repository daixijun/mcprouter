import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import {
  Activity,
  Check,
  CheckCircle,
  Copy,
  Server,
  Wrench,
  XCircle,
} from 'lucide-react'
import { useEffect, useState } from 'react'
import { ApiService } from '../services/api'

const Dashboard: React.FC = () => {
  const [dashboardStats, setDashboardStats] = useState<any>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const [selectedClient, setSelectedClient] = useState('claude-desktop')
  const [currentTime, setCurrentTime] = useState(Date.now())

  // Client types and their display names
  const clientTypes = [
    { id: 'claude-desktop', name: 'Claude Desktop', icon: '🖥️' },
    { id: 'cherry-studio', name: 'CherryStudio', icon: '🍒' },
    { id: 'cursor', name: 'Cursor', icon: '👆' },
    { id: 'continue', name: 'Continue', icon: '▶️' },
    { id: 'windsurf', name: 'Windsurf', icon: '🌊' },
    { id: 'custom', name: '自定义配置', icon: '⚙️' },
  ]

  // Fetch dashboard stats
  const fetchDashboardStats = async () => {
    try {
      const result = await ApiService.getDashboardStats()
      setDashboardStats(result)
    } catch (error) {
      console.error('Failed to fetch dashboard stats:', error)
      setError('无法获取仪表盘数据，请在桌面应用中打开或检查后台服务')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchDashboardStats()

    // Update current time every second to refresh running duration
    const interval = setInterval(() => {
      setCurrentTime(Date.now())
    }, 1000)

    // Listen for backend services-loaded event to refresh dashboard stats
    let unlistenFn: (() => void) | undefined
    ;(async () => {
      try {
        const unlisten = await listen('services-loaded', () => {
          fetchDashboardStats()
        })
        unlistenFn = unlisten
      } catch (err) {
        // If running in plain web preview without Tauri, this may fail
        console.warn('服务加载事件监听失败（非 Tauri 环境或 API 不可用）:', err)
      }
    })()

    // 移除轮询：不再进行定时刷新，依赖事件与首次加载
    return () => {
      clearInterval(interval)
      try {
        unlistenFn && unlistenFn()
      } catch {}
    }
  }, [])

  // Calculate running duration from startup time
  const calculateRunningDuration = (startupTime: string): string => {
    if (!startupTime) return 'Unknown'

    const startup = new Date(startupTime).getTime()
    const diffInSeconds = Math.floor((currentTime - startup) / 1000)

    if (diffInSeconds < 60) {
      return `${diffInSeconds}秒`
    } else if (diffInSeconds < 3600) {
      const minutes = Math.floor(diffInSeconds / 60)
      const remainingSeconds = diffInSeconds % 60
      return `${minutes}分${remainingSeconds}秒`
    } else if (diffInSeconds < 86400) {
      const hours = Math.floor(diffInSeconds / 3600)
      const remainingMinutes = Math.floor((diffInSeconds % 3600) / 60)
      return `${hours}小时${remainingMinutes}分`
    } else {
      const days = Math.floor(diffInSeconds / 86400)
      const remainingHours = Math.floor((diffInSeconds % 86400) / 3600)
      return `${days}天${remainingHours}小时`
    }
  }

  // Extract stats from dashboard data
  const totalServers = dashboardStats?.services?.total || 0
  const enabledServers = dashboardStats?.services?.enabled || 0
  const disabledServers = dashboardStats?.services?.disabled || 0
  const connectedServers = dashboardStats?.connections?.active_services || 0
  const totalTools = dashboardStats?.tools?.total_count || 0

  // Generate client configuration based on selected client type
  const generateClientConfig = () => {
    if (!dashboardStats?.aggregator?.endpoint) return '{}'
    const endpoint = dashboardStats.aggregator.endpoint

    switch (selectedClient) {
      case 'claude-desktop':
        return JSON.stringify(
          {
            mcpServers: {
              mcprouter: {
                type: 'http',
                url: endpoint,
                description:
                  'MCP Router Aggregator - Unified access to all MCP services',
              },
            },
          },
          null,
          2,
        )

      case 'cherry-studio':
        return JSON.stringify(
          {
            mcpServers: {
              mcprouter: {
                type: 'streamableHttp',
                baseUrl: endpoint,
                isActive: true,
                description:
                  'MCP Router Aggregator - Unified access to all MCP services',
              },
            },
          },
          null,
          2,
        )

      case 'cursor':
        return JSON.stringify(
          {
            mcpServers: {
              mcprouter: {
                url: endpoint,
              },
            },
          },
          null,
          2,
        )

      case 'continue':
        return JSON.stringify(
          {
            mcpServers: {
              mcprouter: {
                type: 'http',
                url: endpoint,
                description:
                  'MCP Router Aggregator - Unified access to all MCP services',
              },
            },
          },
          null,
          2,
        )

      case 'windsurf':
        return JSON.stringify(
          {
            mcpServers: {
              mcprouter: {
                type: 'http',
                url: endpoint,
                description:
                  'MCP Router Aggregator - Unified access to all MCP services',
              },
            },
          },
          null,
          2,
        )

      case 'custom':
        return JSON.stringify(
          {
            mcpServers: {
              'your-service-name': {
                type: 'http',
                url: endpoint,
                description:
                  'MCP Router Aggregator - Unified access to all MCP services',
              },
            },
          },
          null,
          2,
        )
      default:
        return JSON.stringify(
          {
            mcpServers: {
              mcprouter: {
                type: 'http',
                url: endpoint,
                description:
                  'MCP Router Aggregator - Unified access to all MCP services',
              },
            },
          },
          null,
          2,
        )
    }
  }

  // Map selected client to config path
  const getConfigPath = () => {
    switch (selectedClient) {
      case 'claude-desktop':
        return {
          description: '将以下配置添加到 Claude Desktop 的配置文件中：',
          path: getClientConfigPath('claude'),
        }
      case 'cherry-studio':
        return {
          description: '将以下配置添加到 CherryStudio 的配置文件中：',
          path: getClientConfigPath('cherry'),
        }
      case 'cursor':
        return {
          description: '将以下配置添加到 Cursor 的配置文件中：',
          path: getClientConfigPath('cursor'),
        }
      case 'continue':
        return {
          description: '将以下配置添加到 Continue 的配置文件中：',
          path: getClientConfigPath('continue'),
        }
      case 'windsurf':
        return {
          description: '将以下配置添加到 Windsurf 的配置文件中：',
          path: getClientConfigPath('windsurf'),
        }
      case 'custom':
        return {
          description: '将以下配置添加到你的客户端配置文件中：',
          path: getClientConfigPath('custom'),
        }
      default:
        return {
          description: '将以下配置添加到你的客户端配置文件中：',
          path: getClientConfigPath('custom'),
        }
    }
  }

  function getClientConfigPath(client: string): string {
    const platform =
      dashboardStats?.os_info?.platform?.toLowerCase() ||
      (typeof navigator !== 'undefined'
        ? navigator.userAgent.toLowerCase()
        : 'linux')
    const isWin = platform.includes('win')
    const isMac = platform.includes('mac') || platform.includes('darwin')

    const paths = {
      claude: isWin
        ? '%APPDATA%/Claude/claude_desktop_config.json'
        : isMac
        ? '~/Library/Application Support/Claude/claude_desktop_config.json'
        : '~/.config/claude/claude_desktop_config.json',
      cherry: isWin
        ? '%APPDATA%/CherryStudio/cherry_studio_config.json'
        : isMac
        ? '~/Library/Application Support/CherryStudio/cherry_studio_config.json'
        : '~/.config/cherrystudio/cherry_studio_config.json',
      cursor: isWin
        ? '%APPDATA%/Cursor/cursor_config.json'
        : isMac
        ? '~/Library/Application Support/Cursor/cursor_config.json'
        : '~/.config/cursor/cursor_config.json',
      continue: isWin
        ? '%APPDATA%/Continue/continue_config.json'
        : isMac
        ? '~/Library/Application Support/Continue/continue_config.json'
        : '~/.config/continue/continue_config.json',
      windsurf: isWin
        ? '%APPDATA%/Windsurf/windsurf_config.json'
        : isMac
        ? '~/Library/Application Support/Windsurf/windsurf_config.json'
        : '~/.config/windsurf/windsurf_config.json',
      custom: isWin
        ? '%APPDATA%/YourClient/config.json'
        : isMac
        ? '~/Library/Application Support/YourClient/config.json'
        : '~/.config/yourclient/config.json',
    }

    switch (client) {
      case 'claude':
        return paths.claude
      case 'cherry':
        return paths.cherry
      case 'cursor':
        return paths.cursor
      case 'continue':
        return paths.continue
      case 'windsurf':
        return paths.windsurf
      case 'custom':
        return paths.custom
      default:
        return paths.claude
    }
  }

  // Copy configuration to clipboard
  const copyToClipboard = async () => {
    try {
      const config = generateClientConfig()
      // Use Tauri clipboard API
      await writeText(config)

      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      console.error('Failed to copy to clipboard:', error)
    }
  }

  if (loading) {
    return (
      <div className='flex items-center justify-center h-64'>
        <div className='text-center'>
          <div className='animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-3'></div>
          <p className='text-sm text-gray-600 dark:text-gray-300'>加载中...</p>
        </div>
      </div>
    )
  }

  return (
    <div className='h-full overflow-y-auto scrollbar-custom'>
      <div className='space-y-4'>
        {/* Statistics Cards */}
        <div className='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-3'>
          <div className='card-glass'>
            <div className='flex items-center'>
              <div className='p-2 bg-blue-100 rounded-lg'>
                <Server className='h-5 w-5 text-blue-600' />
              </div>
              <div className='ml-3'>
                <p className='text-xs font-medium text-gray-500 dark:text-gray-400'>
                  服务总数
                </p>
                <p className='text-xl font-semibold text-gray-900 dark:text-gray-100'>
                  {totalServers}
                </p>
              </div>
            </div>
          </div>

          <div className='card-glass'>
            <div className='flex items-center'>
              <div className='p-2 bg-green-100 rounded-lg'>
                <CheckCircle className='h-5 w-5 text-green-600' />
              </div>
              <div className='ml-3'>
                <p className='text-xs font-medium text-gray-500 dark:text-gray-400'>
                  已启用
                </p>
                <p className='text-xl font-semibold text-gray-900 dark:text-gray-100'>
                  {enabledServers}
                </p>
              </div>
            </div>
          </div>

          <div className='card-glass'>
            <div className='flex items-center'>
              <div className='p-2 bg-red-100 rounded-lg'>
                <XCircle className='h-5 w-5 text-red-600' />
              </div>
              <div className='ml-3'>
                <p className='text-xs font-medium text-gray-500 dark:text-gray-400'>
                  已禁用
                </p>
                <p className='text-xl font-semibold text-gray-900 dark:text-gray-100'>
                  {disabledServers}
                </p>
              </div>
            </div>
          </div>

          <div className='card-glass'>
            <div className='flex items-center'>
              <div className='p-2 bg-orange-100 rounded-lg'>
                <Wrench className='h-5 w-5 text-orange-600' />
              </div>
              <div className='ml-3'>
                <p className='text-xs font-medium text-gray-500 dark:text-gray-400'>
                  工具总数
                </p>
                <p className='text-xl font-semibold text-gray-900 dark:text-gray-100'>
                  {totalTools}
                </p>
              </div>
            </div>
          </div>

          <div className='card-glass'>
            <div className='flex items-center'>
              <div className='p-2 bg-purple-100 rounded-lg'>
                <Activity className='h-5 w-5 text-purple-600' />
              </div>
              <div className='ml-3'>
                <p className='text-xs font-medium text-gray-500 dark:text-gray-400'>
                  已连接
                </p>
                <p className='text-xl font-semibold text-gray-900 dark:text-gray-100'>
                  {connectedServers}
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* Error Notice */}
        {error && (
          <div className='card-glass bg-red-50 border border-red-200'>
            <p className='text-sm text-red-700 dark:text-red-400'>{error}</p>
          </div>
        )}

        {/* Client Configuration */}
        <div className='card-glass'>
          <div className='flex items-center justify-between mb-1'>
            <h2 className='text-lg font-semibold text-gray-900 dark:text-gray-100'>
              客户端配置
            </h2>
            <button
              onClick={copyToClipboard}
              className='btn-modern btn-primary-modern flex items-center space-x-1'>
              {copied ? (
                <>
                  <Check size={14} />
                  <span>已复制</span>
                </>
              ) : (
                <>
                  <Copy size={14} />
                  <span>复制配置</span>
                </>
              )}
            </button>
          </div>

          {/* Client Tabs */}
          <div className='border-b border-gray-200 mb-1'>
            <nav className='-mb-px flex space-x-8'>
              {clientTypes.map((client) => (
                <button
                  key={client.id}
                  onClick={() => setSelectedClient(client.id)}
                  className={`py-2 px-1 border-b-2 font-medium text-sm ${
                    selectedClient === client.id
                      ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                      : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:border-gray-300'
                  }`}>
                  <span className='mr-1'>{client.icon}</span>
                  {client.name}
                </button>
              ))}
            </nav>
          </div>

          {/* Configuration Path */}
          <div className='mb-3 p-2 bg-blue-50 dark:bg-blue-900/20 rounded-md'>
            <div className='text-xs font-medium text-blue-800 dark:text-blue-300'>
              {getConfigPath().description}
            </div>
            <div className='text-xs text-blue-600 dark:text-blue-400 mt-1 font-mono'>
              {getConfigPath().path}
            </div>
          </div>

          {/* Configuration Content */}
          <div className='bg-gray-100 dark:bg-gray-900 rounded-lg p-3 overflow-x-auto border border-gray-200 dark:border-gray-700'>
            <pre className='text-xs text-gray-800 dark:text-gray-200 font-mono whitespace-pre-wrap'>
              {generateClientConfig()}
            </pre>
          </div>
        </div>

        {/* System Information */}
        <div className='grid grid-cols-1 lg:grid-cols-2 gap-3 compact-grid'>
          <div className='card-glass compact-card'>
            <h2 className='text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3 compact-title'>
              系统信息
            </h2>
            <div className='space-y-2 compact-list'>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-300'>
                  MCP Router 版本
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  0.1.0
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-300'>
                  运行时间
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  {dashboardStats?.startup_time
                    ? calculateRunningDuration(dashboardStats.startup_time)
                    : 'Unknown'}
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-300'>
                  操作系统
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  {dashboardStats?.os_info?.platform &&
                  dashboardStats?.os_info?.version
                    ? `${dashboardStats.os_info.platform} ${dashboardStats.os_info.version}`
                    : 'Unknown'}
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-300'>
                  架构
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  {dashboardStats?.os_info?.arch || 'Unknown'}
                </span>
              </div>
            </div>
          </div>

          <div className='card-glass compact-card'>
            <h2 className='text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3 compact-title'>
              MCP 聚合接口
            </h2>
            <div className='space-y-2 compact-list'>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-300'>
                  接口地址
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  {dashboardStats?.aggregator?.endpoint ?? 'Unknown'}
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-300'>
                  连接数
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  {dashboardStats?.connections?.active_clients || 0}
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-300'>
                  已连接服务
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  {dashboardStats?.connections?.active_services || 0}
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-300'>
                  最大连接数
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  {dashboardStats?.aggregator?.max_connections || 0}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default Dashboard
