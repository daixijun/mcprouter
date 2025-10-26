import React, { useEffect, useRef, useState } from 'react'
import { ApiService } from '../services/api'
import toastService from '../services/toastService'
import type { MarketplaceService, MarketplaceServiceListItem } from '../types'
import InstallConfirmModal from '../components/InstallConfirmModal'
import ServiceDetail from '../components/ServiceDetail'

const Marketplace: React.FC = () => {
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

  // Keep refs in sync with state
  useEffect(() => {
    hasMoreRef.current = hasMore
  }, [hasMore])

  useEffect(() => {
    loadInitialPlatformCounts()
  }, [])

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
      searchServices(true)
    } else {
      // When search is cleared, reload initial data
      loadInitialPlatformCounts()
    }
  }, [searchQuery])

  // Removed: loadPopularServices and related popular services feature

  async function loadInitialPlatformCounts() {
    setLoading(true)
    try {
      const result = await ApiService.listMarketplaceServices('', 1, page_size)
      setServices(result.services)
      setModelScopePagination({
        page: 2,
        hasMore: result.has_more,
      })
      setHasMore(result.has_more)
      hasMoreRef.current = result.has_more
    } catch (error) {
      console.error('Failed to load initial services:', error)
      toastService.sendErrorNotification(
        '加载服务失败，请检查网络连接或稍后重试。',
      )
    } finally {
      setLoading(false)
    }
  }

  async function searchServices(isReset = false) {
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

    console.log(`Loading services: isReset=${isReset}`)

    try {
      // Unified marketplace pagination
      const currentPage = isReset ? 1 : modelScopePagination.page

      const result = await ApiService.listMarketplaceServices(
        searchQuery,
        currentPage,
        page_size,
      )

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
      console.error('Failed to search services:', error)
      toastService.sendErrorNotification(`搜索服务失败: ${error}`)
    } finally {
      setLoading(false)
      setLoadingMore(false)
      isLoadingRef.current = false
    }
  }

  // Scroll listener for infinite loading
  const handleScroll = () => {
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
  }

  useEffect(() => {
    const container = scrollContainerRef.current
    if (!container) return

    console.log('Attaching scroll listener (once on mount)')
    container.addEventListener('scroll', handleScroll)
    return () => {
      console.log('Removing scroll listener')
      container.removeEventListener('scroll', handleScroll)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleInstall = async (
    service: MarketplaceServiceListItem | MarketplaceService,
  ) => {
    setPendingInstallService(service)

    // 使用服务详情中的env_schema，如果没有则从后端获取详情
    let schema = 'env_schema' in service ? service.env_schema : null

    // 如果列表项中没有env_schema，从后端获取服务详情
    if (!schema) {
      try {
        const detailedService = await ApiService.getMcpServerDetails(service.id)
        schema = detailedService.env_schema
      } catch (e) {
        console.warn('获取服务详情失败:', e)
      }
    }

    setEnvSchema(schema || null)
    setShowInstallModal(true)
  }

  const handleConfirmInstall = async (envVars: Record<string, string>) => {
    if (!pendingInstallService) return

    setIsInstalling(true)
    try {
      const envEntries = Object.entries(envVars)
      await ApiService.installMarketplaceService(
        pendingInstallService.id,
        envEntries.length > 0 ? envEntries : undefined,
      )
      toastService.sendSuccessNotification(
        `服务 "${pendingInstallService.name}" 安装成功！`,
      )
      setShowInstallModal(false)
      setPendingInstallService(null)
      setEnvSchema(null)
    } catch (error) {
      console.error('安装失败:', error)
      toastService.sendErrorNotification('安装失败，请检查日志获取详细信息。')
    } finally {
      setIsInstalling(false)
    }
  }

  const handleCancelInstall = () => {
    setShowInstallModal(false)
    setPendingInstallService(null)
    setEnvSchema(null)
  }

  const handleViewDetails = async (service: MarketplaceServiceListItem) => {
    setLoadingDetail(true)
    setSelectedService(null) // Clear previous selection
    setViewMode('detail') // Switch to detail view
    try {
      const details = await ApiService.getMcpServerDetails(service.id)
      setSelectedService(details) // Set the full service details
    } catch (error) {
      console.error('Failed to load service details:', error)
      toastService.sendErrorNotification(
        `加载服务 "${service.name}" 详情失败，请检查网络连接或稍后重试。`,
      )
      setSelectedService(null) // Clear on error
      setViewMode('list') // Back to list on error
    } finally {
      setLoadingDetail(false)
    }
  }

  const handleBackToList = () => {
    setViewMode('list')
    setSelectedService(null)
  }

  const getPlatformBadgeColor = (platform: string) => {
    switch (platform) {
      case '魔搭社区':
        return 'bg-rose-100 dark:bg-rose-900/30 text-rose-800 dark:text-rose-300'
      default:
        return 'bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200'
    }
  }

  const getPlatformIcon = (platform: string) => {
    switch (platform) {
      case '魔搭社区':
        return (
          <img
            src={'https://g.alicdn.com/sail-web/maas/2.9.94/favicon/128.ico'}
            alt='魔搭社区'
            className='inline-block w-4 h-4 mr-1 align-middle object-contain'
            loading='lazy'
          />
        )
      default:
        return '📦'
    }
  }

  const renderServiceCard = (service: MarketplaceServiceListItem) => (
    <div
      key={service.id}
      onClick={() => handleViewDetails(service)}
      className='card-glass p-6 hover:shadow-lg hover:border-blue-500 border-2 border-transparent transition-all duration-200 cursor-pointer'>
      <div className='flex gap-4 mb-4'>
        {/* Logo */}
        <div className='flex-shrink-0 w-12 h-12 rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-700 flex items-center justify-center'>
          {service.logo_url ? (
            <img
              src={service.logo_url}
              alt={service.name}
              className='w-full h-full object-cover'
            />
          ) : (
            <span className='text-2xl'>📦</span>
          )}
        </div>

        {/* Title and metadata */}
        <div className='flex-1 min-w-0'>
          <h4 className='font-bold text-lg text-gray-800 dark:text-gray-100 truncate'>
            {service.name}
          </h4>
          <div className='flex items-center gap-2 mt-1 text-sm text-gray-600 dark:text-gray-300'>
            <span className='flex items-center gap-1'>
              <span>👤</span>
              <span className='text-xs'>{service.author}</span>
            </span>
            {service.license && (
              <>
                <span className='text-gray-400 dark:text-gray-500'>•</span>
                <span className='flex items-center gap-1'>
                  <span>📄</span>
                  <span>{service.license}</span>
                </span>
              </>
            )}
          </div>
        </div>
      </div>

      <p className='text-gray-600 dark:text-gray-300 text-sm mb-4 line-clamp-2'>
        {service.description}
      </p>

      <div className='flex flex-wrap gap-2 mb-4'>
        <span
          className={`badge-modern ${getPlatformBadgeColor(service.platform)}`}>
          {getPlatformIcon(service.platform)} {service.platform}
        </span>
        {service.is_verified && (
          <span className='badge-modern bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300'>
            ✅ 已验证
          </span>
        )}
        {service.is_hosted && (
          <span className='badge-modern bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300'>
            🖥️ 托管
          </span>
        )}
      </div>

      <div className='flex items-center justify-between'>
        <div className='flex items-center space-x-3 text-sm'>
          {typeof service.github_stars === 'number' &&
            service.github_stars > 0 && (
              <span className='text-yellow-500'>
                ⭐ {service.github_stars.toLocaleString()}
              </span>
            )}
          <span className='text-gray-500 dark:text-gray-400'>
            📥 {service.downloads.toLocaleString()}
          </span>
        </div>
      </div>
    </div>
  )

  return (
    <div className='h-full flex flex-col overflow-hidden'>
      {viewMode === 'list' ? (
        // List View
        <div
          ref={scrollContainerRef}
          className='flex-1 flex flex-col space-y-6 overflow-y-auto'>
          {/* Search Bar */}
          <div className='card-glass p-4'>
            <input
              type='text'
              placeholder='🔍 搜索 MCP 服务...'
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className='input-modern w-full'
            />
          </div>

          {/* Search Results */}
          <div>
            {loading ? (
              <div className='card-glass p-12 text-center'>
                <div className='animate-spin rounded-full h-12 w-12 border-4 border-blue-500 border-t-transparent mx-auto mb-4'></div>
                <p className='text-gray-600 dark:text-gray-300'>
                  正在加载精彩的 MCP 服务...
                </p>
              </div>
            ) : services.length > 0 ? (
              <>
                <div className='grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6'>
                  {services.map(renderServiceCard)}
                </div>

                {/* Loading More Indicator */}
                {loadingMore && (
                  <div className='mt-8 text-center'>
                    <div className='animate-spin rounded-full h-8 w-8 border-4 border-blue-500 border-t-transparent mx-auto mb-2'></div>
                    <p className='text-gray-600 dark:text-gray-300'>
                      加载更多服务...
                    </p>
                  </div>
                )}

                {/* No More Data Indicator */}
                {!hasMore && services.length > 0 && (
                  <div className='mt-8 text-center'>
                    <p className='text-gray-500 dark:text-gray-400'>
                      已显示全部 {services.length} 个服务
                    </p>
                    <p className='text-sm text-amber-600 mt-2'>
                      ⚠️ 由于官方接口限制，最多能获取 100 条数据
                    </p>
                  </div>
                )}
              </>
            ) : (
              <div className='card-glass p-12 text-center'>
                <div className='text-6xl mb-4'>😔</div>
                <h3 className='text-xl font-semibold text-gray-700 mb-2'>
                  未找到服务
                </h3>
                <p className='text-gray-500 dark:text-gray-400'>
                  请尝试调整您的搜索词或分类筛选。
                </p>
              </div>
            )}
          </div>
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
    </div>
  )
}

export default Marketplace
