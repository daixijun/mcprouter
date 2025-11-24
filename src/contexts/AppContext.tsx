import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import React, {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useReducer,
} from 'react'
import { useTranslation } from 'react-i18next'

// 状态类型定义
export type ThemeMode = 'light' | 'dark' | 'auto'
export type TabType = 'overview' | 'servers' | 'market' | 'settings' | 'tokens'

interface AppState {
  themeMode: ThemeMode
  isDarkMode: boolean
  activeTab: TabType
  isMenuOpen: boolean
  isAboutOpen: boolean
  isTransitioning: boolean
  loading: boolean
  error: string | null
}

// Action 类型定义
type AppAction =
  | { type: 'SET_THEME_MODE'; payload: ThemeMode }
  | { type: 'SET_IS_DARK_MODE'; payload: boolean }
  | { type: 'SET_ACTIVE_TAB'; payload: TabType }
  | { type: 'SET_IS_MENU_OPEN'; payload: boolean }
  | { type: 'SET_IS_ABOUT_OPEN'; payload: boolean }
  | { type: 'SET_IS_TRANSITIONING'; payload: boolean }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null }
  | { type: 'CLEAR_ERROR' }

// 初始状态
const initialState: AppState = {
  themeMode: 'auto',
  isDarkMode: false,
  activeTab: 'overview',
  isMenuOpen: false,
  isAboutOpen: false,
  isTransitioning: false,
  loading: false,
  error: null,
}

// Reducer
const appReducer = (state: AppState, action: AppAction): AppState => {
  switch (action.type) {
    case 'SET_THEME_MODE':
      return { ...state, themeMode: action.payload }
    case 'SET_IS_DARK_MODE':
      return { ...state, isDarkMode: action.payload }
    case 'SET_ACTIVE_TAB':
      return { ...state, activeTab: action.payload }
    case 'SET_IS_MENU_OPEN':
      return { ...state, isMenuOpen: action.payload }
    case 'SET_IS_ABOUT_OPEN':
      return { ...state, isAboutOpen: action.payload }
    case 'SET_IS_TRANSITIONING':
      return { ...state, isTransitioning: action.payload }
    case 'SET_LOADING':
      return { ...state, loading: action.payload }
    case 'SET_ERROR':
      return { ...state, error: action.payload }
    case 'CLEAR_ERROR':
      return { ...state, error: null }
    default:
      return state
  }
}

// Context 类型定义
interface AppContextType {
  state: AppState
  dispatch: React.Dispatch<AppAction>
  // 便捷方法
  setThemeMode: (mode: ThemeMode) => void
  setActiveTab: (tab: TabType) => void
  toggleMenu: () => void
  toggleAbout: () => void
  setError: (error: string | null) => void
  clearError: () => void
}

// 创建 Context
const AppContext = createContext<AppContextType | undefined>(undefined)

// Provider 组件
interface AppProviderProps {
  children: ReactNode
}

export const AppProvider: React.FC<AppProviderProps> = ({ children }) => {
  const [state, dispatch] = useReducer(appReducer, initialState)
  const { i18n } = useTranslation()

  // 便捷方法
  const setThemeMode = (mode: ThemeMode) => {
    dispatch({ type: 'SET_THEME_MODE', payload: mode })
  }

  const setActiveTab = (tab: TabType) => {
    if (tab === state.activeTab) return

    dispatch({ type: 'SET_IS_TRANSITIONING', payload: true })
    setTimeout(() => {
      dispatch({ type: 'SET_ACTIVE_TAB', payload: tab })
      dispatch({ type: 'SET_IS_TRANSITIONING', payload: false })
    }, 150)
  }

  const toggleMenu = () => {
    dispatch({ type: 'SET_IS_MENU_OPEN', payload: !state.isMenuOpen })
  }

  const toggleAbout = () => {
    dispatch({ type: 'SET_IS_ABOUT_OPEN', payload: !state.isAboutOpen })
  }

  const setError = (error: string | null) => {
    dispatch({ type: 'SET_ERROR', payload: error })
  }

  const clearError = () => {
    dispatch({ type: 'CLEAR_ERROR' })
  }

  // 主题管理副作用
  useEffect(() => {
    const loadTheme = async () => {
      try {
        dispatch({ type: 'SET_LOADING', payload: true })
        const savedTheme = await invoke<string>('get_theme')
        setThemeMode(savedTheme as ThemeMode)
      } catch (error) {
        console.error('Failed to load theme:', error)
        setError('加载主题设置失败')
      } finally {
        dispatch({ type: 'SET_LOADING', payload: false })
      }
    }
    loadTheme()

    // 监听主题变化
    const setupThemeListener = async () => {
      try {
        const unlistenTheme = await listen<string>('theme-changed', (event) => {
          const newTheme = event.payload as ThemeMode
          setThemeMode(newTheme)
        })
        return unlistenTheme
      } catch (error) {
        console.error('Failed to setup theme listener:', error)
        setError('设置主题监听器失败')
      }
    }

    const themeCleanup = setupThemeListener()
    return () => {
      themeCleanup.then((cleanup) => cleanup && cleanup())
    }
  }, [])

  // 语言监听副作用
  useEffect(() => {
    const setupLanguageListener = async () => {
      try {
        const unlistenLanguage = await listen<string>('language-changed', (event) => {
          const newLanguage = event.payload
          i18n.changeLanguage(newLanguage)
        })
        return unlistenLanguage
      } catch (error) {
        console.error('Failed to setup language listener:', error)
        setError('设置语言监听器失败')
      }
    }

    const languageCleanup = setupLanguageListener()
    return () => {
      languageCleanup.then((cleanup) => cleanup && cleanup())
    }
  }, [i18n])

  // 主题应用副作用
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

    const applyTheme = () => {
      if (state.themeMode === 'auto') {
        dispatch({ type: 'SET_IS_DARK_MODE', payload: mediaQuery.matches })
      } else {
        dispatch({
          type: 'SET_IS_DARK_MODE',
          payload: state.themeMode === 'dark',
        })
      }
    }

    applyTheme()

    const handleSystemThemeChange = (e: MediaQueryListEvent) => {
      if (state.themeMode === 'auto') {
        dispatch({ type: 'SET_IS_DARK_MODE', payload: e.matches })
      }
    }

    if (state.themeMode === 'auto') {
      mediaQuery.addEventListener('change', handleSystemThemeChange)
    }

    return () => {
      mediaQuery.removeEventListener('change', handleSystemThemeChange)
    }
  }, [state.themeMode])

  // 监听系统托盘导航事件
  useEffect(() => {
    const setupTrayListeners = async () => {
      try {
        const unlistenNavigate = await listen<string>(
          'navigate-to',
          (event) => {
            const targetTab = event.payload
            if (targetTab === 'servers') {
              setActiveTab('servers')
            } else if (targetTab === 'marketplace') {
              setActiveTab('market')
            } else if (targetTab === 'tokens') {
              setActiveTab('tokens')
            } else if (targetTab === 'settings') {
              setActiveTab('settings')
            }
          },
        )

        const unlistenAbout = await listen('show-about-dialog', () => {
          dispatch({ type: 'SET_IS_ABOUT_OPEN', payload: true })
        })

        return () => {
          unlistenNavigate()
          unlistenAbout()
        }
      } catch (error) {
        console.error('Failed to setup tray listeners:', error)
        setError('设置系统托盘监听器失败')
      }
    }

    setupTrayListeners()
  }, [])

  const contextValue: AppContextType = {
    state,
    dispatch,
    setThemeMode,
    setActiveTab,
    toggleMenu,
    toggleAbout,
    setError,
    clearError,
  }

  return (
    <AppContext.Provider value={contextValue}>{children}</AppContext.Provider>
  )
}

// Hook to use the context
export const useAppContext = (): AppContextType => {
  const context = useContext(AppContext)
  if (context === undefined) {
    throw new Error('useAppContext must be used within an AppProvider')
  }
  return context
}
