import React, { useState, useCallback, useRef, useEffect } from 'react'
import { Input, Form, message } from 'antd'
import { EditOutlined, CheckOutlined, CloseOutlined } from '@ant-design/icons'
import { useTranslation } from 'react-i18next'
import { permissionService, usePermissionUpdateStatus } from '../services/permissionService'

const { TextArea } = Input

interface InlineEditableCellProps {
  value: string
  onChange?: (value: string) => void
  tokenId: string
  field: 'name' | 'description'
  placeholder?: string
  type?: 'input' | 'textarea'
  maxLength?: number
  disabled?: boolean
  style?: React.CSSProperties
  showEditIcon?: boolean
  onTokenUpdate?: (token: any) => void
}

const InlineEditableCell: React.FC<InlineEditableCellProps> = ({
  value,
  onChange,
  tokenId,
  field,
  placeholder,
  type = 'input',
  maxLength = 200,
  disabled = false,
  style,
  showEditIcon = true,
  onTokenUpdate,
}) => {
  const { t } = useTranslation()
  const { updateStatus, getStatus, clearStatus } = usePermissionUpdateStatus()
  const [isEditing, setIsEditing] = useState(false)
  const [editValue, setEditValue] = useState(value)
  const [originalValue, setOriginalValue] = useState(value)
  const inputRef = useRef<any>(null)

  const statusKey = `${tokenId}_${field}`
  const status = getStatus(statusKey)

  // 当value从外部改变时，更新内部状态
  useEffect(() => {
    if (!isEditing) {
      setEditValue(value)
      setOriginalValue(value)
    }
  }, [value, isEditing])

  // 自动聚焦输入框
  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus()
      if (type === 'input') {
        inputRef.current.select()
      }
    }
  }, [isEditing, type])

  // 开始编辑
  const handleStartEdit = useCallback(() => {
    if (disabled || status.loading) return

    setIsEditing(true)
    setOriginalValue(value)
    setEditValue(value)
  }, [disabled, status.loading, value])

  // 保存编辑
  const handleSave = useCallback(async () => {
    // 如果值没有改变，直接取消编辑
    if (editValue === originalValue) {
      setIsEditing(false)
      return
    }

    // 验证字段
    if (field === 'name' && editValue.trim().length === 0) {
      message.error(t('token.validation.name_required'))
      return
    }

    if (field === 'name' && editValue.length > 100) {
      message.error(t('token.validation.name_max_length'))
      return
    }

    if (field === 'description' && editValue.length > 500) {
      message.error(t('token.validation.description_max_length'))
      return
    }

    try {
      updateStatus(statusKey, { loading: true, error: null, success: false })

      // 调用API更新字段
      const response = await permissionService.updateFieldWithoutRetry({
        token_id: tokenId,
        field,
        value: editValue.trim()
      })

      updateStatus(statusKey, { loading: false, success: true })

      // 更新外部状态
      onChange?.(editValue.trim())

      // 通知token更新
      onTokenUpdate?.(response.token)

      // 清理状态
      setTimeout(() => {
        clearStatus(statusKey)
      }, 2000)

      setIsEditing(false)

    } catch (error) {
      console.error('Failed to update field:', error)

      const errorMessage = error instanceof Error ? error.message : 'Unknown error'
      updateStatus(statusKey, { loading: false, error: errorMessage })

      message.error(`${t('common.validation.field_update_failed', { field: t(`token.form.${field}`) })}: ${errorMessage}`)
    }
  }, [editValue, originalValue, field, tokenId, onChange, onTokenUpdate, updateStatus, clearStatus, statusKey])

  // 取消编辑
  const handleCancel = useCallback(() => {
    setEditValue(originalValue)
    setIsEditing(false)
    clearStatus(statusKey)
  }, [originalValue, clearStatus, statusKey])

  // 键盘事件处理
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && type === 'input') {
      e.preventDefault()
      handleSave()
    } else if (e.key === 'Escape') {
      e.preventDefault()
      handleCancel()
    }
  }, [handleSave, handleCancel, type])

  // 渲染编辑状态
  if (isEditing) {
    const InputComponent = type === 'textarea' ? TextArea : Input

    return (
      <div style={style}>
        <Form.Item
          style={{ margin: 0 }}
          validateStatus={status.error ? 'error' : undefined}
          help={status.error}
        >
          <InputComponent
            ref={inputRef}
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={placeholder}
            maxLength={maxLength}
            showCount={type === 'textarea'}
            autoSize={type === 'textarea' ? { minRows: 2, maxRows: 4 } : undefined}
            disabled={status.loading}
            suffix={
              <div style={{ display: 'flex', gap: '4px' }}>
                {status.loading ? (
                  <div className="loading" />
                ) : (
                  <>
                    <CheckOutlined
                      onClick={handleSave}
                      style={{
                        color: '#52c41a',
                        cursor: 'pointer',
                        fontSize: '14px'
                      }}
                    />
                    <CloseOutlined
                      onClick={handleCancel}
                      style={{
                        color: '#ff4d4f',
                        cursor: 'pointer',
                        fontSize: '14px'
                      }}
                    />
                  </>
                )}
              </div>
            }
          />
        </Form.Item>
      </div>
    )
  }

  // 渲染显示状态
  const displayValue = value || (field === 'description' ? t('common.messages.no_description') : '')
  const isEmpty = !value || value.trim().length === 0

  return (
    <div
      style={{
        ...style,
        cursor: disabled || status.loading ? 'not-allowed' : 'pointer',
        position: 'relative',
        padding: '4px 8px',
        borderRadius: '4px',
        transition: 'background-color 0.2s',
        minHeight: type === 'textarea' ? '60px' : '32px',
        display: 'flex',
        alignItems: isEmpty ? 'center' : 'flex-start'
      }}
      onClick={handleStartEdit}
      onMouseEnter={(e) => {
        if (!disabled && !status.loading) {
          e.currentTarget.style.backgroundColor = '#f5f5f5'
        }
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = 'transparent'
      }}
    >
      <div style={{ flex: 1, wordBreak: 'break-word' }}>
        {isEmpty ? (
          <span style={{ color: '#bfbfbf', fontStyle: 'italic' }}>
            {placeholder || t('common.actions.click_to_edit')}
          </span>
        ) : (
          <span>{displayValue}</span>
        )}
      </div>

      {/* 编辑图标 */}
      {showEditIcon && !disabled && !status.loading && (
        <EditOutlined
          style={{
            marginLeft: '8px',
            color: '#999',
            fontSize: '12px',
            opacity: 0.7
          }}
        />
      )}

      {/* 加载状态 */}
      {status.loading && (
        <div
          style={{
            marginLeft: '8px',
            width: '14px',
            height: '14px',
            border: '1px solid #1890ff',
            borderTopColor: 'transparent',
            borderRadius: '50%',
            animation: 'spin 1s linear infinite'
          }}
        />
      )}

      {/* 成功状态 */}
      {status.success && (
        <div
          style={{
            marginLeft: '8px',
            color: '#52c41a',
            fontSize: '12px'
          }}
        >
          ✓
        </div>
      )}

      {/* 错误状态 */}
      {status.error && (
        <div
          style={{
            marginLeft: '8px',
            color: '#ff4d4f',
            fontSize: '12px'
          }}
          title={status.error}
        >
          ✕
        </div>
      )}

      <style dangerouslySetInnerHTML={{
        __html: `
          @keyframes spin {
            0% { transform: rotate(0deg); }
            100% { transform: rotate(360deg); }
          }
        `
      }} />
    </div>
  )
}

export default InlineEditableCell