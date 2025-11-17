import { GlobalOutlined } from '@ant-design/icons'
import { App as AntdApp, Select, Space, Typography } from 'antd'
import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { LANGUAGE_OPTIONS } from '../constants/language'

const { Text } = Typography
const { Option } = Select

export interface LanguageSelectorProps {
  className?: string
  style?: React.CSSProperties
  size?: 'small' | 'middle' | 'large'
  showLabel?: boolean
  placement?: 'bottomLeft' | 'bottomRight' | 'topLeft' | 'topRight'
}

export const LanguageSelector: React.FC<LanguageSelectorProps> = ({
  className,
  style,
  size = 'middle',
  showLabel = false,
  placement = 'bottomLeft',
}) => {
  const { i18n, t } = useTranslation()
  const [loading, setLoading] = useState(false)
  const { message } = AntdApp.useApp()

  const handleLanguageChange = async (newLanguage: string) => {
    if (newLanguage === i18n.language) return

    setLoading(true)
    try {
      await i18n.changeLanguage(newLanguage)
      message.success(t('common.language.changed_success'))
    } catch (error) {
      message.error(t('common.language.change_failed'))
      console.error('Language change failed:', error)
    } finally {
      setLoading(false)
    }
  }

  const popupRender = (menu: React.ReactElement) => (
    <div style={{ padding: '8px 0' }}>{menu}</div>
  )

  const languageOptions = LANGUAGE_OPTIONS.map((option) => (
    <Option key={option.code} value={option.code}>
      <Space>
        <span style={{ fontSize: '14px' }}>{option.flag}</span>
        <span>{option.nativeName}</span>
      </Space>
    </Option>
  ))

  return (
    <Space className={className} style={style}>
      {showLabel && (
        <Text type='secondary'>
          <GlobalOutlined /> Language
        </Text>
      )}
      <Select
        value={i18n.language}
        onChange={handleLanguageChange}
        loading={loading}
        size={size}
        placement={placement}
        style={{ minWidth: 120 }}
        popupRender={popupRender}
        suffixIcon={<GlobalOutlined />}>
        {languageOptions}
      </Select>
    </Space>
  )
}

// 简洁版语言选择器，用于工具栏等空间受限的地方
export const CompactLanguageSelector: React.FC<
  Omit<LanguageSelectorProps, 'showLabel'>
> = (props) => {
  const { i18n } = useTranslation()
  const [loading, setLoading] = useState(false)

  const handleLanguageChange = async (newLanguage: string) => {
    if (newLanguage === i18n.language) return

    setLoading(true)
    try {
      await i18n.changeLanguage(newLanguage)
    } catch (error) {
      console.error('Language change failed:', error)
    } finally {
      setLoading(false)
    }
  }

  return (
    <Select
      value={i18n.language}
      onChange={handleLanguageChange}
      loading={loading}
      size={props.size || 'small'}
      style={{ minWidth: 80, ...props.style }}
      suffixIcon={<GlobalOutlined />}
      className={props.className}
      placement={props.placement}>
      {LANGUAGE_OPTIONS.map((option) => (
        <Option key={option.code} value={option.code}>
          <Space size='small'>
            <span>{option.flag}</span>
          </Space>
        </Option>
      ))}
    </Select>
  )
}

export default LanguageSelector
