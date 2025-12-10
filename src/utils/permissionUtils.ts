/**
 * 权限管理工具函数
 * 本小姐精心设计的权限验证和管理工具，完美支持 prompt templates 权限控制！
 */

import {
  PermissionResourceType,
  PermissionAction,
  PermissionCheckRequest,
  PermissionCheckResult,
  TokenPermissions,
  PermissionPolicy,
  PermissionCondition,
  TokenWithPermissions,
  PermissionType
} from '../types/permissions'

/**
 * 权限验证器类
 * 提供完整的权限验证功能
 */
export class PermissionValidator {
  private policies: Map<string, PermissionPolicy> = new Map()
  private tokenCache: Map<string, TokenWithPermissions> = new Map()
  private cacheTimeout = 300000 // 5分钟缓存

  constructor() {
    this.initializeDefaultPolicies()
  }

  /**
   * 初始化默认权限策略
   */
  private initializeDefaultPolicies(): void {
    const defaultPolicies: PermissionPolicy[] = [
      {
        id: 'admin-full-access',
        name: '管理员完全访问',
        description: '管理员拥有所有资源的完全访问权限',
        priority: 1000,
        enabled: true,
        created_at: Date.now(),
        updated_at: Date.now(),
        rules: [
          {
            id: 'admin-allow-all',
            resource_type: PermissionResourceType.TOOL,
            action: PermissionAction.EXECUTE,
            effect: 'allow',
            priority: 1000
          },
          {
            id: 'admin-template-manage',
            resource_type: PermissionResourceType.PROMPT_TEMPLATE,
            action: PermissionAction.MANAGE,
            effect: 'allow',
            priority: 1000
          }
        ]
      },
      {
        id: 'basic-template-access',
        name: '基础模板访问',
        description: '基础用户只能访问公共模板',
        priority: 100,
        enabled: true,
        created_at: Date.now(),
        updated_at: Date.now(),
        rules: [
          {
            id: 'basic-public-templates',
            resource_type: PermissionResourceType.PROMPT_TEMPLATE,
            action: PermissionAction.READ,
            effect: 'allow',
            conditions: [
              { field: 'access_level', operator: 'eq', value: 'public' },
              { field: 'is_system', operator: 'eq', value: true }
            ],
            priority: 100
          }
        ]
      }
    ]

    defaultPolicies.forEach(policy => {
      this.policies.set(policy.id, policy)
    })
  }

  /**
   * 检查权限
   */
  async checkPermission(request: PermissionCheckRequest): Promise<PermissionCheckResult> {
    try {
      const token = await this.getTokenInfo(request.token_id)
      if (!token) {
        return this.denyResult(request, 'Token not found or expired')
      }

      if (!token.enabled) {
        return this.denyResult(request, 'Token is disabled')
      }

      if (token.is_expired) {
        return this.denyResult(request, 'Token has expired')
      }

      // 检查基于角色的权限
      const roleResult = await this.checkRoleBasedPermission(token, request)
      if (!roleResult.allowed) {
        return roleResult
      }

      // 检查基于策略的权限
      const policyResult = await this.checkPolicyBasedPermission(token, request)
      if (!policyResult.allowed) {
        return policyResult
      }

      // 检查基于列表的权限
      const listResult = await this.checkListBasedPermission(token, request)
      if (!listResult.allowed) {
        return listResult
      }

      return this.allowResult(request, 'Access granted')
    } catch (error) {
      console.error('Permission check error:', error)
      return this.denyResult(request, 'Permission check failed')
    }
  }

  /**
   * 获取 Token 信息（带缓存）
   */
  private async getTokenInfo(tokenId?: string): Promise<TokenWithPermissions | null> {
    if (!tokenId) return null

    const cached = this.tokenCache.get(tokenId)
    if (cached && (Date.now() - cached.created_at) < this.cacheTimeout) {
      return cached
    }

    try {
      // 这里应该调用实际的 API 获取 Token 信息
      // const tokenInfo = await invoke<TokenWithPermissions>('get_token_with_permissions', { tokenId })
      // 暂时返回 null，实际使用时需要实现
      return null
    } catch (error) {
      console.error('Failed to get token info:', error)
      return null
    }
  }

  /**
   * 检查基于角色的权限
   */
  private async checkRoleBasedPermission(
    token: TokenWithPermissions,
    request: PermissionCheckRequest
  ): Promise<PermissionCheckResult> {
    if (token.roles?.includes('admin')) {
      return this.allowResult(request, 'Admin access granted')
    }

    if (token.roles?.includes('premium') &&
        request.action === PermissionAction.READ &&
        request.resource_type === PermissionResourceType.PROMPT_TEMPLATE) {
      return this.allowResult(request, 'Premium user access granted')
    }

    return this.allowResult(request, 'Role-based check passed') // 继续其他检查
  }

  /**
   * 检查基于策略的权限
   */
  private async checkPolicyBasedPermission(
    token: TokenWithPermissions,
    request: PermissionCheckRequest
  ): Promise<PermissionCheckResult> {
    const applicablePolicies = Array.from(this.policies.values())
      .filter(policy => policy.enabled &&
        (!token.policies?.includes(policy.id) || token.policies.includes(policy.id)))
      .sort((a, b) => b.priority - a.priority)

    for (const policy of applicablePolicies) {
      for (const rule of policy.rules.sort((a, b) => b.priority - a.priority)) {
        if (rule.resource_type !== request.resource_type || rule.action !== request.action) {
          continue
        }

        const matchesConditions = await this.evaluateConditions(rule.conditions, token, request)
        if (matchesConditions) {
          if (rule.effect === 'deny') {
            return this.denyResult(request, `Denied by policy: ${policy.name}`)
          } else if (rule.effect === 'allow') {
            return this.allowResult(request, `Allowed by policy: ${policy.name}`)
          }
        }
      }
    }

    return this.allowResult(request, 'No policy matched') // 继续其他检查
  }

  /**
   * 检查基于列表的权限
   */
  private async checkListBasedPermission(
    token: TokenWithPermissions,
    request: PermissionCheckRequest
  ): Promise<PermissionCheckResult> {
    const permissions = token.permissions

    switch (request.resource_type) {
      case PermissionResourceType.TOOL:
        if (permissions.allowed_tools &&
            !permissions.allowed_tools.includes(request.resource_id)) {
          return this.denyResult(request, 'Tool not in allowed list')
        }
        break

      case PermissionResourceType.RESOURCE:
        if (permissions.allowed_resources &&
            !permissions.allowed_resources.includes(request.resource_id)) {
          return this.denyResult(request, 'Resource not in allowed list')
        }
        break

      case PermissionResourceType.PROMPT_TEMPLATE:
        // 检查模板权限
        if (permissions.allowed_prompt_templates &&
            !permissions.allowed_prompt_templates.includes(request.resource_id)) {
          return this.denyResult(request, 'Prompt template not in allowed list')
        }

        // 检查细粒度权限
        const templateAccess = permissions.prompt_template_access
        if (templateAccess) {
          const actionPermissions = templateAccess[request.action as keyof typeof templateAccess]
          if (actionPermissions && !actionPermissions.includes(request.resource_id)) {
            return this.denyResult(request, `Action ${request.action} not allowed for template`)
          }
        }
        break

      case PermissionResourceType.PROMPT_CATEGORY:
        if (permissions.allowed_prompt_categories &&
            !permissions.allowed_prompt_categories.includes(request.resource_id)) {
          return this.denyResult(request, 'Prompt category not in allowed list')
        }
        break
    }

    return this.allowResult(request, 'List-based check passed')
  }

  /**
   * 评估权限条件
   */
  private async evaluateConditions(
    conditions: PermissionCondition[] | undefined,
    token: TokenWithPermissions,
    request: PermissionCheckRequest
  ): Promise<boolean> {
    if (!conditions || conditions.length === 0) {
      return true
    }

    for (const condition of conditions) {
      const value = this.getFieldValue(condition.field, token, request)
      if (!this.evaluateCondition(value, condition.operator, condition.value)) {
        return false
      }
    }

    return true
  }

  /**
   * 获取字段值
   */
  private getFieldValue(
    field: string,
    token: TokenWithPermissions,
    request: PermissionCheckRequest
  ): any {
    switch (field) {
      case 'access_level':
        return token.access_level
      case 'owner_id':
        return token.id
      case 'resource_id':
        return request.resource_id
      case 'tags':
        return [] // 需要从实际的模板资源获取
      default:
        return null
    }
  }

  /**
   * 评估单个条件
   */
  private evaluateCondition(value: any, operator: string, expected: any): boolean {
    switch (operator) {
      case 'eq':
        return value === expected
      case 'ne':
        return value !== expected
      case 'in':
        return Array.isArray(expected) && expected.includes(value)
      case 'not_in':
        return Array.isArray(expected) && !expected.includes(value)
      case 'contains':
        return typeof value === 'string' && value.includes(expected)
      case 'starts_with':
        return typeof value === 'string' && value.startsWith(expected)
      case 'ends_with':
        return typeof value === 'string' && value.endsWith(expected)
      default:
        return false
    }
  }

  /**
   * 创建允许结果
   */
  private allowResult(request: PermissionCheckRequest, reason: string): PermissionCheckResult {
    return {
      allowed: true,
      reason,
      resource_type: request.resource_type,
      resource_id: request.resource_id,
      action: request.action
    }
  }

  /**
   * 创建拒绝结果
   */
  private denyResult(request: PermissionCheckRequest, reason: string): PermissionCheckResult {
    return {
      allowed: false,
      reason,
      resource_type: request.resource_type,
      resource_id: request.resource_id,
      action: request.action
    }
  }

  /**
   * 清除缓存
   */
  clearCache(tokenId?: string): void {
    if (tokenId) {
      this.tokenCache.delete(tokenId)
    } else {
      this.tokenCache.clear()
    }
  }

  /**
   * 添加权限策略
   */
  addPolicy(policy: PermissionPolicy): void {
    this.policies.set(policy.id, policy)
  }

  /**
   * 移除权限策略
   */
  removePolicy(policyId: string): boolean {
    return this.policies.delete(policyId)
  }

  /**
   * 获取所有策略
   */
  getPolicies(): PermissionPolicy[] {
    return Array.from(this.policies.values())
  }
}

// 全局权限验证器实例
export const permissionValidator = new PermissionValidator()

/**
 * 便捷函数：检查权限
 */
export async function checkPermission(request: PermissionCheckRequest): Promise<PermissionCheckResult> {
  return permissionValidator.checkPermission(request)
}

/**
 * 便捷函数：检查 Token 是否可以访问指定的 prompt template
 */
export async function canAccessPromptTemplate(
  tokenId: string,
  templateId: string,
  action: PermissionAction = PermissionAction.READ
): Promise<boolean> {
  const result = await checkPermission({
    token_id: tokenId,
    resource_type: PermissionResourceType.PROMPT_TEMPLATE,
    resource_id: templateId,
    action
  })
  return result.allowed
}

/**
 * 便捷函数：过滤可访问的 prompt templates
 */
export async function filterAccessiblePromptTemplates(
  tokenId: string,
  templateIds: string[],
  action: PermissionAction = PermissionAction.READ
): Promise<string[]> {
  const results = await Promise.all(
    templateIds.map(async (templateId) => {
      const canAccess = await canAccessPromptTemplate(tokenId, templateId, action)
      return canAccess ? templateId : null
    })
  )

  return results.filter((id): id is string => id !== null)
}

/**
 * 便捷函数：批量检查权限
 */
export async function batchCheckPermissions(
  requests: PermissionCheckRequest[]
): Promise<PermissionCheckResult[]> {
  return Promise.all(requests.map(request => checkPermission(request)))
}

/**
 * 生成权限摘要
 */
export function generatePermissionSummary(permissions: TokenPermissions): string {
  const parts: string[] = []

  if (permissions.allowed_tools?.length) {
    parts.push(`${permissions.allowed_tools.length} tools`)
  }

  if (permissions.allowed_resources?.length) {
    parts.push(`${permissions.allowed_resources.length} resources`)
  }

  if (permissions.allowed_prompt_templates?.length) {
    parts.push(`${permissions.allowed_prompt_templates.length} prompt templates`)
  }

  if (permissions.allowed_prompt_categories?.length) {
    parts.push(`${permissions.allowed_prompt_categories.length} prompt categories`)
  }

  if (parts.length === 0) {
    return 'No permissions'
  }

  return `Access to ${parts.join(', ')}`
}

// ============================================================================
// Token权限管理基础工具函数
// ============================================================================

/**
 * 权限类型验证
 */
export function validatePermissionType(type: string): PermissionType | null {
  const validTypes = Object.values(PermissionType)
  return validTypes.includes(type as PermissionType) ? type as PermissionType : null
}

/**
 * 权限值标准化
 * 将权限值转换为标准格式（小写、去除空格等）
 */
export function normalizePermissionValue(permissionValue: string): string {
  return permissionValue.trim().toLowerCase()
}