import { Button, Card, Flex, Space, Switch, Typography, App } from 'antd'
import { CheckSquare, Square } from 'lucide-react'
import React, { useEffect, useState } from 'react'
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

  useEffect(() => {
    loadTools()
  }, [mcpServer.name])

  const loadTools = async () => {
    setLoading(true)
    try {
      // 直接从数据库获取工具列表（无需连接服务）
      const serverTools = await ToolService.getToolsByServer(mcpServer.name)
      setTools(serverTools)
    } catch (error) {
      console.error('Failed to load tools:', error)
      message.error('加载工具列表失败')
    } finally {
      setLoading(false)
    }
  }

  const handleToggleTool = async (toolName: string, enabled: boolean) => {
    setUpdating(toolName)
    try {
      const tool = tools.find((t) => t.name === toolName)
      if (!tool) {
        message.error('未找到对应工具')
        return
      }
      await ToolService.toggleTool(tool.id, enabled)
      message.success(`工具已${enabled ? '启用' : '禁用'}`)

      // 重新加载工具以获取最新状态
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
      await ToolService.enableAllTools(mcpServer.name)
      message.success('已启用所有工具')
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
      await ToolService.disableAllTools(mcpServer.name)
      message.success('已禁用所有工具')
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

  if (tools.length === 0) {
    return (
      <Card>
        <Flex justify='center' align='center' style={{ height: '128px' }}>
          <Text className='text-gray-500 '>该服务暂无可用工具</Text>
        </Flex>
      </Card>
    )
  }

  return (
    <Flex vertical gap='middle'>
      {/* Header with batch operations */}
      <Flex justify='space-between' align='center'>
        <Text strong>工具管理 ({tools.length} 个工具)</Text>
        <Space>
          <Button
            onClick={handleEnableAll}
            loading={updating === 'all'}
            size='small'
            icon={<CheckSquare size={14} />}
            disabled={tools.every((tool) => tool.enabled)}>
            全部启用
          </Button>
          <Button
            onClick={handleDisableAll}
            loading={updating === 'all'}
            size='small'
            icon={<Square size={14} />}
            disabled={tools.every((tool) => !tool.enabled)}>
            全部禁用
          </Button>
        </Space>
      </Flex>

      {/* Tool List */}
      <Flex vertical gap='small'>
        {tools.map((tool) => (
          <Card key={tool.name} size='small'>
            <Flex justify='space-between' align='center'>
              <div style={{ flex: 1, minWidth: 0, marginRight: '16px' }}>
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
    </Flex>
  )
}

export default ToolManager
