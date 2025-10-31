import {
  App,
  Badge,
  Card,
  Col,
  Flex,
  Input,
  Row,
  Space,
  Spin,
  Typography,
} from 'antd'
import React, { memo, useCallback, useEffect, useRef, useState } from 'react'
import InstallConfirmModal from '../components/InstallConfirmModal'
import ServiceDetail from '../components/ServiceDetail'
import { useErrorContext } from '../contexts/ErrorContext'
import { MarketplaceApi } from '../services/marketplace-service'
import type { MarketplaceService, MarketplaceServiceListItem } from '../types'

const { Title, Text, Paragraph } = Typography

const Marketplace: React.FC = memo(() => {
  const { addError } = useErrorContext()
  const { message } = App.useApp()

  const [searchQuery, setSearchQuery] = useState('')
  const [services, setServices] = useState<MarketplaceServiceListItem[]>([])
  const [loading, setLoading] = useState(false)
  const [loadingMore, setLoadingMore] = useState(false)
  const [loadingDetail, setLoadingDetail] = useState(false)
  const [selectedService, setSelectedService] =
    useState<MarketplaceService | null>(null)
  const [hasMore, setHasMore] = useState(true)
  const [viewMode, setViewMode] = useState<'list' | 'detail'>('list')

  // 安装确认弹窗相关状态
  const [showInstallModal, setShowInstallModal] = useState(false)
  const [pendingInstallService, setPendingInstallService] = useState<
    MarketplaceServiceListItem | MarketplaceService | null
  >(null)
  const [envSchema, setEnvSchema] = useState<{
    properties?: Record<
      string,
      {
        title?: string
        description?: string
        type?: string
        default?: any
        enum?: any[]
      }
    >
    required?: string[]
  } | null>(null)
  const [isInstalling, setIsInstalling] = useState(false)

  // Pagination state
  const [modelScopePagination, setModelScopePagination] = useState({
    page: 1,
    hasMore: true,
  })

  const page_size = 100

  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const isLoadingRef = useRef(false)
  const hasMoreRef = useRef(true)

  // 防抖相关 refs
  const debounceTimerRef = useRef<number | null>(null)
  const abortControllerRef = useRef<AbortController | null>(null)

  // 防抖搜索处理函数
  const handleSearchChange = useCallback((value: string) => {
    // 清除之前的防抖定时器
    if (debounceTimerRef.current) {
      window.clearTimeout(debounceTimerRef.current)
    }

    // 取消之前的请求
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
    }

    // 设置新的防抖定时器（500ms 延迟）
    debounceTimerRef.current = window.setTimeout(() => {
      setSearchQuery(value)
    }, 500)
  }, [])

  // 组件卸载时清理
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) {
        window.clearTimeout(debounceTimerRef.current)
      }
      if (abortControllerRef.current) {
        abortControllerRef.current.abort()
      }
    }
  }, [])

  // Keep refs in sync with state
  useEffect(() => {
    hasMoreRef.current = hasMore
  }, [hasMore])

  // Removed: loadPopularServices and related popular services feature

  const loadInitialPlatformCounts = useCallback(async () => {
    setLoading(true)
    try {
      const result = await MarketplaceApi.listMarketplaceServices(
        '',
        1,
        page_size,
      )
      setServices(result.services)
      setModelScopePagination({
        page: 2,
        hasMore: result.has_more,
      })
      setHasMore(result.has_more)
      hasMoreRef.current = result.has_more
    } catch (error) {
      console.error('Failed to load initial services:', error)
      addError('加载服务失败，请检查网络连接或稍后重试。')
    } finally {
      setLoading(false)
    }
  }, [addError])

  const searchServices = useCallback(
    async (isReset = false) => {
      // Prevent multiple simultaneous requests
      if (isLoadingRef.current) {
        console.log('Already loading, skipping request')
        return
      }

      isLoadingRef.current = true
      if (isReset) {
        setLoading(true)
      } else {
        setLoadingMore(true)
      }

      // 创建新的 AbortController
      const controller = new AbortController()
      abortControllerRef.current = controller

      console.log(`Loading services: isReset=${isReset}`)

      try {
        // Unified marketplace pagination
        const currentPage = isReset ? 1 : modelScopePagination.page

        const result = await MarketplaceApi.listMarketplaceServices(
          searchQuery,
          currentPage,
          page_size,
        )

        // 检查请求是否被取消
        if (controller.signal.aborted) {
          console.log('Request was aborted')
          return
        }

        console.log(
          `Loaded ${result.services.length} services, page=${currentPage}, has_more=${result.has_more}`,
        )

        if (isReset) {
          setServices(result.services)
        } else {
          setServices((prev) => [...prev, ...result.services])
        }

        setModelScopePagination({
          page: currentPage + 1,
          hasMore: result.has_more,
        })
        setHasMore(result.has_more)
      } catch (error) {
        // 如果请求被取消，不显示错误
        if (controller.signal.aborted) {
          console.log('Request was aborted, ignoring error')
          return
        }
        console.error('Failed to search services:', error)
        addError(`搜索服务失败: ${error}`)
      } finally {
        setLoading(false)
        setLoadingMore(false)
        isLoadingRef.current = false
        // 清理 abort controller
        if (abortControllerRef.current === controller) {
          abortControllerRef.current = null
        }
      }
    },
    [searchQuery, modelScopePagination.page, addError],
  )

  useEffect(() => {
    loadInitialPlatformCounts()
  }, [loadInitialPlatformCounts])

  // 使用 useRef 来避免函数依赖循环
  const searchServicesRef = useRef(searchServices)
  const loadInitialPlatformCountsRef = useRef(loadInitialPlatformCounts)

  // 更新 ref 引用
  useEffect(() => {
    searchServicesRef.current = searchServices
    loadInitialPlatformCountsRef.current = loadInitialPlatformCounts
  }, [searchServices, loadInitialPlatformCounts])

  useEffect(() => {
    // Reset pagination when filters change
    setModelScopePagination({
      page: 1,
      hasMore: true,
    })
    setServices([])
    setHasMore(true)
    hasMoreRef.current = true

    // Only trigger search if there's a search query, otherwise reload initial data
    if (searchQuery.trim()) {
      searchServicesRef.current(true)
    } else {
      // When search is cleared, reload initial data
      loadInitialPlatformCountsRef.current()
    }
  }, [searchQuery])

  // Scroll listener for infinite loading
  const handleScroll = useCallback(() => {
    if (!scrollContainerRef.current) return

    const container = scrollContainerRef.current
    const scrollTop = container.scrollTop
    const scrollHeight = container.scrollHeight
    const clientHeight = container.clientHeight

    const distanceFromBottom = scrollHeight - scrollTop - clientHeight

    console.log(
      `Scroll: distance=${distanceFromBottom}, hasMore=${hasMoreRef.current}, isLoading=${isLoadingRef.current}`,
    )

    // Load more when scrolled to within 200px of bottom
    if (
      distanceFromBottom < 200 &&
      !isLoadingRef.current &&
      hasMoreRef.current
    ) {
      console.log('Triggering load more')
      searchServices(false)
    }
  }, [searchServices])

  useEffect(() => {
    const container = scrollContainerRef.current
    if (!container) return

    console.log('Attaching scroll listener (once on mount)')
    container.addEventListener('scroll', handleScroll)
    return () => {
      console.log('Removing scroll listener')
      container.removeEventListener('scroll', handleScroll)
    }
  }, [handleScroll])

  const handleInstall = useCallback(
    async (service: MarketplaceServiceListItem | MarketplaceService) => {
      setPendingInstallService(service)

      // 使用服务详情中的env_schema，如果没有则从后端获取详情
      let schema = 'env_schema' in service ? service.env_schema : null

      // 如果列表项中没有env_schema，从后端获取服务详情
      if (!schema) {
        try {
          const detailedService = await MarketplaceApi.getMcpServerDetails(
            service.id,
          )
          schema = detailedService.env_schema
        } catch (e) {
          console.warn('获取服务详情失败:', e)
        }
      }

      setEnvSchema(schema || null)
      setShowInstallModal(true)
    },
    [],
  )

  const handleConfirmInstall = useCallback(
    async (envVars: Record<string, string>) => {
      if (!pendingInstallService) return

      setIsInstalling(true)
      try {
        const envEntries = Object.entries(envVars)
        await MarketplaceApi.installMarketplaceService(
          pendingInstallService.id,
          envEntries.length > 0 ? envEntries : undefined,
        )
        message.success(`服务 "${pendingInstallService.name}" 安装成功！`)
        setShowInstallModal(false)
        setPendingInstallService(null)
        setEnvSchema(null)
      } catch (error) {
        console.error('安装失败:', error)
        addError('安装失败，请检查日志获取详细信息。')
      } finally {
        setIsInstalling(false)
      }
    },
    [pendingInstallService, addError],
  )

  const handleCancelInstall = useCallback(() => {
    setShowInstallModal(false)
    setPendingInstallService(null)
    setEnvSchema(null)
  }, [])

  const handleViewDetails = useCallback(
    async (service: MarketplaceServiceListItem) => {
      setLoadingDetail(true)
      setSelectedService(null) // Clear previous selection
      setViewMode('detail') // Switch to detail view
      try {
        const details = await MarketplaceApi.getMcpServerDetails(service.id)
        setSelectedService(details) // Set the full service details
      } catch (error) {
        console.error('Failed to load service details:', error)
        addError(
          `加载服务 "${service.name}" 详情失败，请检查网络连接或稍后重试。`,
        )
        setSelectedService(null) // Clear on error
        setViewMode('list') // Back to list on error
      } finally {
        setLoadingDetail(false)
      }
    },
    [addError],
  )

  const handleBackToList = useCallback(() => {
    setViewMode('list')
    setSelectedService(null)
  }, [])

  const getPlatformBadgeColor = useCallback((platform: string) => {
    switch (platform) {
      case '魔搭社区':
        return '#f43f5e'
      default:
        return '#6b7280'
    }
  }, [])

  const getPlatformIcon = useCallback((platform: string) => {
    switch (platform) {
      case '魔搭社区':
        return (
          <img
            src={'https://g.alicdn.com/sail-web/maas/2.9.94/favicon/128.ico'}
            alt='魔搭社区'
            style={{
              width: '16px',
              height: '16px',
              marginRight: '4px',
              verticalAlign: 'middle',
              objectFit: 'contain',
            }}
            loading='lazy'
          />
        )
      default:
        return '📦'
    }
  }, [])

  const renderServiceCard = useCallback(
    (service: MarketplaceServiceListItem) => (
      <Card
        key={service.id}
        hoverable
        onClick={() => handleViewDetails(service)}
        style={{
          marginBottom: '16px',
          cursor: 'pointer',
          transition: 'all 0.2s',
        }}>
        <Flex gap='middle' style={{ marginBottom: '16px' }}>
          {/* Logo */}
          <div
            style={{
              flexShrink: 0,
              width: '48px',
              height: '48px',
              borderRadius: '8px',
              overflow: 'hidden',
              backgroundColor: '#f5f5f5',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}>
            {service.logo_url ? (
              <img
                src={service.logo_url}
                alt={service.name}
                style={{ width: '100%', height: '100%', objectFit: 'cover' }}
              />
            ) : (
              <span style={{ fontSize: '20px' }}>📦</span>
            )}
          </div>

          {/* Title and metadata */}
          <div style={{ flex: 1, minWidth: 0 }}>
            <Title level={4} style={{ margin: 0, fontSize: '18px' }} ellipsis>
              {service.name}
            </Title>
            <Flex
              gap='small'
              align='center'
              style={{ marginTop: '4px', fontSize: '14px', color: '#666' }}>
              <Flex align='center' gap='small'>
                <span>👤</span>
                <Text style={{ fontSize: '12px' }}>{service.author}</Text>
              </Flex>
              {service.license && (
                <>
                  <Text type='secondary'>•</Text>
                  <Flex align='center' gap='small'>
                    <span>📄</span>
                    <Text>{service.license}</Text>
                  </Flex>
                </>
              )}
            </Flex>
          </div>
        </Flex>

        <Paragraph
          type='secondary'
          ellipsis={{ rows: 2 }}
          style={{ marginBottom: '16px', fontSize: '14px' }}>
          {service.description}
        </Paragraph>

        <Flex wrap gap='small' style={{ marginBottom: '16px' }}>
          <Badge color={getPlatformBadgeColor(service.platform)}>
            {getPlatformIcon(service.platform)} {service.platform}
          </Badge>
          {service.is_verified && <Badge color='#52c41a'>✅ 已验证</Badge>}
          {service.is_hosted && <Badge color='#722ed1'>🖥️ 托管</Badge>}
        </Flex>

        <Flex justify='space-between' align='center'>
          <Space size='large'>
            {typeof service.github_stars === 'number' &&
              service.github_stars > 0 && (
                <Text style={{ color: '#faad14' }}>
                  ⭐ {service.github_stars.toLocaleString()}
                </Text>
              )}
            <Text type='secondary'>
              📥 {service.downloads.toLocaleString()}
            </Text>
          </Space>
        </Flex>
      </Card>
    ),
    [handleViewDetails, getPlatformBadgeColor, getPlatformIcon],
  )

  return (
    <Flex
      vertical
      gap='large'
      style={{ height: '100%', overflowY: 'auto', padding: '24px' }}>
      {viewMode === 'list' ? (
        // List View
        <div ref={scrollContainerRef} style={{ flex: 1, overflowY: 'auto' }}>
          <Flex vertical gap='large'>
            {/* Search Bar */}
            <Card>
              <Input
                placeholder='🔍 搜索 MCP 服务...'
                defaultValue={searchQuery}
                onChange={(e) => handleSearchChange(e.target.value)}
                style={{ width: '100%' }}
              />
            </Card>

            {/* Search Results */}
            <div>
              {loading ? (
                <Flex
                  justify='center'
                  align='center'
                  style={{ padding: '48px 16px' }}>
                  <Spin size='large' tip='正在加载精彩的 MCP 服务...' />
                </Flex>
              ) : services.length > 0 ? (
                <>
                  <Row gutter={[16, 16]}>
                    {services.map((service) => (
                      <Col span={8} key={service.id}>
                        {renderServiceCard(service)}
                      </Col>
                    ))}
                  </Row>

                  {/* Loading More Indicator */}
                  {loadingMore && (
                    <Flex justify='center' style={{ marginTop: '32px' }}>
                      <Spin tip='加载更多服务...' />
                    </Flex>
                  )}

                  {/* No More Data Indicator */}
                  {!hasMore && services.length > 0 && (
                    <Flex
                      vertical
                      align='center'
                      style={{ marginTop: '32px', textAlign: 'center' }}>
                      <Text type='secondary'>
                        已显示全部 {services.length} 个服务
                      </Text>
                      <Text
                        type='warning'
                        style={{ marginTop: '8px', fontSize: '14px' }}>
                        ⚠️ 由于官方接口限制，最多能获取 100 条数据
                      </Text>
                    </Flex>
                  )}
                </>
              ) : (
                <Flex
                  vertical
                  align='center'
                  style={{ padding: '48px 16px', textAlign: 'center' }}>
                  <div style={{ fontSize: '48px', marginBottom: '16px' }}>
                    😔
                  </div>
                  <Title level={4} style={{ marginBottom: '8px' }}>
                    未找到服务
                  </Title>
                  <Text type='secondary'>请尝试调整您的搜索词或分类筛选。</Text>
                </Flex>
              )}
            </div>
          </Flex>
        </div>
      ) : (
        // Detail View
        <ServiceDetail
          service={selectedService}
          loading={loadingDetail}
          onBack={handleBackToList}
          onInstall={handleInstall}
        />
      )}

      {/* 安装确认模态框 */}
      <InstallConfirmModal
        isOpen={showInstallModal}
        onClose={handleCancelInstall}
        onConfirm={handleConfirmInstall}
        service={pendingInstallService}
        envSchema={envSchema}
        isLoading={isInstalling}
      />
    </Flex>
  )
})

export default Marketplace
