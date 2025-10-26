import { open } from '@tauri-apps/plugin-shell'
import { ArrowLeft } from 'lucide-react'
import React, { useState } from 'react'
import ReactMarkdown from 'react-markdown'
import rehypeHighlight from 'rehype-highlight'
import rehypeRaw from 'rehype-raw'
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize'
import remarkGfm from 'remark-gfm'
import toastService from '../services/toastService'
import type { MarketplaceService } from '../types'

// Simple tab component
interface TabProps {
  label: string
  isActive: boolean
  onClick: () => void
  isVisible: boolean
}

const Tab: React.FC<TabProps> = ({ label, isActive, onClick, isVisible }) => {
  if (!isVisible) return null

  return (
    <button
      onClick={onClick}
      className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
        isActive
          ? 'border-blue-500 text-blue-600 dark:text-blue-400'
          : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300'
      }`}>
      {label}
    </button>
  )
}

interface ServiceDetailProps {
  service: MarketplaceService | null
  loading: boolean
  onBack: () => void
  onInstall: (service: MarketplaceService) => void
}

const ServiceDetail: React.FC<ServiceDetailProps> = ({
  service,
  loading,
  onBack,
  onInstall,
}) => {
  const [activeTab, setActiveTab] = useState('description')

  // 允许 README 渲染 code/pre，并保留类名用于高亮
  const markdownSanitizeSchema = {
    ...defaultSchema,
    tagNames: [...(defaultSchema.tagNames || []), 'code', 'pre'],
    attributes: {
      ...defaultSchema.attributes,
      code: [...(defaultSchema.attributes?.code || []), ['className']],
      pre: [...(defaultSchema.attributes?.pre || []), ['className']],
      span: [...(defaultSchema.attributes?.span || []), ['className']],
      div: [...(defaultSchema.attributes?.div || []), ['className']],
    },
  }

  const handleOpenUrl = async (url: string) => {
    try {
      await open(url)
    } catch (error) {
      console.error('Failed to open URL:', error)
      toastService.sendErrorNotification(
        '打开链接失败，请检查系统默认浏览器设置。',
      )
    }
  }

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      toastService.sendSuccessNotification('已复制到剪贴板')
    } catch (error) {
      console.error('Copy to clipboard failed:', error)
      toastService.sendErrorNotification('复制失败，请稍后重试')
    }
  }

  const getPlatformBadgeColor = (platform: string) => {
    switch (platform) {
      case '魔搭社区':
        return 'bg-rose-100 text-rose-800'
      default:
        return 'bg-gray-100 dark:bg-gray-700 text-gray-800'
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

  if (loading) {
    return (
      <div className='h-full flex flex-col'>
        <div className='flex items-center mb-6'>
          <button
            onClick={onBack}
            className='btn-modern bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 flex items-center gap-2'>
            <ArrowLeft size={16} />
            返回
          </button>
        </div>
        <div className='flex-1 flex flex-col items-center justify-center'>
          <div className='animate-spin rounded-full h-16 w-16 border-4 border-blue-500 border-t-transparent mb-4'></div>
          <p className='text-gray-600 text-lg'>正在加载服务详情...</p>
        </div>
      </div>
    )
  }

  if (!service) {
    return (
      <div className='h-full flex flex-col'>
        <div className='flex items-center mb-6'>
          <button
            onClick={onBack}
            className='btn-modern bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 flex items-center gap-2'>
            <ArrowLeft size={16} />
            返回
          </button>
        </div>
        <div className='flex-1 flex items-center justify-center'>
          <div className='text-center'>
            <div className='text-6xl mb-4'>😔</div>
            <h3 className='text-xl font-semibold text-gray-700 dark:text-gray-200 mb-2'>
              服务不存在
            </h3>
            <p className='text-gray-500 dark:text-gray-400'>
              未找到服务详情信息
            </p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className='h-full flex flex-col overflow-hidden'>
      {/* Back button */}
      <div className='flex items-center mb-6'>
        <button
          onClick={onBack}
          className='btn-modern bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 flex items-center gap-2'>
          <ArrowLeft size={16} />
          返回
        </button>
      </div>

      {/* Service header */}
      <div className='card-glass p-6 mb-6'>
        <div className='flex justify-between items-start'>
          <div className='flex gap-4 items-start flex-1'>
            {/* Logo */}
            <div className='flex-shrink-0 w-20 h-20 rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-700 flex items-center justify-center'>
              {service.logo_url ? (
                <img
                  src={service.logo_url}
                  alt={service.name}
                  className='w-full h-full object-cover'
                />
              ) : (
                <span className='text-4xl'>📦</span>
              )}
            </div>
            <div className='flex-1'>
              <h3 className='text-2xl font-bold text-gray-800 dark:text-gray-100'>
                {service.name}
              </h3>
              <p className='text-gray-600 dark:text-gray-300 mt-1 flex flex-wrap items-center gap-2'>
                <span>
                  作者：{service.author}
                  {service.license && ` • 许可证：${service.license}`}
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
              </p>
              {/* Install command in header */}
              {service.install_command && (
                <div className='mt-3'>
                  <div className='bg-gray-100 dark:bg-gray-700 px-3 py-2 rounded text-sm font-mono text-gray-700 dark:text-gray-200 border-l-4 border-blue-500'>
                    {service.install_command.command}{' '}
                    {service.install_command.args.join(' ')}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Tab Navigation and Content - merged */}
      <div className='card-glass flex-1 overflow-hidden flex flex-col'>
        <div className='border-b border-gray-200'>
          <nav className='flex justify-between items-center px-6'>
            <div className='flex space-x-8'>
              <Tab
                label='描述'
                isActive={activeTab === 'description'}
                onClick={() => setActiveTab('description')}
                isVisible={true}
              />
              <Tab
                label='README'
                isActive={activeTab === 'readme'}
                onClick={() => setActiveTab('readme')}
                isVisible={!!service.readme}
              />
              <Tab
                label='配置'
                isActive={activeTab === 'serverConfig'}
                onClick={() => setActiveTab('serverConfig')}
                isVisible={
                  !!service.server_config && service.server_config.length > 0
                }
              />
              <Tab
                label='许可证'
                isActive={activeTab === 'license'}
                onClick={() => setActiveTab('license')}
                isVisible={!!service.license}
              />
            </div>
            {/* Action buttons on the right */}
            <div className='flex items-center gap-2'>
              {service.repository && (
                <button
                  onClick={() => handleOpenUrl(service.repository!)}
                  className='btn-modern bg-gray-600 hover:bg-gray-700 text-white text-sm'>
                  📂 代码仓库
                </button>
              )}
              {service.homepage && (
                <button
                  onClick={() => handleOpenUrl(service.homepage!)}
                  className='btn-modern bg-blue-600 hover:bg-blue-700 text-white text-sm'>
                  🌐 主页
                </button>
              )}
              {service.install_command && (
                <button
                  onClick={() => onInstall(service)}
                  className='btn-modern btn-primary-modern text-sm'>
                  ⬇️ 安装服务
                </button>
              )}
            </div>
          </nav>
        </div>

        {/* Tab Content - no extra spacing */}
        <div className='flex-1 overflow-y-auto p-6'>
          {/* Description Tab */}
          {activeTab === 'description' && (
            <div className='space-y-6'>
              <div>
                <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                  📝 描述
                </h4>
                <p className='text-gray-600 dark:text-gray-300 leading-relaxed'>
                  {service.description}
                </p>
              </div>

              <div className='grid grid-cols-1 md:grid-cols-2 gap-6'>
                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    🏢 平台
                  </h4>
                  <div className='flex flex-wrap items-center gap-2'>
                    <span
                      className={`badge-modern ${getPlatformBadgeColor(
                        service.platform,
                      )}`}>
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
                </div>

                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    🔗 传输协议
                  </h4>
                  <p className='text-gray-600 dark:text-gray-300'>
                    {service.transport}
                  </p>
                </div>

                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    ⭐ GitHub Stars
                  </h4>
                  <p className='text-gray-600 dark:text-gray-300'>
                    {typeof service.github_stars === 'number' &&
                    service.github_stars > 0
                      ? service.github_stars.toLocaleString()
                      : '暂无数据'}
                  </p>
                </div>

                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    📥 下载量
                  </h4>
                  <p className='text-gray-600 dark:text-gray-300'>
                    {service.downloads.toLocaleString()}
                  </p>
                </div>

                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    👤 作者
                  </h4>
                  <p className='text-gray-600 dark:text-gray-300'>
                    {service.author}
                  </p>
                </div>
              </div>

              {service.requirements && service.requirements.length > 0 && (
                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    🔧 要求
                  </h4>
                  <ul className='space-y-2'>
                    {service.requirements.map((req, index) => (
                      <li
                        key={index}
                        className='flex items-center text-gray-600 dark:text-gray-300'>
                        <span className='mr-2'>✅</span>
                        {req}
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              {service.tags && service.tags.length > 0 && (
                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    🏷️ 标签
                  </h4>
                  <div className='flex flex-wrap gap-2'>
                    {service.tags.map((tag, index) => (
                      <span
                        key={index}
                        className='badge-modern bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-100'>
                        #{tag}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Readme Tab */}
          {activeTab === 'readme' && service.readme && (
            <div>
              <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                📖 Readme
              </h4>
              <div className='bg-gray-50 dark:bg-gray-800 p-4 rounded-lg text-gray-700 dark:text-gray-200 markdown-content prose prose-sm max-w-none dark:prose-invert'>
                <ReactMarkdown
                  remarkPlugins={[remarkGfm]}
                  rehypePlugins={[
                    rehypeRaw,
                    [rehypeSanitize, markdownSanitizeSchema],
                    rehypeHighlight,
                  ]}>
                  {service.readme}
                </ReactMarkdown>
              </div>
            </div>
          )}

          {/* ServerConfig Tab */}
          {activeTab === 'serverConfig' &&
            service.server_config &&
            service.server_config.length > 0 && (
              <div>
                <div className='flex items-center justify-between mb-3'>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200'>
                    ⚙️ 配置
                  </h4>
                </div>
                <div className='space-y-3'>
                  {service.server_config.map((cfg, idx) => (
                    <div key={idx} className='relative'>
                      <button
                        onClick={() => copyText(JSON.stringify(cfg, null, 2))}
                        className='absolute top-2 right-2 btn-modern bg-gray-600 hover:bg-gray-700 text-white'>
                        复制
                      </button>
                      <pre className='bg-gray-100 dark:bg-gray-800 p-3 rounded-lg text-sm text-gray-800 dark:text-gray-200 overflow-x-auto'>
                        {JSON.stringify(cfg, null, 2)}
                      </pre>
                    </div>
                  ))}
                </div>
              </div>
            )}

          {/* License Tab */}
          {activeTab === 'license' && service.license && (
            <div>
              <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                📄 许可证
              </h4>
              <div className='bg-gray-50 dark:bg-gray-800 p-4 rounded-lg text-gray-700 dark:text-gray-200 markdown-content prose prose-sm max-w-none dark:prose-invert'>
                {service.license}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export default ServiceDetail
