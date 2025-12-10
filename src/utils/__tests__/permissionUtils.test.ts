/**
 * 权限管理工具函数测试
 * 本小姐编写的完美测试用例，确保权限系统的可靠性！
 */

import {
  PermissionValidator,
  checkPermission,
  canAccessPromptTemplate,
  filterAccessiblePromptTemplates,
  batchCheckPermissions,
  generatePermissionSummary,
} from '../permissionUtils'
import {
  PermissionResourceType,
  PermissionAction,
  TokenPermissions,
  TokenWithPermissions,
  PromptTemplatePermissionItem,
  PromptCategoryPermissionItem
} from '../../types/permissions'

// Mock 数据
const mockToken: TokenWithPermissions = {
  id: 'test-token-1',
  name: 'Test Token',
  permissions: {
    allowed_tools: ['tool-1', 'tool-2'],
    allowed_resources: ['resource-1'],
    allowed_prompt_templates: ['template-1', 'template-2'],
    allowed_prompt_categories: ['category-1'],
    access_level: 'standard',
    permission_strategy: 'allow_list',
    prompt_template_access: {
      read: ['template-1', 'template-2', 'template-3'],
      write: ['template-1'],
      execute: ['template-2']
    }
  },
  access_level: 'standard',
  created_at: Date.now() - 86400000, // 1天前
  usage_count: 10,
  is_expired: false,
  enabled: true
}

const mockExpiredToken: TokenWithPermissions = {
  ...mockToken,
  id: 'expired-token',
  is_expired: true
}

const mockDisabledToken: TokenWithPermissions = {
  ...mockToken,
  id: 'disabled-token',
  enabled: false
}

const mockAdminToken: TokenWithPermissions = {
  ...mockToken,
  id: 'admin-token',
  roles: ['admin'],
  access_level: 'admin'
}

describe('PermissionValidator', () => {
  let validator: PermissionValidator

  beforeEach(() => {
    validator = new PermissionValidator()
    // Mock getTokenInfo 方法
    jest.spyOn(validator as any, 'getTokenInfo').mockImplementation(async (tokenId?: string) => {
      switch (tokenId) {
        case 'test-token-1':
          return mockToken
        case 'expired-token':
          return mockExpiredToken
        case 'disabled-token':
          return mockDisabledToken
        case 'admin-token':
          return mockAdminToken
        default:
          return null
      }
    })
  })

  afterEach(() => {
    jest.restoreAllMocks()
  })

  describe('基本权限检查', () => {
    it('应该允许有效 Token 访问授权的工具', async () => {
      const result = await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.TOOL,
        resource_id: 'tool-1',
        action: PermissionAction.EXECUTE
      })

      expect(result.allowed).toBe(true)
      expect(result.resource_type).toBe(PermissionResourceType.TOOL)
      expect(result.resource_id).toBe('tool-1')
      expect(result.action).toBe(PermissionAction.EXECUTE)
    })

    it('应该拒绝 Token 访问未授权的工具', async () => {
      const result = await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.TOOL,
        resource_id: 'tool-3',
        action: PermissionAction.EXECUTE
      })

      expect(result.allowed).toBe(false)
      expect(result.reason).toContain('not in allowed list')
    })

    it('应该拒绝过期 Token 的访问', async () => {
      const result = await validator.checkPermission({
        token_id: 'expired-token',
        resource_type: PermissionResourceType.TOOL,
        resource_id: 'tool-1',
        action: PermissionAction.EXECUTE
      })

      expect(result.allowed).toBe(false)
      expect(result.reason).toContain('expired')
    })

    it('应该拒绝禁用 Token 的访问', async () => {
      const result = await validator.checkPermission({
        token_id: 'disabled-token',
        resource_type: PermissionResourceType.TOOL,
        resource_id: 'tool-1',
        action: PermissionAction.EXECUTE
      })

      expect(result.allowed).toBe(false)
      expect(result.reason).toContain('disabled')
    })

    it('应该拒绝不存在的 Token', async () => {
      const result = await validator.checkPermission({
        token_id: 'nonexistent-token',
        resource_type: PermissionResourceType.TOOL,
        resource_id: 'tool-1',
        action: PermissionAction.EXECUTE
      })

      expect(result.allowed).toBe(false)
      expect(result.reason).toContain('not found')
    })
  })

  describe('Prompt Template 权限检查', () => {
    it('应该允许访问授权的模板', async () => {
      const result = await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.PROMPT_TEMPLATE,
        resource_id: 'template-1',
        action: PermissionAction.READ
      })

      expect(result.allowed).toBe(true)
    })

    it('应该检查细粒度权限', async () => {
      // 测试读取权限
      const readResult = await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.PROMPT_TEMPLATE,
        resource_id: 'template-3',
        action: PermissionAction.READ
      })
      expect(readResult.allowed).toBe(true)

      // 测试写入权限
      const writeResult = await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.PROMPT_TEMPLATE,
        resource_id: 'template-3',
        action: PermissionAction.WRITE
      })
      expect(writeResult.allowed).toBe(false)
    })

    it('应该允许管理员访问所有资源', async () => {
      const result = await validator.checkPermission({
        token_id: 'admin-token',
        resource_type: PermissionResourceType.PROMPT_TEMPLATE,
        resource_id: 'any-template',
        action: PermissionAction.MANAGE
      })

      expect(result.allowed).toBe(true)
      expect(result.reason).toContain('Admin access granted')
    })
  })

  describe('Prompt Category 权限检查', () => {
    it('应该允许访问授权的分类', async () => {
      const result = await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.PROMPT_CATEGORY,
        resource_id: 'category-1',
        action: PermissionAction.READ
      })

      expect(result.allowed).toBe(true)
    })

    it('应该拒绝访问未授权的分类', async () => {
      const result = await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.PROMPT_CATEGORY,
        resource_id: 'category-2',
        action: PermissionAction.READ
      })

      expect(result.allowed).toBe(false)
    })
  })

  describe('缓存管理', () => {
    it('应该缓存 Token 信息', async () => {
      await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.TOOL,
        resource_id: 'tool-1',
        action: PermissionAction.EXECUTE
      })

      expect(validator['getTokenInfo']).toHaveBeenCalledTimes(1)

      // 第二次调用应该使用缓存
      await validator.checkPermission({
        token_id: 'test-token-1',
        resource_type: PermissionResourceType.TOOL,
        resource_id: 'tool-1',
        action: PermissionAction.EXECUTE
      })

      expect(validator['getTokenInfo']).toHaveBeenCalledTimes(1)
    })

    it('应该能清除缓存', () => {
      validator['tokenCache'].set('test-token', mockToken)
      expect(validator['tokenCache'].has('test-token')).toBe(true)

      validator.clearCache('test-token')
      expect(validator['tokenCache'].has('test-token')).toBe(false)

      // 添加更多缓存数据
      validator['tokenCache'].set('test-token-1', mockToken)
      validator['tokenCache'].set('test-token-2', mockToken)

      validator.clearCache() // 清除所有缓存
      expect(validator['tokenCache'].size).toBe(0)
    })
  })

  describe('权限策略管理', () => {
    it('应该能添加和移除权限策略', () => {
      const customPolicy = {
        id: 'custom-policy',
        name: 'Custom Policy',
        description: 'Test policy',
        priority: 500,
        enabled: true,
        rules: [],
        created_at: Date.now(),
        updated_at: Date.now()
      }

      validator.addPolicy(customPolicy)
      expect(validator.getPolicies()).toContainEqual(customPolicy)

      const removed = validator.removePolicy('custom-policy')
      expect(removed).toBe(true)
      expect(validator.getPolicies()).not.toContainEqual(customPolicy)
    })
  })
})

describe('便捷函数', () => {
  beforeEach(() => {
    // Mock 全局权限验证器
    jest.spyOn(require('../permissionUtils'), 'permissionValidator')
    require('../permissionUtils').permissionValidator.checkPermission = jest.fn()
  })

  afterEach(() => {
    jest.restoreAllMocks()
  })

  describe('checkPermission', () => {
    it('应该调用全局权限验证器', async () => {
      const mockResult = { allowed: true, resource_type: 'tool', resource_id: 'test', action: 'execute' }
      require('../permissionUtils').permissionValidator.checkPermission.mockResolvedValue(mockResult)

      const request = {
        token_id: 'test-token',
        resource_type: PermissionResourceType.TOOL,
        resource_id: 'test',
        action: PermissionAction.EXECUTE
      }

      const result = await checkPermission(request)
      expect(result).toEqual(mockResult)
      expect(require('../permissionUtils').permissionValidator.checkPermission).toHaveBeenCalledWith(request)
    })
  })

  describe('canAccessPromptTemplate', () => {
    it('应该检查模板访问权限', async () => {
      const mockResult = { allowed: true }
      require('../permissionUtils').permissionValidator.checkPermission.mockResolvedValue(mockResult)

      const canAccess = await canAccessPromptTemplate('test-token', 'template-1')
      expect(canAccess).toBe(true)
      expect(require('../permissionUtils').permissionValidator.checkPermission).toHaveBeenCalledWith({
        token_id: 'test-token',
        resource_type: PermissionResourceType.PROMPT_TEMPLATE,
        resource_id: 'template-1',
        action: PermissionAction.READ
      })
    })
  })

  describe('filterAccessiblePromptTemplates', () => {
    it('应该过滤可访问的模板', async () => {
      // Mock 权限检查结果：第一个和第三个模板可访问
      require('../permissionUtils').permissionValidator.checkPermission
        .mockResolvedValueOnce({ allowed: true })  // template-1
        .mockResolvedValueOnce({ allowed: false }) // template-2
        .mockResolvedValueOnce({ allowed: true })  // template-3

      const templateIds = ['template-1', 'template-2', 'template-3']
      const accessible = await filterAccessiblePromptTemplates('test-token', templateIds)

      expect(accessible).toEqual(['template-1', 'template-3'])
    })
  })

  describe('batchCheckPermissions', () => {
    it('应该批量检查权限', async () => {
      const mockResults = [
        { allowed: true, resource_id: 'resource-1' },
        { allowed: false, resource_id: 'resource-2' }
      ]
      require('../permissionUtils').permissionValidator.checkPermission
        .mockResolvedValueOnce(mockResults[0])
        .mockResolvedValueOnce(mockResults[1])

      const requests = [
        {
          token_id: 'test-token',
          resource_type: PermissionResourceType.TOOL,
          resource_id: 'resource-1',
          action: PermissionAction.EXECUTE
        },
        {
          token_id: 'test-token',
          resource_type: PermissionResourceType.TOOL,
          resource_id: 'resource-2',
          action: PermissionAction.EXECUTE
        }
      ]

      const results = await batchCheckPermissions(requests)
      expect(results).toEqual(mockResults)
    })
  })

  describe('generatePermissionSummary', () => {
    it('应该生成权限摘要', () => {
      const permissions: TokenPermissions = {
        allowed_tools: ['tool-1', 'tool-2'],
        allowed_resources: ['resource-1'],
        allowed_prompt_templates: ['template-1', 'template-2', 'template-3'],
        access_level: 'standard'
      }

      const summary = generatePermissionSummary(permissions)
      expect(summary).toBe('Access to 2 tools, 1 resources, 3 prompt templates')
    })

    it('应该处理没有权限的情况', () => {
      const permissions: TokenPermissions = {}

      const summary = generatePermissionSummary(permissions)
      expect(summary).toBe('No permissions')
    })
  })
})

describe('边界情况测试', () => {
  it('应该处理空权限数组', async () => {
    const tokenWithEmptyPermissions: TokenWithPermissions = {
      ...mockToken,
      permissions: {
        allowed_tools: [],
        allowed_resources: [],
        allowed_prompt_templates: []
      }
    }

    const validator = new PermissionValidator()
    jest.spyOn(validator as any, 'getTokenInfo').mockResolvedValue(tokenWithEmptyPermissions)

    const result = await validator.checkPermission({
      token_id: 'empty-permissions-token',
      resource_type: PermissionResourceType.TOOL,
      resource_id: 'tool-1',
      action: PermissionAction.EXECUTE
    })

    expect(result.allowed).toBe(false)
  })

  it('应该处理未定义的权限字段', async () => {
    const tokenWithUndefinedPermissions: TokenWithPermissions = {
      ...mockToken,
      permissions: {}
    }

    const validator = new PermissionValidator()
    jest.spyOn(validator as any, 'getTokenInfo').mockResolvedValue(tokenWithUndefinedPermissions)

    const result = await validator.checkPermission({
      token_id: 'undefined-permissions-token',
      resource_type: PermissionResourceType.TOOL,
      resource_id: 'tool-1',
      action: PermissionAction.EXECUTE
    })

    // 没有权限应该拒绝访问
    expect(result.allowed).toBe(false)
  })

  it('应该处理权限检查错误', async () => {
    const validator = new PermissionValidator()
    jest.spyOn(validator as any, 'getTokenInfo').mockRejectedValue(new Error('Network error'))

    const result = await validator.checkPermission({
      token_id: 'error-token',
      resource_type: PermissionResourceType.TOOL,
      resource_id: 'tool-1',
      action: PermissionAction.EXECUTE
    })

    expect(result.allowed).toBe(false)
    expect(result.reason).toContain('Permission check failed')
  })
})