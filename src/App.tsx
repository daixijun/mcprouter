import { invoke } from '@tauri-apps/api/core'
import { App as AntdApp, ConfigProvider } from 'antd'
import {
  Info,
  Menu,
  Monitor,
  Moon,
  Package,
  Server,
  Settings as SettingsIcon,
  Sun,
  X,
} from 'lucide-react'
import { memo } from 'react'
import './App.css'
import AboutModal from './components/AboutModal'
import { ErrorBoundary } from './components/ErrorBoundary'
import { AppProvider, useAppContext } from './contexts/AppContext'
import { ErrorProvider } from './contexts/ErrorContext'
import Dashboard from './pages/Dashboard'
import Marketplace from './pages/Marketplace'
import McpServerManager from './pages/McpServerManager'
import Settings from './pages/Settings'
import { getThemeConfig } from './theme/antd-config'

// 内部应用组件，使用 Context
const AppContent = memo(() => {
  const { state, setThemeMode, setActiveTab, toggleMenu, toggleAbout } =
    useAppContext()
  const { message } = AntdApp.useApp()

  // Handle theme change
  const handleThemeChange = async (mode: 'light' | 'dark' | 'auto') => {
    setThemeMode(mode)
    try {
      await invoke('set_theme', { theme: mode })
    } catch (error) {
      console.error('Failed to save theme:', error)
      message.error('保存主题设置失败')
    }
  }

  const tabs = [
    { id: 'overview', label: '概览', icon: Info },
    { id: 'servers', label: '服务管理', icon: Server },
    { id: 'market', label: 'MCP广场', icon: Package },
    { id: 'settings', label: '设置', icon: SettingsIcon },
  ]

  return (
    <ConfigProvider theme={getThemeConfig(state.isDarkMode)}>
      <AntdApp>
        <div
          className={`h-screen overflow-hidden ${
            state.isDarkMode
              ? 'dark bg-gray-900'
              : 'bg-gradient-to-br from-blue-50 via-white to-indigo-50'
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
                      <h1 className='text-lg font-bold bg-gradient-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent'>
                        MCP Router
                      </h1>
                      <p className='text-xs text-gray-600 '>
                        现代化 MCP 聚合管理工具
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
                            className={`flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.activeTab === tab.id
                                ? 'bg-white text-blue-600 shadow-md dark:bg-gray-700 '
                                : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white'
                            }`}>
                            <Icon size={16} />
                            <span>{tab.label}</span>
                          </button>
                        )
                      })}
                    </nav>
                    {/* Theme Switcher */}
                    <div className='flex items-center space-x-0.5 bg-gray-100/80 dark:bg-gray-800/90 rounded-lg p-0.5 border border-gray-200 dark:border-gray-700'>
                      <button
                        onClick={() => handleThemeChange('auto')}
                        className={`p-1.5 rounded-md transition-all duration-200 ${
                          state.themeMode === 'auto'
                            ? 'bg-white dark:bg-gray-700 text-blue-600  shadow-md border border-blue-200 dark:border-blue-500/30'
                            : 'text-gray-500 hover:text-gray-700 hover:bg-white/60  dark:hover:text-gray-200 dark:hover:bg-gray-700/50'
                        }`}
                        title='自动（跟随系统）'>
                        <Monitor size={16} />
                      </button>
                      <button
                        onClick={() => handleThemeChange('light')}
                        className={`p-1.5 rounded-md transition-all duration-200 ${
                          state.themeMode === 'light'
                            ? 'bg-white dark:bg-gray-700 text-amber-600 dark:text-amber-400 shadow-md border border-amber-200 dark:border-amber-500/30'
                            : 'text-gray-500 hover:text-gray-700 hover:bg-white/60  dark:hover:text-gray-200 dark:hover:bg-gray-700/50'
                        }`}
                        title='亮色模式'>
                        <Sun size={16} />
                      </button>
                      <button
                        onClick={() => handleThemeChange('dark')}
                        className={`p-1.5 rounded-md transition-all duration-200 ${
                          state.themeMode === 'dark'
                            ? 'bg-white dark:bg-gray-700 text-indigo-600 dark:text-indigo-400 shadow-md border border-indigo-200 dark:border-indigo-500/30'
                            : 'text-gray-500 hover:text-gray-700 hover:bg-white/60  dark:hover:text-gray-200 dark:hover:bg-gray-700/50'
                        }`}
                        title='暗色模式'>
                        <Moon size={16} />
                      </button>
                    </div>
                  </div>
                  {/* Mobile menu button */}
                  <button
                    onClick={toggleMenu}
                    className='md:hidden p-2 rounded-lg text-gray-700 hover:bg-gray-100 hover:text-gray-900  dark:hover:bg-gray-800'>
                    {state.isMenuOpen ? <X size={20} /> : <Menu size={20} />}
                  </button>
                </div>
                {/* Mobile Navigation */}
                {state.isMenuOpen && (
                  <div className='md:hidden py-2 border-t border-white/20'>
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
                            className={`flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.activeTab === tab.id
                                ? 'bg-white text-blue-600 shadow-md dark:bg-gray-700 '
                                : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white'
                            }`}>
                            <Icon size={16} />
                            <span>{tab.label}</span>
                          </button>
                        )
                      })}
                      {/* Mobile Theme Switcher */}
                      <div className='pt-2 border-t border-gray-200/50 dark:border-white/20 mt-2'>
                        <div className='px-3 py-1 text-xs text-gray-500 '>
                          主题
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
                                : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white'
                            }`}>
                            <Monitor size={16} />
                            <span>自动</span>
                          </button>
                          <button
                            onClick={() => {
                              handleThemeChange('light')
                              toggleMenu()
                            }}
                            className={`flex-1 flex items-center justify-center space-x-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.themeMode === 'light'
                                ? 'bg-white text-amber-600 shadow-md border border-amber-200 dark:bg-gray-700 dark:text-amber-400 dark:border-amber-500/30'
                                : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white'
                            }`}>
                            <Sun size={16} />
                            <span>亮色</span>
                          </button>
                          <button
                            onClick={() => {
                              handleThemeChange('dark')
                              toggleMenu()
                            }}
                            className={`flex-1 flex items-center justify-center space-x-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                              state.themeMode === 'dark'
                                ? 'bg-white text-indigo-600 shadow-md border border-indigo-200 dark:bg-gray-700 dark:text-indigo-400 dark:border-indigo-500/30'
                                : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white'
                            }`}>
                            <Moon size={16} />
                            <span>暗色</span>
                          </button>
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
                  关于
                </button>
              </p>
            </footer>
          </div>
          {/* About Modal */}
          <AboutModal isOpen={state.isAboutOpen} onClose={toggleAbout} />
        </div>
      </AntdApp>
    </ConfigProvider>
  )
})

// 主 App 组件，包装所有 Provider
const App: React.FC = () => {
  return (
    <ErrorBoundary>
      <ErrorProvider>
        <AppProvider>
          <AppContent />
        </AppProvider>
      </ErrorProvider>
    </ErrorBoundary>
  )
}

export default App
