/**
 * 语言选项配置
 */

export interface LanguageOption {
  code: string
  name: string
  nativeName: string
  flag: string
}

// 语言选项配置
export const LANGUAGE_OPTIONS: LanguageOption[] = [
  {
    code: 'zh-CN',
    name: 'Chinese (Simplified)',
    nativeName: '简体中文',
    flag: '🇨🇳',
  },
  {
    code: 'en-US',
    name: 'English (US)',
    nativeName: 'English',
    flag: '🇺🇸',
  },
]