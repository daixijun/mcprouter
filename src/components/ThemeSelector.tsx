import { Badge, Button, Dropdown, Space, Tooltip } from 'antd'
import { Info, Moon, Sun } from 'lucide-react'
import React, { memo } from 'react'
import { useTranslation } from 'react-i18next'
import { useAppContext } from '../contexts/AppContext'

export type ThemeMode = 'light' | 'dark' | 'auto'

interface ThemeSelectorProps {
  type?: 'desktop' | 'mobile'
  size?: 'small' | 'middle' | 'large'
  showLabel?: boolean
}

const ThemeSelector: React.FC<ThemeSelectorProps> = memo(({
  type = 'desktop',
  size = 'small',
  showLabel = false
}) => {
  const { t } = useTranslation()
  const { state, setThemeMode } = useAppContext()

  const handleThemeChange = async (mode: ThemeMode) => {
    setThemeMode(mode)
    // 这里可以添加保存主题设置到后端的逻辑
  }

  // 获取当前主题的信息
  const getCurrentThemeInfo = () => {
    switch (state.themeMode) {
      case 'auto':
        return {
          icon: <Info size={size === 'small' ? 16 : 18} />,
          label: t('dashboard.theme.auto'),
          key: 'auto'
        }
      case 'light':
        return {
          icon: <Sun size={size === 'small' ? 16 : 18} />,
          label: t('dashboard.theme.light'),
          key: 'light'
        }
      case 'dark':
        return {
          icon: <Moon size={size === 'small' ? 16 : 18} />,
          label: t('dashboard.theme.dark'),
          key: 'dark'
        }
    }
  }

  const currentTheme = getCurrentThemeInfo()

  // 主题选项列表
  const themeOptions = [
    {
      key: 'auto',
      label: (
        <div className='flex items-center gap-2'>
          <Info size={16} />
          <span>{t('dashboard.theme.auto')}</span>
          {state.isDarkMode && (
            <Badge
              size='small'
              style={{
                backgroundColor: 'var(--theme-primary)',
                color: 'white'
              }}
            >
              {t('dashboard.theme.current')}
            </Badge>
          )}
        </div>
      ),
      onClick: () => handleThemeChange('auto')
    },
    {
      key: 'light',
      label: (
        <div className='flex items-center gap-2'>
          <Sun size={16} />
          <span>{t('dashboard.theme.light')}</span>
          {state.themeMode === 'light' && !state.isDarkMode && (
            <Badge
              size='small'
              style={{
                backgroundColor: 'var(--theme-primary)',
                color: 'white'
              }}
            >
              {t('dashboard.theme.current')}
            </Badge>
          )}
        </div>
      ),
      onClick: () => handleThemeChange('light')
    },
    {
      key: 'dark',
      label: (
        <div className='flex items-center gap-2'>
          <Moon size={16} />
          <span>{t('dashboard.theme.dark')}</span>
          {state.isDarkMode && state.themeMode !== 'auto' && (
            <Badge
              size='small'
              style={{
                backgroundColor: 'var(--theme-primary)',
                color: 'white'
              }}
            >
              {t('dashboard.theme.current')}
            </Badge>
          )}
        </div>
      ),
      onClick: () => handleThemeChange('dark')
    }
  ]

  // 桌面端渲染：三个按钮横向排列
  if (type === 'desktop') {
    return (
      <div className='theme-switcher'>
        <Tooltip title={t('dashboard.theme.auto')}>
          <button
            className={`theme-btn ${state.themeMode === 'auto' ? 'active' : ''}`}
            onClick={() => handleThemeChange('auto')}
            aria-label={t('dashboard.theme.auto')}
          >
            <Info size={16} />
          </button>
        </Tooltip>
        <Tooltip title={t('dashboard.theme.light')}>
          <button
            className={`theme-btn ${state.themeMode === 'light' ? 'active' : ''}`}
            onClick={() => handleThemeChange('light')}
            aria-label={t('dashboard.theme.light')}
          >
            <Sun size={16} />
          </button>
        </Tooltip>
        <Tooltip title={t('dashboard.theme.dark')}>
          <button
            className={`theme-btn ${state.themeMode === 'dark' ? 'active' : ''}`}
            onClick={() => handleThemeChange('dark')}
            aria-label={t('dashboard.theme.dark')}
          >
            <Moon size={16} />
          </button>
        </Tooltip>
      </div>
    )
  }

  // 移动端渲染：下拉菜单
  return (
    <Dropdown
      menu={{
        items: themeOptions,
        selectedKeys: [state.themeMode]
      }}
      placement='bottomLeft'
      trigger={['click']}
      arrow
    >
      <Button
        type='text'
        size={size}
        className='btn-modern'
        aria-label={t('dashboard.theme.title')}
      >
        <Space size='small'>
          {currentTheme.icon}
          {showLabel && <span>{currentTheme.label}</span>}
        </Space>
      </Button>
    </Dropdown>
  )
})

ThemeSelector.displayName = 'ThemeSelector'

export default ThemeSelector