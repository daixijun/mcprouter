import { listen } from '@tauri-apps/api/event'
import {
  App,
  Button,
  Card,
  Checkbox,
  Flex,
  Input,
  Space,
  Switch,
  Typography,
} from 'antd'
import { CheckSquare, RefreshCw, Search, Square } from 'lucide-react'
import React, { useEffect, useMemo, useState } from 'react'
import { ToolService } from '../services/tool-service'
import type { McpServerInfo, Tool } from '../types'

const { Text } = Typography

interface ToolManagerProps {
  mcpServer: McpServerInfo
}

const ToolManager: React.FC<ToolManagerProps> = ({ mcpServer }) => {
  const { message } = App.useApp()
  const [tools, setTools] = useState<Tool[]>([])
  const [loading, setLoading] = useState(false)
  const [updating, setUpdating] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedTools, setSelectedTools] = useState<Set<string>>(new Set())
  const [refreshVersion, setRefreshVersion] = useState(0)

  useEffect(() => {
    loadTools()
    // 重置搜索查询和选中状态当服务器切换时
    setSearchQuery('')
    setSelectedTools(new Set())
  }, [mcpServer.name, refreshVersion]) // 添加 refreshVersion 依赖

  useEffect(() => {
    let unlisten: (() => void) | undefined
    ;(async () => {
      unlisten = await listen<string>('tools-updated', (e) => {
        if (e.payload === mcpServer.name) {
          setRefreshVersion((prev) => prev + 1)
        }
      })
    })()
    return () => {
      if (unlisten) unlisten()
    }
  }, [mcpServer.name])

  const loadTools = async () => {
    setLoading(true)
    try {
      // 直接从数据库获取工具列表（无需连接服务）
      const serverTools = await ToolService.listMcpServerTools(mcpServer.name)
      setTools(serverTools)
      console.log(`✅ 成功加载 ${serverTools.length} 个工具`)
    } catch (error) {
      console.error('Failed to load tools:', error)
      message.error('加载工具列表失败')
    } finally {
      setLoading(false)
    }
  }

  // 手动刷新工具列表
  const handleRefresh = () => {
    console.log('🔄 手动刷新工具列表')
    setRefreshVersion((prev) => prev + 1)
  }

  // 过滤工具列表
  const filteredTools = useMemo(() => {
    if (!searchQuery.trim()) {
      return tools
    }

    const query = searchQuery.toLowerCase().trim()
    return tools.filter(
      (tool) =>
        tool.name.toLowerCase().includes(query) ||
        (tool.description && tool.description.toLowerCase().includes(query)),
    )
  }, [tools, searchQuery])

  // 清理无效的选中项（当搜索或工具列表变化时）
  useEffect(() => {
    const validToolNames = new Set(filteredTools.map((tool) => tool.name))
    setSelectedTools((prev) => {
      const newSet = new Set<string>()
      prev.forEach((name) => {
        if (validToolNames.has(name)) {
          newSet.add(name)
        }
      })
      return newSet
    })
  }, [filteredTools])

  // 检查是否全选
  const isAllSelected =
    filteredTools.length > 0 && selectedTools.size === filteredTools.length
  // 检查是否部分选中
  const isIndeterminate =
    selectedTools.size > 0 && selectedTools.size < filteredTools.length

  // 全选/反选
  const handleSelectAll = (checked: boolean) => {
    if (checked) {
      setSelectedTools(new Set(filteredTools.map((tool) => tool.name)))
    } else {
      setSelectedTools(new Set())
    }
  }

  // 单个选择
  const handleSelectTool = (toolName: string, checked: boolean) => {
    setSelectedTools((prev) => {
      const newSet = new Set(prev)
      if (checked) {
        newSet.add(toolName)
      } else {
        newSet.delete(toolName)
      }
      return newSet
    })
  }

  // 批量启用
  const handleBatchEnable = async () => {
    if (selectedTools.size === 0) {
      message.warning('请先选择要启用的工具')
      return
    }

    setUpdating('batch-enable')

    try {
      // 逐个启用选中的工具
      const promises = Array.from(selectedTools).map((toolName) =>
        ToolService.toggleMcpServerTool(mcpServer.name, toolName, true),
      )
      await Promise.all(promises)
      message.success(`已启用 ${selectedTools.size} 个工具`)
      setSelectedTools(new Set())
      // 重新加载工具列表以获取最新状态
      await loadTools()
    } catch (error) {
      console.error('Failed to enable tools:', error)
      message.error('批量启用工具失败')
    } finally {
      setUpdating(null)
    }
  }

  // 批量禁用
  const handleBatchDisable = async () => {
    if (selectedTools.size === 0) {
      message.warning('请先选择要禁用的工具')
      return
    }

    setUpdating('batch-disable')

    try {
      // 逐个禁用选中的工具
      const promises = Array.from(selectedTools).map((toolName) =>
        ToolService.toggleMcpServerTool(mcpServer.name, toolName, false),
      )
      await Promise.all(promises)
      message.success(`已禁用 ${selectedTools.size} 个工具`)
      setSelectedTools(new Set())
      // 重新加载工具列表以获取最新状态
      await loadTools()
    } catch (error) {
      console.error('Failed to disable tools:', error)
      message.error('批量禁用工具失败')
    } finally {
      setUpdating(null)
    }
  }

  const handleToggleTool = async (toolName: string, enabled: boolean) => {
    setUpdating(toolName)

    try {
      await ToolService.toggleMcpServerTool(mcpServer.name, toolName, enabled)
      message.success(`工具已${enabled ? '启用' : '禁用'}`)
      // 重新加载工具列表以获取最新状态
      await loadTools()
    } catch (error) {
      console.error('Failed to toggle tool:', error)
      message.error('切换工具状态失败')
    } finally {
      setUpdating(null)
    }
  }

  const handleEnableAll = async () => {
    setUpdating('all')

    try {
      await ToolService.enableAllMcpServerTools(mcpServer.name)
      message.success('已启用所有工具')
      // 重新加载工具列表以获取最新状态
      await loadTools()
    } catch (error) {
      console.error('Failed to enable all tools:', error)
      message.error('启用所有工具失败')
    } finally {
      setUpdating(null)
    }
  }

  const handleDisableAll = async () => {
    setUpdating('all')

    try {
      await ToolService.disableAllMcpServerTools(mcpServer.name)
      message.success('已禁用所有工具')
      // 重新加载工具列表以获取最新状态
      await loadTools()
    } catch (error) {
      console.error('Failed to disable all tools:', error)
      message.error('禁用所有工具失败')
    } finally {
      setUpdating(null)
    }
  }

  if (loading) {
    return (
      <Flex justify='center' align='center' style={{ height: '128px' }}>
        <Button loading>加载工具中...</Button>
      </Flex>
    )
  }

  const displayToolsCount = filteredTools.length
  const totalToolsCount = tools.length

  // 统计启用的和禁用的工具数量
  const enabledToolsCount = filteredTools.filter((tool) => tool.enabled).length
  const disabledToolsCount = displayToolsCount - enabledToolsCount

  if (totalToolsCount === 0) {
    return (
      <Card>
        <Flex justify='center' align='center' style={{ height: '128px' }}>
          <Text className='text-gray-500 '>该服务暂无可用工具</Text>
        </Flex>
      </Card>
    )
  }

  return (
    <Flex
      vertical
      gap='middle'
      style={{ height: '100%', position: 'relative' }}>
      {/* Sticky Header with search and batch operations */}
      <div
        style={{
          position: 'sticky',
          top: 0,
          zIndex: 10,
          padding: '16px',
          borderRadius: '8px',
          boxShadow: '0 2px 8px rgba(0, 0, 0, 0.08)',
        }}>
        <Flex vertical gap='small'>
          <Flex justify='space-between' align='center' wrap='wrap'>
            <Text strong>
              工具清单 ( 启用:{' '}
              <span
                style={{ color: 'var(--color-success)', fontWeight: 'bold' }}>
                {enabledToolsCount}
              </span>{' '}
              | 禁用:{' '}
              <span style={{ color: 'var(--color-error)', fontWeight: 'bold' }}>
                {disabledToolsCount}
              </span>{' '}
              |{' '}
              {totalToolsCount !== displayToolsCount
                ? `符合条件: ${displayToolsCount}/${totalToolsCount}`
                : `总计: ${totalToolsCount}`}
              )
            </Text>
            <Space wrap>
              <Button
                onClick={handleEnableAll}
                loading={updating === 'all'}
                size='small'
                icon={<CheckSquare size={14} />}
                disabled={
                  filteredTools.length === 0 ||
                  filteredTools.every((tool) => tool.enabled)
                }>
                全部启用
              </Button>
              <Button
                onClick={handleDisableAll}
                loading={updating === 'all'}
                size='small'
                icon={<Square size={14} />}
                disabled={
                  filteredTools.length === 0 ||
                  filteredTools.every((tool) => !tool.enabled)
                }>
                全部禁用
              </Button>
              <Button
                onClick={handleRefresh}
                loading={loading}
                size='small'
                icon={<RefreshCw size={14} />}
                title='刷新工具列表'>
                刷新
              </Button>
              <Button
                onClick={handleBatchEnable}
                loading={updating === 'batch-enable'}
                size='small'
                type='primary'
                disabled={selectedTools.size === 0}>
                启用选中 ({selectedTools.size})
              </Button>
              <Button
                onClick={handleBatchDisable}
                loading={updating === 'batch-disable'}
                size='small'
                danger
                disabled={selectedTools.size === 0}>
                禁用选中 ({selectedTools.size})
              </Button>
            </Space>
          </Flex>
          <Input
            placeholder='搜索工具名称或描述...'
            prefix={<Search size={16} />}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            allowClear
            size='small'
          />
          {/* 全选复选框 - 移到粘性头部 */}
          {filteredTools.length > 0 && (
            <Checkbox
              checked={isAllSelected}
              indeterminate={isIndeterminate}
              onChange={(e) => handleSelectAll(e.target.checked)}
              style={{ fontWeight: 500 }}>
              全选 ({selectedTools.size} / {filteredTools.length})
            </Checkbox>
          )}
        </Flex>
      </div>

      {/* Scrollable Tool List */}
      <div style={{ flex: 1, overflow: 'auto' }}>
        {displayToolsCount === 0 ? (
          <Flex justify='center' align='center' style={{ height: '256px' }}>
            <Text className='text-gray-500'>
              {searchQuery.trim() ? '未找到匹配的工具' : '该服务暂无可用工具'}
            </Text>
          </Flex>
        ) : (
          <Flex vertical gap='small'>
            {filteredTools.map((tool) => (
              <Card key={tool.name} size='small'>
                <Flex justify='space-between' align='center'>
                  <Flex
                    align='center'
                    gap='small'
                    style={{ flex: 1, minWidth: 0 }}>
                    <Checkbox
                      checked={selectedTools.has(tool.name)}
                      onChange={(e) =>
                        handleSelectTool(tool.name, e.target.checked)
                      }
                    />
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <Text
                        strong
                        style={{
                          fontSize: '14px',
                          display: 'block',
                          marginBottom: '4px',
                        }}>
                        {tool.name}
                      </Text>
                      {tool.description && (
                        <Text style={{ fontSize: '12px', display: 'block' }}>
                          {tool.description}
                        </Text>
                      )}
                    </div>
                  </Flex>
                  <Switch
                    checked={tool.enabled}
                    onChange={(checked) => handleToggleTool(tool.name, checked)}
                    loading={updating === tool.name}
                    checkedChildren='启用'
                    unCheckedChildren='禁用'
                    size='small'
                  />
                </Flex>
              </Card>
            ))}
          </Flex>
        )}
      </div>
    </Flex>
  )
}

export default ToolManager
