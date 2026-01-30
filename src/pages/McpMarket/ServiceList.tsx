import { useState, type KeyboardEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Card, Row, Col, Tag, Empty, Spin, Input, Tooltip } from 'antd'
import { Search, Download } from 'lucide-react'
import { useAppContext } from '../../contexts/AppContext'
import type { MarketService } from '../../types/mcp-market'

interface ServiceListProps {
  services: MarketService[]
  loading?: boolean
  onServiceClick?: (service: MarketService) => void
  onLoadMore: () => void
  onSearch: (query: string) => void
  hasMore: boolean
}

export default function ServiceList({
  services,
  loading = false,
  onServiceClick,
  onLoadMore,
  onSearch,
  hasMore,
}: ServiceListProps) {
  const { t } = useTranslation()
  const { openMarketService } = useAppContext()
  const [searchQuery, setSearchQuery] = useState('')
  const marketT = (key: string) => t(`about.market.${key}`)

  const handleSearch = (value: string) => {
    setSearchQuery(value)
    onSearch(value)
  }

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      onSearch(searchQuery)
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* Search bar */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700">
        <Input
          placeholder={marketT('search.placeholder')}
          className="w-full"
          prefix={<Search className="w-4 h-4 text-gray-500 dark:text-gray-400" />}
          value={searchQuery}
          onChange={(e) => handleSearch(e.target.value)}
          onKeyDown={handleKeyDown}
          allowClear
        />
      </div>

      {/* Service cards */}
      <div className="flex-1 overflow-auto p-4">
        <Spin spinning={loading}>
          {services.length === 0 ? (
            <Empty
              description={searchQuery ? marketT('search.no_results') : marketT('search.no_services')}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          ) : (
            <Row gutter={[16, 16]}>
              {services.map((service) => (
                <Col key={service.id} xs={24} sm={24} md={8} lg={8} xl={8} xxl={8}>
                  <Card
                    hoverable
                    onClick={() => openMarketService(service.id)}
                    className="h-full hover:border-blue-500 transition-colors service-card"
                    style={{ boxShadow: '0 2px 8px rgba(0,0,0,0.06)' }}
                  >
                    <div className="flex flex-col h-full">
                      {/* 顶部区域 */}
                      <div className="mb-3">
                        {/* 图标 + 标题 + 分类 */}
                        <div className="flex items-start gap-3 mb-2">
                          {/* Logo 图标 */}
                          {service.metadata?.logoUrl && (
                            <img
                              src={service.metadata.logoUrl}
                              alt={service.displayName}
                              className="w-12 h-12 rounded-lg object-cover flex-shrink-0"
                            />
                          )}

                          {/* 标题和分类 - 分两行 */}
                          <div className="flex-1 min-w-0">
                            {/* 第一行：标题（带截断和tooltip） */}
                            <Tooltip title={service.displayName} placement="topLeft">
                              <div
                                className="font-semibold text-base mb-2 leading-tight truncate"
                                style={{ color: 'var(--card-title-color, #1f2937)' }}
                              >
                                {service.displayName}
                              </div>
                            </Tooltip>

                            {/* 第二行：分类 */}
                            {service.category && (
                              <Tag color="blue" className="text-xs">
                                {service.category}
                              </Tag>
                            )}
                          </div>
                        </div>

                        {/* 作者 - 独立一行 */}
                        {service.author && (
                          <div
                            className="text-xs"
                            style={{ color: 'var(--card-text-secondary, #6b7280)' }}
                          >
                            {service.author}
                          </div>
                        )}
                      </div>

                      {/* 中间描述区域 - 固定两行空间 */}
                      <div className="flex-1 mb-3">
                        <Tooltip title={service.description || '暂无描述'} placement="topLeft">
                          <p
                            className="text-sm line-clamp-2"
                            style={{
                              color: 'var(--card-text-color, #4b5563)',
                              lineHeight: '1.5',
                              minHeight: '3rem',
                              maxHeight: '3rem',
                              overflow: 'hidden',
                              wordBreak: 'break-word',
                            }}
                          >
                            {service.description || '暂无描述'}
                          </p>
                        </Tooltip>
                      </div>

                      {/* 底部统计和标签 - 始终在底部 */}
                      <div className="flex items-center justify-between text-sm">
                        <div
                          className="flex items-center gap-2"
                          style={{ color: 'var(--card-text-secondary, #6b7280)', fontSize: '13px' }}
                        >
                          <Download size={14} />
                          <span>{service.downloads || 0}</span>
                        </div>
                        {service.tags && service.tags.length > 0 && (
                          <div className="flex gap-1 flex-wrap justify-end">
                            {service.tags.slice(0, 2).map((tag) => (
                              <Tag key={tag} className="text-xs m-0" style={{ margin: '0 0 0 4px' }}>
                                {tag.length > 8 ? `${tag.slice(0, 8)}...` : tag}
                              </Tag>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  </Card>
                </Col>
              ))}
            </Row>
          )}
        </Spin>
      </div>

      {/* Load more button */}
      {hasMore && services.length > 0 && !loading && (
        <div className="p-4 text-center border-t border-gray-200 dark:border-gray-700">
          <button
            onClick={onLoadMore}
            className="text-blue-600 hover:text-blue-700 font-medium"
          >
            {marketT('load_more')}
          </button>
        </div>
      )}
    </div>
  )
}
