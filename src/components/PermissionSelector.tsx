import {
  AppstoreOutlined,
  DatabaseOutlined,
  MessageOutlined,
} from '@ant-design/icons'
import type { TabsProps } from 'antd'
import { Badge, Card, Space, Tabs, Typography } from 'antd'
import React, { useMemo, useState } from 'react'
import { AvailablePermissions } from '../types'
import PermissionTab from './PermissionTab'

const { Text } = Typography

interface PermissionSelectorProps {
  value?: {
    allowed_tools?: string[]
    allowed_resources?: string[]
    allowed_prompts?: string[]
  }
  onChange?: (permissions: {
    allowed_tools?: string[]
    allowed_resources?: string[]
    allowed_prompts?: string[]
  }) => void
  availablePermissions: AvailablePermissions
  disabled?: boolean
}

const PermissionSelector: React.FC<PermissionSelectorProps> = ({
  value = {},
  onChange,
  availablePermissions,
  disabled = false,
}) => {
  const [searchText] = useState('')

  // 计算各类型的权限统计
  const permissionStats = useMemo(() => {
    return {
      tools: {
        total: availablePermissions.tools.length,
        selected: value.allowed_tools?.length || 0,
        items: availablePermissions.tools,
        selectedItems: value.allowed_tools || [],
      },
      resources: {
        total: availablePermissions.resources.length,
        selected: value.allowed_resources?.length || 0,
        items: availablePermissions.resources,
        selectedItems: value.allowed_resources || [],
      },
      prompts: {
        total: availablePermissions.prompts.length,
        selected: value.allowed_prompts?.length || 0,
        items: availablePermissions.prompts,
        selectedItems: value.allowed_prompts || [],
      },
    }
  }, [availablePermissions, value])

  const handlePermissionChange = (
    type: keyof typeof permissionStats,
    selectedPermissions: string[],
  ) => {
    const newPermissions = {
      ...value,
      [`allowed_${type}`]: selectedPermissions,
    }
    onChange?.(newPermissions)
  }

  const tabItems: TabsProps['items'] = [
    {
      key: 'tools',
      label: (
        <Space>
          <AppstoreOutlined />
          <span>工具权限</span>
          <Badge
            count={permissionStats.tools.selected}
            showZero
            size='small'
            style={{ backgroundColor: '#52c41a' }}
          />
          <Text type='secondary'>
            ({permissionStats.tools.selected}/{permissionStats.tools.total})
          </Text>
        </Space>
      ),
      children: (
        <PermissionTab
          type='tools'
          permissions={permissionStats.tools.items.map((item) => item.id)}
          selectedPermissions={permissionStats.tools.selectedItems}
          onChange={(permissions) =>
            handlePermissionChange('tools', permissions)
          }
          disabled={disabled}
          searchText={searchText}
          permissionItems={permissionStats.tools.items}
        />
      ),
    },
    {
      key: 'resources',
      label: (
        <Space>
          <DatabaseOutlined />
          <span>资源权限</span>
          <Badge
            count={permissionStats.resources.selected}
            showZero
            size='small'
            style={{ backgroundColor: '#1890ff' }}
          />
          <Text type='secondary'>
            ({permissionStats.resources.selected}/
            {permissionStats.resources.total})
          </Text>
        </Space>
      ),
      children: (
        <PermissionTab
          type='resources'
          permissions={permissionStats.resources.items.map((item) => item.id)}
          selectedPermissions={permissionStats.resources.selectedItems}
          onChange={(permissions) =>
            handlePermissionChange('resources', permissions)
          }
          disabled={disabled}
          searchText={searchText}
          permissionItems={permissionStats.resources.items}
        />
      ),
    },
    {
      key: 'prompts',
      label: (
        <Space>
          <MessageOutlined />
          <span>提示词权限</span>
          <Badge
            count={permissionStats.prompts.selected}
            showZero
            size='small'
            style={{ backgroundColor: '#722ed1' }}
          />
          <Text type='secondary'>
            ({permissionStats.prompts.selected}/{permissionStats.prompts.total})
          </Text>
        </Space>
      ),
      children: (
        <PermissionTab
          type='prompts'
          permissions={permissionStats.prompts.items.map((item) => item.id)}
          selectedPermissions={permissionStats.prompts.selectedItems}
          onChange={(permissions) =>
            handlePermissionChange('prompts', permissions)
          }
          disabled={disabled}
          searchText={searchText}
          permissionItems={permissionStats.prompts.items}
        />
      ),
    },
  ]

  return (
    <Card
      title='权限配置'
      size='small'
      extra={
        <Space>
          <Text type='secondary'>
            总计:{' '}
            {permissionStats.tools.selected +
              permissionStats.resources.selected +
              permissionStats.prompts.selected}{' '}
            项权限
          </Text>
        </Space>
      }>
      <Tabs
        defaultActiveKey='tools'
        items={tabItems}
        size='small'
        type='card'
      />
    </Card>
  )
}

export default PermissionSelector
