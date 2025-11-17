import {
  App,
  Button,
  Flex,
  Input,
  Modal,
  Popconfirm,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Tooltip,
  Typography,
  type TableProps,
} from 'antd'
import {
  AlertCircle,
  CheckCircle,
  Edit3,
  Plus,
  RotateCcw,
  Trash2,
  Wrench,
  XCircle,
} from 'lucide-react'
import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import ToolManager from '../components/ToolManager'
import { McpServerService } from '../services/mcp-server-service'
import type { McpServerInfo } from '../types'

const { TextArea } = Input
const { Title, Text } = Typography

interface McpServerManagerProps {
  onServiceChange?: () => void
}

const McpServerManager: React.FC<McpServerManagerProps> = ({
  onServiceChange,
}) => {
  const { t } = useTranslation()
  const { notification, message } = App.useApp()
  const [mcpServers, setMcpServers] = useState<McpServerInfo[]>([])
  const [showAddService, setShowAddService] = useState(false)
  const [showEditService, setShowEditService] = useState(false)
  const [editingService, setEditingService] = useState<McpServerInfo | null>(
    null,
  )
  const [showToolsModal, setShowToolsModal] = useState(false)
  const [selectedServiceForTools, setSelectedServiceForTools] =
    useState<McpServerInfo | null>(null)
  const [newServiceConfig, setNewServiceConfig] = useState({
    name: '',
    description: '',
    type: 'stdio' as 'stdio' | 'sse' | 'http',
    command: '',
    args: '',
    url: '',
    env: '',
    headers: '',
  })
  const [loading, setLoading] = useState(false)
  const [togglingServers, setTogglingServers] = useState<Set<string>>(new Set())

  // Add service mode: 'form' or 'json'
  const [addServiceMode, setAddServiceMode] = useState<'form' | 'json'>('form')
  const [jsonConfig, setJsonConfig] = useState('')
  const [jsonError, setJsonError] = useState('')

  useEffect(() => {
    fetchMcpServers()
  }, [])

  const fetchMcpServers = async () => {
    setLoading(true)
    try {
      const servers = await McpServerService.listMcpServers()
      setMcpServers(servers)
    } catch (error) {
      console.error('Failed to fetch MCP servers:', error)
      message.error(t('mcp_server.messages.fetch_servers_failed'))
    } finally {
      setLoading(false)
    }
  }

  const handleToggleServer = async (serverName: string) => {
    try {
      // 添加到 toggling 集合
      setTogglingServers((prev) => new Set(prev).add(serverName))

      const newState = await McpServerService.toggleMcpServer(serverName)

      message.success(
        t('mcp_server.messages.toggle_server_success', {
          name: serverName,
          action: newState
            ? t('mcp_server.status.enabled')
            : t('mcp_server.status.disabled'),
        }),
      )

      // 后端已经等待操作完成，现在重新加载服务列表
      await fetchMcpServers()
      onServiceChange?.()
    } catch (error) {
      console.error('Failed to toggle server:', error)

      // 提取错误信息，处理不同类型的错误
      let errorMessage = t('mcp_server.messages.toggle_server_failed')

      if (error instanceof Error) {
        errorMessage = error.message
      } else if (typeof error === 'string') {
        errorMessage = error
      } else if (error && typeof error === 'object') {
        // Tauri 错误可能是一个对象
        const errorObj = error as any
        if (errorObj.message) {
          errorMessage = errorObj.message
        } else if (errorObj.error) {
          errorMessage = errorObj.error
        } else {
          errorMessage = JSON.stringify(error)
        }
      }

      // 使用 notification 显示详细的错误信息
      notification.error({
        message: t('mcp_server.messages.server_operation_failed', {
          name: serverName,
        }),
        description: errorMessage,
        placement: 'topRight',
        duration: 5,
      })

      // 失败后也要刷新列表，以显示最新的失败状态
      await fetchMcpServers()
    } finally {
      // 从 toggling 集合中移除
      setTogglingServers((prev) => {
        const newSet = new Set(prev)
        newSet.delete(serverName)
        return newSet
      })
    }
  }

  const handleDeleteServer = async (serverName: string) => {
    try {
      await McpServerService.removeMcpServer(serverName)
      await fetchMcpServers()
      onServiceChange?.()
      message.success(t('mcp_server.messages.delete_server_success'))
    } catch (error) {
      console.error('Failed to delete server:', error)
      message.error(t('mcp_server.messages.delete_server_failed'))
    }
  }

  // Convert key-value pairs format to JSON object
  const keyValuePairsToJson = (pairsString: string): Record<string, string> => {
    const result: Record<string, string> = {}
    const lines = pairsString.split('\n').filter((line) => line.trim())

    lines.forEach((line) => {
      const equalIndex = line.indexOf('=')
      if (equalIndex > 0) {
        const key = line.substring(0, equalIndex).trim()
        const value = line.substring(equalIndex + 1).trim()
        if (key) {
          result[key] = value
        }
      }
    })

    return result
  }

  // Convert JSON object to key-value pairs format
  const jsonToKeyValuePairs = (jsonObject: Record<string, any>): string => {
    return Object.entries(jsonObject)
      .map(([key, value]) => `${key}=${value}`)
      .join('\n')
  }

  const handleEditServer = (server: McpServerInfo) => {
    setEditingService(server)
    setNewServiceConfig({
      name: server.name,
      description: server.description || '',
      type: server.type as any,
      command: server.command || '',
      args: server.args ? server.args.join(' ') : '',
      url: server.url || '',
      env: server.env ? jsonToKeyValuePairs(server.env) : '',
      headers: server.headers ? jsonToKeyValuePairs(server.headers) : '',
    })
    setShowEditService(true)
  }

  const handleAddService = async () => {
    try {
      if (addServiceMode === 'json') {
        try {
          const configData = JSON.parse(jsonConfig)

          // Validate JSON structure
          if (
            !configData.mcpServers ||
            typeof configData.mcpServers !== 'object'
          ) {
            setJsonError(
              t('mcp_server.messages.config_format_error_mcp_servers_missing'),
            )
            return
          }

          if (Object.keys(configData.mcpServers).length === 0) {
            setJsonError(
              t('mcp_server.messages.config_format_error_mcp_servers_empty'),
            )
            return
          }

          // Call the backend API with JSON data
          const result = await McpServerService.importMcpServersConfig(
            configData,
          )
          message.success(result)
          setShowAddService(false)
          resetForm()
          onServiceChange?.()
          await fetchMcpServers()
        } catch (error: any) {
          console.error('Failed to import JSON config:', error)
          if (error.code === 'JSON_PARSE_ERROR') {
            setJsonError(t('mcp_server.messages.json_invalid'))
          } else {
            setJsonError(
              error.message || t('mcp_server.messages.import_config_failed'),
            )
          }
          return
        }
      } else {
        // Convert environment variables and headers from key-value pairs to [string, string][] format
        const env = newServiceConfig.env
          ? Object.entries(keyValuePairsToJson(newServiceConfig.env))
          : []
        const headers = newServiceConfig.headers
          ? Object.entries(keyValuePairsToJson(newServiceConfig.headers))
          : []

        await McpServerService.addMcpServer(
          newServiceConfig.name,
          newServiceConfig.command,
          newServiceConfig.args
            ? newServiceConfig.args.split(' ').filter((arg) => arg.trim())
            : [],
          newServiceConfig.type,
          newServiceConfig.url || undefined,
          newServiceConfig.description || undefined,
          env,
          headers,
        )
        await fetchMcpServers()
        setShowAddService(false)
        resetForm()
        onServiceChange?.()
        message.success(t('mcp_server.messages.add_service_success'))
      }
    } catch (error) {
      console.error('Failed to add service:', error)
      message.error(t('mcp_server.messages.add_service_failed'))
    }
  }

  const handleUpdateService = async () => {
    if (!editingService) return

    try {
      // Parse environment variables and headers
      const env = newServiceConfig.env
        .split('\n')
        .filter((line) => line.trim())
        .map((line) => {
          const [key, value] = line.split('=')
          return [key.trim(), value?.trim() || ''] as [string, string]
        })

      const headers = newServiceConfig.headers
        .split('\n')
        .filter((line) => line.trim())
        .map((line) => {
          const [key, value] = line.split('=')
          return [key.trim(), value?.trim() || ''] as [string, string]
        })

      // Prepare command and args based on transport type
      const isStdio = newServiceConfig.type === 'stdio'
      const command = isStdio ? newServiceConfig.command : null
      const args = isStdio
        ? newServiceConfig.args.split(' ').filter((arg) => arg.trim())
        : null

      await McpServerService.updateMcpServer(
        newServiceConfig.name,
        command,
        args,
        newServiceConfig.type,
        newServiceConfig.url || null,
        newServiceConfig.description || null,
        env.length > 0 ? env : null,
        headers.length > 0 ? headers : null,
        editingService.enabled,
      )

      message.success(t('mcp_server.messages.update_service_success'))
      setShowEditService(false)
      setEditingService(null)
      resetForm()
      onServiceChange?.()
      await fetchMcpServers()
    } catch (error) {
      console.error('Failed to update service:', error)
      message.error(t('mcp_server.messages.update_service_failed'))
    }
  }

  const resetForm = () => {
    setNewServiceConfig({
      name: '',
      description: '',
      type: 'stdio',
      command: '',
      args: '',
      url: '',
      env: '',
      headers: '',
    })
    setJsonConfig('')
    setJsonError('')
    setAddServiceMode('form')
  }

  // Define table columns
  const columns: TableProps<McpServerInfo>['columns'] = [
    {
      title: t('mcp_server.table.service_name'),
      dataIndex: 'name',
      key: 'name',
      width: 250,
      ellipsis: true,
      filterSearch: true,
      // fixed: true,
      sorter: (a, b) => a.name.localeCompare(b.name),
      // 获取所有唯一的服务名称作为过滤器选项
      filters: Array.from(new Set(mcpServers.map((server) => server.name))).map(
        (name) => ({
          text: name,
          value: name,
        }),
      ),
      onFilter: (value: any, record: McpServerInfo) => record.name === value,
      render: (text: string) => <Text strong>{text}</Text>,
    },

    {
      title: t('mcp_server.table.protocol'),
      dataIndex: 'type',
      key: 'type',
      width: 100,
      filters: [
        { text: 'STDIO', value: 'stdio' },
        { text: 'SSE', value: 'sse' },
        { text: 'HTTP', value: 'http' },
      ],
      onFilter: (value: any, record: McpServerInfo) => record.type === value,
      // render: (transport: string) => {
      //   let tagColor = 'blue'

      //   switch (transport.toLowerCase()) {
      //     case 'stdio':
      //       tagColor = '#52c41a'
      //       break
      //     case 'sse':
      //       tagColor = '#1890ff'
      //       break
      //     case 'http':
      //       tagColor = '#faad14'
      //       break
      //     default:
      //       tagColor = '#d9d9d9'
      //   }

      //   return (
      //     <Tag color={tagColor} style={{ fontSize: '12px' }}>
      //       {transport.toUpperCase()}
      //     </Tag>
      //   )
      // },
    },
    {
      title: t('mcp_server.table.status'),
      dataIndex: 'status',
      key: 'status',
      width: 120,
      filters: [
        { text: t('mcp_server.status.connected'), value: 'connected' },
        { text: t('mcp_server.status.connecting'), value: 'connecting' },
        { text: t('mcp_server.status.disconnected'), value: 'disconnected' },
        { text: t('mcp_server.status.failed'), value: 'failed' },
      ],
      onFilter: (value, record: McpServerInfo) => record.status === value,
      render: (status: string, record: McpServerInfo) => {
        const getStatusConfig = (status: string) => {
          switch (status) {
            case 'connected':
              return {
                color: 'success',
                icon: <CheckCircle size={12} />,
                text: t('mcp_server.status.connected'),
              }
            case 'connecting':
              return {
                color: 'processing',
                icon: <RotateCcw size={12} />,
                text: t('mcp_server.status.connecting'),
              }
            case 'failed':
              return {
                color: 'error',
                icon: <AlertCircle size={12} />,
                text: t('mcp_server.status.failed'),
              }
            default:
              return {
                color: 'default',
                icon: <XCircle size={12} />,
                text: t('mcp_server.status.disconnected'),
              }
          }
        }

        const config = getStatusConfig(status)
        const statusElement = (
          <Tag color={config.color} style={{ fontSize: '12px', margin: 0 }}>
            <Flex align='center' gap={4}>
              {config.icon}
              {config.text}
            </Flex>
          </Tag>
        )

        // 如果是连接失败状态且有错误信息，显示错误详情 Tooltip
        if (status === 'failed' && record.error_message) {
          return (
            <Tooltip title={record.error_message} placement='topLeft'>
              {statusElement}
            </Tooltip>
          )
        }

        return statusElement
      },
    },
    {
      title: t('mcp_server.table.version'),
      dataIndex: 'version',
      key: 'version',
      width: 80,
      render: (version: string) => (
        <Text style={{ fontSize: '12px' }}>
          {version || t('mcp_server.empty.no_version')}
        </Text>
      ),
    },
    {
      title: t('mcp_server.table.tool_count'),
      dataIndex: 'tool_count',
      key: 'tool_count',
      width: 80,
      render: (count: number) => (
        <Tag color='purple' style={{ fontSize: '12px' }}>
          {count || 0}
        </Tag>
      ),
    },
    {
      title: t('mcp_server.table.command_url'),
      key: 'commandOrUrl',
      width: 350,
      render: (_, record: McpServerInfo) => {
        if (record.command) {
          const fullCommand =
            record.command +
            (record.args && record.args.length > 0
              ? ' ' + record.args.join(' ')
              : '')

          return (
            <Typography.Text
              style={{ fontSize: '12px' }}
              copyable={{ text: fullCommand }}
              code
              ellipsis>
              {record.command}{' '}
              {record.args && record.args.length > 0 && (
                <>{record.args.join(' ')}</>
              )}
            </Typography.Text>
          )
        }
        if (record.url) {
          return (
            <Typography.Link
              style={{ fontSize: '12px' }}
              copyable
              code
              ellipsis>
              {record.url}
            </Typography.Link>
          )
        }
        return <>{t('mcp_server.empty.no_command_url')}</>
      },
    },
    {
      title: t('mcp_server.table.description'),
      dataIndex: 'description',
      key: 'description',
      width: 200,
      ellipsis: true,
      render: (text: string) => (
        <Text style={{ fontSize: '12px' }}>
          {text || t('mcp_server.empty.no_description')}
        </Text>
      ),
    },
    {
      title: t('mcp_server.table.actions'),
      key: 'actions',
      width: 120,
      fixed: 'right',
      render: (_, record: McpServerInfo) => (
        <Space size='small'>
          <Switch
            size='small'
            checked={record.enabled}
            loading={togglingServers.has(record.name)}
            onChange={() => handleToggleServer(record.name)}
          />
          <Button
            size='small'
            type='text'
            icon={<Wrench size={12} />}
            onClick={() => {
              setSelectedServiceForTools(record)
              setShowToolsModal(true)
            }}
          />
          <Button
            size='small'
            type='text'
            icon={<Edit3 size={12} />}
            onClick={() => handleEditServer(record)}
          />
          <Popconfirm
            title={t('mcp_server.modals.delete_service_confirm')}
            description={t('mcp_server.modals.delete_service_description', {
              name: record.name,
            })}
            okText={t('mcp_server.modals.delete_service_ok')}
            cancelText={t('mcp_server.modals.delete_service_cancel')}
            okType='danger'
            onConfirm={() => handleDeleteServer(record.name)}>
            <Button
              size='small'
              type='text'
              danger
              icon={<Trash2 size={12} />}
            />
          </Popconfirm>
        </Space>
      ),
    },
  ]

  if (loading && mcpServers.length === 0) {
    return (
      <Flex justify='center' align='center' style={{ height: '256px' }}>
        <Button loading>{t('mcp_server.messages.loading_servers')}</Button>
      </Flex>
    )
  }

  return (
    <>
      <Flex vertical gap='middle' style={{ height: '100%', overflowY: 'auto' }}>
        {/* Add Service Button */}
        <Flex justify='flex-end'>
          <Space>
            <Button icon={<RotateCcw size={16} />} onClick={fetchMcpServers}>
              {t('mcp_server.actions.refresh')}
            </Button>
            <Button
              type='primary'
              icon={<Plus size={16} />}
              onClick={() => setShowAddService(true)}>
              {t('mcp_server.actions.add_service')}
            </Button>
          </Space>
        </Flex>

        {/* Server Table */}
        <Table
          columns={columns}
          dataSource={mcpServers}
          rowKey='name'
          loading={loading}
          scroll={{ x: 1200 }}
          pagination={false}
          sticky
          size='small'
          locale={{
            emptyText: (
              <Flex
                vertical
                align='center'
                style={{ textAlign: 'center', padding: '32px 16px' }}>
                <Title level={4} style={{ marginBottom: '8px' }}>
                  {t('mcp_server.messages.no_services_title')}
                </Title>
                <Text type='secondary' style={{ marginBottom: '16px' }}>
                  {t('mcp_server.messages.no_services_description')}
                </Text>
                <Button
                  onClick={() => setShowAddService(true)}
                  type='primary'
                  icon={<Plus size={16} />}>
                  {t('mcp_server.messages.add_first_service')}
                </Button>
              </Flex>
            ),
          }}
        />
      </Flex>

      {/* Add Service Modal */}
      <Modal
        title={t('mcp_server.modals.add_service_title')}
        open={showAddService}
        onCancel={() => {
          setShowAddService(false)
          resetForm()
        }}
        footer={[
          <Button key='cancel' onClick={() => setShowAddService(false)}>
            {t('mcp_server.actions.cancel')}
          </Button>,
          <Button key='add' type='primary' onClick={handleAddService}>
            {t('mcp_server.actions.add_service')}
          </Button>,
        ]}
        width={640}>
        <Flex vertical gap='middle'>
          {/* Mode Tabs */}
          <Flex>
            <Button
              type={addServiceMode === 'form' ? 'primary' : 'text'}
              onClick={() => setAddServiceMode('form')}
              style={{ marginRight: '8px' }}>
              {t('mcp_server.form.form_config')}
            </Button>
            <Button
              type={addServiceMode === 'json' ? 'primary' : 'text'}
              onClick={() => setAddServiceMode('json')}>
              {t('mcp_server.form.json_config')}
            </Button>
          </Flex>

          {addServiceMode === 'form' ? (
            <Flex vertical gap='middle'>
              <div>
                <Text strong>
                  {t('mcp_server.form.service_name')}{' '}
                  <Text type='danger'>*</Text>
                </Text>
                <Input
                  value={newServiceConfig.name}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      name: e.target.value,
                    })
                  }
                  placeholder={t('mcp_server.form.service_name_placeholder')}
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>{t('mcp_server.form.description')}</Text>
                <Input
                  value={newServiceConfig.description}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      description: e.target.value,
                    })
                  }
                  placeholder={t('mcp_server.form.description_placeholder')}
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>
                  {t('mcp_server.form.transport_protocol')}{' '}
                  <Text type='danger'>*</Text>
                </Text>
                <Select
                  value={newServiceConfig.type}
                  onChange={(value) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      type: value as any,
                    })
                  }
                  options={[
                    {
                      value: 'stdio',
                      label: t('mcp_server.protocol_types.stdio'),
                    },
                    { value: 'sse', label: t('mcp_server.protocol_types.sse') },
                    {
                      value: 'http',
                      label: t('mcp_server.protocol_types.http'),
                    },
                  ]}
                  style={{ marginTop: '4px', width: '100%' }}
                />
              </div>

              {newServiceConfig.type === 'stdio' && (
                <>
                  <div>
                    <Text strong>
                      {t('mcp_server.form.command')}{' '}
                      <Text type='danger'>*</Text>
                    </Text>
                    <Input
                      value={newServiceConfig.command}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          command: e.target.value,
                        })
                      }
                      placeholder={t('mcp_server.form.command_placeholder')}
                      style={{ marginTop: '4px' }}
                    />
                  </div>

                  <div>
                    <Text strong>{t('mcp_server.form.args')}</Text>
                    <Input
                      value={newServiceConfig.args}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          args: e.target.value,
                        })
                      }
                      placeholder={t('mcp_server.form.args_placeholder')}
                      style={{ marginTop: '4px' }}
                    />
                  </div>

                  <div>
                    <Text strong>{t('mcp_server.form.env_vars')}</Text>
                    <TextArea
                      value={newServiceConfig.env}
                      onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          env: e.target.value,
                        })
                      }
                      placeholder={t('mcp_server.form.env_vars_placeholder')}
                      rows={4}
                      style={{ marginTop: '4px' }}
                    />
                    <Text
                      type='secondary'
                      style={{
                        fontSize: '12px',
                        marginTop: '4px',
                        display: 'block',
                      }}>
                      {t('mcp_server.form.env_vars_help')}
                    </Text>
                  </div>
                </>
              )}

              {(newServiceConfig.type === 'sse' ||
                newServiceConfig.type === 'http') && (
                <>
                  <div>
                    <Text strong>
                      {t('mcp_server.form.service_url')}{' '}
                      <Text type='danger'>*</Text>
                    </Text>
                    <Input
                      value={newServiceConfig.url}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          url: e.target.value,
                        })
                      }
                      placeholder={t('mcp_server.form.service_url_placeholder')}
                      style={{ marginTop: '4px' }}
                    />
                  </div>

                  <div>
                    <Text strong>{t('mcp_server.form.headers')}</Text>
                    <TextArea
                      value={newServiceConfig.headers}
                      onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          headers: e.target.value,
                        })
                      }
                      placeholder={t('mcp_server.form.headers_placeholder')}
                      rows={4}
                      style={{ marginTop: '4px' }}
                    />
                    <Text
                      type='secondary'
                      style={{
                        fontSize: '12px',
                        marginTop: '4px',
                        display: 'block',
                      }}>
                      {t('mcp_server.form.headers_help')}
                    </Text>
                  </div>
                </>
              )}
            </Flex>
          ) : (
            <Flex vertical gap='middle'>
              <div>
                <Text strong>
                  {t('mcp_server.form.json_config')}{' '}
                  <Text type='danger'>*</Text>
                </Text>
                <TextArea
                  value={jsonConfig}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                    setJsonConfig(e.target.value)
                  }
                  placeholder={t('mcp_server.form.json_placeholder')}
                  rows={12}
                  style={{ marginTop: '4px' }}
                />
                <div
                  style={{
                    fontSize: '12px',
                    marginTop: '4px',
                    display: 'block',
                    color: 'rgba(0, 0, 0, 0.45)',
                  }}
                  dangerouslySetInnerHTML={{
                    __html: t('mcp_server.form.json_config_help'),
                  }}
                />
              </div>
              {jsonError && (
                <Text type='danger' style={{ fontSize: '14px' }}>
                  {jsonError}
                </Text>
              )}
            </Flex>
          )}
        </Flex>
      </Modal>

      {/* Edit Service Modal */}
      <Modal
        title={t('mcp_server.modals.edit_service_title')}
        open={showEditService}
        onCancel={() => {
          setShowEditService(false)
          setEditingService(null)
          resetForm()
        }}
        footer={[
          <Button key='cancel' onClick={() => setShowEditService(false)}>
            {t('mcp_server.actions.cancel')}
          </Button>,
          <Button key='save' type='primary' onClick={handleUpdateService}>
            {t('mcp_server.actions.save_changes')}
          </Button>,
        ]}
        width={640}>
        <Flex vertical gap='middle'>
          <div>
            <Text strong>{t('mcp_server.form.service_name')}</Text>
            <Input
              value={newServiceConfig.name}
              onChange={(e) =>
                setNewServiceConfig({
                  ...newServiceConfig,
                  name: e.target.value,
                })
              }
              disabled
              style={{ marginTop: '4px' }}
            />
          </div>

          <div>
            <Text strong>{t('mcp_server.form.description')}</Text>
            <Input
              value={newServiceConfig.description}
              onChange={(e) =>
                setNewServiceConfig({
                  ...newServiceConfig,
                  description: e.target.value,
                })
              }
              placeholder={t('mcp_server.form.description_placeholder')}
              style={{ marginTop: '4px' }}
            />
          </div>

          <div>
            <Text strong>{t('mcp_server.form.transport_protocol')}</Text>
            <Select
              value={newServiceConfig.type}
              onChange={(value) =>
                setNewServiceConfig({
                  ...newServiceConfig,
                  type: value as any,
                })
              }
              options={[
                { value: 'stdio', label: t('mcp_server.protocol_types.stdio') },
                { value: 'sse', label: t('mcp_server.protocol_types.sse') },
                { value: 'http', label: t('mcp_server.protocol_types.http') },
              ]}
              style={{ marginTop: '4px', width: '100%' }}
            />
          </div>

          {newServiceConfig.type === 'stdio' && (
            <>
              <div>
                <Text strong>{t('mcp_server.form.command')}</Text>
                <Input
                  value={newServiceConfig.command}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      command: e.target.value,
                    })
                  }
                  placeholder={t('mcp_server.form.command_placeholder')}
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>{t('mcp_server.form.args')}</Text>
                <Input
                  value={newServiceConfig.args}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      args: e.target.value,
                    })
                  }
                  placeholder={t('mcp_server.form.args_placeholder')}
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>{t('mcp_server.form.env_vars')}</Text>
                <TextArea
                  value={newServiceConfig.env}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      env: e.target.value,
                    })
                  }
                  placeholder={t('mcp_server.form.env_vars_placeholder')}
                  rows={4}
                  style={{ marginTop: '4px' }}
                />
                <Text
                  type='secondary'
                  style={{
                    fontSize: '12px',
                    marginTop: '4px',
                    display: 'block',
                  }}>
                  {t('mcp_server.form.env_vars_help')}
                </Text>
              </div>
            </>
          )}

          {(newServiceConfig.type === 'sse' ||
            newServiceConfig.type === 'http') && (
            <>
              <div>
                <Text strong>{t('mcp_server.form.service_url')}</Text>
                <Input
                  value={newServiceConfig.url}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      url: e.target.value,
                    })
                  }
                  placeholder={t('mcp_server.form.service_url_placeholder')}
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>{t('mcp_server.form.headers')}</Text>
                <TextArea
                  value={newServiceConfig.headers}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      headers: e.target.value,
                    })
                  }
                  placeholder={t('mcp_server.form.headers_placeholder')}
                  rows={4}
                  style={{ marginTop: '4px' }}
                />
                <Text
                  type='secondary'
                  style={{
                    fontSize: '12px',
                    marginTop: '4px',
                    display: 'block',
                  }}>
                  {t('mcp_server.form.headers_help')}
                </Text>
              </div>
            </>
          )}
        </Flex>
      </Modal>

      {/* Tools Modal */}
      <Modal
        title={t('mcp_server.modals.manage_tools_title')}
        open={showToolsModal}
        onCancel={() => {
          setShowToolsModal(false)
          setSelectedServiceForTools(null)
        }}
        footer={null}
        width={1024}
        styles={{
          body: {
            padding: '16px',
            maxHeight: '70vh',
            overflowY: 'auto',
          },
        }}>
        {selectedServiceForTools && (
          <ToolManager mcpServer={selectedServiceForTools as any} />
        )}
      </Modal>
    </>
  )
}

export default McpServerManager
