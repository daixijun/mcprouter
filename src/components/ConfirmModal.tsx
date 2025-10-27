import React from 'react'

interface ConfirmModalProps {
  isOpen: boolean
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  type?: 'warning' | 'danger' | 'info'
  onConfirm: () => void
  onCancel: () => void
}

const ConfirmModal: React.FC<ConfirmModalProps> = ({
  isOpen,
  title,
  message,
  confirmText = '确认',
  cancelText = '取消',
  type = 'warning',
  onConfirm,
  onCancel,
}) => {
  if (!isOpen) return null

  const getIconAndColor = () => {
    switch (type) {
      case 'danger':
        return {
          icon: '⚠️',
          bgColor: 'bg-red-50',
          borderColor: 'border-red-200',
          titleColor: 'text-red-800',
          buttonColor: 'btn-modern btn-danger-modern',
        }
      case 'warning':
        return {
          icon: '⚠️',
          bgColor: 'bg-orange-50',
          borderColor: 'border-orange-200',
          titleColor: 'text-orange-800',
          buttonColor:
            'btn-modern bg-orange-500 hover:bg-orange-600 text-white',
        }
      case 'info':
        return {
          icon: 'ℹ️',
          bgColor: 'bg-blue-50',
          borderColor: 'border-blue-200',
          titleColor: 'text-blue-800',
          buttonColor: 'btn-modern btn-primary-modern',
        }
      default:
        return {
          icon: '⚠️',
          bgColor: 'bg-gray-50',
          borderColor: 'border-gray-200',
          titleColor: 'text-gray-800',
          buttonColor: 'btn-modern btn-primary-modern',
        }
    }
  }

  const { icon, bgColor, borderColor, titleColor, buttonColor } =
    getIconAndColor()

  return (
    <div className='modal-modern'>
      <div className='modal-content-modern max-w-md compact-modal'>
        <div className={`p-4 rounded-lg border ${bgColor} ${borderColor}`}>
          {/* Icon and Title */}
          <div className='flex items-center space-x-3 mb-3'>
            <span className='text-2xl'>{icon}</span>
            <h3 className={`text-lg font-bold ${titleColor}`}>{title}</h3>
          </div>

          {/* Message */}
          <p className='text-gray-700 mb-4 whitespace-pre-line'>{message}</p>

          {/* Buttons */}
          <div className='flex justify-end space-x-3'>
            <button
              onClick={onCancel}
              className='btn-modern btn-secondary-modern text-sm px-4 py-2'>
              {cancelText}
            </button>
            <button
              onClick={onConfirm}
              className={`${buttonColor} text-sm px-4 py-2`}>
              {confirmText}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

export { ConfirmModal }
export default ConfirmModal
