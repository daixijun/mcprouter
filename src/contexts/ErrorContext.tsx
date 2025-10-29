import React, { createContext, ReactNode, useContext, useReducer } from 'react'

// 错误类型定义
export interface AppError {
  id: string
  message: string
  type: 'error' | 'warning' | 'info'
  timestamp: Date
  action?: {
    label: string
    handler: () => void
  }
}

interface ErrorState {
  errors: AppError[]
  hasError: boolean
  errorBoundary: Error | null
}

// Action 类型定义
type ErrorAction =
  | { type: 'ADD_ERROR'; payload: AppError }
  | { type: 'REMOVE_ERROR'; payload: string }
  | { type: 'CLEAR_ERRORS' }
  | { type: 'SET_ERROR_BOUNDARY'; payload: Error | null }

// 初始状态
const initialState: ErrorState = {
  errors: [],
  hasError: false,
  errorBoundary: null,
}

// Reducer
const errorReducer = (state: ErrorState, action: ErrorAction): ErrorState => {
  switch (action.type) {
    case 'ADD_ERROR':
      return {
        ...state,
        errors: [...state.errors, action.payload],
        hasError: true,
      }
    case 'REMOVE_ERROR':
      return {
        ...state,
        errors: state.errors.filter((error) => error.id !== action.payload),
        hasError: state.errors.length > 1,
      }
    case 'CLEAR_ERRORS':
      return {
        ...state,
        errors: [],
        hasError: false,
      }
    case 'SET_ERROR_BOUNDARY':
      return {
        ...state,
        errorBoundary: action.payload,
      }
    default:
      return state
  }
}

// Context 类型定义
interface ErrorContextType {
  state: ErrorState
  dispatch: React.Dispatch<ErrorAction>
  // 便捷方法
  addError: (
    message: string,
    type?: AppError['type'],
    action?: AppError['action'],
  ) => void
  removeError: (id: string) => void
  clearErrors: () => void
  setErrorBoundary: (error: Error | null) => void
}

// 创建 Context
const ErrorContext = createContext<ErrorContextType | undefined>(undefined)

// Provider 组件
interface ErrorProviderProps {
  children: ReactNode
}

export const ErrorProvider: React.FC<ErrorProviderProps> = ({ children }) => {
  const [state, dispatch] = useReducer(errorReducer, initialState)

  // 便捷方法
  const addError = (
    message: string,
    type: AppError['type'] = 'error',
    action?: AppError['action'],
  ) => {
    const error: AppError = {
      id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
      message,
      type,
      timestamp: new Date(),
      action,
    }
    dispatch({ type: 'ADD_ERROR', payload: error })
  }

  const removeError = (id: string) => {
    dispatch({ type: 'REMOVE_ERROR', payload: id })
  }

  const clearErrors = () => {
    dispatch({ type: 'CLEAR_ERRORS' })
  }

  const setErrorBoundary = (error: Error | null) => {
    dispatch({ type: 'SET_ERROR_BOUNDARY', payload: error })
  }

  const contextValue: ErrorContextType = {
    state,
    dispatch,
    addError,
    removeError,
    clearErrors,
    setErrorBoundary,
  }

  return (
    <ErrorContext.Provider value={contextValue}>
      {children}
    </ErrorContext.Provider>
  )
}

// Hook to use the context
export const useErrorContext = (): ErrorContextType => {
  const context = useContext(ErrorContext)
  if (context === undefined) {
    throw new Error('useErrorContext must be used within an ErrorProvider')
  }
  return context
}
