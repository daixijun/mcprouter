/**
 * 权限管理相关类型定义
 * 本小姐专门为 Token 管理和 prompt templates 权限控制设计的完美类型系统！
 */

// 权限资源类型
export enum PermissionResourceType {
  TOOL = 'tool',
  RESOURCE = 'resource',
  PROMPT_TEMPLATE = 'prompt_template',
  PROMPT_CATEGORY = 'prompt_category'
}

// Token权限类型（用于Token权限管理）
export enum PermissionType {
  TOOLS = 'tools',
  RESOURCES = 'resources',
  PROMPTS = 'prompts',
  PROMPT_TEMPLATES = 'prompt_templates'
}

// 权限操作类型
export enum PermissionAction {
  READ = 'read',
  WRITE = 'write',
  DELETE = 'delete',
  EXECUTE = 'execute',
  MANAGE = 'manage'
}

// 基础权限项 - 简化版本，使用 resource_path
export interface PermissionItem {
  id: string                    // 保留 UUID 用于数据库主键
  resource_path: string         // 格式：server__resource_name
  resource_type: string         // 'tool' | 'resource' | 'prompt'
}

// Prompt Template 特殊权限项
export interface PromptTemplatePermissionItem extends PermissionItem {
  type: PermissionResourceType.PROMPT_TEMPLATE
  template_id: string
  template_name: string
  template_category?: string
  is_system?: boolean  // 是否为系统模板
  is_public?: boolean  // 是否为公共模板
  owner_id?: string    // 所有者ID
  access_level: 'private' | 'public' | 'shared'
}

// Prompt Category 权限项
export interface PromptCategoryPermissionItem extends PermissionItem {
  type: PermissionResourceType.PROMPT_CATEGORY
  category_id: string
  parent_category?: string
  template_count: number
  is_system: boolean
}

// 可用权限集合 - 简化版本
export interface AvailablePermissions {
  tools: PermissionItem[]
  resources: PermissionItem[]
  prompts: PermissionItem[]
  prompt_templates?: PermissionItem[] // 可选，暂不支持
  prompt_categories?: PromptCategoryPermissionItem[] // 可选，暂不支持
}

// Token 权限配置
export interface TokenPermissions {
  allowed_tools?: string[]
  allowed_resources?: string[]
  allowed_prompts?: string[]
  allowed_prompt_templates?: string[]
  allowed_prompt_categories?: string[]
  // 细粒度权限控制
  prompt_template_access?: {
    read?: string[]
    write?: string[]
    delete?: string[]
    execute?: string[]
  }
  // 权限策略
  permission_strategy?: 'allow_list' | 'deny_list' | 'role_based'
  // 权限等级
  access_level?: 'basic' | 'standard' | 'premium' | 'admin'
}

// 权限验证请求
export interface PermissionCheckRequest {
  token_id?: string
  resource_type: PermissionResourceType
  resource_id: string
  action: PermissionAction
  context?: Record<string, any>
}

// 权限验证结果
export interface PermissionCheckResult {
  allowed: boolean
  reason?: string
  token_id?: string
  resource_type: PermissionResourceType
  resource_id: string
  action: PermissionAction
  access_level?: string
  applied_policies?: string[]
}

// 权限策略
export interface PermissionPolicy {
  id: string
  name: string
  description: string
  rules: PermissionRule[]
  priority: number
  enabled: boolean
  created_at: number
  updated_at: number
}

// 权限规则
export interface PermissionRule {
  id: string
  resource_type: PermissionResourceType
  resource_pattern?: string  // 支持通配符，如 "template.*"
  action: PermissionAction
  effect: 'allow' | 'deny'
  conditions?: PermissionCondition[]
  priority: number
}

// 权限条件
export interface PermissionCondition {
  field: string  // 如 "access_level", "owner_id", "tags"
  operator: 'eq' | 'ne' | 'in' | 'not_in' | 'contains' | 'starts_with' | 'ends_with'
  value: any
}

// 角色定义
export interface Role {
  id: string
  name: string
  description: string
  permissions: string[]
  is_system: boolean
  created_at: number
  updated_at: number
}

// Token 扩展信息（包含角色和权限策略）
export interface TokenWithPermissions {
  id: string
  name: string
  description?: string
  permissions: TokenPermissions
  roles?: string[]
  policies?: string[]
  access_level: string
  created_at: number
  expires_at?: number
  last_used_at?: number
  usage_count: number
  is_expired: boolean
  enabled: boolean
}

// 权限管理统计
export interface PermissionStats {
  total_tokens: number
  tokens_with_prompt_permissions: number
  total_prompt_templates: number
  accessible_templates: number
  permission_distribution: {
    tools: number
    resources: number
    prompts: number
    prompt_templates: number
    prompt_categories: number
  }
  access_level_distribution: {
    basic: number
    standard: number
    premium: number
    admin: number
  }
}

// 权限审计日志
export interface PermissionAuditLog {
  id: string
  token_id: string
  resource_type: PermissionResourceType
  resource_id: string
  action: PermissionAction
  allowed: boolean
  reason?: string
  ip_address?: string
  user_agent?: string
  timestamp: number
  context?: Record<string, any>
}

// 批量权限操作
export interface BatchPermissionOperation {
  token_ids: string[]
  operation: 'grant' | 'revoke' | 'update'
  permissions: Partial<TokenPermissions>
  reason?: string
}

// 权限模板
export interface PermissionTemplate {
  id: string
  name: string
  description: string
  permissions: TokenPermissions
  roles?: string[]
  is_system: boolean
  usage_count: number
  created_at: number
  updated_at: number
}

// Token权限管理相关类型

// 权限验证结果
export interface PermissionValidationResult {
  isValid: boolean
  error?: string
  normalizedValue?: string
}

// 统一的权限更新请求
export interface EnhancedPermissionRequest {
  token_id: string
  permission_type: PermissionType
  permission_value: string
  action: 'add' | 'remove'
}

// 新的权限更新请求结构
export interface NewPermissionRequest {
  token_id: string
  permission_type: PermissionType
  resource_id: string
  server_id: string
  name: string
  action: 'add' | 'remove'
}

// 权限更新响应
export interface PermissionUpdateResponse {
  token: any
}