import {
  App,
  Badge,
  Button,
  Col,
  Flex,
  Input,
  Row,
  Space,
  Spin,
  Typography,
} from 'antd'
import Card from 'antd/es/card'
import React, {
  ChangeEvent,
  memo,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react'
import { useTranslation } from 'react-i18next'
import InstallConfirmModal from '../components/InstallConfirmModal'
import ServiceDetail from '../components/ServiceDetail'
import { MarketplaceApi } from '../services/marketplace-service'
import type { MarketplaceService, MarketplaceServiceListItem } from '../types'

const { Title, Text, Paragraph } = Typography

const Marketplace: React.FC = memo(() => {
  const { t } = useTranslation()
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
      message.error(t('marketplace.messages.load_services_failed'))
    } finally {
      setLoading(false)
    }
  }, [message.error])

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
        message.error(t('marketplace.messages.search_failed', { error }))
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
    [searchQuery, modelScopePagination.page, message.error],
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

  // Handle load more button click
  const handleLoadMore = useCallback(() => {
    console.log('Load more button clicked')
    searchServices(false)
  }, [searchServices])

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
          console.warn(t('marketplace.messages.get_service_details_failed'), e)
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
        message.success(
          t('marketplace.messages.install_success', {
            name: pendingInstallService.name,
          }),
        )
        setShowInstallModal(false)
        setPendingInstallService(null)
        setEnvSchema(null)
      } catch (error) {
        console.error('安装失败:', error)
        message.error(t('marketplace.messages.install_failed'))
      } finally {
        setIsInstalling(false)
      }
    },
    [pendingInstallService, message.error],
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
        message.error(
          t('marketplace.messages.load_service_details_failed', {
            name: service.name,
          }),
        )
        setSelectedService(null) // Clear on error
        setViewMode('list') // Back to list on error
      } finally {
        setLoadingDetail(false)
      }
    },
    [message.error],
  )

  const handleBackToList = useCallback(() => {
    setViewMode('list')
    setSelectedService(null)
  }, [])

  const getPlatformBadgeColor = useCallback((platform: string) => {
    switch (platform) {
      case t('marketplace.platforms.modelscope'):
        return '#ea580c' // orange-600
      default:
        return '#6b7280' // gray-500
    }
  }, [])

  const getPlatformIcon = useCallback((platform: string) => {
    switch (platform) {
      case t('marketplace.platforms.modelscope'):
        return (
          <img
            src={'https://g.alicdn.com/sail-web/maas/2.9.94/favicon/128.ico'}
            alt={t('marketplace.platforms.modelscope')}
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
          marginBottom: '5px',
          cursor: 'pointer',
          transition: 'all 0.2s',
          height: '220px',
          display: 'flex',
          flexDirection: 'column',
        }}>
        <Flex gap='small' style={{ marginBottom: '12px' }}>
          {/* Logo */}
          <div
            className='shrink-0 w-12 h-12 rounded-lg overflow-hidden bg-gray-50 dark:bg-gray-800 flex items-center justify-center'
            style={{
              width: '48px',
              height: '48px',
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
              className='mt-1 text-sm text-gray-500 dark:text-gray-400'
              style={{
                marginTop: '4px',
                fontSize: '14px',
              }}>
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
          style={{ marginBottom: '12px', fontSize: '14px', flex: 1 }}>
          {service.description}
        </Paragraph>

        <Flex gap='small' style={{ marginBottom: '12px' }}>
          <Badge color={getPlatformBadgeColor(service.platform)}>
            <Flex align='center' gap={4}>
              {getPlatformIcon(service.platform)}
              <Text style={{ fontSize: '12px', whiteSpace: 'nowrap' }}>
                {service.platform}
              </Text>
            </Flex>
          </Badge>
          {service.is_verified && (
            <Badge color='#10b981'>{t('marketplace.badges.verified')}</Badge>
          )}
          {service.is_hosted && (
            <Badge color='#2563eb'>{t('marketplace.badges.hosted')}</Badge>
          )}
        </Flex>

        <Flex justify='space-between' align='center'>
          <Space size='large'>
            {typeof service.github_stars === 'number' &&
              service.github_stars > 0 && (
                <Text className='text-yellow-600 dark:text-yellow-400'>
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
        <div style={{ flex: 1, overflowY: 'auto' }}>
          <Flex vertical gap='large'>
            {/* Search Bar */}
            <Card>
              <Input
                placeholder={t('marketplace.search.placeholder')}
                defaultValue={searchQuery}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  handleSearchChange(e.target.value)
                }
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
                  <Spin
                    size='large'
                    tip={t('marketplace.status.loading_services')}
                  />
                </Flex>
              ) : services.length > 0 ? (
                <>
                  <Row gutter={[16, 16]}>
                    {services.map((service) => (
                      <Col
                        xs={24}
                        sm={24}
                        md={12}
                        lg={8}
                        xl={6}
                        xxl={6}
                        key={service.id}>
                        {renderServiceCard(service)}
                      </Col>
                    ))}
                  </Row>

                  {/* Load More Button */}
                  {hasMore && (
                    <Flex justify='center' style={{ marginTop: '32px' }}>
                      <Button
                        type='primary'
                        size='large'
                        onClick={handleLoadMore}
                        loading={loadingMore}>
                        {loadingMore
                          ? t('marketplace.actions.loading_more')
                          : t('marketplace.actions.load_more')}
                      </Button>
                    </Flex>
                  )}

                  {/* No More Data Indicator */}
                  {!hasMore && services.length > 0 && (
                    <Flex
                      vertical
                      align='center'
                      style={{ marginTop: '32px', textAlign: 'center' }}>
                      <Text type='secondary'>
                        {t('marketplace.messages.show_all_services', {
                          count: services.length,
                        })}
                      </Text>
                      <Text
                        type='warning'
                        style={{ marginTop: '8px', fontSize: '14px' }}>
                        {t('marketplace.messages.data_limit_warning')}
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
                    {t('marketplace.empty.title')}
                  </Title>
                  <Text type='secondary'>
                    {t('marketplace.empty.description')}
                  </Text>
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

      {/* Install confirmation modal */}
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
