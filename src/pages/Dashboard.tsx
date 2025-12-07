import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { App, Space, Statistic, Typography } from 'antd'
import Card from 'antd/es/card'
import { Activity, Server, TrendingUp, Wrench, XCircle } from 'lucide-react'
import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { DashboardStats } from '../types'

const { Text } = Typography

const Dashboard: React.FC = () => {
  const { t } = useTranslation()
  const [stats, setStats] = useState<DashboardStats>({
    total_servers: 0,
    enabled_servers: 0,
    failed_servers: 0,
    healthy_services: 0,
    connected_services: 0,
    total_tools: 0,
    active_clients: 0,
    startup_time: '',
  })
  const { message } = App.useApp()
  const [isLoading, setIsLoading] = useState(true)
  const [currentUptime, setCurrentUptime] = useState<string>('-')

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
  }, [])

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

  const generatedConfig = {
    mcpServers: {
      mcprouter: {
        type: 'http',
        url: stats.aggregator?.endpoint || '',
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
    const base = {
      mcpServers: {
        mcprouter: {
          type: 'http',
          url: endpoint,
        },
      },
    }
    switch (client) {
      case 'cherrystudio':
        const cherrystudioConfig = {
          ...base,
          mcpServers: {
            ...base.mcpServers,
            mcprouter: {
              ...base.mcpServers.mcprouter,
              type: 'streamableHttp',
              baseUrl: endpoint,
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
            title={t('dashboard.stats.failed_servers')}
            value={stats.failed_servers}
            prefix={<XCircle size={14} />}
            styles={{ content: { color: '#ff4d4f', fontSize: '18px' } }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.connected_services')}
            value={formatNumber(stats.connected_services)}
            prefix={<TrendingUp size={14} />}
            styles={{ content: { color: '#13c2c2', fontSize: '18px' } }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.system_info.uptime')}
            value={currentUptime}
            prefix={<Activity size={14} />}
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
          <div className='relative'>
            <div
              className='bg-transparent dark:bg-gray-900 rounded p-3 overflow-auto max-h-64 border border-gray-200 dark:border-gray-700'
              // style={{ backgroundColor: 'var(--color-bg-secondary)' }}
            >
              <pre className='text-xs whitespace-pre-wrap text-gray-800 dark:text-gray-100 pr-16 font-mono'>
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

      {/* 系统状态概览 */}
      <Card
        title={
          <Space>
            <Activity size={16} />
            {t('dashboard.system_info.title')}
          </Space>
        }>
        <div className='flex flex-wrap gap-8'>
          {/* 系统信息 */}
          <div className='flex-1 min-w-[200px]'>
            <Text strong className='block mb-2'>
              {t('dashboard.system_info.version')}
            </Text>
            <Space direction='vertical' size='small' className='text-sm'>
              <div>
                <Text type='secondary'>{t('dashboard.system_info.os')}: </Text>
                <Text>{stats.os_info?.type || 'Unknown'}</Text>
              </div>
              <div>
                <Text type='secondary'>
                  {t('dashboard.system_info.os_version')}:{' '}
                </Text>
                <Text>{stats.os_info?.version || 'Unknown'}</Text>
              </div>
              <div>
                <Text type='secondary'>
                  {t('dashboard.system_info.arch')}:{' '}
                </Text>
                <Text>{stats.os_info?.arch || 'Unknown'}</Text>
              </div>
            </Space>
          </div>

          {/* 聚合接口信息 */}
          <div className='flex-1 min-w-[200px]'>
            <Text strong className='block mb-2'>
              {t('dashboard.aggregator.title')}
            </Text>
            <Space direction='vertical' size='small' className='text-sm'>
              <div>
                <Text type='secondary'>
                  {t('dashboard.aggregator.endpoint')}:{' '}
                </Text>
                <Text copyable={{ text: stats.aggregator?.endpoint }}>
                  {stats.aggregator?.endpoint}
                </Text>
              </div>
              <div>
                <Text type='secondary'>
                  {t('dashboard.aggregator.status')}:{' '}
                </Text>
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
              <div>
                <Text type='secondary'>
                  {t('dashboard.aggregator.connections')}:{' '}
                </Text>
                <Text>{stats.aggregator?.connected_services}</Text>
              </div>
            </Space>
          </div>
        </div>
      </Card>
    </div>
  )
}

export default Dashboard
