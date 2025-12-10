import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { PermissionType } from '../types/permissions'

export interface PermissionUpdateRequest {
  token_id: string
  permission_type: 'tools' | 'resources' | 'prompts' | 'prompt_templates'
  permission_value: string
}

// 新的统一权限更新请求类型（与后端保持一致）
export interface UnifiedPermissionUpdateRequest {
  token_id: string
  resource_type: PermissionType
  resource_id: string
  action: 'add' | 'remove'
}

// TokenPermissionsResponse has been removed - permissions are now included in list_tokens response

export interface FieldUpdateRequest {
  token_id: string
  field: 'name' | 'description'
  value: string
}

export interface PermissionUpdateResponse {
  token: any
}

// 简化的权限更新响应
export interface SimplePermissionUpdateResponse {
  success: boolean
  message: string
}

export interface UpdateStatus {
  loading: boolean
  error: string | null
  success: boolean
}

// 权限更新服务
class PermissionService {
  private pendingRequests = new Map<string, AbortController>()

  // 添加权限
  async addPermission(request: PermissionUpdateRequest): Promise<PermissionUpdateResponse> {
    const key = `add_${request.token_id}_${request.permission_type}_${request.permission_value}`

    // 取消之前的相同请求
    if (this.pendingRequests.has(key)) {
      this.pendingRequests.get(key)?.abort()
    }

    const controller = new AbortController()
    this.pendingRequests.set(key, controller)

    try {
      const response = await invoke<PermissionUpdateResponse>('add_token_permission', {
        request
      })
      return response
    } finally {
      this.pendingRequests.delete(key)
    }
  }

  // 移除权限
  async removePermission(request: PermissionUpdateRequest): Promise<PermissionUpdateResponse> {
    const key = `remove_${request.token_id}_${request.permission_type}_${request.permission_value}`

    // 取消之前的相同请求
    if (this.pendingRequests.has(key)) {
      this.pendingRequests.get(key)?.abort()
    }

    const controller = new AbortController()
    this.pendingRequests.set(key, controller)

    try {
      const response = await invoke<PermissionUpdateResponse>('remove_token_permission', {
        request
      })
      return response
    } finally {
      this.pendingRequests.delete(key)
    }
  }

  // 更新字段
  async updateField(request: FieldUpdateRequest): Promise<PermissionUpdateResponse> {
    const key = `field_${request.token_id}_${request.field}`

    // 取消之前的相同请求
    if (this.pendingRequests.has(key)) {
      this.pendingRequests.get(key)?.abort()
    }

    const controller = new AbortController()
    this.pendingRequests.set(key, controller)

    try {
      const response = await invoke<PermissionUpdateResponse>('update_token_field', {
        request
      })
      return response
    } finally {
      this.pendingRequests.delete(key)
    }
  }

  // 取消所有进行中的请求
  cancelAllRequests(): void {
    this.pendingRequests.forEach((controller) => {
      controller.abort()
    })
    this.pendingRequests.clear()
  }

  // 旧的 updatePermission 方法已移除，使用新的类型安全方法

  // 确保返回的是 Error 对象
  private ensureError(error: any): Error {
    if (error instanceof Error) {
      return error
    }

    // 如果不是 Error 对象，包装成 Error
    const errorMessage = error?.message || error?.toString() || 'Unknown error'
    return new Error(errorMessage)
  }

  // 字段更新（移除重试逻辑，直接调用基础方法）
  async updateFieldWithoutRetry(
    request: FieldUpdateRequest
  ): Promise<PermissionUpdateResponse> {
    try {
      return await this.updateField(request)
    } catch (error) {
      // 确保返回Error对象，直接抛出让上层处理
      throw this.ensureError(error)
    }
  }

  // 统一的权限更新方法 - 调用新的后端接口
  async updateTokenPermission(
    tokenId: string,
    resourceType: PermissionType,
    resourceId: string,
    isAdd: boolean
  ): Promise<SimplePermissionUpdateResponse> {
    const key = `unified_${tokenId}_${resourceType}_${resourceId}_${isAdd ? 'add' : 'remove'}`

    // 取消之前的相同请求
    if (this.pendingRequests.has(key)) {
      this.pendingRequests.get(key)?.abort()
    }

    const controller = new AbortController()
    this.pendingRequests.set(key, controller)

    try {
      // 调用新的统一权限更新接口
      const request: UnifiedPermissionUpdateRequest = {
        token_id: tokenId,
        resource_type: resourceType,
        resource_id: resourceId,
        action: isAdd ? 'add' : 'remove'
      }

      const response = await invoke<SimplePermissionUpdateResponse>('update_token_permission', {
        request
      })
      return response
    } finally {
      this.pendingRequests.delete(key)
    }
  }

  // getTokenPermissions has been removed - permissions are now included in list_tokens response

  // 兼容性方法：从字符串类型转换为枚举类型
  async updatePermission(
    tokenId: string,
    permissionType: string,
    permissionValue: string,
    isAdd: boolean
  ): Promise<SimplePermissionUpdateResponse> {
    // 将字符串类型转换为PermissionType枚举
    let enumPermissionType: PermissionType
    switch (permissionType) {
      case 'tools':
        enumPermissionType = PermissionType.TOOLS
        break
      case 'resources':
        enumPermissionType = PermissionType.RESOURCES
        break
      case 'prompts':
        enumPermissionType = PermissionType.PROMPTS
        break
      case 'prompt_templates':
        enumPermissionType = PermissionType.PROMPT_TEMPLATES
        break
      default:
        throw new Error(`Invalid permission type: ${permissionType}`)
    }

    return await this.updateTokenPermission(tokenId, enumPermissionType, permissionValue, isAdd)
  }
}

export const permissionService = new PermissionService()

// Hook for managing permission update status
export function usePermissionUpdateStatus() {
  const [statusMap, setStatusMap] = React.useState<Map<string, UpdateStatus>>(new Map())

  const updateStatus = (key: string, updates: Partial<UpdateStatus>) => {
    setStatusMap(prev => {
      const newMap = new Map(prev)
      const current = newMap.get(key) || { loading: false, error: null, success: false }
      newMap.set(key, { ...current, ...updates })
      return newMap
    })
  }

  const clearStatus = (key: string) => {
    setStatusMap(prev => {
      const newMap = new Map(prev)
      newMap.delete(key)
      return newMap
    })
  }

  const getStatus = (key: string): UpdateStatus => {
    return statusMap.get(key) || { loading: false, error: null, success: false }
  }

  return {
    updateStatus,
    clearStatus,
    getStatus
  }
}