import { AlertTriangle, Home, RefreshCw } from 'lucide-react'
import React, { Component, ErrorInfo, ReactNode } from 'react'
import { useErrorContext } from '../contexts/ErrorContext'

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
  errorInfo: ErrorInfo | null
}

class ErrorBoundaryInner extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null, errorInfo: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error, errorInfo: null }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.setState({ error, errorInfo })
    console.error('ErrorBoundary caught an error:', error, errorInfo)
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback
      }

      return (
        <ErrorFallback
          error={this.state.error}
          errorInfo={this.state.errorInfo}
        />
      )
    }

    return this.props.children
  }
}

const ErrorFallback: React.FC<{
  error: Error | null
  errorInfo: ErrorInfo | null
}> = ({ error, errorInfo }) => {
  const { addError, clearErrors } = useErrorContext()

  const handleReload = () => {
    window.location.reload()
  }

  const handleGoHome = () => {
    clearErrors()
    window.location.href = '/'
  }

  const handleReportError = () => {
    if (error) {
      addError(`错误报告: ${error.message}`, 'info', {
        label: '复制错误信息',
        handler: () => {
          navigator.clipboard.writeText(
            `错误: ${error.message}\n堆栈: ${error.stack}\n组件堆栈: ${errorInfo?.componentStack}`,
          )
        },
      })
    }
  }

  return (
    <div className='min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center p-4'>
      <div className='max-w-md w-full bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6'>
        <div className='flex items-center justify-center w-12 h-12 mx-auto bg-red-100 dark:bg-red-900 rounded-full mb-4'>
          <AlertTriangle className='w-6 h-6 text-red-600 dark:text-red-400' />
        </div>
        <h1 className='text-xl font-semibold text-gray-900 dark:text-white text-center mb-2'>
          应用程序遇到了错误
        </h1>
        <p className='text-gray-600 dark:text-gray-400 text-center mb-6'>
          很抱歉，应用程序遇到了意外错误。您可以尝试刷新页面或返回主页。
        </p>

        {error && (
          <div className='bg-gray-100 dark:bg-gray-700 rounded-lg p-3 mb-6'>
            <p className='text-sm text-gray-800 dark:text-gray-200 font-mono break-all'>
              {error.message}
            </p>
          </div>
        )}

        <div className='space-y-3'>
          <button
            onClick={handleReload}
            className='w-full flex items-center justify-center space-x-2 bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-lg transition-colors'>
            <RefreshCw className='w-4 h-4' />
            <span>刷新页面</span>
          </button>

          <button
            onClick={handleGoHome}
            className='w-full flex items-center justify-center space-x-2 bg-gray-600 hover:bg-gray-700 text-white font-medium py-2 px-4 rounded-lg transition-colors'>
            <Home className='w-4 h-4' />
            <span>返回主页</span>
          </button>

          {error && (
            <button
              onClick={handleReportError}
              className='w-full bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 font-medium py-2 px-4 rounded-lg transition-colors'>
              报告错误
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

// 包装器组件，使用 Hook
export const ErrorBoundary: React.FC<Props> = (props) => {
  return <ErrorBoundaryInner {...props} />
}

export default ErrorBoundary
