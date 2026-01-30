import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button, Card, Tag, Table, Empty, Spin, Tabs } from 'antd'
import { Download, Tag as TagIcon, ChevronLeft, Settings, Wrench } from 'lucide-react'
import { useAppContext } from '../../contexts/AppContext'
import { modelScopeProvider } from '../../services/modelscope-provider'
import type { MarketServiceDetail } from '../../types/mcp-market'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeRaw from 'rehype-raw'
import rehypeSanitize from 'rehype-sanitize'

export default function ServiceDetail() {
  const { t, i18n } = useTranslation()
  const { state, closeMarketService } = useAppContext()
  const [service, setService] = useState<MarketServiceDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState('readme')

  useEffect(() => {
    const fetchServiceDetail = async () => {
      if (!state.marketServiceId) return

      setLoading(true)
      try {
        const detail = await modelScopeProvider.getServiceDetail(state.marketServiceId, i18n.language)
        setService(detail)
      } catch (error) {
        console.error('Failed to fetch service detail:', error)
      } finally {
        setLoading(false)
      }
    }

    fetchServiceDetail()
  }, [state.marketServiceId, i18n.language])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Spin size="large" />
      </div>
    )
  }

  if (!service) {
    return (
      <div className="flex items-center justify-center h-full">
        <Empty description="服务不存在" />
      </div>
    )
  }

  const marketT = (key: string) => t(`about.market.${key}`)

  // 标签页配置
  const tabItems = [
    {
      key: 'readme',
      label: (
        <span className="flex items-center gap-2">
          <span>README</span>
        </span>
      ),
      children: service.readme ? (
        <div className="markdown-content overflow-auto p-4 w-full min-w-0">
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            rehypePlugins={[rehypeRaw, rehypeSanitize]}
          >
            {service.readme}
          </ReactMarkdown>
        </div>
      ) : (
        <div className="p-4 w-full">
          <Empty description="暂无 README" />
        </div>
      ),
    },
    {
      key: 'config',
      label: (
        <span className="flex items-center gap-2">
          <Settings size={14} />
          <span>Server Config</span>
        </span>
      ),
      children: (
        <div className="overflow-auto p-4 w-full min-w-0">
          {service.serverConfig ? (
            <div className="rounded-lg p-4 w-full overflow-x-auto border dark:border-gray-700" style={{ background: '#f1f5f9', borderColor: '#e2e8f0' }}>
              <pre className="text-sm" style={{ color: '#1f2937' }}>
                <code>
                  {JSON.stringify(
                    Array.isArray(service.serverConfig) && service.serverConfig.length > 0
                      ? service.serverConfig[0]
                      : service.serverConfig,
                    null,
                    2
                  )}
                </code>
              </pre>
            </div>
          ) : (
            <Empty description="暂无服务器配置" />
          )}
        </div>
      ),
    },
    {
      key: 'tools',
      label: (
        <span className="flex items-center gap-2">
          <Wrench size={14} />
          <span>Tools</span>
        </span>
      ),
      children: (
        <div className="overflow-auto p-4 w-full min-w-0">
          {service.tools && service.tools.length > 0 ? (
            <Table
              dataSource={service.tools.map((tool, index) => ({
                key: index,
                name: tool.name,
                description: tool.description,
              }))}
              columns={[
                {
                  title: 'Tool Name',
                  dataIndex: 'name',
                  key: 'name',
                  width: 200,
                  ellipsis: true,
                },
                {
                  title: 'Description',
                  dataIndex: 'description',
                  key: 'description',
                  ellipsis: true,
                },
              ]}
              pagination={false}
              size="small"
              tableLayout="fixed"
            />
          ) : (
            <Empty description="暂无工具信息" />
          )}
        </div>
      ),
    },
  ]

  return (
    <div className="w-full max-w-6xl mx-auto h-full flex flex-col p-4">
      {/* 返回按钮 */}
      <Button
        icon={<ChevronLeft size={16} />}
        onClick={closeMarketService}
        className="mb-4 flex-shrink-0"
        type="text"
      >
        {marketT('detail.back')}
      </Button>

      {/* 头部信息 */}
      <Card className="mb-4 flex-shrink-0 w-full" style={{ boxShadow: '0 2px 8px rgba(0,0,0,0.06)' }}>
        <div className="flex gap-4 w-full">
          {/* Logo */}
          {service.metadata?.logoUrl && (
            <img
              src={service.metadata.logoUrl}
              alt={service.displayName}
              className="w-20 h-20 rounded-lg object-cover flex-shrink-0"
            />
          )}

          {/* 标题和元信息 */}
          <div className="flex-1 min-w-0">
            <h1
              className="text-2xl font-bold mb-2"
              style={{ color: 'var(--card-title-color, #1f2937)' }}
            >
              {service.displayName}
            </h1>

            <div className="flex items-center gap-3 mb-3">
              {service.author && (
                <span
                  className="text-sm"
                  style={{ color: 'var(--card-text-secondary, #6b7280)' }}
                >
                  {service.author}
                </span>
              )}
              {service.category && (
                <Tag color="blue">{service.category}</Tag>
              )}
              <div className="flex items-center gap-1 text-sm" style={{ color: 'var(--card-text-secondary, #6b7280)' }}>
                <Download size={14} />
                <span>{service.downloads?.toLocaleString() || 0}</span>
              </div>
            </div>

            {/* 标签 */}
            {service.tags && service.tags.length > 0 && (
              <div className="flex gap-2 flex-wrap">
                {service.tags.map((tag) => (
                  <Tag key={tag} icon={<TagIcon size={12} />} className="text-xs">
                    {tag}
                  </Tag>
                ))}
              </div>
            )}
          </div>
        </div>
      </Card>

      {/* 标签页内容 */}
      <Card
        style={{
          boxShadow: '0 2px 8px rgba(0,0,0,0.06)',
          flex: 1,
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
        }}
        styles={{
          body: {
            padding: 0,
            height: '100%',
            display: 'flex',
            flexDirection: 'column',
          },
        }}
      >
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={tabItems}
          className="service-detail-tabs"
          tabBarStyle={{ padding: '16px 16px 0 16px', marginBottom: 0 }}
        />
      </Card>
    </div>
  )
}
