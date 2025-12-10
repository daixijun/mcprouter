import {
  AppstoreOutlined,
  DatabaseOutlined,
  FileTextOutlined,
  MessageOutlined,
} from '@ant-design/icons'
import type { TabsProps } from 'antd'
import { Badge, Card, Space, Tabs, Typography } from 'antd'
import React, { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { AvailablePermissions } from '../types'
import PermissionTab from './PermissionTab'

const { Text } = Typography

interface PermissionSelectorProps {
  value?: {
    allowed_tools?: string[]
    allowed_resources?: string[]
    allowed_prompts?: string[]
    allowed_prompt_templates?: string[]
  }
  onChange?: (permissions: {
    allowed_tools?: string[]
    allowed_resources?: string[]
    allowed_prompts?: string[]
    allowed_prompt_templates?: string[]
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
  const { t } = useTranslation()
  const [searchText] = useState('')

  // 计算各类型的权限统计
  const permissionStats = useMemo(() => {

    const stats = {
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
      prompt_templates: {
        total: (availablePermissions.prompt_templates || []).length,
        selected: value.allowed_prompt_templates?.length || 0,
        items: availablePermissions.prompt_templates || [],
        selectedItems: value.allowed_prompt_templates || [],
      },
    }

    
    return stats
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
          <span>{t('token.permissions.tools_permission')}</span>
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
          <span>{t('token.permissions.resources_permission')}</span>
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
          <span>{t('token.permissions.prompts_permission')}</span>
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
    {
      key: 'prompt_templates',
      label: (
        <Space>
          <FileTextOutlined />
          <span>{t('token.permissions.prompt_templates')}</span>
          <Badge
            count={permissionStats.prompt_templates.selected}
            showZero
            size='small'
            style={{ backgroundColor: '#fa8c16' }}
          />
          <Text type='secondary'>
            ({permissionStats.prompt_templates.selected}/{permissionStats.prompt_templates.total})
          </Text>
        </Space>
      ),
      children: (
        <PermissionTab
          type='prompt_templates'
          selectedPermissions={permissionStats.prompt_templates.selectedItems}
          onChange={(permissions) =>
            handlePermissionChange('prompt_templates', permissions)
          }
          disabled={disabled}
          searchText={searchText}
          permissionItems={permissionStats.prompt_templates.items}
        />
      ),
    },
  ]

  return (
    <Card
      title={t('token.permissions.configuration')}
      size='small'
      extra={
        <Space>
          <Text type='secondary'>
            {t('token.permissions.total')}:{' '}
            {permissionStats.tools.selected +
              permissionStats.resources.selected +
              permissionStats.prompts.selected}{' '}
            {t('token.permissions.items_count')}
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
