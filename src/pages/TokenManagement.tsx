import React, { useState, useEffect } from 'react';
import {
  Table,
  Button,
  Modal,
  Form,
  Input,
  Select,
  message,
  Card,
  Statistic,
  Tag,
  Typography,
  Space,
  Popconfirm,
  Tooltip,
  Alert,
  Empty,
  Row,
  Col,
  Switch,
} from 'antd';
import {
  PlusOutlined,
  DeleteOutlined,
  CopyOutlined,
  KeyOutlined,
  ClockCircleOutlined,
  CheckCircleOutlined,
  ReloadOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import type { ColumnsType } from 'antd/es/table';
import { Token, CreateTokenRequest, TokenStats } from '../types';

const { Text, Paragraph } = Typography;

const TokenManagement: React.FC = () => {
  const [tokens, setTokens] = useState<Token[]>([]);
  const [stats, setStats] = useState<TokenStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [createModalVisible, setCreateModalVisible] = useState(false);
  const [createdToken, setCreatedToken] = useState<{id: string, value: string, name: string, description?: string} | null>(null);
  const [form] = Form.useForm();

  // Fetch tokens and stats on component mount
  useEffect(() => {
    fetchTokens();
    fetchStats();
  }, []);

  const fetchTokens = async () => {
    setLoading(true);
    try {
      const response = await invoke<Token[]>('list_tokens');
      setTokens(response);
    } catch (error) {
      message.error('Failed to load tokens: ' + error);
    } finally {
      setLoading(false);
    }
  };

  const fetchStats = async (retryCount = 0) => {
    try {
      console.log('Fetching token stats...');
      const response = await invoke<TokenStats>('get_token_stats');
      console.log('Token stats response:', response);
      console.log('Response type:', typeof response);
      console.log('Response keys:', response ? Object.keys(response) : 'null');
      console.log('total_count value:', response?.total_count);
      console.log('active_count value:', response?.active_count);
      console.log('expired_count value:', response?.expired_count);
      console.log('total_usage value:', response?.total_usage);

      // Check if the response has the expected fields
      if (response && typeof response === 'object') {
        const stats: TokenStats = {
          total_count: response.total_count ?? 0,
          active_count: response.active_count ?? 0,
          expired_count: response.expired_count ?? 0,
          total_usage: response.total_usage ?? 0,
          last_used: response.last_used,
        };
        console.log('Processed stats:', stats);
        setStats(stats);
      } else {
        console.error('Invalid response format:', response);
        setStats({
          total_count: 0,
          active_count: 0,
          expired_count: 0,
          total_usage: 0,
        });
      }
    } catch (error) {
      console.error('Failed to fetch stats:', error);
      console.error('Error details:', error);

      // If it's an initialization error and we haven't retried too many times, wait and retry
      if (retryCount < 3 && String(error).includes('TokenManager not initialized')) {
        console.log(`TokenManager not initialized, retrying in 1 second... (${retryCount + 1}/3)`);
        setTimeout(() => fetchStats(retryCount + 1), 1000);
        return;
      }

      // Set default empty stats to prevent UI from breaking
      setStats({
        total_count: 0,
        active_count: 0,
        expired_count: 0,
        total_usage: 0,
      });
    }
  };

  const handleCreateToken = async (values: CreateTokenRequest) => {
    try {
      const response = await invoke<any>('create_token', {
        request: {
          name: values.name,
          description: values.description,
          expires_in: values.expires_in,
        },
      });
      setCreatedToken({
        id: response.token.id,
        value: response.token.value,
        name: response.token.name,
        description: response.token.description,
      });
      message.success('Token created successfully!');
    } catch (error) {
      message.error('Failed to create token: ' + error);
    }
  };

  const handleDeleteToken = async (tokenId: string) => {
    try {
      await invoke('delete_token', { tokenId });
      message.success('Token deleted successfully!');
      await fetchTokens();
      await fetchStats();
    } catch (error) {
      message.error('Failed to delete token: ' + error);
    }
  };

  const handleToggleToken = async (tokenId: string, checked: boolean) => {
    try {
      await invoke('toggle_token', { tokenId });
      message.success(`Token ${checked ? 'enabled' : 'disabled'} successfully!`);
      await fetchTokens();
    } catch (error) {
      message.error('Failed to toggle token: ' + error);
    }
  };

  const handleCopyToClipboard = async (token: string) => {
    try {
      await navigator.clipboard.writeText(token);
      message.success('Token copied to clipboard!');
    } catch (error) {
      // Fallback: create a temporary input element
      const tempInput = document.createElement('input');
      tempInput.value = token;
      document.body.appendChild(tempInput);
      tempInput.select();
      document.execCommand('copy');
      document.body.removeChild(tempInput);
      message.success('Token copied to clipboard!');
    }
  };

  const handleCleanupExpired = async () => {
    try {
      const response = await invoke<any>('cleanup_expired_tokens');
      if (response.removed_count > 0) {
        message.success(response.message);
      } else {
        message.info('No expired tokens found');
      }
      await fetchTokens();
      await fetchStats();
    } catch (error) {
      message.error('Failed to cleanup expired tokens: ' + error);
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const formatRelativeTime = (timestamp: number) => {
    const now = Date.now();
    const diff = now - timestamp * 1000;
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(hours / 24);

    if (days > 0) {
      return `${days} day${days > 1 ? 's' : ''} ago`;
    } else if (hours > 0) {
      return `${hours} hour${hours > 1 ? 's' : ''} ago`;
    } else {
      return 'Recently';
    }
  };

  const getStatusColor = (token: Token) => {
    if (token.is_expired) return 'default';
    if (token.usage_count > 100) return 'success';
    if (token.usage_count > 10) return 'processing';
    return 'warning';
  };

  const getStatusText = (token: Token) => {
    if (token.is_expired) return 'Expired';
    if (token.last_used_at) {
      return `Active (${formatRelativeTime(token.last_used_at)})`;
    }
    return 'Unused';
  };

  const columns: ColumnsType<Token> = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (text: string) => <Text strong>{text}</Text>,
    },
    {
      title: 'Description',
      dataIndex: 'description',
      key: 'description',
      render: (text?: string) => <Text type="secondary">{text || '-'}</Text>,
    },
    {
      title: 'Status',
      dataIndex: 'is_expired',
      key: 'status',
      render: (_, record) => (
        <Tag color={getStatusColor(record)}>
          {getStatusText(record)}
        </Tag>
      ),
    },
    {
      title: 'Enabled',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean, record) => (
        <Switch
          checked={enabled}
          onChange={(checked) => handleToggleToken(record.id, checked)}
          disabled={record.is_expired}
          size="small"
        />
      ),
    },
    {
      title: 'Usage Count',
      dataIndex: 'usage_count',
      key: 'usage_count',
      render: (count: number) => <Text>{count}</Text>,
    },
    {
      title: 'Last Used',
      dataIndex: 'last_used_at',
      key: 'last_used_at',
      render: (timestamp?: number) =>
        timestamp ? <Text type="secondary">{formatRelativeTime(timestamp)}</Text> : <Text type="secondary">Never</Text>,
    },
    {
      title: 'Expires',
      dataIndex: 'expires_at',
      key: 'expires_at',
      render: (timestamp?: number) =>
        timestamp ? (
          <Tooltip title={formatDate(timestamp)}>
            <Text type="secondary">{formatRelativeTime(timestamp)}</Text>
          </Tooltip>
        ) : <Tag color="green">Never</Tag>,
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (timestamp: number) => (
        <Tooltip title={formatDate(timestamp)}>
          <Text type="secondary">{formatRelativeTime(timestamp)}</Text>
        </Tooltip>
      ),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space size="small">
          <Popconfirm
            title="Delete this token?"
            description="This action cannot be undone."
            onConfirm={() => handleDeleteToken(record.id)}
            okText="Delete"
            cancelText="Cancel"
            okType="danger"
          >
            <Tooltip title="Delete token">
              <Button
                type="text"
                danger
                icon={<DeleteOutlined />}
              />
            </Tooltip>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const expiresInOptions = [
    { value: 3600, label: '1 hour' },
    { value: 86400, label: '1 day' },
    { value: 604800, label: '1 week' },
    { value: 2592000, label: '30 days' },
    { value: null, label: 'Never' },
  ];

  return (
    <div>
      {/* Token Creation Modal */}
      <Modal
        title={
          <Space>
            <KeyOutlined />
            Create New Token
          </Space>
        }
        open={createModalVisible}
        onCancel={() => setCreateModalVisible(false)}
        footer={null}
        width={600}
      >
        {!createdToken ? (
          <Form
            form={form}
            layout="vertical"
            onFinish={handleCreateToken}
          >
            <Form.Item
              name="name"
              label="Token Name"
              rules={[
                { required: true, message: 'Please enter token name' },
                { max: 100, message: 'Name cannot exceed 100 characters' },
              ]}
            >
              <Input placeholder="Enter a descriptive name for this token" />
            </Form.Item>

            <Form.Item
              name="description"
              label="Description"
              rules={[
                { max: 500, message: 'Description cannot exceed 500 characters' },
              ]}
            >
              <Input.TextArea
                placeholder="Describe the purpose of this token (optional)"
                rows={3}
              />
            </Form.Item>

            <Form.Item
              name="expires_in"
              label="Expiration Time"
              initialValue={2592000} // 30 days default
            >
              <Select options={expiresInOptions} />
            </Form.Item>

            <Form.Item>
              <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
                <Button onClick={() => setCreateModalVisible(false)}>
                  Cancel
                </Button>
                <Button type="primary" htmlType="submit">
                  Create Token
                </Button>
              </Space>
            </Form.Item>
          </Form>
        ) : (
          <div>
            <Alert
              message="Token Created Successfully!"
              description="Please copy this token now. You won't be able to see it again."
              type="success"
              showIcon
              style={{ marginBottom: 16 }}
            />

            <Card>
              <Space direction="vertical" style={{ width: '100%' }}>
                <div>
                  <Text strong>Token Name:</Text>
                  <Text>{createdToken.name}</Text>
                </div>

                {createdToken.description && (
                  <div>
                    <Text strong>Description:</Text>
                    <Text>{createdToken.description}</Text>
                  </div>
                )}

                <div>
                  <Text strong>Token ID:</Text>
                  <Text code copyable>{createdToken.id}</Text>
                </div>

                <div>
                  <Text strong>Token Value:</Text>
                  <Input.Password
                    value={createdToken.value}
                    readOnly
                    addonAfter={
                      <Button
                        icon={<CopyOutlined />}
                        onClick={() => handleCopyToClipboard(createdToken.value)}
                      >
                        Copy
                      </Button>
                    }
                  />
                  <div style={{ marginTop: 8 }}>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      🔒 This token is sensitive information. Store it securely and never share it publicly.
                    </Text>
                  </div>
                </div>
              </Space>
            </Card>

            <Space style={{ width: '100%', justifyContent: 'space-between', marginTop: 16 }}>
              <Button
                onClick={() => {
                  setCreatedToken(null);
                  form.resetFields();
                }}
              >
                Create Another Token
              </Button>
              <Button
                type="primary"
                onClick={async () => {
                  setCreateModalVisible(false);
                  setCreatedToken(null);
                  form.resetFields();
                  await fetchTokens();
                  await fetchStats();
                }}
              >
                Done
              </Button>
            </Space>
          </div>
        )}
      </Modal>

      {/* Stats Cards */}
      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <Card>
            <Statistic
              title="Total Tokens"
              value={stats?.total_count || 0}
              prefix={<KeyOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Active Tokens"
              value={stats?.active_count || 0}
              valueStyle={{ color: '#3f8600' }}
              prefix={<CheckCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Expired Tokens"
              value={stats?.expired_count || 0}
              valueStyle={{ color: '#cf1322' }}
              prefix={<ClockCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Total Usage"
              value={stats?.total_usage || 0}
              prefix={<InfoCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {/* Actions */}
      <Space style={{ marginBottom: 16 }}>
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => {
            setCreatedToken(null);
            setCreateModalVisible(true);
            form.resetFields();
          }}
        >
          Create New Token
        </Button>
        <Button
          icon={<ReloadOutlined />}
          onClick={() => {
            fetchTokens();
            fetchStats();
          }}
          loading={loading}
        >
          Refresh
        </Button>
        <Button
          icon={<ReloadOutlined />}
          onClick={handleCleanupExpired}
        >
          Cleanup Expired
        </Button>
      </Space>

      {/* Tokens Table */}
      <Table
        columns={columns}
        dataSource={tokens}
        rowKey="id"
        loading={loading}
        locale={{
          emptyText: (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <Space direction="vertical">
                  <Paragraph>No tokens found</Paragraph>
                  <Text type="secondary">Create your first token to start using the MCP Router API</Text>
                  <Button
                    type="primary"
                    icon={<PlusOutlined />}
                    onClick={() => {
                      setCreatedToken(null);
                      setCreateModalVisible(true);
                      form.resetFields();
                    }}
                  >
                    Create Your First Token
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
            `${range[0]}-${range[1]} of ${total} tokens`,
        }}
      />
    </div>
  );
};

export default TokenManagement;