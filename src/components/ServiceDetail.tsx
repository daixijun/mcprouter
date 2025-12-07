import { open } from '@tauri-apps/plugin-shell'
import { App } from 'antd'
import { ArrowLeft } from 'lucide-react'
import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import ReactMarkdown from 'react-markdown'
import rehypeHighlight from 'rehype-highlight'
import rehypeRaw from 'rehype-raw'
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize'
import remarkGfm from 'remark-gfm'
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
  const { t } = useTranslation()
  const { message } = App.useApp()
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
      message.error(t('service.detail.error.open_link_failed'))
    }
  }

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      message.success(t('service.detail.success.copied_to_clipboard'))
    } catch (error) {
      console.error('Copy to clipboard failed:', error)
      message.error(t('service.detail.error.copy_failed'))
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
            {t('common.actions.back')}
          </button>
        </div>
        <div className='flex-1 flex flex-col items-center justify-center'>
          <div className='animate-spin rounded-full h-16 w-16 border-4 border-blue-500 border-t-transparent mb-4'></div>
          <p className='text-gray-600 text-lg'>{t('service.detail.loading')}</p>
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
            {t('common.actions.back')}
          </button>
        </div>
        <div className='flex-1 flex items-center justify-center'>
          <div className='text-center'>
            <div className='text-6xl mb-4'>😔</div>
            <h3 className='text-xl font-semibold text-gray-700 dark:text-gray-200 mb-2'>
              {t('service.detail.not_found')}
            </h3>
            <p className='text-gray-500 dark:text-gray-400'>
              {t('service.detail.not_found_description')}
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
          {t('common.actions.back')}
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
                  {t('service.detail.author')}: {service.author}
                  {service.license && ` • ${t('service.detail.license')}: ${service.license}`}
                </span>
                {service.is_verified && (
                  <span className='badge-modern bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300'>
                    ✅ {t('service.detail.verified')}
                  </span>
                )}
                {service.is_hosted && (
                  <span className='badge-modern bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300'>
                    🖥️ {t('service.detail.hosted')}
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
                label={t('service.detail.tabs.description')}
                isActive={activeTab === 'description'}
                onClick={() => setActiveTab('description')}
                isVisible={true}
              />
              <Tab
                label={t('service.detail.tabs.readme')}
                isActive={activeTab === 'readme'}
                onClick={() => setActiveTab('readme')}
                isVisible={!!service.readme}
              />
              <Tab
                label={t('service.detail.tabs.config')}
                isActive={activeTab === 'serverConfig'}
                onClick={() => setActiveTab('serverConfig')}
                isVisible={
                  !!service.server_config && service.server_config.length > 0
                }
              />
              <Tab
                label={t('service.detail.tabs.license')}
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
                  📂 {t('service.detail.buttons.repository')}
                </button>
              )}
              {service.homepage && (
                <button
                  onClick={() => handleOpenUrl(service.homepage!)}
                  className='btn-modern bg-blue-600 hover:bg-blue-700 text-white text-sm'>
                  🌐 {t('service.detail.buttons.homepage')}
                </button>
              )}
              {service.install_command && (
                <button
                  onClick={() => onInstall(service)}
                  className='btn-modern btn-primary-modern text-sm'>
                  ⬇️ {t('service.detail.buttons.install')}
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
                  📝 {t('service.detail.sections.description')}
                </h4>
                <p className='text-gray-600 dark:text-gray-300 leading-relaxed'>
                  {service.description}
                </p>
              </div>

              <div className='grid grid-cols-1 md:grid-cols-2 gap-6'>
                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    🏢 {t('service.detail.sections.platform')}
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
                        ✅ {t('service.detail.verified')}
                      </span>
                    )}
                    {service.is_hosted && (
                      <span className='badge-modern bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300'>
                        🖥️ {t('service.detail.hosted')}
                      </span>
                    )}
                  </div>
                </div>

                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    🔗 {t('service.detail.sections.transport')}
                  </h4>
                  <p className='text-gray-600 dark:text-gray-300'>
                    {service.transport}
                  </p>
                </div>

                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    ⭐ {t('service.detail.sections.github_stars')}
                  </h4>
                  <p className='text-gray-600 dark:text-gray-300'>
                    {typeof service.github_stars === 'number' &&
                    service.github_stars > 0
                      ? service.github_stars.toLocaleString()
                      : t('service.detail.no_data')}
                  </p>
                </div>

                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    📥 {t('service.detail.sections.downloads')}
                  </h4>
                  <p className='text-gray-600 dark:text-gray-300'>
                    {service.downloads.toLocaleString()}
                  </p>
                </div>

                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    👤 {t('service.detail.sections.author')}
                  </h4>
                  <p className='text-gray-600 dark:text-gray-300'>
                    {service.author}
                  </p>
                </div>
              </div>

              {service.requirements && service.requirements.length > 0 && (
                <div>
                  <h4 className='font-semibold text-gray-700 dark:text-gray-200 mb-3'>
                    🔧 {t('service.detail.sections.requirements')}
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
                    🏷️ {t('service.detail.sections.tags')}
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
                📖 {t('service.detail.sections.readme')}
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
                    ⚙️ {t('service.detail.sections.config')}
                  </h4>
                </div>
                <div className='space-y-3'>
                  {service.server_config.map((cfg, idx) => (
                    <div key={idx} className='relative'>
                      <button
                        onClick={() => copyText(JSON.stringify(cfg, null, 2))}
                        className='absolute top-2 right-2 btn-modern bg-gray-600 hover:bg-gray-700 text-white'>
                        {t('service.detail.buttons.copy')}
                      </button>
                      <pre className='bg-gray-100 dark:bg-gray-900 p-3 rounded-lg text-sm text-gray-800 dark:text-gray-200 overflow-x-auto'>
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
                📄 {t('service.detail.sections.license')}
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
