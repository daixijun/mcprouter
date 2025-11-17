import i18n from 'i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { initReactI18next } from 'react-i18next'

// 导入合并后的语言包
import enUS from './en-US.json'
import zhCN from './zh-CN.json'

// 语言资源 - 转换为命名空间格式
const resources = {
  'zh-CN': {
    translation: zhCN,
  },
  'en-US': {
    translation: enUS,
  },
}

// 初始化 i18n
i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'zh-CN',
    defaultNS: 'translation', // 使用默认的translation命名空间

    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
      lookupLocalStorage: 'i18nextLng',
    },

    interpolation: {
      escapeValue: false,
    },

    react: {
      useSuspense: false,
    },

    debug: process.env.NODE_ENV === 'development', // 开发环境下启用调试
    saveMissing: false, // 关闭自动保存缺失键，避免性能问题

    // 确保所有语言都被预加载
    preload: ['zh-CN', 'en-US'],
  })

// 开发环境下输出额外的诊断信息
if (process.env.NODE_ENV === 'development') {
  console.log('=== I18n 初始化完成 ===')
  console.log('配置信息:', {
    fallbackLng: i18n.options.fallbackLng,
    defaultNS: i18n.options.defaultNS,
    ns: i18n.options.ns,
    currentLanguage: i18n.language,
    resources: Object.keys(i18n.options.resources || {}),
  })
}

export default i18n
