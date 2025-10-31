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
    transport: 'stdio' as 'stdio' | 'sse' | 'http',
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
      message.error('获取 MCP 服务器列表失败')
    } finally {
      setLoading(false)
    }
  }

  const handleToggleServer = async (serverName: string) => {
    try {
      // 添加到 toggling 集合
      setTogglingServers((prev) => new Set(prev).add(serverName))

      const newState = await McpServerService.toggleMcpServer(serverName)

      message.success(`服务器 "${serverName}" 已${newState ? '启用' : '禁用'}`)

      // 后端已经等待操作完成，现在重新加载服务列表
      await fetchMcpServers()
      onServiceChange?.()
    } catch (error) {
      console.error('Failed to toggle server:', error)

      // 提取错误信息，处理不同类型的错误
      let errorMessage = '切换服务器状态失败，请检查服务配置'

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
        message: `服务器 "${serverName}" 操作失败`,
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
      message.success('服务器已删除')
    } catch (error) {
      console.error('Failed to delete server:', error)
      message.error('删除服务器失败')
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
      transport: server.transport as any,
      command: server.command || '',
      args: server.args ? server.args.join(' ') : '',
      url: server.url || '',
      env: server.env_vars ? jsonToKeyValuePairs(server.env_vars) : '',
      headers: server.headers ? jsonToKeyValuePairs(server.headers) : '',
    })
    setShowEditService(true)
  }

  const handleAddService = async () => {
    try {
      if (addServiceMode === 'json') {
        try {
          JSON.parse(jsonConfig)
          // Call the backend API with JSON data
          // TODO: Implement actual JSON import functionality
          message.success('服务已添加')
          setShowAddService(false)
          resetForm()
          onServiceChange?.()
          await fetchMcpServers()
        } catch (error) {
          setJsonError('JSON 格式无效，请检查配置')
          return
        }
      } else {
        // Convert environment variables and headers from key-value pairs to [string, string][] format
        const envVars = newServiceConfig.env
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
          newServiceConfig.transport,
          newServiceConfig.url || undefined,
          newServiceConfig.description || undefined,
          envVars,
          headers,
        )
        await fetchMcpServers()
        setShowAddService(false)
        resetForm()
        onServiceChange?.()
        message.success('服务已添加')
      }
    } catch (error) {
      console.error('Failed to add service:', error)
      message.error('添加服务失败')
    }
  }

  const handleUpdateService = async () => {
    if (!editingService) return

    try {
      // Parse environment variables and headers
      const env_vars = newServiceConfig.env
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
      const isStdio = newServiceConfig.transport === 'stdio'
      const command = isStdio ? newServiceConfig.command : null
      const args = isStdio
        ? newServiceConfig.args.split(' ').filter((arg) => arg.trim())
        : null

      await McpServerService.updateMcpServer(
        newServiceConfig.name,
        command,
        args,
        newServiceConfig.transport,
        newServiceConfig.url || null,
        newServiceConfig.description || null,
        env_vars.length > 0 ? env_vars : null,
        headers.length > 0 ? headers : null,
        editingService.enabled,
      )

      message.success('服务已更新')
      setShowEditService(false)
      setEditingService(null)
      resetForm()
      onServiceChange?.()
      await fetchMcpServers()
    } catch (error) {
      console.error('Failed to update service:', error)
      message.error('更新服务失败')
    }
  }

  const resetForm = () => {
    setNewServiceConfig({
      name: '',
      description: '',
      transport: 'stdio',
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
      title: '服务名称',
      dataIndex: 'name',
      key: 'name',
      width: 250,
      ellipsis: true,
      filterSearch: true,
      fixed: true,
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
      title: '协议',
      dataIndex: 'transport',
      key: 'transport',
      width: 100,
      filters: [
        { text: 'STDIO', value: 'stdio' },
        { text: 'SSE', value: 'sse' },
        { text: 'HTTP', value: 'http' },
      ],
      onFilter: (value: any, record: McpServerInfo) =>
        record.transport === value,
      render: (transport: string) => {
        let tagColor = 'blue'

        switch (transport.toLowerCase()) {
          case 'stdio':
            tagColor = '#52c41a'
            break
          case 'sse':
            tagColor = '#1890ff'
            break
          case 'http':
            tagColor = '#faad14'
            break
          default:
            tagColor = '#d9d9d9'
        }

        return (
          <Tag color={tagColor} style={{ fontSize: '12px' }}>
            {transport.toUpperCase()}
          </Tag>
        )
      },
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 90,
      filters: [
        { text: '已连接', value: 'connected' },
        { text: '连接中', value: 'connecting' },
        { text: '已断开', value: 'disconnected' },
        { text: '连接失败', value: 'failed' },
      ],
      onFilter: (value, record: McpServerInfo) => record.status === value,
      render: (status: string, record: McpServerInfo) => {
        const getStatusConfig = (status: string) => {
          switch (status) {
            case 'connected':
              return {
                color: 'success',
                icon: <CheckCircle size={12} />,
                text: '已连接',
              }
            case 'connecting':
              return {
                color: 'processing',
                icon: <RotateCcw size={12} />,
                text: '连接中',
              }
            case 'failed':
              return {
                color: 'error',
                icon: <AlertCircle size={12} />,
                text: '连接失败',
              }
            default:
              return {
                color: 'default',
                icon: <XCircle size={12} />,
                text: '已断开',
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
      title: '版本',
      dataIndex: 'version',
      key: 'version',
      width: 80,
      render: (version: string) => (
        <Text style={{ fontSize: '12px' }}>{version || '-'}</Text>
      ),
    },
    {
      title: '工具数量',
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
      title: '命令/URL',
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
        return <>-</>
      },
    },
    {
      title: '描述',
      dataIndex: 'description',
      key: 'description',
      width: 200,
      ellipsis: true,
      render: (text: string) => (
        <Text style={{ fontSize: '12px' }}>{text || '-'}</Text>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 200,
      fixed: 'right',
      render: (_, record: McpServerInfo) => (
        <Space size='small'>
          <Switch
            size='small'
            checked={record.enabled}
            loading={togglingServers.has(record.name)}
            onChange={() => handleToggleServer(record.name)}
            checkedChildren='启用'
            unCheckedChildren='禁用'
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
            title='删除服务'
            description={`确定要删除服务 "${record.name}" 吗？此操作不可撤销。`}
            okText='确认'
            cancelText='取消'
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
        <Button loading>加载 MCP 服务器...</Button>
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
              刷新
            </Button>
            <Button
              type='primary'
              icon={<Plus size={16} />}
              onClick={() => setShowAddService(true)}>
              添加服务
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
                  暂无 MCP 服务
                </Title>
                <Text type='secondary' style={{ marginBottom: '16px' }}>
                  添加您的第一个 MCP 服务来开始使用
                </Text>
                <Button
                  onClick={() => setShowAddService(true)}
                  type='primary'
                  icon={<Plus size={16} />}>
                  添加服务
                </Button>
              </Flex>
            ),
          }}
        />
      </Flex>

      {/* Add Service Modal */}
      <Modal
        title='添加 MCP 服务'
        open={showAddService}
        onCancel={() => {
          setShowAddService(false)
          resetForm()
        }}
        footer={[
          <Button key='cancel' onClick={() => setShowAddService(false)}>
            取消
          </Button>,
          <Button key='add' type='primary' onClick={handleAddService}>
            添加服务
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
              表单配置
            </Button>
            <Button
              type={addServiceMode === 'json' ? 'primary' : 'text'}
              onClick={() => setAddServiceMode('json')}>
              JSON 配置
            </Button>
          </Flex>

          {addServiceMode === 'form' ? (
            <Flex vertical gap='middle'>
              <div>
                <Text strong>
                  服务名称 <Text type='danger'>*</Text>
                </Text>
                <Input
                  value={newServiceConfig.name}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      name: e.target.value,
                    })
                  }
                  placeholder='例如: my-mcp-server'
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>描述</Text>
                <Input
                  value={newServiceConfig.description}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      description: e.target.value,
                    })
                  }
                  placeholder='服务的简要描述'
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>
                  传输协议 <Text type='danger'>*</Text>
                </Text>
                <Select
                  value={newServiceConfig.transport}
                  onChange={(value) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      transport: value as any,
                    })
                  }
                  options={[
                    { value: 'stdio', label: 'STDIO (标准输入输出)' },
                    { value: 'sse', label: 'SSE (服务器发送事件)' },
                    { value: 'http', label: 'HTTP' },
                  ]}
                  style={{ marginTop: '4px', width: '100%' }}
                />
              </div>

              {newServiceConfig.transport === 'stdio' && (
                <>
                  <div>
                    <Text strong>
                      命令 <Text type='danger'>*</Text>
                    </Text>
                    <Input
                      value={newServiceConfig.command}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          command: e.target.value,
                        })
                      }
                      placeholder='例如: python main.py'
                      style={{ marginTop: '4px' }}
                    />
                  </div>

                  <div>
                    <Text strong>参数</Text>
                    <Input
                      value={newServiceConfig.args}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          args: e.target.value,
                        })
                      }
                      placeholder='例如: --port 3000 --debug'
                      style={{ marginTop: '4px' }}
                    />
                  </div>

                  <div>
                    <Text strong>环境变量</Text>
                    <TextArea
                      value={newServiceConfig.env}
                      onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          env: e.target.value,
                        })
                      }
                      placeholder={
                        'API_KEY=your-api-key\nDEBUG=true\nPORT=3000'
                      }
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
                      键值对格式，每行一个，例如: API_KEY=your-api-key
                    </Text>
                  </div>
                </>
              )}

              {(newServiceConfig.transport === 'sse' ||
                newServiceConfig.transport === 'http') && (
                <>
                  <div>
                    <Text strong>
                      服务 URL <Text type='danger'>*</Text>
                    </Text>
                    <Input
                      value={newServiceConfig.url}
                      onChange={(e) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          url: e.target.value,
                        })
                      }
                      placeholder='例如: http://localhost:3000/mcp'
                      style={{ marginTop: '4px' }}
                    />
                  </div>

                  <div>
                    <Text strong>Headers</Text>
                    <TextArea
                      value={newServiceConfig.headers}
                      onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                        setNewServiceConfig({
                          ...newServiceConfig,
                          headers: e.target.value,
                        })
                      }
                      placeholder={
                        'Authorization=Bearer token\nContent-Type=application/json\nX-Custom-Header=value'
                      }
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
                      键值对格式，每行一个，例如: Content-Type=application/json
                    </Text>
                  </div>
                </>
              )}
            </Flex>
          ) : (
            <Flex vertical gap='middle'>
              <div>
                <Text strong>
                  JSON 配置 <Text type='danger'>*</Text>
                </Text>
                <TextArea
                  value={jsonConfig}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                    setJsonConfig(e.target.value)
                  }
                  placeholder={
                    '{"name": "my-server", "transport": "stdio", ...}'
                  }
                  rows={12}
                  style={{ marginTop: '4px' }}
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
        title='编辑 MCP 服务'
        open={showEditService}
        onCancel={() => {
          setShowEditService(false)
          setEditingService(null)
          resetForm()
        }}
        footer={[
          <Button key='cancel' onClick={() => setShowEditService(false)}>
            取消
          </Button>,
          <Button key='save' type='primary' onClick={handleUpdateService}>
            保存更改
          </Button>,
        ]}
        width={640}>
        <Flex vertical gap='middle'>
          <div>
            <Text strong>服务名称</Text>
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
            <Text strong>描述</Text>
            <Input
              value={newServiceConfig.description}
              onChange={(e) =>
                setNewServiceConfig({
                  ...newServiceConfig,
                  description: e.target.value,
                })
              }
              placeholder='服务的简要描述'
              style={{ marginTop: '4px' }}
            />
          </div>

          <div>
            <Text strong>传输协议</Text>
            <Select
              value={newServiceConfig.transport}
              onChange={(value) =>
                setNewServiceConfig({
                  ...newServiceConfig,
                  transport: value as any,
                })
              }
              options={[
                { value: 'stdio', label: 'STDIO (标准输入输出)' },
                { value: 'sse', label: 'SSE (服务器发送事件)' },
                { value: 'streamablehttp', label: 'Streamable HTTP' },
              ]}
              style={{ marginTop: '4px', width: '100%' }}
            />
          </div>

          {newServiceConfig.transport === 'stdio' && (
            <>
              <div>
                <Text strong>命令</Text>
                <Input
                  value={newServiceConfig.command}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      command: e.target.value,
                    })
                  }
                  placeholder='例如: python main.py'
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>参数</Text>
                <Input
                  value={newServiceConfig.args}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      args: e.target.value,
                    })
                  }
                  placeholder='例如: --port 3000 --debug'
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>环境变量</Text>
                <TextArea
                  value={newServiceConfig.env}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      env: e.target.value,
                    })
                  }
                  placeholder={'API_KEY=your-api-key\nDEBUG=true\nPORT=3000'}
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
                  键值对格式，每行一个，例如: API_KEY=your-api-key
                </Text>
              </div>
            </>
          )}

          {(newServiceConfig.transport === 'sse' ||
            newServiceConfig.transport === 'http') && (
            <>
              <div>
                <Text strong>服务 URL</Text>
                <Input
                  value={newServiceConfig.url}
                  onChange={(e) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      url: e.target.value,
                    })
                  }
                  placeholder='例如: http://localhost:3000/mcp'
                  style={{ marginTop: '4px' }}
                />
              </div>

              <div>
                <Text strong>Headers</Text>
                <TextArea
                  value={newServiceConfig.headers}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                    setNewServiceConfig({
                      ...newServiceConfig,
                      headers: e.target.value,
                    })
                  }
                  placeholder={
                    'Authorization=Bearer token\nContent-Type=application/json\nX-Custom-Header=value'
                  }
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
                  键值对格式，每行一个，例如: Content-Type=application/json
                </Text>
              </div>
            </>
          )}
        </Flex>
      </Modal>

      {/* Tools Modal */}
      <Modal
        title='管理 MCP 服务器工具'
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
