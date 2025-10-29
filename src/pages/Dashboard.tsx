import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import {
  Badge,
  Button,
  Card,
  Col,
  Flex,
  Row,
  Space,
  Tabs,
  Typography,
} from 'antd'
import {
  Activity,
  Check,
  CheckCircle,
  Copy,
  Server,
  Wrench,
  XCircle,
} from 'lucide-react'
import { memo, useCallback, useEffect, useMemo, useState } from 'react'
import { useErrorContext } from '../contexts/ErrorContext'
import { DashboardService } from '../services/dashboard-service'
import type { DashboardStats } from '../types'

const { Text } = Typography

interface ClientType {
  id: string
  name: string
  icon: string
}

const Dashboard: React.FC = memo(() => {
  const { addError } = useErrorContext()
  const [dashboardStats, setDashboardStats] = useState<DashboardStats | null>(
    null,
  )
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const [selectedClient, setSelectedClient] = useState('claude-desktop')
  const [currentTime, setCurrentTime] = useState(Date.now())

  // Client types - 使用 useMemo 优化
  const clientTypes = useMemo<ClientType[]>(
    () => [
      { id: 'claude-desktop', name: 'Claude Desktop', icon: '🖥️' },
      { id: 'cherry-studio', name: 'CherryStudio', icon: '🍒' },
      { id: 'cursor', name: 'Cursor', icon: '👆' },
      { id: 'continue', name: 'Continue', icon: '▶️' },
      { id: 'windsurf', name: 'Windsurf', icon: '🌊' },
      { id: 'custom', name: '自定义配置', icon: '⚙️' },
    ],
    [],
  )

  // Fetch dashboard stats - 使用 useCallback 优化
  const fetchDashboardStats = useCallback(
    async (forceRefresh?: boolean) => {
      try {
        const result = await DashboardService.getDashboardStats(forceRefresh)
        setDashboardStats(result)
        setError(null)
      } catch (error) {
        console.error('Failed to fetch dashboard stats:', error)
        const errorMessage =
          '无法获取仪表盘数据，请在桌面应用中打开或检查后台服务'
        setError(errorMessage)
        addError(errorMessage)
      } finally {
        setLoading(false)
      }
    },
    [addError],
  )

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
          fetchDashboardStats(true) // Force refresh when services are loaded
        })
        unlistenFn = unlisten
      } catch (err) {
        console.warn('服务加载事件监听失败（非 Tauri 环境或 API 不可用）:', err)
      }
    })()

    return () => {
      clearInterval(interval)
      try {
        unlistenFn && unlistenFn()
      } catch {}
    }
  }, [fetchDashboardStats])

  // Calculate running duration - 使用 useMemo 优化
  const calculateRunningDuration = useCallback(
    (startupTime: string): string => {
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
    },
    [currentTime],
  )

  // Extract stats - 使用 useMemo 优化
  const stats = useMemo(() => {
    if (!dashboardStats) {
      return {
        total_servers: 0,
        enabled_servers: 0,
        disabled_servers: 0,
        connected_services: 0,
        total_tools: 0,
      }
    }

    return {
      total_servers: dashboardStats.total_servers,
      enabled_servers: dashboardStats.enabled_servers,
      disabled_servers: dashboardStats.disabled_servers,
      connected_services: dashboardStats.connected_services,
      total_tools: dashboardStats.total_tools,
    }
  }, [dashboardStats])

  // Generate client configuration - 使用 useMemo 优化
  const generateClientConfig = useCallback(() => {
    if (!dashboardStats?.aggregator?.endpoint) return '{}'
    const endpoint = dashboardStats.aggregator.endpoint

    const configs = {
      'claude-desktop': {
        mcpServers: {
          mcprouter: {
            type: 'http',
            url: endpoint,
            headers: {
              Authorization: 'Bearer <Your-API-Key>',
            },
            description:
              'MCP Router Aggregator - Unified access to all MCP services',
          },
        },
      },
      'cherry-studio': {
        mcpServers: {
          mcprouter: {
            type: 'streamableHttp',
            baseUrl: endpoint,
            isActive: true,
            headers: {
              Authorization: 'Bearer <Your-API-Key>',
            },
            description:
              'MCP Router Aggregator - Unified access to all MCP services',
          },
        },
      },
      cursor: {
        mcpServers: {
          mcprouter: {
            url: endpoint,
            headers: {
              Authorization: 'Bearer <Your-API-Key>',
            },
          },
        },
      },
      continue: {
        mcpServers: {
          mcprouter: {
            type: 'http',
            url: endpoint,
            headers: {
              Authorization: 'Bearer <Your-API-Key>',
            },
            description:
              'MCP Router Aggregator - Unified access to all MCP services',
          },
        },
      },
      windsurf: {
        mcpServers: {
          mcprouter: {
            type: 'http',
            url: endpoint,
            headers: {
              Authorization: 'Bearer <Your-API-Key>',
            },
            description:
              'MCP Router Aggregator - Unified access to all MCP services',
          },
        },
      },
      custom: {
        mcpServers: {
          'your-service-name': {
            type: 'http',
            url: endpoint,
            headers: {
              Authorization: 'Bearer <Your-API-Key>',
            },
            description:
              'MCP Router Aggregator - Unified access to all MCP services',
          },
        },
      },
    }

    return JSON.stringify(
      configs[selectedClient as keyof typeof configs] ||
        configs['claude-desktop'],
      null,
      2,
    )
  }, [dashboardStats?.aggregator?.endpoint, selectedClient])

  // Get config path - 使用 useMemo 优化
  const getConfigPath = useCallback(() => {
    const descriptions = {
      'claude-desktop': '将以下配置添加到 Claude Desktop 的配置文件中：',
      'cherry-studio': '将以下配置添加到 CherryStudio 的配置文件中：',
      cursor: '将以下配置添加到 Cursor 的配置文件中：',
      continue: '将以下配置添加到 Continue 的配置文件中：',
      windsurf: '将以下配置添加到 Windsurf 的配置文件中：',
      custom: '将以下配置添加到你的客户端配置文件中：',
    }

    return {
      description:
        descriptions[selectedClient as keyof typeof descriptions] ||
        descriptions['custom'],
      path: getClientConfigPath(selectedClient, dashboardStats),
    }
  }, [selectedClient, dashboardStats])

  // Copy configuration to clipboard - 使用 useCallback 优化
  const copyToClipboard = useCallback(async () => {
    try {
      const config = generateClientConfig()
      await writeText(config)

      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      console.error('Failed to copy to clipboard:', error)
      addError('复制配置失败')
    }
  }, [generateClientConfig, addError])

  if (loading) {
    return (
      <Flex justify='center' align='center' style={{ height: '256px' }}>
        <Button loading>加载中...</Button>
      </Flex>
    )
  }

  return (
    <Flex vertical gap='middle' style={{ height: '100%', overflowY: 'auto' }}>
      {/* Statistics Cards - Compact Layout */}
      <Row gutter={[8, 8]}>
        <Col span={4}>
          <StatsCard
            icon={<Server style={{ width: '16px', height: '16px' }} />}
            iconColor='#1890ff'
            bgColor='#e6f7ff'
            label='服务总数'
            value={stats?.total_servers || 0}
          />
        </Col>
        <Col span={4}>
          <StatsCard
            icon={<CheckCircle style={{ width: '16px', height: '16px' }} />}
            iconColor='#52c41a'
            bgColor='#f6ffed'
            label='已启用'
            value={stats?.enabled_servers || 0}
          />
        </Col>
        <Col span={4}>
          <StatsCard
            icon={<XCircle style={{ width: '16px', height: '16px' }} />}
            iconColor='#ff4d4f'
            bgColor='#fff2f0'
            label='已禁用'
            value={stats?.disabled_servers || 0}
          />
        </Col>
        <Col span={4}>
          <StatsCard
            icon={<Wrench style={{ width: '16px', height: '16px' }} />}
            iconColor='#fa8c16'
            bgColor='#fff7e6'
            label='工具总数'
            value={stats?.total_tools || 0}
          />
        </Col>
        <Col span={4}>
          <StatsCard
            icon={<Activity style={{ width: '16px', height: '16px' }} />}
            iconColor='#722ed1'
            bgColor='#f9f0ff'
            label='已连接'
            value={stats?.connected_services || 0}
          />
        </Col>
      </Row>

      {/* Error Notice */}
      {error && (
        <Card style={{ borderColor: '#ff7875', backgroundColor: '#fff2f0' }}>
          <Text type='danger' style={{ fontSize: '14px' }}>
            {error}
          </Text>
        </Card>
      )}

      {/* Client Configuration */}
      <ClientConfigurationCard
        clientTypes={clientTypes}
        selectedClient={selectedClient}
        onClientChange={setSelectedClient}
        configPath={getConfigPath()}
        configContent={generateClientConfig()}
        onCopyConfig={copyToClipboard}
        copied={copied}
      />

      {/* System Information */}
      <Row gutter={16}>
        <Col span={12}>
          <SystemInfoCard
            stats={dashboardStats}
            calculateRunningDuration={calculateRunningDuration}
          />
        </Col>
        <Col span={12}>
          <AggregatorInfoCard stats={dashboardStats} />
        </Col>
      </Row>
    </Flex>
  )
})

// 子组件：统计卡片
interface StatsCardProps {
  icon: React.ReactNode
  iconColor: string
  bgColor: string
  label: string
  value: number
}

const StatsCard: React.FC<StatsCardProps> = memo(
  ({ icon, iconColor, bgColor, label, value }) => (
    <Card size='small' style={{ height: '64px' }}>
      <Flex align='center' gap='small' style={{ height: '100%' }}>
        <div
          style={{
            padding: '8px',
            backgroundColor: bgColor,
            borderRadius: '6px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}>
          <div style={{ color: iconColor }}>{icon}</div>
        </div>
        <div style={{ minWidth: 0, flex: 1 }}>
          <Text
            type='secondary'
            style={{ fontSize: '12px', display: 'block', lineHeight: 1.2 }}>
            {label}
          </Text>
          <Text
            strong
            style={{ fontSize: '16px', display: 'block', lineHeight: 1.2 }}>
            {value}
          </Text>
        </div>
      </Flex>
    </Card>
  ),
)

// 子组件：客户端配置卡片
interface ClientConfigurationCardProps {
  clientTypes: ClientType[]
  selectedClient: string
  onClientChange: (client: string) => void
  configPath: { description: string; path: string }
  configContent: string
  onCopyConfig: () => void
  copied: boolean
}

const ClientConfigurationCard: React.FC<ClientConfigurationCardProps> = memo(
  ({
    clientTypes,
    selectedClient,
    onClientChange,
    configPath,
    configContent,
    onCopyConfig,
    copied,
  }) => (
    <Card
      title='客户端配置'
      extra={
        <Button
          onClick={onCopyConfig}
          icon={copied ? <Check size={14} /> : <Copy size={14} />}
          type='primary'
          size='middle'>
          {copied ? '已复制' : '复制配置'}
        </Button>
      }>
      <Flex vertical gap='middle'>
        {/* Client Tabs */}
        <Tabs
          activeKey={selectedClient}
          onChange={onClientChange}
          items={clientTypes.map((client) => ({
            key: client.id,
            label: (
              <Space>
                <span>{client.icon}</span>
                {client.name}
              </Space>
            ),
            children: null,
          }))}
          style={{ marginBottom: '16px' }}
        />

        {/* Configuration Path */}
        <div
          className=' '
          style={{
            marginBottom: '16px',
            padding: '12px',
            backgroundColor: '#e6f7ff',
            borderRadius: '6px',
            border: '1px solid #91d5ff',
          }}>
          <Text
            className='text-primary-600 '
            style={{ fontSize: '12px', display: 'block', fontWeight: 500 }}>
            {configPath.description}
          </Text>
          <Text
            code
            className='text-primary-600 '
            style={{ fontSize: '12px', display: 'block', marginTop: '4px' }}>
            {configPath.path}
          </Text>
        </div>

        {/* Configuration Content */}
        <div
          style={{
            backgroundColor: 'var(--ant-color-bg-container)',
            borderRadius: '8px',
            padding: '16px',
            overflowX: 'auto',
            border: '1px solid var(--ant-color-border)',
            boxShadow:
              '0 1px 2px 0 rgba(0, 0, 0, 0.03), 0 1px 6px -1px rgba(0, 0, 0, 0.02), 0 2px 4px 0 rgba(0, 0, 0, 0.02)',
          }}>
          <pre
            style={{
              fontSize: '12px',
              color: 'var(--ant-color-text)',
              fontFamily: 'monospace',
              whiteSpace: 'pre-wrap',
              margin: 0,
              border: '2px solid var(--ant-color-border)',
              borderRadius: '6px',
              padding: '12px',
              backgroundColor: 'var(--ant-color-bg-elevated)',
              boxShadow: 'inset 0 1px 3px rgba(0, 0, 0, 0.1)',
            }}>
            {configContent}
          </pre>
        </div>
      </Flex>
    </Card>
  ),
)

// 子组件：系统信息卡片
interface SystemInfoCardProps {
  stats: DashboardStats | null
  calculateRunningDuration: (time: string) => string
}

const SystemInfoCard: React.FC<SystemInfoCardProps> = memo(
  ({ stats, calculateRunningDuration }) => (
    <Card title='系统信息'>
      <Flex vertical gap='small'>
        <InfoRow label='MCP Router 版本' value='0.1.0' />
        <InfoRow
          label='运行时间'
          value={
            stats?.startup_time
              ? calculateRunningDuration(stats.startup_time)
              : 'Unknown'
          }
        />
        <InfoRow
          label='操作系统'
          value={
            stats?.os_info
              ? `${stats.os_info.type} ${stats.os_info.version}`
              : 'Unknown'
          }
        />
        <InfoRow label='架构' value={stats?.os_info?.arch || 'Unknown'} />
      </Flex>
    </Card>
  ),
)

// 子组件：聚合接口信息卡片
interface AggregatorInfoCardProps {
  stats: DashboardStats | null
}

const AggregatorInfoCard: React.FC<AggregatorInfoCardProps> = memo(
  ({ stats }) => (
    <Card title='MCP 聚合接口'>
      <Flex vertical gap='small'>
        <InfoRow
          label='接口地址'
          value={stats?.aggregator?.endpoint ?? 'Unknown'}
        />
        <InfoRow
          label='连接数'
          value={stats?.connections?.active_clients || 0}
        />
        <InfoRow
          label='已连接服务'
          value={stats?.connections?.active_services || 0}
        />
        <InfoRow
          label='最大连接数'
          value={stats?.aggregator?.max_connections || 0}
        />
        <Flex justify='space-between' align='center'>
          <Text type='secondary' style={{ fontSize: '14px' }}>
            运行状态
          </Text>
          <Badge
            color={stats?.aggregator?.is_running ? 'green' : 'red'}
            text={stats?.aggregator?.is_running ? '运行中' : '已停止'}
          />
        </Flex>
      </Flex>
    </Card>
  ),
)

// 子组件：信息行
interface InfoRowProps {
  label: string
  value: string | number
}

const InfoRow: React.FC<InfoRowProps> = memo(({ label, value }) => (
  <Flex justify='space-between' align='center'>
    <Text type='secondary' style={{ fontSize: '14px' }}>
      {label}
    </Text>
    <Text strong style={{ fontSize: '14px' }}>
      {value}
    </Text>
  </Flex>
))

// 辅助函数
function getClientConfigPath(
  client: string,
  stats: DashboardStats | null,
): string {
  const platform =
    stats?.os_info?.platform?.toLowerCase() ||
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
    case 'claude-desktop':
      return paths.claude
    case 'cherry-studio':
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

export default Dashboard
