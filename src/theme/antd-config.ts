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
    colorTextPlaceholder: '#9ca3af', // 亮色模式占位符颜色
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
      colorBorder: 'var(--color-border-primary)',
      colorBorderHover: 'var(--color-border-hover)',
      colorBorderFocus: 'var(--color-border-focus)',
    },
    Select: {
      controlHeight: 32,
      controlHeightLG: 40,
      controlHeightSM: 24,
      borderRadius: 8,
      colorBorder: 'var(--color-border-primary)',
      colorBorderHover: 'var(--color-border-hover)',
      colorBorderFocus: 'var(--color-border-focus)',
      optionSelectedBg: 'var(--color-bg-tertiary)',
    },
    Card: {
      borderRadiusLG: 12,
    },
    Modal: {
      borderRadiusLG: 12,
    },
    Switch: {
      colorPrimary: '#10B981', // 开启状态：绿色
      colorText: '#ffffff', // 开启状态文字：白色
      colorBorder: '#d1d5db', // 边框颜色
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

    // 中性色 - 暗色模式，使用分层的文本颜色以提供更好的对比度
    colorBgBase: '#111827', // gray-900
    colorBgContainer: '#1f2937', // gray-800
    colorBgElevated: '#1f2937', // gray-800
    colorBgLayout: '#0f172a', // slate-900
    colorBorder: '#374151', // gray-700
    colorBorderSecondary: '#4b5563', // gray-600
    colorText: '#f9fafb', // 主文本：亮灰色 (gray-50)
    colorTextSecondary: '#d1d5db', // 次要文本：中灰色 (gray-300)
    colorTextTertiary: '#9ca3af', // 三级文本：暗灰色 (gray-400)
    colorTextQuaternary: '#6b7280', // 四级文本：更暗灰色 (gray-500)
    colorTextPlaceholder: '#4b5563', // 暗色模式占位符颜色
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
      colorBorder: 'var(--color-border-primary)',
      colorBorderHover: 'var(--color-border-hover)',
      colorBorderFocus: 'var(--color-border-focus)',
    },
    Select: {
      controlHeight: 32,
      controlHeightLG: 40,
      controlHeightSM: 24,
      borderRadius: 8,
      colorBorder: 'var(--color-border-primary)',
      colorBorderHover: 'var(--color-border-hover)',
      colorBorderFocus: 'var(--color-border-focus)',
      optionSelectedBg: 'var(--color-bg-tertiary)',
    },
    Card: {
      borderRadiusLG: 12,
      colorBgContainer: 'transparent', // 设置为透明，让子元素显示正确的背景色
      bodyStyle: {
        backgroundColor: 'transparent',
        padding: '12px',
      },
    },
    Statistic: {
      colorTextDescription: '#9ca3af', // 统计描述文字颜色
    },
    Typography: {
      colorText: '#f9fafb',
      colorTextSecondary: '#d1d5db',
    },
    Modal: {
      borderRadiusLG: 12,
      titleColor: '#ffffff', // 暗黑模式下标题使用白色
    },
    Switch: {
      colorPrimary: '#10B981', // 开启状态：绿色
      colorText: '#ffffff', // 开启状态文字：白色
      colorBorder: '#374151', // 边框颜色（暗色）
    },
    Divider: {
      colorSplit: 'var(--color-divider)',
      colorText: 'var(--color-text-secondary)',
    },
    Menu: {
      colorItemBgSelected: 'var(--color-bg-tertiary)',
      colorItemTextSelected: 'var(--color-primary)',
      colorItemText: 'var(--color-text-primary)',
      itemBorderRadius: 8,
      itemBg: 'transparent',
      itemHoverBg: 'var(--color-bg-secondary)',
    },
    Dropdown: {
      colorBgElevated: 'var(--color-bg-elevated)',
      colorBgSpotlight: 'var(--color-bg-tertiary)',
      colorText: 'var(--color-text-primary)',
      colorTextQuaternary: 'var(--color-text-tertiary)',
      borderColor: 'var(--color-border-primary)',
    },
  },
}

/**
 * 根据主题名称获取主题配置
 */
export function getThemeConfig(isDark: boolean): ThemeConfig {
  return isDark ? darkTheme : lightTheme
}
