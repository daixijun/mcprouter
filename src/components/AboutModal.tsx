import { Button, Modal, Typography } from 'antd'
import React from 'react'

const { Title, Paragraph, Text } = Typography

interface AboutModalProps {
  isOpen: boolean
  onClose: () => void
}

const AboutModal: React.FC<AboutModalProps> = ({ isOpen, onClose }) => {
  return (
    <Modal
      open={isOpen}
      onCancel={onClose}
      title='关于 MCP Router'
      footer={[
        <Button key='ok' type='primary' onClick={onClose}>
          确定
        </Button>,
      ]}
      width={600}>
      <div className='space-y-6'>
        {/* 头部描述 */}
        <Paragraph className='mb-6'>
          现代化 MCP 聚合管理工具，帮助你统一管理和使用多种 MCP 服务。
        </Paragraph>

        {/* 功能特性 */}
        <div className='rounded-lg p-4'>
          <Title level={4} className='!mb-3'>
            功能特性
          </Title>
          <ul className='list-disc list-inside space-y-1 ml-2'>
            <li>MCP 服务器管理与监控</li>
            <li>应用市场集成，便捷安装与配置</li>
            <li>系统托盘支持与快捷导航</li>
            <li>自动启动与跨平台支持</li>
          </ul>
        </div>

        {/* 版本与版权 */}
        <div className='rounded-lg p-4'>
          <Title level={4} className='!mb-3'>
            版本与版权
          </Title>
          <div className='space-y-1'>
            <Text className='block'>MCP Router v0.1.0</Text>
            <Text className='block'>
              © 2025 MCP Router. All rights reserved.
            </Text>
          </div>
        </div>
      </div>
    </Modal>
  )
}

export default AboutModal
