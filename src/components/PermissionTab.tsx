import {
  ApiOutlined,
  CloseOutlined,
  DownOutlined,
  FileTextOutlined,
  FolderOutlined,
  MessageOutlined,
  SearchOutlined,
  SelectOutlined,
  UpOutlined,
} from '@ant-design/icons'
import {
  Button,
  Card,
  Checkbox,
  Divider,
  Input,
  Space,
  Tooltip,
  Typography,
} from 'antd'
import type { CheckboxGroupProps } from 'antd/es/checkbox'
import React, { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { PermissionItem } from '../types'

const { Search } = Input
const { Text } = Typography

interface PermissionTabProps {
  type: 'tools' | 'resources' | 'prompts' | 'prompt_templates'
  permissions: string[]
  selectedPermissions: string[]
  onChange: (permissions: string[]) => void
  disabled?: boolean
  searchText?: string
  permissionItems?: PermissionItem[]
}

const PermissionTab: React.FC<PermissionTabProps> = ({
  type,
  permissions,
  selectedPermissions,
  onChange,
  disabled = false,
  searchText = '',
  permissionItems = [],
}) => {
  const { t } = useTranslation()
  const [searchValue, setSearchValue] = useState(searchText)
  const [expandedServers, setExpandedServers] = useState<string[]>([])

  // 根据权限ID获取描述信息
  const getPermissionDescription = (permissionId: string): string => {
    const item = permissionItems.find((item) => item.id === permissionId)
    return item?.description || ''
  }

  // 获取权限类型对应的图标
  const getTypeIcon = () => {
    switch (type) {
      case 'tools':
        return <ApiOutlined />
      case 'resources':
        return <FolderOutlined />
      case 'prompts':
        return <FileTextOutlined />
      case 'prompt_templates':
        return <MessageOutlined />
      default:
        return <ApiOutlined />
    }
  }

  // 按服务名称分组权限
  const groupedPermissions = useMemo(() => {
    const groups: Record<string, string[]> = {}

    permissions.forEach((permission) => {
      if (type === 'prompt_templates') {
        // 提示词模板使用特殊分组逻辑
        const templateGroupName = t('token.permissions.prompt_templates_group')
        if (!groups[templateGroupName]) {
          groups[templateGroupName] = []
        }
        groups[templateGroupName].push(permission)
      } else {
        // 其他权限类型使用原有的服务器分组逻辑
        const [server, ...rest] = permission.split('__')
        if (server && rest.length > 0) {
          if (!groups[server]) {
            groups[server] = []
          }
          groups[server].push(permission)
        }
      }
    })

    return Object.entries(groups)
      .map(([server, serverPermissions]) => ({
        server,
        permissions: serverPermissions,
        selectedCount: serverPermissions.filter((p) =>
          selectedPermissions.includes(p),
        ).length,
      }))
      .sort((a, b) => a.server.localeCompare(b.server))
  }, [permissions, selectedPermissions, type])

  // 过滤后的分组权限
  const filteredGroups = useMemo(() => {
    if (!searchValue.trim()) return groupedPermissions

    const searchTerm = searchValue.toLowerCase()
    return groupedPermissions
      .map((group) => ({
        ...group,
        permissions: group.permissions.filter(
          (permission) =>
            permission.toLowerCase().includes(searchTerm) ||
            group.server.toLowerCase().includes(searchTerm),
        ),
      }))
      .filter((group) => group.permissions.length > 0)
  }, [groupedPermissions, searchValue])

  // 全选/反选/清空操作
  const handleSelectAll = () => {
    const allPermissions = filteredGroups.flatMap((group) => group.permissions)
    onChange([...new Set([...selectedPermissions, ...allPermissions])])
  }

  const handleSelectNone = () => {
    const permissionsToRemove = filteredGroups.flatMap(
      (group) => group.permissions,
    )
    onChange(
      selectedPermissions.filter((p) => !permissionsToRemove.includes(p)),
    )
  }

  const handleInvert = () => {
    const permissionsToToggle = filteredGroups.flatMap(
      (group) => group.permissions,
    )
    const newSelected = selectedPermissions.filter(
      (p) => !permissionsToToggle.includes(p),
    )
    const newToAdd = permissionsToToggle.filter(
      (p) => !selectedPermissions.includes(p),
    )
    onChange([...new Set([...newSelected, ...newToAdd])])
  }

  // 服务器级别的选择操作
  const handleServerSelect = (server: string, checked: boolean) => {
    const group = groupedPermissions.find((g) => g.server === server)
    if (!group) return

    if (checked) {
      // 选中服务器下的所有权限
      const serverPermissions = group.permissions
      onChange([...new Set([...selectedPermissions, ...serverPermissions])])
    } else {
      // 取消选中服务器下的所有权限
      onChange(
        selectedPermissions.filter((p) => !group.permissions.includes(p)),
      )
    }
  }

  // 权限级别的选择操作
  const handlePermissionChange: CheckboxGroupProps['onChange'] = (
    checkedValues,
  ) => {
    onChange(checkedValues as string[])
  }

  // 切换服务器展开状态
  const toggleServerExpanded = (server: string) => {
    setExpandedServers((prev) =>
      prev.includes(server)
        ? prev.filter((s) => s !== server)
        : [...prev, server],
    )
  }

  // 展开/收起所有服务器
  const expandAll = () => {
    setExpandedServers(filteredGroups.map((g) => g.server))
  }

  const collapseAll = () => {
    setExpandedServers([])
  }

  const allSelectedCount = filteredGroups.reduce(
    (sum, group) => sum + group.selectedCount,
    0,
  )
  const totalCount = filteredGroups.reduce(
    (sum, group) => sum + group.permissions.length,
    0,
  )
  const allSelected = allSelectedCount === totalCount && totalCount > 0

  return (
    <div>
      {/* 操作栏 */}
      <Card size='small' style={{ marginBottom: 16 }}>
        <Space orientation='vertical' style={{ width: '100%' }}>
          {/* 搜索框 */}
          <Search
            placeholder={t('tool.tab.search_placeholder')}
            allowClear
            value={searchValue}
            onChange={(e) => setSearchValue(e.target.value)}
            prefix={<SearchOutlined />}
          />

          {/* 批量操作按钮 */}
          <Space wrap>
            <Button
              size='small'
              icon={<SelectOutlined />}
              onClick={handleSelectAll}
              disabled={disabled || totalCount === 0 || allSelected}>
              {t('tool.tab.select_all_current')}
            </Button>
            <Button
              size='small'
              icon={<CloseOutlined />}
              onClick={handleSelectNone}
              disabled={disabled || allSelectedCount === 0}>
              {t('tool.tab.clear_current')}
            </Button>
            <Button
              size='small'
              onClick={handleInvert}
              disabled={disabled || totalCount === 0}>
              {t('tool.tab.invert')}
            </Button>
            <Divider type='vertical' />
            <Button size='small' onClick={expandAll} disabled={disabled}>
              {t('tool.tab.expand_all')}
            </Button>
            <Button size='small' onClick={collapseAll} disabled={disabled}>
              {t('tool.tab.collapse_all')}
            </Button>
          </Space>

          {/* 统计信息 */}
          <Space>
            <Text type='secondary'>
              {getTypeIcon()}
              {type === 'tools' && t('tool.tab.tools_permissions_selected')}
              {type === 'resources' &&
                t('tool.tab.resources_permissions_selected')}
              {type === 'prompts' && t('tool.tab.prompts_permissions_selected')}
              {type === 'prompt_templates' && t('tool.tab.prompt_templates_permissions_selected')}{' '}
              {allSelectedCount} / {totalCount}{' '}
              {t('tool.tab.permissions_suffix')}
            </Text>
            {filteredGroups.length < groupedPermissions.length && (
              <Text type='warning'>
                ({t('tool.tab.filtered_services')} {filteredGroups.length} /{' '}
                {groupedPermissions.length} {t('tool.tab.services_suffix')})
              </Text>
            )}
          </Space>
        </Space>
      </Card>

      {/* 权限列表 */}
      {filteredGroups.length === 0 ? (
        <Card size='small'>
          <div style={{ textAlign: 'center', padding: '20px' }}>
            {searchValue
              ? t('tool.tab.no_match_found')
              : t('tool.tab.no_permissions_available')}
          </div>
        </Card>
      ) : (
        <div>
          {filteredGroups.map((group) => {
            const isServerSelected =
              group.selectedCount === group.permissions.length
            const isServerIndeterminate =
              group.selectedCount > 0 &&
              group.selectedCount < group.permissions.length
            const isExpanded = expandedServers.includes(group.server)

            return (
              <Card
                key={group.server}
                size='small'
                style={{ marginBottom: 8, overflow: 'hidden' }}
                title={
                  <Space>
                    <Checkbox
                      checked={isServerSelected}
                      indeterminate={isServerIndeterminate}
                      onChange={(e) =>
                        handleServerSelect(group.server, e.target.checked)
                      }
                      disabled={disabled}>
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          flexWrap: 'wrap',
                          gap: '4px',
                        }}>
                        <Text strong>{group.server}</Text>
                        <Text type='secondary'>
                          ({group.selectedCount}/{group.permissions.length})
                        </Text>
                        {(() => {
                          const serverDescription = permissionItems.find(
                            (item) => item.id.startsWith(group.server + '__'),
                          )?.description
                          if (!serverDescription) return null

                          return (
                            <Tooltip title={serverDescription} placement='top'>
                              <Text
                                type='secondary'
                                ellipsis
                                style={{
                                  fontSize: '12px',
                                  overflow: 'hidden',
                                  textOverflow: 'ellipsis',
                                  whiteSpace: 'nowrap',
                                  flex: 1,
                                  minWidth: 0,
                                }}>
                                - {serverDescription}
                              </Text>
                            </Tooltip>
                          )
                        })()}
                      </div>
                    </Checkbox>
                  </Space>
                }
                extra={
                  <Button
                    type='text'
                    size='small'
                    icon={isExpanded ? <UpOutlined /> : <DownOutlined />}
                    onClick={() => toggleServerExpanded(group.server)}>
                    {isExpanded ? t('tool.tab.collapse') : t('tool.tab.expand')}
                  </Button>
                }>
                {isExpanded && (
                  <Checkbox.Group
                    value={group.permissions.filter((p) =>
                      selectedPermissions.includes(p),
                    )}
                    onChange={handlePermissionChange}
                    disabled={disabled}
                    style={{ width: '100%' }}>
                    <Space orientation='vertical' style={{ width: '100%' }}>
                      {group.permissions.map((permission) => (
                        <div key={permission} style={{ paddingLeft: 24 }}>
                          <Checkbox value={permission}>
                            <div
                              style={{
                                display: 'flex',
                                alignItems: 'center',
                                flexWrap: 'wrap',
                                gap: '4px',
                              }}>
                              <Text>
                                {type === 'prompt_templates'
                                  ? (() => {
                                      const item = permissionItems.find(
                                        (item) => item.id === permission,
                                      )
                                      return item
                                        ? (item as any).name || permission
                                        : permission
                                    })()
                                  : permission.split('__')[1] || permission}
                              </Text>
                              {(() => {
                                const description =
                                  getPermissionDescription(permission)
                                if (!description) return null

                                return (
                                  <Tooltip
                                    title={description}
                                    placement='right'>
                                    <Text
                                      type='secondary'
                                      ellipsis
                                      style={{
                                        fontSize: '11px',
                                        fontStyle: 'italic',
                                        overflow: 'hidden',
                                        // textOverflow: 'ellipsis',
                                        whiteSpace: 'nowrap',
                                        flex: 1,
                                        minWidth: 0,
                                      }}>
                                      - {description}
                                    </Text>
                                  </Tooltip>
                                )
                              })()}
                            </div>
                          </Checkbox>
                        </div>
                      ))}
                    </Space>
                  </Checkbox.Group>
                )}
              </Card>
            )
          })}
        </div>
      )}
    </div>
  )
}

export default PermissionTab
