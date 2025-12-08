import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import {
  App,
  Button,
  Select,
  Space,
  Statistic,
  Tooltip,
  Typography,
} from 'antd'
import Card from 'antd/es/card'
import {
  Activity,
  Database,
  Key,
  MessageSquare,
  Server,
  Wrench,
} from 'lucide-react'
import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useAppContext } from '../contexts/AppContext'
import type { DashboardStats, Token } from '../types'

const { Text } = Typography

const Dashboard: React.FC = () => {
  const { t } = useTranslation()
  const { state } = useAppContext()
  const [stats, setStats] = useState<DashboardStats>({
    total_servers: 0,
    enabled_servers: 0,
    failed_servers: 0,
    healthy_services: 0,
    total_tools: 0,
    total_resources: 0,
    total_prompts: 0,
    total_prompt_templates: 0,
    active_clients: 0,
    startup_time: '',
  })
  const { message } = App.useApp()
  const [isLoading, setIsLoading] = useState(true)
  const [currentUptime, setCurrentUptime] = useState<string>('-')

  // Token 状态管理
  const [tokens, setTokens] = useState<Token[]>([])
  const [selectedTokenId, setSelectedTokenId] = useState<string | undefined>()
  const [isTokensLoading, setIsTokensLoading] = useState(false)
  const [tokenDropdownOpen, setTokenDropdownOpen] = useState(false)

  // 加载仪表板数据
  const loadDashboardData = async () => {
    try {
      setIsLoading(true)

      // 并行加载所有数据
      const [statsResult] = await Promise.allSettled([
        invoke<DashboardStats>('get_dashboard_stats'),
      ])

      // 处理统计数据
      if (statsResult.status === 'fulfilled') {
        setStats(statsResult.value)
      }
    } catch (error) {
      console.error('Failed to load dashboard data:', error)
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    loadDashboardData()
    loadTokens()
  }, [])

  // 加载 Token 列表
  const loadTokens = async () => {
    try {
      setIsTokensLoading(true)
      const tokenList = await invoke<Token[]>('get_tokens_for_dashboard')
      setTokens(tokenList)

      // 如果有可用的 token，选择第一个有效的
      const availableTokens = tokenList.filter(
        (token) => token.enabled && !token.is_expired,
      )
      if (availableTokens.length > 0 && !selectedTokenId) {
        setSelectedTokenId(availableTokens[0].id)
      }
    } catch (error) {
      console.error('Failed to load tokens:', error)
      message.error(t('dashboard.token.load_failed'))
    } finally {
      setIsTokensLoading(false)
    }
  }

  // Token 选择处理
  const handleTokenChange = (tokenId: string | undefined) => {
    setSelectedTokenId(tokenId)
    setTokenDropdownOpen(false) // 选择后立即关闭下拉菜单
  }

  // 获取选中的 Token
  const getSelectedToken = () => {
    return tokens.find((token) => token.id === selectedTokenId)
  }

  // 格式化运行时间
  const formatUptime = (seconds: number) => {
    if (seconds < 60) return t('dashboard.uptime.seconds', { count: seconds })
    if (seconds < 3600) {
      const minutes = Math.floor(seconds / 60)
      return t('dashboard.uptime.minutes', { count: minutes })
    }
    if (seconds < 86400) {
      const hours = Math.floor(seconds / 3600)
      return t('dashboard.uptime.hours', { count: hours })
    }
    const days = Math.floor(seconds / 86400)
    const hours = Math.floor((seconds % 86400) / 3600)
    return t('dashboard.uptime.days_hours', { days, hours })
  }

  // 格式化数字
  const formatNumber = (num: number) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`
    return num.toString()
  }

  const selectedToken = getSelectedToken()

  // Token 状态辅助函数
  const formatTokenExpiration = (token: Token) => {
    if (!token.expires_at) return t('dashboard.token.status.never_expires')
    const now = Date.now()
    const expiry = token.expires_at * 1000 // Convert from seconds to milliseconds
    const diff = expiry - now

    if (diff <= 0) return t('dashboard.token.status.expired')
    const days = Math.floor(diff / (1000 * 60 * 60 * 24))
    const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60))

    if (days > 0) return t('dashboard.token.status.expires_days', { days })
    if (hours > 0) return t('dashboard.token.status.expires_hours', { hours })
    return t('dashboard.token.status.expires_soon')
  }

  const getTokenStatusColor = (token: Token) => {
    if (!token.enabled) return '#8c8c8c' // gray
    if (token.is_expired) return '#ff4d4f' // red
    const now = Date.now()
    const expiry = token.expires_at ? token.expires_at * 1000 : 0
    const diff = expiry - now
    if (diff > 0 && diff < 24 * 60 * 60 * 1000) return '#faad14' // orange (less than 24h)
    return '#52c41a' // green
  }

  const generatedConfig = {
    mcpServers: {
      mcprouter: {
        type: 'http',
        url: stats.aggregator?.endpoint || '',
        ...(selectedToken && {
          headers: {
            Authorization: `Bearer ${selectedToken.value}`,
          },
        }),
      },
    },
  }

  const generateClientInstallConfig = (
    client:
      | 'cherrystudio'
      | 'claude'
      | 'claudecode'
      | 'cursor'
      | 'windsurf'
      | 'vscode',
  ) => {
    const endpoint = stats.aggregator?.endpoint || ''
    const authHeaders = selectedToken
      ? { Authorization: `Bearer ${selectedToken.value}` }
      : {}

    switch (client) {
      case 'cherrystudio':
        const cherrystudioConfig = {
          mcpServers: {
            mcprouter: {
              type: 'streamableHttp',
              baseUrl: endpoint,
              ...(selectedToken && { headers: authHeaders }),
            },
          },
        }
        return `cherrystudio://mcp/install?servers=${btoa(
          JSON.stringify(cherrystudioConfig),
        )}`
      case 'claude':
      case 'claudecode':
      case 'cursor':
        const cursorConfig = {
          name: 'mcprouter',
          type: 'streamable_http',
          url: endpoint,
          ...(selectedToken && { headers: authHeaders }),
        }
        return `cursor://anysphere.cursor-deeplink/mcp/install?name=mcprouter&config=${btoa(
          JSON.stringify(cursorConfig),
        )}`
      case 'windsurf':
      case 'vscode':
        const vscodeConfig = {
          name: 'mcprouter',
          url: endpoint,
          type: 'http',
          ...(selectedToken && { headers: authHeaders }),
        }
        return `vscode:mcp/install?${encodeURIComponent(
          JSON.stringify(vscodeConfig),
        )}`
      default:
        return ''
    }
  }

  const copyMcpServersJson = async () => {
    const json = JSON.stringify(generatedConfig, null, 2)
    try {
      await navigator.clipboard.writeText(json)
      message.success(t('dashboard.client_config.copy_config'))
    } catch (e) {
      message.error(t('common.messages.copy_failed'))
    }
  }

  // 实时计算 uptime
  useEffect(() => {
    const updateUptime = () => {
      if (stats.startup_time) {
        const startTime = new Date(stats.startup_time)
        const now = new Date()
        const uptimeSeconds = Math.floor(
          (now.getTime() - startTime.getTime()) / 1000,
        )
        setCurrentUptime(formatUptime(uptimeSeconds))
      }
    }

    // 初始更新
    updateUptime()

    // 每秒更新一次
    const interval = setInterval(updateUptime, 1000)

    return () => clearInterval(interval)
  }, [stats.startup_time])

  return (
    <div className='space-y-6 h-full overflow-y-auto pb-6'>
      {/* 统计卡片 */}
      <div className='grid grid-cols-6 gap-3'>
        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.total_servers')}
            value={stats.total_servers}
            prefix={<Server size={14} />}
            styles={{ content: { color: '#1890ff', fontSize: '18px' } }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.enabled_servers')}
            value={stats.enabled_servers}
            prefix={<Activity size={14} />}
            styles={{ content: { color: '#52c41a', fontSize: '18px' } }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.total_tools')}
            value={formatNumber(stats.total_tools)}
            prefix={<Wrench size={14} />}
            styles={{ content: { color: '#722ed1', fontSize: '18px' } }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.total_prompts')}
            value={formatNumber(stats.total_prompts)}
            prefix={<MessageSquare size={14} />}
            styles={{ content: { color: '#722ed1', fontSize: '18px' } }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.total_prompt_templates')}
            value={formatNumber(stats.total_prompt_templates)}
            prefix={<MessageSquare size={14} />}
            styles={{ content: { color: '#52c41a', fontSize: '18px' } }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.total_resources')}
            value={formatNumber(stats.total_resources)}
            prefix={<Database size={14} />}
            styles={{ content: { color: '#fa8c16', fontSize: '18px' } }}
            className='text-xs'
          />
        </Card>
      </div>

      {/* 聚合接口配置 */}
      <Card
        title={
          <Space>
            <Activity size={16} />
            {t('dashboard.client_config.title')}
          </Space>
        }
        loading={isLoading}>
        <div className='w-full space-y-3'>
          {/* Token 选择区域 */}
          <div
            className='flex items-center justify-between p-3 rounded border'
            style={{
              backgroundColor: state.isDarkMode ? '#1f2937' : '#f9fafb',
              borderColor: state.isDarkMode ? '#374151' : '#e5e7eb',
            }}>
            <div className='flex items-center space-x-3 flex-1'>
              <Key
                size={16}
                className={state.isDarkMode ? 'text-blue-400' : 'text-blue-600'}
              />
              <Text className='font-medium'>{t('dashboard.token.title')}</Text>
              <Select
                value={selectedTokenId}
                onChange={handleTokenChange}
                open={tokenDropdownOpen}
                onOpenChange={setTokenDropdownOpen}
                placeholder={
                  tokens.length === 0
                    ? t('dashboard.token.no_available')
                    : t('dashboard.token.select')
                }
                loading={isTokensLoading}
                disabled={tokens.length === 0}
                style={{ width: 200 }}
                className='flex-1 max-w-xs'>
                <Select.Option value={undefined} key='no-token'>
                  <Space>
                    <span style={{ color: '#8c8c8c' }}>
                      {t('dashboard.token.none')}
                    </span>
                  </Space>
                </Select.Option>
                {tokens.map((token) => (
                  <Select.Option
                    value={token.id}
                    key={token.id}
                    disabled={!token.enabled || token.is_expired}>
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        width: '100%',
                        minWidth: 0, // 防止 flex 子项溢出
                        gap: '8px', // 确保间隔
                      }}>
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          flex: 1,
                          minWidth: 0,
                          overflow: 'hidden',
                        }}>
                        <div
                          style={{
                            width: 8,
                            height: 8,
                            borderRadius: '50%',
                            backgroundColor: getTokenStatusColor(token),
                            flexShrink: 0, // 防止被压缩
                            marginRight: 8,
                          }}
                        />
                        <span
                          style={{
                            fontWeight: 500,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                            flex: 1,
                          }}>
                          {token.name}
                        </span>
                      </div>
                      <span
                        style={{
                          fontSize: 12,
                          color: '#8c8c8c',
                          flexShrink: 0, // 防止被压缩
                          marginLeft: 8,
                        }}>
                        {formatTokenExpiration(token)}
                      </span>
                    </div>
                  </Select.Option>
                ))}
              </Select>
              {selectedToken && (
                <Tooltip
                  title={t('dashboard.token.selected_tooltip', {
                    name: getSelectedToken()?.name,
                  })}>
                  <div className='ml-3 text-sm text-green-600 dark:text-green-400'>
                    ✓ {t('dashboard.token.configured')}
                  </div>
                </Tooltip>
              )}
            </div>
            {tokens.length === 0 && (
              <Button
                type='primary'
                size='small'
                onClick={() => {
                  // 可以在这里添加跳转到 Token 管理页面的逻辑
                  message.info(t('dashboard.token.create_hint'))
                }}>
                创建 Token
              </Button>
            )}
          </div>

          <div className='relative'>
            <div
              className='rounded p-3 overflow-auto max-h-64 border'
              style={{
                backgroundColor: state.isDarkMode ? '#1f2937' : '#f3f4f6',
                borderColor: state.isDarkMode ? '#374151' : '#e5e7eb',
              }}>
              <pre
                className='text-xs whitespace-pre-wrap pr-16 font-mono'
                style={{
                  color: state.isDarkMode ? '#f9fafb' : '#111827',
                  backgroundColor: 'transparent',
                }}>
                {JSON.stringify(generatedConfig, null, 2)}
              </pre>
            </div>
            <button
              onClick={copyMcpServersJson}
              className='absolute top-2 right-2 btn-modern bg-blue-600 hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-800 text-white px-3 py-1.5 rounded text-sm shadow-lg z-50'
              style={{ zIndex: 9999 }}>
              {t('dashboard.client_config.copy_config')}
            </button>
          </div>

          <div className='flex items-center gap-3 pt-1'>
            <Text className='text-sm text-gray-600 dark:text-gray-300'>
              {t('dashboard.client_config.add_to_clients')}
            </Text>
            <div className='flex flex-wrap gap-1'>
              {[
                {
                  label: 'Cherry Studio',
                  installUrl: generateClientInstallConfig('cherrystudio'),
                  iconUrl: '/cherry-studio.png',
                },
                {
                  label: 'Claude Desktop',
                  installUrl: generateClientInstallConfig('claude'),
                  iconUrl: '/anthropic.ico',
                },
                {
                  label: 'Claude Code',
                  installUrl: generateClientInstallConfig('claude'),
                  iconUrl: '/claude-code.svg',
                },
                {
                  label: 'Cursor',
                  installUrl: generateClientInstallConfig('cursor'),
                  iconUrl: '/cursor.ico',
                },
                {
                  label: 'Windsurf',
                  installUrl: generateClientInstallConfig('windsurf'),
                  iconUrl: '/windsurf.ico',
                },
                {
                  label: 'VSCode',
                  installUrl: generateClientInstallConfig('vscode'),
                  iconUrl: '/vscode.ico',
                },
              ].map((c) => (
                <div
                  key={c.label}
                  onClick={() => openUrl(c.installUrl)}
                  className='group flex items-center gap-2 cursor-pointer rounded-lg px-0.5 py-0.5 text-gray-700 dark:text-gray-300 hover:bg-gray-200 hover:text-gray-900 dark:hover:bg-gray-500/50 dark:hover:text-white transition-all duration-300 hover:scale-105'>
                  <img
                    src={c.iconUrl}
                    alt={c.label}
                    className='w-5 h-5 rounded'
                  />
                  <Text className='text-xs overflow-hidden max-w-0 group-hover:max-w-[140px] transition-all duration-300 delay-75 whitespace-nowrap'>
                    {c.label}
                  </Text>
                </div>
              ))}
            </div>
          </div>
        </div>
      </Card>

      {/* 系统状态 */}
      <Card
        title={
          <Space>
            <Activity size={16} />
            {t('dashboard.system_status.title')}
          </Space>
        }>
        <div className='grid grid-cols-1 md:grid-cols-2 gap-6'>
          {/* MCP 聚合接口状态 */}
          <div className='space-y-4'>
            <Text strong className='block mb-3'>
              {t('dashboard.aggregator.title')}
            </Text>
            <Space orientation='vertical' size='small' className='w-full'>
              <div className='flex justify-between items-center'>
                <Text type='secondary'>
                  {t('dashboard.aggregator.endpoint')}:
                </Text>
                <Text
                  copyable={{ text: stats.aggregator?.endpoint }}
                  className='font-mono text-sm'>
                  {stats.aggregator?.endpoint}
                </Text>
              </div>
              <div className='flex justify-between items-center'>
                <Text type='secondary'>
                  {t('dashboard.aggregator.status')}:
                </Text>
                <div className='flex items-center space-x-2'>
                  <div
                    className='w-2 h-2 rounded-full'
                    style={{
                      backgroundColor:
                        stats.aggregator?.status === 'running'
                          ? '#52c41a'
                          : stats.aggregator?.status === 'error'
                          ? '#ff4d4f'
                          : '#8c8c8c',
                    }}
                  />
                  <Text
                    style={{
                      color:
                        stats.aggregator?.status === 'running'
                          ? '#52c41a'
                          : stats.aggregator?.status === 'error'
                          ? '#ff4d4f'
                          : '#8c8c8c',
                    }}>
                    {stats.aggregator?.status === 'running' &&
                      t('dashboard.aggregator.running')}
                    {stats.aggregator?.status === 'stopped' &&
                      t('dashboard.aggregator.stopped')}
                    {stats.aggregator?.status === 'error' &&
                      t('dashboard.aggregator.error')}
                  </Text>
                </div>
              </div>
              <div className='flex justify-between items-center'>
                <Text type='secondary'>
                  {t('dashboard.aggregator.connected_services')}:
                </Text>
                <Text strong>{stats.aggregator?.connected_services || 0}</Text>
              </div>
            </Space>
          </div>

          {/* 系统信息 */}
          <div className='space-y-4'>
            <Text strong className='block mb-3'>
              {t('dashboard.system_info.runtime')}
            </Text>
            <Space orientation='vertical' size='small' className='w-full'>
              <div className='flex justify-between items-center'>
                <Text type='secondary'>
                  {t('dashboard.system_info.uptime')}:
                </Text>
                <Text strong>{currentUptime}</Text>
              </div>
              <div className='flex justify-between items-center'>
                <Text type='secondary'>
                  {t('dashboard.system_info.startup_time')}:
                </Text>
                <Text className='text-sm'>
                  {stats.startup_time
                    ? new Date(stats.startup_time).toLocaleString()
                    : '-'}
                </Text>
              </div>
              <div className='flex justify-between items-center'>
                <Text type='secondary'>
                  {t('dashboard.system_info.active_clients')}:
                </Text>
                <Text strong>{stats.active_clients}</Text>
              </div>
            </Space>
          </div>
        </div>
      </Card>
    </div>
  )
}

export default Dashboard
