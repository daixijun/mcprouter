import { invoke } from '@tauri-apps/api/core'
import { Card, List, Space, Statistic, Typography } from 'antd'
import {
  Activity,
  AlertCircle,
  Server,
  TrendingUp,
  Wrench,
  XCircle,
} from 'lucide-react'
import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { DashboardStats } from '../types'

const { Text } = Typography

interface RecentActivity {
  id: string
  type: 'server_start' | 'server_stop' | 'request' | 'error'
  message: string
  timestamp: string
}

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
  const [activities, setActivities] = useState<RecentActivity[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [currentUptime, setCurrentUptime] = useState<string>('-')

  // 加载仪表板数据
  const loadDashboardData = async () => {
    try {
      setIsLoading(true)

      // 并行加载所有数据
      const [statsResult, activitiesResult] = await Promise.allSettled([
        invoke<DashboardStats>('get_dashboard_stats'),
        invoke<RecentActivity[]>('get_recent_activities'),
      ])

      // 处理统计数据
      if (statsResult.status === 'fulfilled') {
        setStats(statsResult.value)
      }

      // 处理活动记录
      if (activitiesResult.status === 'fulfilled') {
        setActivities(activitiesResult.value)
      }
    } catch (error) {
      console.error('Failed to load dashboard data:', error)
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    loadDashboardData()
    // 每30秒刷新一次数据
    const interval = setInterval(loadDashboardData, 30000)
    return () => clearInterval(interval)
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

  // 获取活动图标
  const getActivityIcon = (type: RecentActivity['type']) => {
    switch (type) {
      case 'server_start':
        return <Activity size={16} className='text-green-500' />
      case 'server_stop':
        return <AlertCircle size={16} className='text-red-500' />
      case 'request':
        return <Activity size={16} className='text-blue-500' />
      case 'error':
        return <AlertCircle size={16} className='text-red-600' />
      default:
        return <Activity size={16} className='text-gray-500' />
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
            valueStyle={{ color: '#1890ff', fontSize: '18px' }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.enabled_servers')}
            value={stats.enabled_servers}
            prefix={<Activity size={14} />}
            valueStyle={{ color: '#52c41a', fontSize: '18px' }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.total_tools')}
            value={formatNumber(stats.total_tools)}
            prefix={<Wrench size={14} />}
            valueStyle={{ color: '#722ed1', fontSize: '18px' }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.failed_servers')}
            value={stats.failed_servers}
            prefix={<XCircle size={14} />}
            valueStyle={{ color: '#ff4d4f', fontSize: '18px' }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.stats.connected_services')}
            value={formatNumber(stats.connected_services)}
            prefix={<TrendingUp size={14} />}
            valueStyle={{ color: '#13c2c2', fontSize: '18px' }}
            className='text-xs'
          />
        </Card>

        <Card size='small' className='p-2'>
          <Statistic
            title={t('dashboard.system_info.uptime')}
            value={currentUptime}
            prefix={<Activity size={14} />}
            valueStyle={{ color: '#fa8c16', fontSize: '18px' }}
            className='text-xs'
          />
        </Card>
      </div>

      {/* 活动记录 */}
      <Card
        title={
          <Space>
            <Activity size={16} />
            {t('dashboard.sections.recent_activity')}
          </Space>
        }
        loading={isLoading}>
        {activities.length > 0 ? (
          <List
            dataSource={activities.slice(0, 5)}
            renderItem={(activity) => (
              <List.Item>
                <List.Item.Meta
                  avatar={getActivityIcon(activity.type)}
                  title={<Text className='text-sm'>{activity.message}</Text>}
                  description={
                    <Text type='secondary' className='text-xs'>
                      {new Date(activity.timestamp).toLocaleString()}
                    </Text>
                  }
                />
              </List.Item>
            )}
          />
        ) : (
          <div className='text-center py-8 text-gray-500'>
            <Activity size={32} className='mx-auto mb-2 opacity-50' />
            <Text type='secondary'>{t('dashboard.activity.empty')}</Text>
          </div>
        )}
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
