import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { Layout } from 'antd'
import type { MarketProvider, MarketService } from '../../types/mcp-market'
import { modelScopeProvider } from '../../services/modelscope-provider'
import { useAntdConfig } from '../../components/AntdConfigProvider'
import { useAppContext } from '../../contexts/AppContext'
import ProviderList from './ProviderList'
import ServiceList from './ServiceList'
import ServiceDetail from './ServiceDetail'

const PAGE_SIZE = 15

interface McpMarketState {
  services: MarketService[]
  loading: boolean
  hasMore: boolean
  searchQuery: string
  currentPage: number
}

export default function McpMarket() {
  const { t, i18n } = useTranslation()
  const { isDarkMode } = useAntdConfig()
  const { state: appState } = useAppContext()

  const [selectedProviderId, setSelectedProviderId] = useState<string | null>('modelscope')
  const [state, setState] = useState<McpMarketState>({
    services: [],
    loading: false,
    hasMore: true,
    searchQuery: '',
    currentPage: 1,
  })

  const providers: MarketProvider[] = [modelScopeProvider]

  const loadServices = useCallback(async (page: number, query: string, isNewSearch: boolean) => {
    setState((prev) => ({ ...prev, loading: true }))

    try {
      const result = await modelScopeProvider.searchServices({
        query,
        page,
        pageSize: PAGE_SIZE,
      }, i18n.language)

      setState((prev) => ({
        ...prev,
        loading: false,
        services: isNewSearch ? result.services : [...prev.services, ...result.services],
        hasMore: result.hasMore,
        currentPage: page,
      }))
    } catch (error) {
      console.error('Failed to load services:', error)
      setState((prev) => ({ ...prev, loading: false }))
    }
  }, [i18n.language])

  // 初始加载、Provider 切换、语言切换时加载
  useEffect(() => {
    loadServices(1, state.searchQuery, true)
  }, [selectedProviderId, i18n.language, loadServices, state.searchQuery])

  const handleProviderSelect = (providerId: string) => {
    setSelectedProviderId(providerId)
    setState((prev) => ({ ...prev, services: [], currentPage: 1, hasMore: true }))
  }

  const handleSearch = (query: string) => {
    setState((prev) => ({ ...prev, searchQuery: query }))
    loadServices(1, query, true)
  }

  const handleLoadMore = () => {
    if (!state.loading && state.hasMore) {
      loadServices(state.currentPage + 1, state.searchQuery, false)
    }
  }

  const marketT = (key: string) => t(`about.market.${key}`)

  // 根据主题设置侧边栏背景色
  const siderBgColor = isDarkMode ? undefined : '#f9fafb'

  return (
    <div className="h-full flex">
      {/* 如果选择了服务详情 */}
      {appState.marketServiceId ? (
        <ServiceDetail />
      ) : (
        <Layout className="h-full flex-auto">
          <Layout.Sider
            width={240}
            style={{
              background: siderBgColor,
              borderRight: '1px solid #e5e7eb',
            }}
            className={isDarkMode ? 'dark-sider' : ''}
          >
            <div className="p-4">
              <h2
                className="text-lg font-semibold mb-4"
                style={{ color: isDarkMode ? '#f9fafb' : '#111827' }}
              >
                {marketT('title')}
              </h2>
              <ProviderList providers={providers} selectedProviderId={selectedProviderId} onProviderSelect={handleProviderSelect} />
            </div>
          </Layout.Sider>
          <Layout.Content
            style={{ background: isDarkMode ? undefined : '#ffffff' }}
            className={`overflow-auto ${isDarkMode ? 'dark-content' : ''}`}
          >
            <ServiceList
              services={state.services}
              loading={state.loading}
              onServiceClick={() => {
                // 可以添加其他逻辑，比如埋点统计
              }}
              onLoadMore={handleLoadMore}
              onSearch={handleSearch}
              hasMore={state.hasMore}
            />
          </Layout.Content>
        </Layout>
      )}
    </div>
  )
}
