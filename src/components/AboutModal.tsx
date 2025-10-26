import React from 'react'
import Modal from '../components/Modal'

interface AboutModalProps {
  isOpen: boolean
  onClose: () => void
}

const AboutModal: React.FC<AboutModalProps> = ({ isOpen, onClose }) => {
  return (
    <Modal isOpen={isOpen} onClose={onClose} maxWidth='md' maxHeight='80vh'>
      <div className='space-y-6'>
        {/* 头部 */}
        <div className='flex items-start justify-between'>
          <div>
            <h2 className='text-2xl font-bold text-gray-800 dark:text-gray-100 mb-2'>
              关于 MCP Router
            </h2>
            <p className='text-gray-600 dark:text-gray-300'>
              现代化 MCP 聚合管理工具，帮助你统一管理和使用多种 MCP 服务。
            </p>
          </div>
        </div>

        {/* 内容 */}
        <div className='space-y-4'>
          <div className='bg-gray-50 dark:bg-gray-800 rounded-lg p-4'>
            <h3 className='font-semibold text-gray-800 dark:text-gray-100 mb-2'>
              功能特性
            </h3>
            <ul className='list-disc list-inside text-gray-700 dark:text-gray-300 space-y-1'>
              <li>MCP 服务器管理与监控</li>
              <li>应用市场集成，便捷安装与配置</li>
              <li>系统托盘支持与快捷导航</li>
              <li>自动启动与跨平台支持</li>
            </ul>
          </div>
          <div className='bg-gray-50 dark:bg-gray-800 rounded-lg p-4'>
            <h3 className='font-semibold text-gray-800 dark:text-gray-100 mb-2'>
              版本与版权
            </h3>
            <p className='text-gray-700 dark:text-gray-300'>
              MCP Router v0.1.0
            </p>
            <p className='text-gray-700 dark:text-gray-300'>
              © 2025 MCP Router. All rights reserved.
            </p>
          </div>
        </div>

        {/* 操作 */}
        <div className='flex justify-end gap-3 pt-4 border-t'>
          <button
            type='button'
            onClick={onClose}
            className='btn-modern btn-primary-modern'>
            确定
          </button>
        </div>
      </div>
    </Modal>
  )
}

export default AboutModal
