import {
  Button,
  Card,
  Flex,
  Space,
  Tag,
  Typography,
  Progress,
  Tooltip,
  App
} from 'antd'
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  DownloadOutlined,
  ReloadOutlined,
  InfoCircleOutlined,
  WarningOutlined
} from '@ant-design/icons'
import { memo, useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ToolInfo, PythonRuntimeInfo } from '../types'
import { ToolStatus } from '../types'
import { ToolManagerService } from '../services/tool-manager-service'

const { Title, Text } = Typography

interface SystemToolManagerProps {
  className?: string
}

const SystemToolManager: React.FC<SystemToolManagerProps> = memo(({ className }) => {
  const { t } = useTranslation()
  const { message: antMessage } = App.useApp()

  const [tools, setTools] = useState<ToolInfo[]>([])
  const [pythonRuntime, setPythonRuntime] = useState<PythonRuntimeInfo>({ available: false })
  const [loading, setLoading] = useState(false)
  const [installing, setInstalling] = useState<{ [key: string]: boolean }>({})

  // 加载工具信息
  const loadToolsInfo = useCallback(async () => {
    setLoading(true)
    try {
      const [toolsData, pythonData] = await Promise.all([
        ToolManagerService.getToolsInfo(),
        ToolManagerService.checkPythonRuntime()
      ])
      setTools(toolsData)
      setPythonRuntime(pythonData)
    } catch (error) {
      console.error('Failed to load tools info:', error)
      antMessage.error(t('tool_manager.errors.load_tools_failed'))
    } finally {
      setLoading(false)
    }
  }, [antMessage])

  // 安装所有工具
  const handleInstallAllTools = useCallback(async () => {
    setInstalling({ all: true })
    try {
      await ToolManagerService.installAllTools()
      antMessage.success(t('tool_manager.messages.install_all_success'))
      // 重新加载工具信息
      await loadToolsInfo()
    } catch (error) {
      console.error('Failed to install all tools:', error)
      antMessage.error(t('tool_manager.errors.install_all_failed'))
    } finally {
      setInstalling({})
    }
  }, [antMessage, loadToolsInfo])

  // 安装特定工具
  const handleInstallTool = useCallback(async (toolName: string) => {
    setInstalling(prev => ({ ...prev, [toolName]: true }))
    try {
      await ToolManagerService.installTool(toolName)
      antMessage.success(t('tool_manager.messages.install_success', { tool: toolName }))
      // 重新加载工具信息
      await loadToolsInfo()
    } catch (error) {
      console.error(`Failed to install ${toolName}:`, error)
      antMessage.error(t('tool_manager.errors.install_failed', { tool: toolName }))
    } finally {
      setInstalling(prev => ({ ...prev, [toolName]: false }))
    }
  }, [antMessage, loadToolsInfo])

  // 获取状态图标和颜色
  const getStatusConfig = (status: ToolStatus) => {
    switch (status) {
      case ToolStatus.Installed:
        return {
          icon: <CheckCircleOutlined />,
          color: 'success',
          text: t('tool_manager.status.installed')
        }
      case ToolStatus.NotInstalled:
        return {
          icon: <CloseCircleOutlined />,
          color: 'error',
          text: t('tool_manager.status.not_installed')
        }
      case ToolStatus.Installing:
        return {
          icon: <DownloadOutlined />,
          color: 'processing',
          text: t('tool_manager.status.installing')
        }
      case ToolStatus.Error:
        return {
          icon: <WarningOutlined />,
          color: 'warning',
          text: t('tool_manager.status.error')
        }
      case ToolStatus.Outdated:
        return {
          icon: <InfoCircleOutlined />,
          color: 'warning',
          text: t('tool_manager.status.outdated')
        }
      default:
        return {
          icon: <CloseCircleOutlined />,
          color: 'default',
          text: status
        }
    }
  }

  // 组件挂载时加载数据
  useEffect(() => {
    loadToolsInfo()
  }, [loadToolsInfo])

  if (loading) {
    return (
      <Flex justify="center" align="center" style={{ height: '200px' }}>
        <Button loading>{t('tool_manager.loading')}</Button>
      </Flex>
    )
  }

  return (
    <div className={className}>
      <Flex vertical gap="large">
        {/* 标题和操作 */}
        <Flex justify="space-between" align="center">
          <Title level={4} style={{ margin: 0 }}>
            {t('tool_manager.title')}
          </Title>
          <Button
            type="primary"
            icon={<DownloadOutlined />}
            loading={installing.all}
            onClick={handleInstallAllTools}
          >
            {t('tool_manager.actions.install_all')}
          </Button>
        </Flex>

        {/* Python 运行时信息 */}
        <Card size="small">
          <Flex justify="space-between" align="center">
            <div>
              <Text strong>{t('tool_manager.python_runtime.title')}</Text>
              <br />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {t('tool_manager.python_runtime.description')}
              </Text>
            </div>
            <Flex align="center" gap="small">
              {pythonRuntime.available ? (
                <>
                  <CheckCircleOutlined style={{ color: '#52c41a' }} />
                  <Text>
                    {t('tool_manager.python_runtime.available')}
                    {pythonRuntime.version && ` (${pythonRuntime.version})`}
                  </Text>
                </>
              ) : (
                <>
                  <CloseCircleOutlined style={{ color: '#ff4d4f' }} />
                  <Text type="danger">{t('tool_manager.python_runtime.not_available')}</Text>
                </>
              )}
            </Flex>
          </Flex>
        </Card>

        {/* 工具列表 */}
        <div>
          {tools.map((tool) => {
            const statusConfig = getStatusConfig(tool.status)
            const isInstalling = installing[tool.name] || installing.all

            return (
              <Card
                key={tool.name}
                size="small"
                style={{ marginBottom: '12px' }}
                styles={{ body: { padding: '16px' } }}
              >
                <Flex justify="space-between" align="center">
                  <div style={{ flex: 1 }}>
                    <Flex align="center" gap="small" style={{ marginBottom: '8px' }}>
                      <Text strong>{tool.name}</Text>
                      <Tag color={statusConfig.color} icon={statusConfig.icon}>
                        {statusConfig.text}
                      </Tag>
                      {tool.python_required && (
                        <Tooltip title={t('tool_manager.python_required')}>
                          <InfoCircleOutlined style={{ color: '#1890ff' }} />
                        </Tooltip>
                      )}
                    </Flex>

                    <Text type="secondary" style={{ fontSize: '12px', display: 'block', marginBottom: '4px' }}>
                      {tool.full_name}
                    </Text>

                    {tool.version && (
                      <Text type="secondary" style={{ fontSize: '12px', display: 'block', marginBottom: '4px' }}>
                        {t('tool_manager.version')}: {tool.version}
                      </Text>
                    )}

                    <Text code style={{ fontSize: '11px', display: 'block' }}>
                      {tool.path}
                    </Text>
                  </div>

                  <Space>
                    {tool.status === ToolStatus.NotInstalled && !isInstalling && (
                      <Button
                        size="small"
                        type="primary"
                        icon={<DownloadOutlined />}
                        loading={isInstalling}
                        onClick={() => handleInstallTool(tool.name)}
                      >
                        {t('tool_manager.actions.install')}
                      </Button>
                    )}

                    {tool.status === ToolStatus.Installed && !isInstalling && (
                      <Button
                        size="small"
                        icon={<ReloadOutlined />}
                        loading={isInstalling}
                        onClick={() => handleInstallTool(tool.name)}
                      >
                        {t('tool_manager.actions.reinstall')}
                      </Button>
                    )}

                    {isInstalling && installing[tool.name] && (
                      <Progress
                        type="circle"
                        percent={100}
                        size="small"
                        showInfo={false}
                        style={{ marginLeft: '8px' }}
                      />
                    )}
                  </Space>
                </Flex>
              </Card>
            )
          })}
        </div>
      </Flex>
    </div>
  )
})

SystemToolManager.displayName = 'SystemToolManager'

export default SystemToolManager