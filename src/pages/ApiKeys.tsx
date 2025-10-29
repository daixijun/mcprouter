import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import {
  Badge,
  Button,
  Card,
  Flex,
  Input,
  Modal,
  Popconfirm,
  Space,
  Table,
  Typography,
  App,
} from 'antd'
import { Check, Copy, Key, Plus, Trash2 } from 'lucide-react'
import { memo, useCallback, useEffect, useState } from 'react'
import ApiKeyPermissionSelector from '../components/ApiKeyPermissionSelector'
import { useErrorContext } from '../contexts/ErrorContext'
import { ApiKeyService } from '../services/api-key-service'
import type { ApiKey, ApiKeyPermissions } from '../types'

const { Title, Text } = Typography

// 类型定义
interface ApiKeyWithCount extends ApiKey {
  tool_count?: number
}

const ApiKeys: React.FC = memo(() => {
  const { addError } = useErrorContext()
  const { message } = App.useApp()

  // State
  const [apiKeys, setApiKeys] = useState<ApiKeyWithCount[]>([])
  const [loading, setLoading] = useState(true)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [showDetailsModal, setShowDetailsModal] = useState(false)
  const [showEditModal, setShowEditModal] = useState(false)
  const [selectedApiKey, setSelectedApiKey] = useState<ApiKey | null>(null)
  const [newKeyDetails, setNewKeyDetails] = useState<ApiKey | null>(null)
  const [copied, setCopied] = useState(false)

  // Form state for new API key
  const [newKeyName, setNewKeyName] = useState('')
  const [newKeyPermissions, setNewKeyPermissions] = useState<ApiKeyPermissions>(
    {
      allowed_servers: [],
      allowed_tools: [],
    },
  )

  // Form state for editing permissions
  const [editPermissions, setEditPermissions] = useState<ApiKeyPermissions>({
    allowed_servers: [],
    allowed_tools: [],
  })

  // Data fetching
  const loadApiKeys = useCallback(async () => {
    setLoading(true)
    try {
      const keys = await ApiKeyService.listApiKeys()

      // Load tool counts for each API key
      const keysWithCounts = await Promise.all(
        keys.map(async (key) => {
          try {
            const toolIds = await ApiKeyService.getApiKeyTools(key.id)
            return { ...key, tool_count: toolIds.length }
          } catch (error) {
            console.error(`Failed to load tool count for key ${key.id}:`, error)
            return key
          }
        }),
      )

      setApiKeys(keysWithCounts)
    } catch (error) {
      console.error('Failed to load API keys:', error)
      addError('加载API Key列表失败')
    } finally {
      setLoading(false)
    }
  }, [addError])

  useEffect(() => {
    loadApiKeys()
  }, [loadApiKeys])

  // Action handlers
  const handleCreateApiKey = useCallback(async () => {
    if (!newKeyName.trim()) {
      addError('请输入API Key名称')
      return
    }

    try {
      const createdKey = await ApiKeyService.createApiKey(
        newKeyName,
        newKeyPermissions,
      )
      message.success('API Key创建成功')

      // Show the full key in details modal
      setNewKeyDetails(createdKey)
      setShowCreateModal(false)
      setShowDetailsModal(true)

      // Reset form
      setNewKeyName('')
      setNewKeyPermissions({
        allowed_servers: [],
        allowed_tools: [],
      })

      // Reload list
      loadApiKeys()
    } catch (error) {
      console.error('Failed to create API key:', error)
      addError('创建API Key失败')
    }
  }, [newKeyName, newKeyPermissions, loadApiKeys, addError])

  const handleCopyKey = useCallback(
    async (key: string) => {
      try {
        await writeText(key)
        setCopied(true)
        message.success('API Key已复制到剪贴板')
        setTimeout(() => setCopied(false), 2000)
      } catch (error) {
        console.error('Failed to copy key:', error)
        addError('复制失败')
      }
    },
    [addError],
  )

  const handleToggleKey = useCallback(
    async (id: string) => {
      try {
        await ApiKeyService.toggleApiKey(id)
        message.success('API Key状态已更新')
        loadApiKeys()
      } catch (error) {
        console.error('Failed to toggle API key:', error)
        addError('更新失败')
      }
    },
    [loadApiKeys, addError],
  )

  const handleEditPermissions = useCallback(
    async (apiKey: ApiKey) => {
      setSelectedApiKey(apiKey)
      try {
        const details = await ApiKeyService.getApiKeyDetails(apiKey.id)
        setEditPermissions(
          details.permissions ?? { allowed_servers: [], allowed_tools: [] },
        )
        setShowEditModal(true)
      } catch (error) {
        console.error('Failed to fetch API key permissions:', error)
        addError('获取权限信息失败')
      }
    },
    [addError],
  )

  const handleSavePermissions = useCallback(async () => {
    if (!selectedApiKey) return

    try {
      await ApiKeyService.updateApiKeyPermissions(
        selectedApiKey.id,
        editPermissions,
      )
      message.success('权限更新成功')
      setShowEditModal(false)
      loadApiKeys()
    } catch (error) {
      console.error('Failed to update permissions:', error)
      addError('更新权限失败')
    }
  }, [selectedApiKey, editPermissions, loadApiKeys, addError])

  const handleDeleteKey = useCallback(async () => {
    if (!selectedApiKey) return

    try {
      await ApiKeyService.deleteApiKey(selectedApiKey.id)
      message.success('API Key已删除')
      setSelectedApiKey(null)
      loadApiKeys()
    } catch (error) {
      console.error('Failed to delete API key:', error)
      addError('删除失败')
    }
  }, [selectedApiKey, loadApiKeys, addError])

  const formatDate = useCallback((dateString: string) => {
    return new Date(dateString).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    })
  }, [])

  if (loading) {
    return (
      <Flex justify='center' align='center' style={{ height: '256px' }}>
        <Button loading>加载 API Keys...</Button>
      </Flex>
    )
  }

  return (
    <Flex vertical gap='large' style={{ height: '100%', overflowY: 'auto' }}>
      {/* Header */}
      <Flex justify='space-between' align='center'>
        <div>
          <Title level={2}>API Keys</Title>
          <Text type='secondary'>管理用于访问MCP Router的API密钥</Text>
        </div>
        <Button
          onClick={() => setShowCreateModal(true)}
          icon={<Plus size={16} />}
          type='primary'>
          创建API Key
        </Button>
      </Flex>

      {/* API Keys Table */}
      <Card>
        {apiKeys.length === 0 ? (
          <Flex
            vertical
            align='center'
            style={{ textAlign: 'center', padding: '48px 16px' }}>
            <Key className='w-12 h-12 text-gray-400 mb-2' />
            <Title level={4} style={{ marginBottom: '4px' }}>
              暂无API Keys
            </Title>
            <Text type='secondary'>
              点击"创建API Key"按钮添加您的第一个API密钥
            </Text>
          </Flex>
        ) : (
          <Table
            dataSource={apiKeys}
            rowKey='id'
            pagination={false}
            columns={[
              {
                title: '名称',
                dataIndex: 'name',
                key: 'name',
                render: (text) => <Text strong>{text}</Text>,
              },
              {
                title: 'Key',
                dataIndex: 'key',
                key: 'key',
                render: (text) => (
                  <Text code style={{ fontSize: '14px' }}>
                    {text}
                  </Text>
                ),
              },
              {
                title: '状态',
                dataIndex: 'enabled',
                key: 'enabled',
                render: (enabled) => (
                  <Badge color={enabled ? 'green' : 'default'}>
                    {enabled ? '启用' : '禁用'}
                  </Badge>
                ),
              },
              {
                title: '创建时间',
                dataIndex: 'created_at',
                key: 'created_at',
                render: (date) => (
                  <Text type='secondary'>{formatDate(date)}</Text>
                ),
              },
              {
                title: '授权工具',
                key: 'permissions',
                render: (_, record) => (
                  <Flex vertical gap='small'>
                    <div>
                      {(record.permissions?.allowed_servers?.length ?? 0) >
                      0 ? (
                        <Text className='text-primary-600 '>
                          {record.permissions?.allowed_servers?.length ?? 0}{' '}
                          个服务器
                        </Text>
                      ) : (
                        <Text type='secondary'>0 个服务器</Text>
                      )}
                    </div>
                    <div>
                      {record.tool_count !== undefined ? (
                        record.tool_count > 0 ? (
                          <Text className='text-primary-600 '>
                            {record.tool_count} 个工具
                          </Text>
                        ) : (
                          <Text type='secondary'>0 个工具</Text>
                        )
                      ) : (
                        <Text type='secondary' style={{ fontSize: '12px' }}>
                          加载中...
                        </Text>
                      )}
                    </div>
                  </Flex>
                ),
              },
              {
                title: '操作',
                key: 'actions',
                render: (_, record) => (
                  <Space>
                    <Button
                      onClick={() => handleToggleKey(record.id)}
                      type='text'
                      size='small'>
                      {record.enabled ? '禁用' : '启用'}
                    </Button>
                    <Button
                      onClick={() => handleEditPermissions(record)}
                      type='text'
                      size='small'>
                      编辑权限
                    </Button>
                    <Popconfirm
                      title='删除API Key'
                      description={`确定要删除 "${record.name}" 吗?此操作无法撤销。`}
                      okText='删除'
                      cancelText='取消'
                      okType='danger'
                      onConfirm={() => {
                        setSelectedApiKey(record)
                        handleDeleteKey()
                      }}>
                      <Button
                        type='text'
                        size='small'
                        danger
                        icon={<Trash2 size={16} />}>
                        删除
                      </Button>
                    </Popconfirm>
                  </Space>
                ),
              },
            ]}
          />
        )}
      </Card>

      {/* Create API Key Modal */}
      <Modal
        title='创建新API Key'
        open={showCreateModal}
        onCancel={() => setShowCreateModal(false)}
        footer={[
          <Button key='cancel' onClick={() => setShowCreateModal(false)}>
            取消
          </Button>,
          <Button key='create' type='primary' onClick={handleCreateApiKey}>
            创建
          </Button>,
        ]}
        width={768}>
        <Flex vertical gap='middle'>
          <div>
            <Text strong>
              名称 <Text type='danger'>*</Text>
            </Text>
            <Input
              value={newKeyName}
              onChange={(e) => setNewKeyName(e.target.value)}
              placeholder='例如: Production API Key'
              style={{ marginTop: '4px' }}
            />
          </div>

          <div>
            <Text strong>权限配置</Text>
            <div style={{ marginTop: '4px' }}>
              <ApiKeyPermissionSelector
                permissions={newKeyPermissions}
                onChange={setNewKeyPermissions}
              />
            </div>
          </div>
        </Flex>
      </Modal>

      {/* API Key Details Modal (shows full key after creation) */}
      <Modal
        title='API Key 创建成功'
        open={showDetailsModal}
        onCancel={() => {
          setShowDetailsModal(false)
          setNewKeyDetails(null)
        }}
        footer={[
          <Button
            key='close'
            type='primary'
            onClick={() => {
              setShowDetailsModal(false)
              setNewKeyDetails(null)
            }}>
            关闭
          </Button>,
        ]}
        width={640}>
        {newKeyDetails && (
          <Flex vertical gap='middle'>
            <div className='bg-amber-50  border border-amber-200  rounded-lg p-4'>
              <Text className='text-amber-800  text-sm'>
                <Text strong>重要提示:</Text> 这是唯一一次显示完整API
                Key的机会,请妥善保存!
              </Text>
            </div>

            <div>
              <Text strong>API Key</Text>
              <Flex gap='small' style={{ marginTop: '4px' }}>
                <Input
                  value={newKeyDetails.key}
                  readOnly
                  style={{ flex: 1, fontFamily: 'monospace', fontSize: '14px' }}
                />
                <Button
                  onClick={() => handleCopyKey(newKeyDetails.key)}
                  icon={copied ? <Check size={16} /> : <Copy size={16} />}
                  type='primary'
                  size='small'>
                  {copied ? '已复制' : '复制'}
                </Button>
              </Flex>
            </div>

            <div>
              <Text strong>名称</Text>
              <Text
                type='secondary'
                style={{ display: 'block', marginTop: '4px' }}>
                {newKeyDetails.name}
              </Text>
            </div>
          </Flex>
        )}
      </Modal>

      {/* Edit Permissions Modal */}
      <Modal
        title={`编辑权限: ${selectedApiKey?.name}`}
        open={showEditModal}
        onCancel={() => setShowEditModal(false)}
        footer={[
          <Button key='cancel' onClick={() => setShowEditModal(false)}>
            取消
          </Button>,
          <Button key='save' type='primary' onClick={handleSavePermissions}>
            保存
          </Button>,
        ]}
        width={768}>
        <ApiKeyPermissionSelector
          permissions={editPermissions}
          onChange={setEditPermissions}
          apiKeyId={selectedApiKey?.id}
        />
      </Modal>
    </Flex>
  )
})

export default ApiKeys
