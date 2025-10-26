import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import {
  Info,
  Key,
  Menu,
  Monitor,
  Moon,
  Package,
  Server,
  Settings as SettingsIcon,
  Sun,
  X,
} from 'lucide-react'
import { useEffect, useState } from 'react'
import './App.css'
import AboutModal from './components/AboutModal'
import ApiKeys from './pages/ApiKeys'
import Dashboard from './pages/Dashboard'
import Marketplace from './pages/Marketplace'
import McpServerManager from './pages/McpServerManager'
import Settings from './pages/Settings'

function App() {
  const [activeTab, setActiveTab] = useState<
    'overview' | 'servers' | 'market' | 'apikeys' | 'settings'
  >('overview')
  const [isMenuOpen, setIsMenuOpen] = useState(false)
  const [themeMode, setThemeMode] = useState<'light' | 'dark' | 'auto'>('auto')
  const [isDarkMode, setIsDarkMode] = useState(false)
  const [isTransitioning, setIsTransitioning] = useState(false)
  const [isAboutOpen, setIsAboutOpen] = useState(false)

  // Theme management
  useEffect(() => {
    // Load saved theme from backend
    const loadTheme = async () => {
      try {
        const savedTheme = await invoke<string>('get_theme')
        setThemeMode(savedTheme as 'light' | 'dark' | 'auto')
      } catch (error) {
        console.error('Failed to load theme:', error)
      }
    }
    loadTheme()

    // Listen for theme changes from tray menu
    const setupThemeListener = async () => {
      try {
        const unlistenTheme = await listen<string>('theme-changed', (event) => {
          const newTheme = event.payload as 'light' | 'dark' | 'auto'
          setThemeMode(newTheme)
        })
        return unlistenTheme
      } catch (error) {
        console.error('Failed to setup theme listener:', error)
      }
    }

    const themeCleanup = setupThemeListener()

    return () => {
      themeCleanup.then((cleanup) => cleanup && cleanup())
    }
  }, [])

  // Apply theme based on themeMode
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

    const applyTheme = () => {
      if (themeMode === 'auto') {
        setIsDarkMode(mediaQuery.matches)
      } else {
        setIsDarkMode(themeMode === 'dark')
      }
    }

    applyTheme()

    const handleSystemThemeChange = (e: MediaQueryListEvent) => {
      if (themeMode === 'auto') {
        setIsDarkMode(e.matches)
      }
    }

    if (themeMode === 'auto') {
      mediaQuery.addEventListener('change', handleSystemThemeChange)
    }

    return () => {
      mediaQuery.removeEventListener('change', handleSystemThemeChange)
    }
  }, [themeMode])

  // Listen for tray navigation events
  useEffect(() => {
    const setupTrayListeners = async () => {
      try {
        // Listen for navigation events
        const unlistenNavigate = await listen<string>(
          'navigate-to',
          (event) => {
            const targetTab = event.payload
            if (targetTab === 'servers') {
              handleTabChange('servers')
            } else if (targetTab === 'marketplace') {
              handleTabChange('market')
            } else if (targetTab === 'settings') {
              handleTabChange('settings')
            }
          },
        )

        // Listen for about dialog event
        const unlistenAbout = await listen('show-about-dialog', () => {
          // Open about modal
          setIsAboutOpen(true)
        })

        return () => {
          unlistenNavigate()
          unlistenAbout()
        }
      } catch (error) {
        console.error('Failed to setup tray listeners:', error)
      }
    }

    setupTrayListeners()
  }, [])

  // Handle theme change
  const handleThemeChange = async (mode: 'light' | 'dark' | 'auto') => {
    setThemeMode(mode)
    try {
      await invoke('set_theme', { theme: mode })
    } catch (error) {
      console.error('Failed to save theme:', error)
    }
  }

  const tabs = [
    { id: 'overview', label: '概览', icon: Info },
    { id: 'servers', label: '服务管理', icon: Server },
    { id: 'market', label: 'MCP广场', icon: Package },
    { id: 'apikeys', label: 'API Keys', icon: Key },
    { id: 'settings', label: '设置', icon: SettingsIcon },
  ]

  // Handle tab change with transition
  const handleTabChange = (
    tabId: 'overview' | 'servers' | 'market' | 'apikeys' | 'settings',
  ) => {
    if (tabId === activeTab) return

    setIsTransitioning(true)
    setTimeout(() => {
      setActiveTab(tabId)
      setIsTransitioning(false)
    }, 150)
  }

  return (
    <div
      className={`h-screen overflow-hidden ${
        isDarkMode
          ? 'dark bg-gray-900'
          : 'bg-gradient-to-br from-blue-50 via-white to-indigo-50'
      }`}>
      <div className='h-full flex flex-col'>
        {/* Header */}
        <header className='nav-glass sticky top-0 z-50'>
          <div className='max-w-7xl mx-auto px-3 sm:px-4 lg:px-6'>
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
                  <p className='text-xs text-gray-600 dark:text-gray-400'>
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
                        onClick={() => handleTabChange(tab.id as any)}
                        className={`flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                          activeTab === tab.id
                            ? 'bg-white text-blue-600 shadow-md dark:bg-gray-700 dark:text-blue-400'
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
                      themeMode === 'auto'
                        ? 'bg-white dark:bg-gray-700 text-blue-600 dark:text-blue-400 shadow-md border border-blue-200 dark:border-blue-500/30'
                        : 'text-gray-500 hover:text-gray-700 hover:bg-white/60 dark:text-gray-400 dark:hover:text-gray-200 dark:hover:bg-gray-700/50'
                    }`}
                    title='自动（跟随系统）'>
                    <Monitor size={16} />
                  </button>
                  <button
                    onClick={() => handleThemeChange('light')}
                    className={`p-1.5 rounded-md transition-all duration-200 ${
                      themeMode === 'light'
                        ? 'bg-white dark:bg-gray-700 text-amber-600 dark:text-amber-400 shadow-md border border-amber-200 dark:border-amber-500/30'
                        : 'text-gray-500 hover:text-gray-700 hover:bg-white/60 dark:text-gray-400 dark:hover:text-gray-200 dark:hover:bg-gray-700/50'
                    }`}
                    title='亮色模式'>
                    <Sun size={16} />
                  </button>
                  <button
                    onClick={() => handleThemeChange('dark')}
                    className={`p-1.5 rounded-md transition-all duration-200 ${
                      themeMode === 'dark'
                        ? 'bg-white dark:bg-gray-700 text-indigo-600 dark:text-indigo-400 shadow-md border border-indigo-200 dark:border-indigo-500/30'
                        : 'text-gray-500 hover:text-gray-700 hover:bg-white/60 dark:text-gray-400 dark:hover:text-gray-200 dark:hover:bg-gray-700/50'
                    }`}
                    title='暗色模式'>
                    <Moon size={16} />
                  </button>
                </div>
              </div>
              {/* Mobile menu button */}
              <button
                onClick={() => setIsMenuOpen(!isMenuOpen)}
                className='md:hidden p-2 rounded-lg text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-400 dark:hover:bg-gray-800'>
                {isMenuOpen ? <X size={20} /> : <Menu size={20} />}
              </button>
            </div>
            {/* Mobile Navigation */}
            {isMenuOpen && (
              <div className='md:hidden py-2 border-t border-white/20'>
                <nav className='flex flex-col space-y-1'>
                  {tabs.map((tab) => {
                    const Icon = tab.icon
                    return (
                      <button
                        key={tab.id}
                        onClick={() => {
                          handleTabChange(tab.id as any)
                          setIsMenuOpen(false)
                        }}
                        className={`flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                          activeTab === tab.id
                            ? 'bg-white text-blue-600 shadow-md dark:bg-gray-700 dark:text-blue-400'
                            : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white'
                        }`}>
                        <Icon size={16} />
                        <span>{tab.label}</span>
                      </button>
                    )
                  })}
                  {/* Mobile Theme Switcher */}
                  <div className='pt-2 border-t border-gray-200/50 dark:border-white/20 mt-2'>
                    <div className='px-3 py-1 text-xs text-gray-500 dark:text-gray-400'>
                      主题
                    </div>
                    <div className='flex items-center space-x-1 mt-1'>
                      <button
                        onClick={() => {
                          handleThemeChange('auto')
                          setIsMenuOpen(false)
                        }}
                        className={`flex-1 flex items-center justify-center space-x-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                          themeMode === 'auto'
                            ? 'bg-white text-blue-600 shadow-md border border-blue-200 dark:bg-gray-700 dark:text-blue-400 dark:border-blue-500/30'
                            : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white'
                        }`}>
                        <Monitor size={16} />
                        <span>自动</span>
                      </button>
                      <button
                        onClick={() => {
                          handleThemeChange('light')
                          setIsMenuOpen(false)
                        }}
                        className={`flex-1 flex items-center justify-center space-x-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                          themeMode === 'light'
                            ? 'bg-white text-amber-600 shadow-md border border-amber-200 dark:bg-gray-700 dark:text-amber-400 dark:border-amber-500/30'
                            : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white'
                        }`}>
                        <Sun size={16} />
                        <span>亮色</span>
                      </button>
                      <button
                        onClick={() => {
                          handleThemeChange('dark')
                          setIsMenuOpen(false)
                        }}
                        className={`flex-1 flex items-center justify-center space-x-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 ${
                          themeMode === 'dark'
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
          <div className='h-full max-w-7xl mx-auto px-3 sm:px-4 lg:px-6 py-4'>
            <div
              className={`h-full transition-opacity duration-300 ease-in-out ${
                isTransitioning ? 'opacity-0' : 'opacity-100'
              }`}>
              {activeTab === 'overview' && <Dashboard />}
              {activeTab === 'servers' && <McpServerManager />}
              {activeTab === 'market' && <Marketplace />}
              {activeTab === 'apikeys' && <ApiKeys />}
              {activeTab === 'settings' && <Settings />}
            </div>
          </div>
        </main>

        {/* Footer */}
        <footer className='py-3 text-center text-xs text-gray-500 dark:text-gray-400 border-t border-gray-200 dark:border-gray-700'>
          {' '}
          {/* 减小内边距和字体大小 */}
          <p>
            &copy; 2025 MCP Router. All rights reserved.
            <button
              type='button'
              onClick={() => setIsAboutOpen(true)}
              className='ml-3 text-blue-600 hover:text-blue-700 dark:text-blue-400'>
              关于
            </button>
          </p>
        </footer>
      </div>
      {/* About Modal */}
      <AboutModal isOpen={isAboutOpen} onClose={() => setIsAboutOpen(false)} />
    </div>
  )
}

export default App
