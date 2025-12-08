import {
  CheckCircleOutlined,
  ClockCircleOutlined,
  CopyOutlined,
  DeleteOutlined,
  EditOutlined,
  InfoCircleOutlined,
  KeyOutlined,
  PlusOutlined,
  ReloadOutlined,
  SaveOutlined,
} from '@ant-design/icons'
import { invoke } from '@tauri-apps/api/core'
import {
  Alert,
  App as AntdApp,
  Button,
  Card,
  Col,
  Drawer,
  Empty,
  Form,
  Input,
  Popconfirm,
  Row,
  Select,
  Skeleton,
  Space,
  Statistic,
  Switch,
  Table,
  Tag,
  Tooltip,
  Typography,
} from 'antd'
import type { ColumnsType } from 'antd/es/table'
import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import PermissionSelector from '../components/PermissionSelector'
import {
  AvailablePermissions,
  Token,
  TokenStats,
  UpdateTokenRequest,
  UpdateTokenResponse,
} from '../types'

const { Text, Paragraph } = Typography

const TokenManagement: React.FC = () => {
  const { t } = useTranslation()
  const { message } = AntdApp.useApp()
  const [tokens, setTokens] = useState<Token[]>([])
  const [stats, setStats] = useState<TokenStats | null>(null)
  const [loading, setLoading] = useState(false)
  const [createModalVisible, setCreateModalVisible] = useState(false)
  const [createdToken, setCreatedToken] = useState<{
    id: string
    value: string
    name: string
    description?: string
  } | null>(null)
  const [editModalVisible, setEditModalVisible] = useState(false)
  const [editingToken, setEditingToken] = useState<Token | null>(null)
  const [form] = Form.useForm()
  const [editForm] = Form.useForm()
  const [permissionsLoading, setPermissionsLoading] = useState(false)

  // Permission options state
  const [availablePermissions, setAvailablePermissions] =
    useState<AvailablePermissions>({
      tools: [],
      resources: [],
      prompts: [],
      prompt_templates: [],
      prompt_categories: [],
    })

  // Fetch tokens and stats on component mount
  useEffect(() => {
    fetchTokens()
    fetchStats()
    fetchAvailablePermissions()
  }, [])

  const fetchTokens = async () => {
    setLoading(true)
    try {
      const response = await invoke<Token[]>('list_tokens')
      setTokens(response)
    } catch (error) {
      message.error(t('token.messages.load_tokens_failed') + ': ' + error)
    } finally {
      setLoading(false)
    }
  }

  const fetchStats = async (retryCount = 0) => {
    try {
      console.log('Fetching token stats...')
      const response = await invoke<TokenStats>('get_token_stats')
      console.log('Token stats response:', response)
      console.log('Response type:', typeof response)
      console.log('Response keys:', response ? Object.keys(response) : 'null')
      console.log('total_count value:', response?.total_count)
      console.log('active_count value:', response?.active_count)
      console.log('expired_count value:', response?.expired_count)
      console.log('total_usage value:', response?.total_usage)

      // Check if the response has the expected fields
      if (response && typeof response === 'object') {
        const stats: TokenStats = {
          total_count: response.total_count ?? 0,
          active_count: response.active_count ?? 0,
          expired_count: response.expired_count ?? 0,
          total_usage: response.total_usage ?? 0,
          last_used: response.last_used,
        }
        console.log('Processed stats:', stats)
        setStats(stats)
      } else {
        console.error('Invalid response format:', response)
        setStats({
          total_count: 0,
          active_count: 0,
          expired_count: 0,
          total_usage: 0,
        })
      }
    } catch (error) {
      console.error('Failed to fetch stats:', error)
      console.error('Error details:', error)

      // If it's an initialization error and we haven't retried too many times, wait and retry
      if (
        retryCount < 3 &&
        String(error).includes('TokenManager not initialized')
      ) {
        console.log(
          `TokenManager not initialized, retrying in 1 second... (${
            retryCount + 1
          }/3)`,
        )
        setTimeout(() => fetchStats(retryCount + 1), 1000)
        return
      }

      // Set default empty stats to prevent UI from breaking
      setStats({
        total_count: 0,
        active_count: 0,
        expired_count: 0,
        total_usage: 0,
      })
    }
  }

  // Fetch available permissions options
  const fetchAvailablePermissions = async (): Promise<void> => {
    setPermissionsLoading(true)
    try {
      const result = await invoke<AvailablePermissions>(
        'get_available_permissions',
      )

      // 设置真实权限数据，确保包含提示词模板字段
      setAvailablePermissions({
        ...result,
        prompt_templates: result.prompt_templates || [],
        prompt_categories: result.prompt_categories || [],
      })
    } catch (error) {
      console.error('Failed to fetch available permissions:', error)

      // 设置空结构而不是模拟数据，确保界面可以正常工作
      setAvailablePermissions({
        tools: [],
        resources: [],
        prompts: [],
        prompt_templates: [],
        prompt_categories: [],
      })
      message.error('权限数据加载失败')
    } finally {
      setPermissionsLoading(false)
    }
  }

  const handleCreateToken = async (values: any) => {
    try {
      const request = {
        name: values.name,
        description: values.description,
        expires_in: values.expires_in === -1 ? null : values.expires_in,
        allowed_tools: values.permissions?.allowed_tools || [],
        allowed_resources: values.permissions?.allowed_resources || [],
        allowed_prompts: values.permissions?.allowed_prompts || [],
        allowed_prompt_templates:
          values.permissions?.allowed_prompt_templates || [],
      }

      const response = await invoke<any>('create_token', {
        request,
      })
      setCreatedToken({
        id: response.token.id,
        value: response.token.value,
        name: response.token.name,
        description: response.token.description,
      })
      message.success(t('token.messages.create_success'))
    } catch (error) {
      message.error(t('token.messages.create_failed') + ': ' + error)
    }
  }

  const handleDeleteToken = async (tokenId: string) => {
    try {
      await invoke('delete_token', { tokenId })
      message.success(t('token.messages.delete_success'))
      await fetchTokens()
      await fetchStats()
    } catch (error) {
      message.error(t('token.messages.delete_failed') + ': ' + error)
    }
  }

  const handleToggleToken = async (tokenId: string, checked: boolean) => {
    try {
      await invoke('toggle_token', { tokenId })
      const action = checked
        ? t('common.actions.enable')
        : t('common.actions.disable')
      message.success(t('token.messages.toggle_success', { action }))
      await fetchTokens()
    } catch (error) {
      message.error(t('token.messages.toggle_failed') + ': ' + error)
    }
  }

  const handleCopyToClipboard = async (token: string) => {
    try {
      await navigator.clipboard.writeText(token)
      message.success(t('token.messages.copy_success'))
    } catch (error) {
      // Fallback: create a temporary input element
      const tempInput = document.createElement('input')
      tempInput.value = token
      document.body.appendChild(tempInput)
      tempInput.select()
      document.execCommand('copy')
      document.body.removeChild(tempInput)
      message.success(t('token.messages.copy_success'))
    }
  }

  const handleCleanupExpired = async () => {
    try {
      const response = await invoke<any>('cleanup_expired_tokens')
      if (response.removed_count > 0) {
        message.success(
          t('token.messages.cleanup_success', {
            count: response.removed_count,
          }),
        )
      } else {
        message.info(t('token.messages.cleanup_no_expired'))
      }
      await fetchTokens()
      await fetchStats()
    } catch (error) {
      message.error(t('token.messages.cleanup_failed') + ': ' + error)
    }
  }

  const handleEditToken = async (token: Token) => {
    setEditingToken(token)
    setEditModalVisible(true) // 先打开模态框
    await fetchAvailablePermissions() // 异步加载权限数据
  }

  const handleUpdateToken = async (values: any) => {
    if (!editingToken) return

    try {
      const updateRequest: UpdateTokenRequest = {
        id: editingToken.id,
        name: values.name,
        description: values.description,
        allowed_tools:
          values.permissions?.allowed_tools?.length > 0
            ? values.permissions.allowed_tools
            : undefined,
        allowed_resources:
          values.permissions?.allowed_resources?.length > 0
            ? values.permissions.allowed_resources
            : undefined,
        allowed_prompts:
          values.permissions?.allowed_prompts?.length > 0
            ? values.permissions.allowed_prompts
            : undefined,
        allowed_prompt_templates:
          values.permissions?.allowed_prompt_templates?.length > 0
            ? values.permissions.allowed_prompt_templates
            : undefined,
      }

      const response = await invoke<UpdateTokenResponse>('update_token', {
        request: updateRequest,
      })

      message.success(
        t('token.messages.update_success', { name: response.token.name }),
      )
      setEditModalVisible(false)
      setEditingToken(null)
      await fetchTokens()
    } catch (error: any) {
      console.error('Failed to update token:', error)
      message.error(t('token.messages.update_failed', { error }))
    }
  }

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString()
  }

  const formatRelativeTime = (timestamp: number) => {
    const now = Date.now()
    const diff = now - timestamp * 1000
    const hours = Math.floor(diff / (1000 * 60 * 60))
    const days = Math.floor(hours / 24)

    if (days > 0) {
      return t('token.status.ago', {
        time: t('token.time.days', { count: days }),
      })
    } else if (hours > 0) {
      return t('token.status.ago', {
        time: t('token.time.hours', { count: hours }),
      })
    } else {
      return t('token.status.recently')
    }
  }

  // 辅助函数：从editingToken获取初始权限数据
  const getInitialPermissions = () => {
    if (!editingToken) return {}
    return {
      allowed_tools: editingToken.allowed_tools || [],
      allowed_resources: editingToken.allowed_resources || [],
      allowed_prompts: editingToken.allowed_prompts || [],
      allowed_prompt_templates: editingToken.allowed_prompt_templates || [],
    }
  }

  const getStatusColor = (token: Token) => {
    if (token.is_expired) return 'default'
    if (token.usage_count > 100) return 'success'
    if (token.usage_count > 10) return 'processing'
    return 'warning'
  }

  const getStatusText = (token: Token) => {
    if (token.is_expired) return t('token.status.expired')
    if (token.last_used_at) {
      return `${t('token.status.active')} (${formatRelativeTime(
        token.last_used_at,
      )})`
    }
    return t('token.status.unused')
  }

  const columns: ColumnsType<Token> = [
    {
      title: t('token.table.name'),
      dataIndex: 'name',
      key: 'name',
      render: (text: string) => <Text strong>{text}</Text>,
    },
    {
      title: t('token.table.description'),
      dataIndex: 'description',
      key: 'description',
      render: (text?: string) => <Text type='secondary'>{text || '-'}</Text>,
    },
    {
      title: t('token.table.permissions'),
      key: 'permissions',
      render: (_, record) => {
        // 计算所有权限类型的数量
        const hasPermissions =
          record.allowed_tools?.length ||
          record.allowed_resources?.length ||
          record.allowed_prompts?.length ||
          record.allowed_prompt_templates?.length

        if (!hasPermissions) {
          return <Tag color='green'>{t('token.permissions.unrestricted')}</Tag>
        }

        const permissionCount =
          (record.allowed_tools?.length || 0) +
          (record.allowed_resources?.length || 0) +
          (record.allowed_prompts?.length || 0) +
          (record.allowed_prompt_templates?.length || 0)

        const totalCount = permissionCount

        return (
          <Tooltip
            title={
              <div style={{ maxWidth: 300 }}>
                {record.allowed_tools && record.allowed_tools.length > 0 && (
                  <div>
                    <strong>{t('token.permissions.tools')}:</strong>{' '}
                    {record.allowed_tools.slice(0, 3).join(', ')}
                    {record.allowed_tools.length > 3 && '...'}
                  </div>
                )}
                {record.allowed_resources &&
                  record.allowed_resources.length > 0 && (
                    <div>
                      <strong>{t('token.permissions.resources')}:</strong>{' '}
                      {record.allowed_resources.slice(0, 3).join(', ')}
                      {record.allowed_resources.length > 3 && '...'}
                    </div>
                  )}
                {record.allowed_prompts &&
                  record.allowed_prompts.length > 0 && (
                    <div>
                      <strong>{t('token.permissions.prompts')}:</strong>{' '}
                      {record.allowed_prompts.slice(0, 3).join(', ')}
                      {record.allowed_prompts.length > 3 && '...'}
                    </div>
                  )}
                {record.allowed_prompt_templates &&
                  record.allowed_prompt_templates.length > 0 && (
                    <div>
                      <strong>提示词模板:</strong>{' '}
                      {record.allowed_prompt_templates.slice(0, 3).join(', ')}
                      {record.allowed_prompt_templates.length > 3 && '...'}
                    </div>
                  )}
              </div>
            }>
            <Tag
              color={
                totalCount > 10 ? 'red' : totalCount > 5 ? 'orange' : 'blue'
              }>
              {t('token.permissions.permissions_count', {
                count: totalCount,
              })}
            </Tag>
          </Tooltip>
        )
      },
    },
    {
      title: t('token.table.status'),
      dataIndex: 'is_expired',
      key: 'status',
      render: (_, record) => (
        <Tag color={getStatusColor(record)}>{getStatusText(record)}</Tag>
      ),
    },
    {
      title: t('token.table.enabled'),
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean, record) => (
        <Switch
          checked={enabled}
          onChange={(checked) => handleToggleToken(record.id, checked)}
          disabled={record.is_expired}
          size='small'
        />
      ),
    },
    {
      title: t('token.table.usage_count'),
      dataIndex: 'usage_count',
      key: 'usage_count',
      render: (count: number) => <Text>{count}</Text>,
    },
    {
      title: t('token.table.last_used'),
      dataIndex: 'last_used_at',
      key: 'last_used_at',
      render: (timestamp?: number) =>
        timestamp ? (
          <Text type='secondary'>{formatRelativeTime(timestamp)}</Text>
        ) : (
          <Text type='secondary'>{t('token.status.never')}</Text>
        ),
    },
    {
      title: t('token.table.expires'),
      dataIndex: 'expires_at',
      key: 'expires_at',
      render: (timestamp?: number) =>
        timestamp ? (
          <Tooltip title={formatDate(timestamp)}>
            <Text type='secondary'>{formatRelativeTime(timestamp)}</Text>
          </Tooltip>
        ) : (
          <Tag color='green'>{t('token.form.expiry_never')}</Tag>
        ),
    },
    {
      title: t('token.table.created'),
      dataIndex: 'created_at',
      key: 'created_at',
      render: (timestamp: number) => (
        <Tooltip title={formatDate(timestamp)}>
          <Text type='secondary'>{formatRelativeTime(timestamp)}</Text>
        </Tooltip>
      ),
    },
    {
      title: t('token.table.actions'),
      key: 'actions',
      render: (_, record) => (
        <Space size='small'>
          <Tooltip title={t('token.modal.edit_tooltip')}>
            <Button
              type='text'
              icon={<EditOutlined />}
              onClick={() => handleEditToken(record)}
              disabled={record.is_expired}
            />
          </Tooltip>
          <Popconfirm
            title={t('token.modal.delete_confirm')}
            description={t('token.modal.delete_warning')}
            onConfirm={() => handleDeleteToken(record.id)}
            okText={t('token.modal.delete_ok')}
            cancelText={t('token.modal.delete_cancel')}
            okType='danger'>
            <Tooltip title={t('token.modal.delete_tooltip')}>
              <Button type='text' danger icon={<DeleteOutlined />} />
            </Tooltip>
          </Popconfirm>
        </Space>
      ),
    },
  ]

  const expiresInOptions = [
    { value: 3600, label: t('token.time.one_hour') },
    { value: 86400, label: t('token.time.one_day') },
    { value: 604800, label: t('token.time.one_week') },
    { value: 2592000, label: t('token.time.thirty_days') },
    { value: -1, label: t('token.form.expiry_never') },
  ]

  return (
    <div>
      {/* Token Creation Drawer */}
      <Drawer
        title={
          <Space>
            <KeyOutlined />
            {t('token.modal.create_title')}
          </Space>
        }
        open={createModalVisible}
        onClose={() => {
          setCreateModalVisible(false)
          setCreatedToken(null)
        }}
        afterOpenChange={(open) => {
          if (open) {
            form.resetFields()
          }
        }}
        footer={null}
        size={800}
        placement='right'>
        {!createdToken ? (
          <Form form={form} layout='vertical' onFinish={handleCreateToken}>
            <Form.Item
              name='name'
              label={t('token.form.name')}
              rules={[
                {
                  required: true,
                  message: t('token.validation.name_required'),
                },
                { max: 100, message: t('token.validation.name_max_length') },
              ]}>
              <Input placeholder={t('token.form.name_placeholder')} />
            </Form.Item>

            <Form.Item
              name='description'
              label={t('token.form.description')}
              rules={[
                {
                  max: 500,
                  message: t('token.validation.description_max_length'),
                },
              ]}>
              <Input.TextArea
                placeholder={t('token.form.description_placeholder')}
                rows={3}
              />
            </Form.Item>

            <Form.Item
              name='expires_in'
              label={t('token.form.expiry')}
              initialValue={2592000} // 30 days default
            >
              <Select options={expiresInOptions} />
            </Form.Item>

            <Form.Item
              name='permissions'
              label={t('token.form.permissions')}
              tooltip={t('token.form.permissions_tooltip')}>
              {permissionsLoading ? (
                <Card>
                  <Skeleton active paragraph={{ rows: 4 }} />
                </Card>
              ) : (
                <PermissionSelector
                  value={{}}
                  onChange={(permissions) => {
                    form.setFieldsValue({ permissions })
                  }}
                  availablePermissions={availablePermissions}
                  disabled={permissionsLoading}
                />
              )}
            </Form.Item>

            <Form.Item>
              <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
                <Button onClick={() => setCreateModalVisible(false)}>
                  {t('token.actions.cancel')}
                </Button>
                <Button type='primary' htmlType='submit'>
                  {t('token.actions.create_token')}
                </Button>
              </Space>
            </Form.Item>
          </Form>
        ) : (
          <div>
            <Alert
              message={t('token.messages.token_created_warning')}
              description={t('token.messages.token_copy_instruction')}
              type='success'
              showIcon
              style={{ marginBottom: 16 }}
            />

            <Card>
              <Space orientation='vertical' style={{ width: '100%' }}>
                <div>
                  <Text strong>{t('token.form.name')}:</Text>
                  <Text>{createdToken.name}</Text>
                </div>

                {createdToken.description && (
                  <div>
                    <Text strong>{t('token.form.description')}:</Text>
                    <Text>{createdToken.description}</Text>
                  </div>
                )}

                <div>
                  <Text strong>{t('token.form.token_id')}:</Text>
                  <Text code copyable>
                    {createdToken.id}
                  </Text>
                </div>

                <div>
                  <Text strong>{t('token.form.token_value')}:</Text>
                  <Input.Password
                    value={createdToken.value}
                    readOnly
                    addonAfter={
                      <Button
                        icon={<CopyOutlined />}
                        onClick={() =>
                          handleCopyToClipboard(createdToken.value)
                        }>
                        {t('token.actions.copy')}
                      </Button>
                    }
                  />
                  <div style={{ marginTop: 8 }}>
                    <Text type='secondary' style={{ fontSize: 12 }}>
                      {t('token.messages.token_security_warning')}
                    </Text>
                  </div>
                </div>
              </Space>
            </Card>

            <Space
              style={{
                width: '100%',
                justifyContent: 'space-between',
                marginTop: 16,
              }}>
              <Button
                onClick={() => {
                  setCreatedToken(null)
                }}>
                {t('token.actions.create_another')}
              </Button>
              <Button
                type='primary'
                onClick={async () => {
                  setCreateModalVisible(false)
                  setCreatedToken(null)
                  await fetchTokens()
                  await fetchStats()
                }}>
                {t('token.actions.complete')}
              </Button>
            </Space>
          </div>
        )}
      </Drawer>

      {/* Stats Cards */}
      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <Card>
            <Statistic
              title={t('token.stats.total_tokens')}
              value={stats?.total_count || 0}
              prefix={<KeyOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title={t('token.stats.active_tokens')}
              value={stats?.active_count || 0}
              styles={{ content: { color: '#3f8600' } }}
              prefix={<CheckCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title={t('token.stats.expired_tokens')}
              value={stats?.expired_count || 0}
              styles={{ content: { color: '#cf1322' } }}
              prefix={<ClockCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title={t('token.stats.total_usage')}
              value={stats?.total_usage || 0}
              prefix={<InfoCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {/* Actions */}
      <Space style={{ marginBottom: 16 }}>
        <Button
          type='primary'
          icon={<PlusOutlined />}
          onClick={async () => {
            setCreatedToken(null)
            await fetchAvailablePermissions()
            setCreateModalVisible(true)
          }}>
          {t('token.actions.create_new_token')}
        </Button>
        <Button
          icon={<ReloadOutlined />}
          onClick={() => {
            fetchTokens()
            fetchStats()
          }}
          loading={loading}>
          {t('token.actions.refresh')}
        </Button>
        <Button icon={<ReloadOutlined />} onClick={handleCleanupExpired}>
          {t('token.actions.cleanup_expired')}
        </Button>
      </Space>

      {/* Tokens Table */}
      <Table
        columns={columns}
        dataSource={tokens}
        rowKey='id'
        loading={loading}
        locale={{
          emptyText: (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <Space orientation='vertical'>
                  <Paragraph>{t('token.empty.title')}</Paragraph>
                  <Text type='secondary'>{t('token.empty.description')}</Text>
                  <Button
                    type='primary'
                    icon={<PlusOutlined />}
                    onClick={() => {
                      setCreatedToken(null)
                      setCreateModalVisible(true)
                    }}>
                    {t('token.empty.create_first')}
                  </Button>
                </Space>
              }
            />
          ),
        }}
        pagination={{
          showSizeChanger: true,
          showQuickJumper: true,
          showTotal: (total, range) =>
            t('token.pagination.total', {
              start: range[0],
              end: range[1],
              total,
            }),
        }}
      />

      {/* Edit Token Drawer */}
      <Drawer
        title={
          <Space>
            <EditOutlined />
            {t('token.modal.edit_title')}
          </Space>
        }
        open={editModalVisible}
        onClose={() => {
          setEditModalVisible(false)
          setEditingToken(null)
        }}
        footer={
          <div style={{ textAlign: 'right' }}>
            <Space>
              <Button
                onClick={() => {
                  setEditModalVisible(false)
                  setEditingToken(null)
                }}>
                {t('token.actions.cancel')}
              </Button>
              <Button
                type='primary'
                icon={<SaveOutlined />}
                onClick={() => editForm.submit()}>
                {t('token.actions.save_changes')}
              </Button>
            </Space>
          </div>
        }
        size={800}
        placement='right'
        afterOpenChange={(open) => {
          if (open && editingToken) {
            editForm.setFieldsValue({
              name: editingToken.name,
              description: editingToken.description,
              permissions: getInitialPermissions(),
            })
          }
        }}>
        <Form form={editForm} layout='vertical' onFinish={handleUpdateToken}>
          <Form.Item
            name='name'
            label={t('token.form.name')}
            rules={[
              { required: true, message: t('token.validation.name_required') },
              { max: 100, message: t('token.validation.name_max_length') },
            ]}>
            <Input
              placeholder={t('token.form.name_placeholder')}
              readOnly
              style={{ cursor: 'not-allowed', backgroundColor: '#f5f5f5' }}
            />
          </Form.Item>

          <Form.Item
            name='description'
            label={t('token.form.description')}
            rules={[
              {
                max: 500,
                message: t('token.validation.description_max_length'),
              },
            ]}>
            <Input.TextArea
              placeholder={t('token.form.description_placeholder')}
              rows={3}
            />
          </Form.Item>

          <Form.Item
            name='permissions'
            label={t('token.form.permissions')}
            tooltip={t('token.form.permissions_tooltip')}>
            {permissionsLoading ? (
              <Card>
                <Skeleton active paragraph={{ rows: 6 }} />
              </Card>
            ) : (
              <PermissionSelector
                value={getInitialPermissions()}
                onChange={(permissions) => {
                  editForm.setFieldsValue({ permissions })
                }}
                availablePermissions={availablePermissions}
                disabled={permissionsLoading}
              />
            )}
          </Form.Item>
        </Form>
      </Drawer>
    </div>
  )
}

export default TokenManagement
