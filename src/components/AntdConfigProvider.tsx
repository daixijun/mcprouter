/**
 * Ant Design 统一配置提供者
 * 统一管理主题、语言和全局配置
 */

import type { ThemeConfig } from 'antd'
import { ConfigProvider, message, notification } from 'antd'
import React, { ReactNode, useMemo } from 'react'
import { useAppContext } from '../contexts/AppContext'
import { useTranslation } from 'react-i18next'
import { getThemeConfig } from '../theme/antd-config'

interface AntdConfigProviderProps {
  children: ReactNode
  className?: string
  style?: React.CSSProperties
}

/**
 * Antd 统一配置提供者组件
 * 整合主题配置和语言配置
 */
export const AntdConfigProvider: React.FC<AntdConfigProviderProps> = ({
  children,
  className,
  style,
}) => {
  const { state } = useAppContext()
  useTranslation() // 保持用于i18n实例更新，即使不直接使用

  // 使用 useMemo 优化性能，避免每次渲染都重新计算配置
  const themeConfig = useMemo<ThemeConfig>(() => {
    return getThemeConfig(state.isDarkMode)
  }, [state.isDarkMode])

  // 全局 Antd 配置
  const configProviderProps = useMemo(
    () => ({
      theme: themeConfig,
      locale: undefined, // 暂时禁用 Antd 的内置国际化，使用我们自己的翻译
      // 组件尺寸全局配置
      componentSize: 'middle' as const,
      // 其他全局配置
      direction: 'ltr' as const, // 目前只支持从左到右
      // 波浪效果配置
      wave: {
        disabled: false, // 启用点击波纹效果
      },
      // 表单验证消息配置（可选）
      form: {
        validateMessages: {
          required: '${label}是必填项',
          // 可以添加更多验证消息模板
        },
      },
    }),
    [themeConfig],
  )

  // 配置全局消息和通知（需要在组件外部设置）
  React.useEffect(() => {
    message.config({
      maxCount: 3,
      duration: 4.5,
    })

    notification.config({
      placement: 'topRight',
      maxCount: 3,
      duration: 4.5,
    })
  }, [])

  return (
    <ConfigProvider {...configProviderProps}>
      <div className={className} style={style}>
        {children}
      </div>
    </ConfigProvider>
  )
}

/**
 * 获取当前 Antd 配置信息的钩子
 */
export const useAntdConfig = () => {
  const { state } = useAppContext()

  return useMemo(
    () => ({
      theme: getThemeConfig(state.isDarkMode),
      locale: undefined,
      isDarkMode: state.isDarkMode,
    }),
    [state.isDarkMode],
  )
}

export default AntdConfigProvider
