import { ThemeConfig, theme } from 'antd'

/**
 * 亮色主题配置
 * 颜色映射自 Tailwind CSS
 */
export const lightTheme: ThemeConfig = {
  algorithm: theme.defaultAlgorithm,
  token: {
    // 主色调
    colorPrimary: '#3B82F6', // blue-600
    colorSuccess: '#10B981', // green-500
    colorWarning: '#F59E0B', // amber-500
    colorError: '#EF4444', // red-500
    colorInfo: '#3B82F6', // blue-600

    // 圆角
    borderRadius: 8,
    borderRadiusLG: 12,
    borderRadiusSM: 6,

    // 字体
    fontFamily:
      '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
    fontSize: 14,
    fontSizeHeading1: 30,
    fontSizeHeading2: 24,
    fontSizeHeading3: 20,
    fontSizeHeading4: 16,
    fontSizeHeading5: 14,

    // 间距
    controlHeight: 32,
    controlHeightLG: 40,
    controlHeightSM: 24,

    // 中性色 - 与 Tailwind 的灰度系统对齐
    colorBgBase: '#ffffff',
    colorBgContainer: '#ffffff',
    colorBgElevated: '#ffffff',
    colorBgLayout: '#f9fafb', // gray-50
    colorBorder: '#e5e7eb', // gray-200
    colorBorderSecondary: '#f3f4f6', // gray-100
    colorText: '#111827', // gray-900
    colorTextSecondary: '#6b7280', // gray-500
    colorTextTertiary: '#9ca3af', // gray-400
    colorTextQuaternary: '#d1d5db', // gray-300
  },
  components: {
    Button: {
      controlHeight: 32,
      controlHeightLG: 40,
      controlHeightSM: 24,
      borderRadius: 8,
      fontWeight: 500,
    },
    Input: {
      controlHeight: 32,
      controlHeightLG: 40,
      controlHeightSM: 24,
      borderRadius: 8,
    },
    Select: {
      controlHeight: 32,
      controlHeightLG: 40,
      controlHeightSM: 24,
      borderRadius: 8,
    },
    Card: {
      borderRadiusLG: 12,
    },
    Modal: {
      borderRadiusLG: 12,
    },
  },
}

/**
 * 暗色主题配置
 * 使用 antd 的暗色算法自动计算颜色变体
 */
export const darkTheme: ThemeConfig = {
  algorithm: theme.darkAlgorithm,
  token: {
    // 主色调（与亮色主题相同）
    colorPrimary: '#3B82F6', // blue-600
    colorSuccess: '#10B981', // green-500
    colorWarning: '#F59E0B', // amber-500
    colorError: '#EF4444', // red-500
    colorInfo: '#3B82F6', // blue-600

    // 圆角（与亮色主题相同）
    borderRadius: 8,
    borderRadiusLG: 12,
    borderRadiusSM: 6,

    // 字体（与亮色主题相同）
    fontFamily:
      '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
    fontSize: 14,
    fontSizeHeading1: 30,
    fontSizeHeading2: 24,
    fontSizeHeading3: 20,
    fontSizeHeading4: 16,
    fontSizeHeading5: 14,

    // 间距（与亮色主题相同）
    controlHeight: 32,
    controlHeightLG: 40,
    controlHeightSM: 24,

    // 中性色 - 暗色模式，所有文本都使用白色
    colorBgBase: '#111827', // gray-900
    colorBgContainer: '#1f2937', // gray-800
    colorBgElevated: '#1f2937', // gray-800
    colorBgLayout: '#0f172a', // slate-900
    colorBorder: '#374151', // gray-700
    colorBorderSecondary: '#4b5563', // gray-600
    colorText: '#ffffff', // 纯白色
    colorTextSecondary: '#ffffff', // 纯白色
    colorTextTertiary: '#ffffff', // 纯白色
    colorTextQuaternary: '#ffffff', // 纯白色
  },
  components: {
    Button: {
      controlHeight: 32,
      controlHeightLG: 40,
      controlHeightSM: 24,
      borderRadius: 8,
      fontWeight: 500,
    },
    Input: {
      controlHeight: 32,
      controlHeightLG: 40,
      controlHeightSM: 24,
      borderRadius: 8,
    },
    Select: {
      controlHeight: 32,
      controlHeightLG: 40,
      controlHeightSM: 24,
      borderRadius: 8,
    },
    Card: {
      borderRadiusLG: 12,
    },
    Modal: {
      borderRadiusLG: 12,
      titleColor: '#ffffff', // 暗黑模式下标题使用白色
    },
  },
}

/**
 * 根据主题名称获取主题配置
 */
export function getThemeConfig(isDark: boolean): ThemeConfig {
  return isDark ? darkTheme : lightTheme
}
