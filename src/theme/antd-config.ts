import { ThemeConfig, theme } from 'antd'

/**
 * 亮色主题配置
 * 使用 Tailwind 内置颜色，移除自定义组件 Token
 */
export const lightTheme: ThemeConfig = {
  algorithm: theme.defaultAlgorithm,
  token: {
    // 主色调使用 Tailwind blue-600
    colorPrimary: '#2563eb',
    // 成功色使用 Tailwind green-500
    colorSuccess: '#10b981',
    // 警告色使用 Tailwind amber-500
    colorWarning: '#f59e0b',
    // 错误色使用 Tailwind red-500
    colorError: '#ef4444',
    // 信息色使用 Tailwind blue-600
    colorInfo: '#2563eb',

    // 背景色
    colorBgBase: '#ffffff',
    colorBgContainer: '#ffffff',
    colorBgElevated: '#ffffff',
    colorBgLayout: '#f9fafb',

    // 文本色
    colorText: '#111827',
    colorTextSecondary: '#6b7280',
    colorTextTertiary: '#9ca3af',
    colorTextQuaternary: '#d1d5db',

    // 边框色
    colorBorder: '#e5e7eb',
    colorBorderSecondary: '#f3f4f6',

    // 基础配置
    borderRadius: 8,
    borderRadiusLG: 12,
    borderRadiusSM: 6,
    controlHeight: 32,
    controlHeightLG: 40,
    controlHeightSM: 24,
    fontSize: 14,
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
  },
  // 移除自定义组件配色 Token，使用 Ant Design 默认值
}

/**
 * 暗色主题配置
 * 使用 Tailwind 内置颜色，移除自定义组件 Token
 */
export const darkTheme: ThemeConfig = {
  algorithm: theme.darkAlgorithm,
  token: {
    // 保持主色调一致
    colorPrimary: '#2563eb',
    colorSuccess: '#10b981',
    colorWarning: '#f59e0b',
    colorError: '#ef4444',
    colorInfo: '#2563eb',

    // 暗色背景
    colorBgBase: '#111827',
    colorBgContainer: '#1f2937',
    colorBgElevated: '#1f2937',
    colorBgLayout: '#0f172a',

    // 暗色文本
    colorText: '#f9fafb',
    colorTextSecondary: '#d1d5db',
    colorTextTertiary: '#9ca3af',
    colorTextQuaternary: '#6b7280',

    // 暗色边框
    colorBorder: '#374151',
    colorBorderSecondary: '#4b5563',

    borderRadius: 8,
    borderRadiusLG: 12,
    borderRadiusSM: 6,
    controlHeight: 32,
    controlHeightLG: 40,
    controlHeightSM: 24,
    fontSize: 14,
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
  },
  // 移除自定义组件配色 Token，让 Ant Design 的 darkAlgorithm 自然生效
}

/**
 * 根据主题名称获取主题配置
 */
export function getThemeConfig(isDark: boolean): ThemeConfig {
  return isDark ? darkTheme : lightTheme
}