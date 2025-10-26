import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { Check, Copy, Key, Plus, Trash2 } from 'lucide-react'
import React, { useEffect, useState } from 'react'
import ApiKeyPermissionSelector from '../components/ApiKeyPermissionSelector'
import ConfirmModal from '../components/ConfirmModal'
import Modal from '../components/Modal'
import { ApiService } from '../services/api'
import toastService from '../services/toastService'
import type { ApiKey, ApiKeyPermissions } from '../types'

const ApiKeys: React.FC = () => {
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([])
  const [loading, setLoading] = useState(true)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [showDetailsModal, setShowDetailsModal] = useState(false)
  const [showEditModal, setShowEditModal] = useState(false)
  const [showDeleteModal, setShowDeleteModal] = useState(false)
  const [selectedApiKey, setSelectedApiKey] = useState<ApiKey | null>(null)
  const [newKeyDetails, setNewKeyDetails] = useState<ApiKey | null>(null)
  const [copied, setCopied] = useState(false)

  // Form state for new API key
  const [newKeyName, setNewKeyName] = useState('')
  const [newKeyPermissions, setNewKeyPermissions] = useState<ApiKeyPermissions>({
    allowed_servers: [],
  })

  // Form state for editing permissions
  const [editPermissions, setEditPermissions] = useState<ApiKeyPermissions>({
    allowed_servers: [],
  })

  useEffect(() => {
    loadApiKeys()
  }, [])

  const loadApiKeys = async () => {
    setLoading(true)
    try {
      const keys = await ApiService.listApiKeys()
      setApiKeys(keys)
    } catch (error) {
      console.error('Failed to load API keys:', error)
      toastService.sendErrorNotification('加载API Key列表失败')
    } finally {
      setLoading(false)
    }
  }

  const handleCreateApiKey = async () => {
    if (!newKeyName.trim()) {
      toastService.sendErrorNotification('请输入API Key名称')
      return
    }

    try {
      const createdKey = await ApiService.createApiKey(
        newKeyName,
        newKeyPermissions,
      )
      toastService.sendSuccessNotification('API Key创建成功')

      // Show the full key in details modal
      setNewKeyDetails(createdKey)
      setShowCreateModal(false)
      setShowDetailsModal(true)

      // Reset form
      setNewKeyName('')
      setNewKeyPermissions({
        allowed_servers: [],
      })

      // Reload list
      loadApiKeys()
    } catch (error) {
      console.error('Failed to create API key:', error)
      toastService.sendErrorNotification('创建API Key失败')
    }
  }

  const handleCopyKey = async (key: string) => {
    try {
      await writeText(key)
      setCopied(true)
      toastService.sendSuccessNotification('API Key已复制到剪贴板')
      setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      console.error('Failed to copy key:', error)
      toastService.sendErrorNotification('复制失败')
    }
  }

  const handleToggleKey = async (id: string) => {
    try {
      await ApiService.toggleApiKey(id)
      toastService.sendSuccessNotification('API Key状态已更新')
      loadApiKeys()
    } catch (error) {
      console.error('Failed to toggle API key:', error)
      toastService.sendErrorNotification('更新失败')
    }
  }

  const handleEditPermissions = async (apiKey: ApiKey) => {
    setSelectedApiKey(apiKey)
    try {
      const details = await ApiService.getApiKeyDetails(apiKey.id)
      setEditPermissions(details.permissions ?? { allowed_servers: [] })
      setShowEditModal(true)
    } catch (error) {
      console.error('Failed to fetch API key permissions:', error)
      toastService.sendErrorNotification('获取权限信息失败')
    }
  }

  const handleSavePermissions = async () => {
    if (!selectedApiKey) return

    try {
      await ApiService.updateApiKeyPermissions(
        selectedApiKey.id,
        editPermissions,
      )
      toastService.sendSuccessNotification('权限更新成功')
      setShowEditModal(false)
      loadApiKeys()
    } catch (error) {
      console.error('Failed to update permissions:', error)
      toastService.sendErrorNotification('更新权限失败')
    }
  }

  const handleDeleteKey = async () => {
    if (!selectedApiKey) return

    try {
      await ApiService.deleteApiKey(selectedApiKey.id)
      toastService.sendSuccessNotification('API Key已删除')
      setShowDeleteModal(false)
      setSelectedApiKey(null)
      loadApiKeys()
    } catch (error) {
      console.error('Failed to delete API key:', error)
      toastService.sendErrorNotification('删除失败')
    }
  }

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  if (loading) {
    return (
      <div className='flex items-center justify-center h-64'>
        <div className='animate-spin rounded-full h-12 w-12 border-4 border-blue-500 border-t-transparent'></div>
      </div>
    )
  }

  return (
    <div className='h-full flex flex-col space-y-4 compact-container overflow-y-auto'>
      {/* Header */}
      <div className='flex items-center justify-between'>
        <div>
          <h2 className='text-2xl font-bold text-gray-800 dark:text-gray-100'>
            API Keys
          </h2>
          <p className='text-sm text-gray-600 dark:text-gray-400 mt-1'>
            管理用于访问MCP Router的API密钥
          </p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className='btn-modern btn-primary-modern flex items-center space-x-2'>
          <Plus size={16} />
          <span>创建API Key</span>
        </button>
      </div>

      {/* API Keys Table */}
      <div className='card-glass'>
        {apiKeys.length === 0 ? (
          <div className='text-center py-12'>
            <Key className='mx-auto h-12 w-12 text-gray-400 dark:text-gray-500' />
            <h3 className='mt-2 text-sm font-medium text-gray-900 dark:text-gray-100'>
              暂无API Keys
            </h3>
            <p className='mt-1 text-sm text-gray-500 dark:text-gray-400'>
              点击"创建API Key"按钮添加您的第一个API密钥
            </p>
          </div>
        ) : (
          <div className='overflow-x-auto'>
            <table className='min-w-full divide-y divide-gray-200 dark:divide-gray-700'>
              <thead className='bg-gray-50 dark:bg-gray-800'>
                <tr>
                  <th className='px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider'>
                    名称
                  </th>
                  <th className='px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider'>
                    Key
                  </th>
                  <th className='px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider'>
                    状态
                  </th>
                  <th className='px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider'>
                    创建时间
                  </th>
                  <th className='px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider'>
                    授权服务器
                  </th>
                  <th className='px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider'>
                    操作
                  </th>
                </tr>
              </thead>
              <tbody className='bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-700'>
                {apiKeys.map((apiKey) => (
                  <tr
                    key={apiKey.id}
                    className='hover:bg-gray-50 dark:hover:bg-gray-800'>
                    <td className='px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-gray-100'>
                      {apiKey.name}
                    </td>
                    <td className='px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400 font-mono'>
                      {apiKey.key}
                    </td>
                    <td className='px-6 py-4 whitespace-nowrap'>
                      <span
                        className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${
                          apiKey.enabled
                            ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400'
                            : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300'
                        }`}>
                        {apiKey.enabled ? '启用' : '禁用'}
                      </span>
                    </td>
                    <td className='px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400'>
                      {formatDate(apiKey.created_at)}
                    </td>
                    <td className='px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400'>
                      {(apiKey.permissions?.allowed_servers?.length ?? 0) > 0 ? (
                        <span className='text-blue-600 dark:text-blue-400'>
                          {apiKey.permissions?.allowed_servers?.length ?? 0} 个服务器
                        </span>
                      ) : (
                        <span className='text-gray-400 dark:text-gray-400'>
                          未授权
                        </span>
                      )}
                    </td>
                    <td className='px-6 py-4 whitespace-nowrap text-right text-sm font-medium space-x-2'>
                      <button
                        onClick={() => handleToggleKey(apiKey.id)}
                        className='text-blue-600 hover:text-blue-900 dark:text-blue-400 dark:hover:text-blue-300'>
                        {apiKey.enabled ? '禁用' : '启用'}
                      </button>
                      <button
                        onClick={() => handleEditPermissions(apiKey)}
                        className='text-indigo-600 hover:text-indigo-900 dark:text-indigo-400 dark:hover:text-indigo-300'>
                        编辑权限
                      </button>
                      <button
                        onClick={() => {
                          setSelectedApiKey(apiKey)
                          setShowDeleteModal(true)
                        }}
                        className='text-red-600 hover:text-red-900 dark:text-red-400 dark:hover:text-red-300'>
                        <Trash2 size={16} className='inline' />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Create API Key Modal */}
      <Modal isOpen={showCreateModal} onClose={() => setShowCreateModal(false)} maxWidth='3xl'>
        <div className='p-6'>
        <h2 className='text-xl font-bold text-gray-900 dark:text-gray-100 mb-4'>
          创建新API Key
        </h2>
          <div className='space-y-4'>
            <div>
              <label className='block text-sm font-medium text-gray-900 dark:text-gray-300 mb-1'>
                名称 <span className='text-red-500'>*</span>
              </label>
              <input
                type='text'
                value={newKeyName}
                onChange={(e) => setNewKeyName(e.target.value)}
                placeholder='例如: Production API Key'
                className='input-modern w-full'
              />
            </div>

            <div>
              <label className='block text-sm font-medium text-gray-900 dark:text-gray-300 mb-2'>
                权限配置
              </label>
              <ApiKeyPermissionSelector
                permissions={newKeyPermissions}
                onChange={setNewKeyPermissions}
              />
            </div>

            <div className='flex justify-end space-x-3 pt-4'>
              <button
                onClick={() => setShowCreateModal(false)}
                className='btn-modern bg-gray-200 hover:bg-gray-300 text-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-200'>
                取消
              </button>
              <button
                onClick={handleCreateApiKey}
                className='btn-modern btn-primary-modern'>
                创建
              </button>
            </div>
          </div>
        </div>
      </Modal>

      {/* API Key Details Modal (shows full key after creation) */}
      <Modal
        isOpen={showDetailsModal}
        onClose={() => {
          setShowDetailsModal(false)
          setNewKeyDetails(null)
        }}>
        {newKeyDetails && (
          <div className='p-6'>
            <h2 className='text-xl font-bold text-gray-900 dark:text-gray-100 mb-4'>
              API Key 创建成功
            </h2>
            <div className='space-y-4'>
              <div className='bg-yellow-50 dark:bg-yellow-900/30 border border-yellow-200 dark:border-yellow-700 rounded-lg p-4'>
                <p className='text-sm text-yellow-800 dark:text-yellow-200'>
                  <strong>重要提示:</strong> 这是唯一一次显示完整API
                  Key的机会,请妥善保存!
                </p>
              </div>

              <div>
                <label className='block text-sm font-medium text-gray-900 dark:text-gray-300 mb-2'>
                  API Key
                </label>
                <div className='flex items-center space-x-2'>
                  <input
                    type='text'
                    value={newKeyDetails.key}
                    readOnly
                    className='input-modern flex-1 font-mono text-sm'
                  />
                  <button
                    onClick={() => handleCopyKey(newKeyDetails.key)}
                    className='btn-modern bg-blue-500 hover:bg-blue-600 text-white flex items-center space-x-1'>
                    {copied ? <Check size={16} /> : <Copy size={16} />}
                    <span>{copied ? '已复制' : '复制'}</span>
                  </button>
                </div>
              </div>

              <div>
                <label className='block text-sm font-medium text-gray-900 dark:text-gray-300 mb-1'>
                  名称
                </label>
                <p className='text-sm text-gray-600 dark:text-gray-400'>
                  {newKeyDetails.name}
                </p>
              </div>

              <div className='flex justify-end pt-4'>
                <button
                  onClick={() => {
                    setShowDetailsModal(false)
                    setNewKeyDetails(null)
                  }}
                  className='btn-modern btn-primary-modern'>
                  关闭
                </button>
              </div>
            </div>
          </div>
        )}
      </Modal>

      {/* Edit Permissions Modal */}
      <Modal isOpen={showEditModal} onClose={() => setShowEditModal(false)}>
        <div className='p-6'>
          <h2 className='text-xl font-bold text-gray-900 dark:text-gray-100 mb-4'>
            编辑权限: {selectedApiKey?.name}
          </h2>
          <div className='space-y-4'>
            <ApiKeyPermissionSelector
              permissions={editPermissions}
              onChange={setEditPermissions}
            />

            <div className='flex justify-end space-x-3 pt-4'>
              <button
                onClick={() => setShowEditModal(false)}
                className='btn-modern bg-gray-200 hover:bg-gray-300 text-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-200'>
                取消
              </button>
              <button
                onClick={handleSavePermissions}
                className='btn-modern btn-primary-modern'>
                保存
              </button>
            </div>
          </div>
        </div>
      </Modal>

      {/* Delete Confirmation Modal */}
      <ConfirmModal
        isOpen={showDeleteModal}
        onCancel={() => setShowDeleteModal(false)}
        onConfirm={handleDeleteKey}
        title='删除API Key'
        message={`确定要删除 "${selectedApiKey?.name}" 吗?此操作无法撤销。`}
        confirmText='删除'
        cancelText='取消'
        type='danger'
      />
    </div>
  )
}

export default ApiKeys
