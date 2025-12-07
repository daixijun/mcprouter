import { invoke } from '@tauri-apps/api/core'
import { App as AntdApp } from 'antd'
import {
  Activity,
  Info,
  Key,
  Menu,
  Moon,
  Package,
  Server,
  Settings as SettingsIcon,
  Sun,
  X,
} from 'lucide-react'
import { memo } from 'react'
import { useTranslation } from 'react-i18next'
import './App.css'
import AboutModal from './components/AboutModal'
import AntdConfigProvider from './components/AntdConfigProvider'
import LanguageSelector from './components/LanguageSelector'
import { AppProvider, useAppContext } from './contexts/AppContext'
import Dashboard from './pages/Dashboard'
import Marketplace from './pages/Marketplace'
import McpServerManager from './pages/McpServerManager'
import Settings from './pages/Settings'
import TokenManagement from './pages/TokenManagement'

// 内部应用组件，使用 Context
const AppContent = memo(() => {
  const { state, setThemeMode, setActiveTab, toggleMenu, toggleAbout } =
    useAppContext()
  const { message } = AntdApp.useApp()
  const { t } = useTranslation()

  // Handle theme change
  const handleThemeChange = async (mode: 'light' | 'dark' | 'auto') => {
    setThemeMode(mode)
    try {
      await invoke('set_theme', { theme: mode })
    } catch (error) {
      console.error('Failed to save theme:', error)
      message.error(t('common.error.save_theme_failed'))
    }
  }

  const tabs = [
    { id: 'overview', label: t('nav.overview'), icon: Activity },
    { id: 'servers', label: t('nav.server_management'), icon: Server },
    { id: 'market', label: t('nav.marketplace'), icon: Package },
    { id: 'tokens', label: t('nav.token_management'), icon: Key },
    { id: 'settings', label: t('nav.settings'), icon: SettingsIcon },
  ]

  return (
    <AntdConfigProvider>
      <AntdApp>
        <div
          className={`h-screen overflow-hidden ${
            state.isDarkMode
              ? 'dark bg-gray-900'
              : 'bg-linear-to-br from-blue-50 via-white to-indigo-50'
          }`}>
          <div className='h-full flex flex-col'>
            {/* Header */}
            <header className='nav-glass sticky top-0 z-50'>
              <div
                className='w-full px-3 sm:px-4 lg:px-6'
                style={{ maxWidth: '1600px', margin: '0 auto' }}>
                {' '}
                {/* 减小内边距 */}
                <div className='flex items-center justify-between h-14'>
                  {' '}
                  {/* 减小头部高度 */}
                  {/* Logo */}
                  <div className='flex items-center space-x-3'>
                    <div className='w-8 h-8 flex items-center justify-center'>
                      <img
                        src='/favicon.png'
                        alt='MCP Router Logo'
                        className='w-full h-full object-contain'
                      />
                    </div>
                    <div>
                      <h1 className='text-lg font-bold bg-linear-to-r from-primary-600 to-indigo-600 bg-clip-text text-transparent'>
                        MCP Router
                      </h1>
                      <p className='text-xs text-gray-600 dark:text-gray-400'>
                        {t('dashboard.app.subtitle')}
                      </p>
                    </div>
                  </div>
                  {/* Desktop Navigation */}
                  <div className='hidden md:flex items-center space-x-2'>
                    <nav className='flex items-center space-x-1'>
                      {tabs.map((tab) => {
                        const Icon = tab.icon
                        return (
                          <button
                            key={tab.id}
                            onClick={() => setActiveTab(tab.id as any)}
                            className={`nav-link flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.activeTab === tab.id
                                ? 'active bg-gray-100 text-gray-900 shadow-sm border border-gray-200 dark:bg-gray-400 dark:text-white dark:border-blue-500/30'
                                : 'text-gray-900 hover:text-gray-900 hover:bg-gray-100 dark:text-gray-500 dark:hover:text-white dark:hover:bg-gray-400'
                            }`}>
                            <Icon size={16} />
                            <span>{tab.label}</span>
                          </button>
                        )
                      })}
                    </nav>
                    {/* Theme Switcher */}
                    <div
                    className='flex items-center space-x-0.5 rounded-lg p-0.5 border border-gray-200 dark:border-gray-700'
                    style={{
                      backgroundColor: 'var(--color-bg-secondary)',
                      border: '1px solid var(--color-border)'
                    }}
                  >
                      <button
                        onClick={() => handleThemeChange('auto')}
                        className={`p-1.5 rounded-md transition-all duration-200 ${
                          state.themeMode === 'auto'
                            ? 'bg-gray-50 dark:bg-gray-700 text-blue-600 dark:text-blue-400 shadow-sm border border-blue-200 dark:border-blue-500/30'
                            : 'text-gray-700 hover:text-blue-600 dark:text-gray-300 dark:hover:text-white dark:hover:bg-gray-800/50'
                        }`}
                        title={t('dashboard.theme.auto')}>
                        <Info size={16} />
                      </button>
                      <button
                        onClick={() => handleThemeChange('light')}
                        className={`p-1.5 rounded-md transition-all duration-200 ${
                          state.themeMode === 'light'
                            ? 'bg-gray-50 dark:bg-gray-700 text-amber-600 dark:text-amber-400 shadow-sm border border-amber-200 dark:border-amber-500/30'
                            : 'text-gray-700 hover:text-amber-600 dark:text-gray-300 dark:hover:text-white dark:hover:bg-gray-800/50'
                        }`}
                        title={t('dashboard.theme.light')}>
                        <Sun size={16} />
                      </button>
                      <button
                        onClick={() => handleThemeChange('dark')}
                        className={`p-1.5 rounded-md transition-all duration-200 ${
                          state.themeMode === 'dark'
                            ? 'bg-gray-50 dark:bg-gray-700 text-indigo-600 dark:text-indigo-400 shadow-sm border border-indigo-200 dark:border-indigo-500/30'
                            : 'text-gray-700 hover:text-indigo-600 dark:text-gray-300 dark:hover:text-white dark:hover:bg-gray-800/50'
                        }`}
                        title={t('dashboard.theme.dark')}>
                        <Moon size={16} />
                      </button>
                    </div>
                    {/* Language Selector */}
                    <div className='ml-3'>
                      <LanguageSelector size='small' />
                    </div>
                  </div>
                  {/* Mobile menu button */}
                  <button
                    onClick={toggleMenu}
                    className='md:hidden p-2 rounded-lg text-gray-700 hover:bg-gray-200 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800/50 dark:hover:text-white'>
                    {state.isMenuOpen ? <X size={20} /> : <Menu size={20} />}
                  </button>
                </div>
                {/* Mobile Navigation */}
                {state.isMenuOpen && (
                  <div className='md:hidden py-2 border-t border border-solid'>
                    <nav className='flex flex-col space-y-1'>
                      {tabs.map((tab) => {
                        const Icon = tab.icon
                        return (
                          <button
                            key={tab.id}
                            onClick={() => {
                              setActiveTab(tab.id as any)
                              toggleMenu()
                            }}
                            className={`nav-link flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.activeTab === tab.id
                                ? 'active bg-gray-100 text-gray-900 shadow-md border border-gray-200 dark:bg-gray-700/60 dark:text-blue-400 dark:border dark:border-blue-500/30'
                                : 'text-gray-900 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800/50 dark:hover:text-white'
                            }`}>
                            <Icon size={16} />
                            <span>{tab.label}</span>
                          </button>
                        )
                      })}
                      {/* Mobile Theme Switcher */}
                      <div className='pt-2 border-t border-gray-200/50 dark:border border-solid mt-2'>
                        <div className='px-3 py-1 text-xs text-gray-500 '>
                          {t('dashboard.theme.title')}
                        </div>
                        <div className='flex items-center space-x-1 mt-1'>
                          <button
                            onClick={() => {
                              handleThemeChange('auto')
                              toggleMenu()
                            }}
                            className={`flex-1 flex items-center justify-center space-x-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.themeMode === 'auto'
                                ? 'bg-white text-blue-600 shadow-md border border-blue-200 dark:bg-gray-700  dark:border-blue-500/30'
                                : 'text-gray-700 hover:bg-gray-200 hover:text-gray-900 dark:text-gray-200 dark:hover:bg-gray-800 dark:hover:text-white'
                            }`}>
                            <Info size={16} />
                            <span>{t('dashboard.theme.auto')}</span>
                          </button>
                          <button
                            onClick={() => {
                              handleThemeChange('light')
                              toggleMenu()
                            }}
                            className={`flex-1 flex items-center justify-center space-x-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.themeMode === 'light'
                                ? 'bg-white text-amber-600 shadow-md border border-amber-200 dark:bg-gray-700 dark:text-amber-400 dark:border-amber-500/30'
                                : 'text-gray-700 hover:bg-gray-200 hover:text-gray-900 dark:text-gray-200 dark:hover:bg-gray-800 dark:hover:text-white'
                            }`}>
                            <Sun size={16} />
                            <span>{t('dashboard.theme.light')}</span>
                          </button>
                          <button
                            onClick={() => {
                              handleThemeChange('dark')
                              toggleMenu()
                            }}
                            className={`flex-1 flex items-center justify-center space-x-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.themeMode === 'dark'
                                ? 'bg-white text-indigo-600 shadow-md border border-indigo-200 dark:bg-gray-700 dark:text-indigo-400 dark:border-indigo-500/30'
                                : 'text-gray-700 hover:bg-gray-200 hover:text-gray-900 dark:text-gray-200 dark:hover:bg-gray-800 dark:hover:text-white'
                            }`}>
                            <Moon size={16} />
                            <span>{t('dashboard.theme.dark')}</span>
                          </button>
                        </div>
                        {/* Mobile Language Selector */}
                        <div className='pt-2 border-t border-gray-200/50 dark:border border-solid mt-2'>
                          <div className='px-3 py-1 text-xs text-gray-500 '>
                            {t('dashboard.language.title')}
                          </div>
                          <div className='px-3 py-2'>
                            <LanguageSelector size='small' />
                          </div>
                        </div>
                      </div>
                    </nav>
                  </div>
                )}
              </div>
            </header>

            {/* Main Content */}
            <main className='flex-1 min-h-0'>
              <div
                className='h-full w-full px-3 sm:px-4 lg:px-6 py-4'
                style={{ maxWidth: '1600px', margin: '0 auto' }}>
                <div
                  className={`h-full transition-opacity duration-300 ease-in-out ${
                    state.isTransitioning ? 'opacity-0' : 'opacity-100'
                  }`}>
                  {state.activeTab === 'overview' && <Dashboard />}
                  {state.activeTab === 'servers' && <McpServerManager />}
                  {state.activeTab === 'market' && <Marketplace />}
                  {state.activeTab === 'tokens' && <TokenManagement />}
                  {state.activeTab === 'settings' && <Settings />}
                </div>
              </div>
            </main>

            {/* Footer */}
            <footer className='py-3 text-center text-xs text-gray-500  border-t border-gray-200 dark:border-gray-700'>
              {' '}
              {/* 减小内边距和字体大小 */}
              <p>
                &copy; 2025 MCP Router. All rights reserved.
                <button
                  type='button'
                  onClick={toggleAbout}
                  className='ml-3 text-blue-600 hover:text-blue-700 '>
                  {t('dashboard.footer.about')}
                </button>
              </p>
            </footer>
          </div>
          {/* About Modal */}
          <AboutModal isOpen={state.isAboutOpen} onClose={toggleAbout} />
        </div>
      </AntdApp>
    </AntdConfigProvider>
  )
})

// 主 App 组件，包装所有 Provider
const App: React.FC = () => {
  return (
    <AppProvider>
      <AppContent />
    </AppProvider>
  )
}

export default App
