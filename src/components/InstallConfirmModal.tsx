import React, { useEffect, useState } from 'react'
import { Modal, Button, Typography } from 'antd'
import type { MarketplaceService, MarketplaceServiceListItem } from '../types'

interface EnvSchema {
  properties?: Record<
    string,
    {
      title?: string
      description?: string
      type?: string
      default?: any
      enum?: any[]
    }
  >
  required?: string[]
}

interface InstallConfirmModalProps {
  isOpen: boolean
  onClose: () => void
  onConfirm: (envVars: Record<string, string>) => Promise<void>
  service: MarketplaceService | MarketplaceServiceListItem | null
  envSchema: EnvSchema | null
  isLoading?: boolean
}

const InstallConfirmModal: React.FC<InstallConfirmModalProps> = ({
  isOpen,
  onClose,
  onConfirm,
  service,
  envSchema,
  isLoading = false,
}) => {
  const [envValues, setEnvValues] = useState<Record<string, string>>({})
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [showAdvanced, setShowAdvanced] = useState(false)

  // 初始化环境变量值
  useEffect(() => {
    if (envSchema && isOpen) {
      const initial: Record<string, string> = {}

      // 处理必传变量
      envSchema.required?.forEach((key) => {
        const prop = envSchema.properties?.[key]
        initial[key] = prop?.default?.toString() || ''
      })

      // 处理可选变量
      if (envSchema.properties) {
        Object.keys(envSchema.properties).forEach((key) => {
          if (!envSchema.required?.includes(key)) {
            const prop = envSchema.properties![key]
            initial[key] = prop?.default?.toString() || ''
          }
        })
      }

      setEnvValues(initial)
      setErrors({})
    }
  }, [envSchema, isOpen])

  // 验证环境变量
  const validateEnvVar = (key: string, value: string): string => {
    const prop = envSchema?.properties?.[key]
    if (!prop) return ''

    // 必传验证
    if (envSchema?.required?.includes(key) && !value.trim()) {
      return `${prop.title || key} 是必填项`
    }

    // 类型验证
    if (value.trim() && prop.type) {
      switch (prop.type) {
        case 'number':
          if (isNaN(Number(value))) {
            return `${prop.title || key} 必须是数字`
          }
          break
        case 'boolean':
          if (!['true', 'false', '1', '0'].includes(value.toLowerCase())) {
            return `${prop.title || key} 必须是 true/false`
          }
          break
        case 'array':
          // 简单的数组验证
          try {
            JSON.parse(value)
          } catch {
            return `${prop.title || key} 必须是有效的JSON数组`
          }
          break
        case 'object':
          // 简单的对象验证
          try {
            JSON.parse(value)
          } catch {
            return `${prop.title || key} 必须是有效的JSON对象`
          }
          break
      }
    }

    // 枚举验证
    if (value.trim() && prop.enum && !prop.enum.includes(value)) {
      return `${prop.title || key} 必须是以下值之一: ${prop.enum.join(', ')}`
    }

    return ''
  }

  // 处理环境变量值变化
  const handleEnvChange = (key: string, value: string) => {
    setEnvValues((prev) => ({ ...prev, [key]: value }))

    // 实时验证
    const error = validateEnvVar(key, value)
    setErrors((prev) => ({ ...prev, [key]: error }))
  }

  // 检查是否可以提交
  const canSubmit = () => {
    if (!envSchema) return true

    // 检查必传字段
    for (const key of envSchema.required || []) {
      if (!envValues[key]?.trim()) return false
    }

    // 检查错误
    return Object.values(errors).every((error) => !error)
  }

  // 获取安装命令显示
  const getInstallCommand = () => {
    if (!service) return ''

    // 使用install_command字段（后端已经从server_config中提取）
    if ('install_command' in service && service.install_command) {
      const { command, args } = service.install_command
      return `${command} ${args.join(' ')}`
    }

    return '未知安装命令'
  }

  // 处理确认安装
  const handleConfirm = async () => {
    if (!canSubmit()) return

    // 过滤出有值的环境变量
    const filteredEnvVars: Record<string, string> = {}
    Object.entries(envValues).forEach(([key, value]) => {
      if (value.trim()) {
        filteredEnvVars[key] = value.trim()
      }
    })

    await onConfirm(filteredEnvVars)
  }

  if (!service) return null

  const requiredEnvVars = envSchema?.required || []
  const optionalEnvVars = envSchema?.properties
    ? Object.keys(envSchema.properties).filter(
        (key) => !requiredEnvVars.includes(key),
      )
    : []

  const { Paragraph } = Typography

  return (
    <Modal
      open={isOpen}
      onCancel={onClose}
      title="确认安装服务"
      footer={[
        <Button key="cancel" onClick={onClose} disabled={isLoading}>
          取消
        </Button>,
        <Button
          key="confirm"
          type="primary"
          onClick={handleConfirm}
          disabled={!canSubmit() || isLoading}
          loading={isLoading}
        >
          确认安装
        </Button>
      ]}
      width={800}
    >
      <div className='space-y-6'>
        <Paragraph className='text-gray-600 dark:text-gray-300 !mb-6'>
          请检查以下服务信息并配置必要的环境变量
        </Paragraph>

        {/* 服务信息 */}
        <div className='bg-gray-50 dark:bg-gray-800 rounded-lg p-4 space-y-3'>
          <div className='flex items-center gap-3'>
            <div className='w-12 h-12 rounded-lg overflow-hidden bg-gray-200 flex items-center justify-center flex-shrink-0'>
              {service.logo_url ? (
                <img
                  src={service.logo_url}
                  alt={service.name}
                  className='w-full h-full object-cover'
                />
              ) : (
                <span className='text-2xl'>📦</span>
              )}
            </div>
            <div className='flex-1'>
              <h3 className='font-semibold text-gray-800 dark:text-gray-100'>
                {service.name}
              </h3>
              <p className='text-sm text-gray-600 dark:text-gray-300'>
                作者: {service.author} • 平台: {service.platform}
              </p>
            </div>
          </div>

          <div>
            <h4 className='font-medium text-gray-700 dark:text-gray-300 mb-1'>
              服务描述
            </h4>
            <p className='text-sm text-gray-600 dark:text-gray-300'>
              {service.description}
            </p>
          </div>

          <div>
            <h4 className='font-medium text-gray-700 dark:text-gray-300 mb-1'>
              安装命令
            </h4>
            <code className='block bg-gray-800 dark:bg-gray-900 text-green-400 dark:text-green-300 px-3 py-2 rounded text-sm border border-gray-700 dark:border-gray-600'>
              {getInstallCommand()}
            </code>
          </div>
        </div>

        {/* 环境变量配置 */}
        {envSchema &&
          (requiredEnvVars.length > 0 || optionalEnvVars.length > 0) && (
            <div>
              <div className='flex items-center justify-between mb-3'>
                <h3 className='font-semibold text-gray-800 dark:text-gray-100'>
                  环境变量配置
                </h3>
                {optionalEnvVars.length > 0 && (
                  <button
                    type='button'
                    onClick={() => setShowAdvanced(!showAdvanced)}
                    className='text-sm text-blue-600 hover:text-blue-700'>
                    {showAdvanced ? '隐藏' : '显示'}可选参数 (
                    {optionalEnvVars.length})
                  </button>
                )}
              </div>

              {/* 必传参数 */}
              {requiredEnvVars.length > 0 && (
                <div className='space-y-4 mb-4'>
                  {requiredEnvVars.map((key) => {
                    const prop = envSchema.properties?.[key]
                    const error = errors[key]

                    return (
                      <div key={key} className='space-y-1'>
                        <div className='flex items-center gap-4'>
                          <label className='text-sm font-medium text-gray-700 dark:text-gray-300 min-w-32 flex-shrink-0'>
                            {prop?.title || key}
                            <span className='text-red-500 ml-1'>*</span>
                          </label>
                          <input
                            type='text'
                            value={envValues[key] || ''}
                            onChange={(e) =>
                              handleEnvChange(key, e.target.value)
                            }
                            className={`flex-1 px-3 py-2 border rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 ${
                              error
                                ? 'border-red-500 dark:border-red-400'
                                : 'border-gray-300 dark:border-gray-600'
                            }`}
                            placeholder={prop?.description || `请输入 ${key}`}
                          />
                        </div>
                        {error && (
                          <p className='text-sm text-red-600 dark:text-red-400 ml-36'>
                            {error}
                          </p>
                        )}
                      </div>
                    )
                  })}
                </div>
              )}

              {/* 可选参数 */}
              {showAdvanced && optionalEnvVars.length > 0 && (
                <div className='space-y-4'>
                  <h4 className='font-medium text-gray-700 dark:text-gray-300'>
                    可选环境变量
                  </h4>
                  {optionalEnvVars.map((key) => {
                    const prop = envSchema.properties?.[key]
                    const error = errors[key]

                    return (
                      <div key={key} className='space-y-1'>
                        <div className='flex items-center gap-4'>
                          <label className='text-sm font-medium text-gray-700 dark:text-gray-300 min-w-32 flex-shrink-0'>
                            {prop?.title || key}
                          </label>
                          <input
                            type='text'
                            value={envValues[key] || ''}
                            onChange={(e) =>
                              handleEnvChange(key, e.target.value)
                            }
                            className={`flex-1 px-3 py-2 border rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 ${
                              error
                                ? 'border-red-500 dark:border-red-400'
                                : 'border-gray-300 dark:border-gray-600'
                            }`}
                            placeholder={
                              prop?.description || `请输入 ${key} (可选)`
                            }
                          />
                        </div>
                        {prop?.default !== undefined && (
                          <p className='text-xs text-blue-600 dark:text-blue-400 ml-36'>
                            默认值: {prop.default}
                          </p>
                        )}
                        {error && (
                          <p className='text-sm text-red-600 dark:text-red-400 ml-36'>
                            {error}
                          </p>
                        )}
                      </div>
                    )
                  })}
                </div>
              )}
            </div>
          )}

        {/* 无环境变量提示 */}
        {(!envSchema ||
          (requiredEnvVars.length === 0 && optionalEnvVars.length === 0)) && (
          <div className='bg-green-50 border border-green-200 rounded-lg p-4'>
            <div className='flex'>
              <div className='flex-shrink-0'>
                <span className='text-green-400 text-lg'>✓</span>
              </div>
              <div className='ml-3'>
                <h3 className='text-sm font-medium text-green-800'>
                  无需额外配置
                </h3>
                <div className='mt-2 text-sm text-green-700'>
                  该服务无需配置环境变量，可以直接安装。
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </Modal>
  )
}

export default InstallConfirmModal
