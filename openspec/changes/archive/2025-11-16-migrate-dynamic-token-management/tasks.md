# Tasks: migrate-dynamic-token-management

## Implementation Tasks

### 1. Backend Core Implementation

- [x] **Create TokenManager struct and core logic**

  - 实现 TokenManager 数据结构 (`src-tauri/src/token_manager.rs`)
  - 添加 Arc<RwLock<HashMap<String, Token>>> 存储结构
  - 实现文件存储路径管理 (~/.mcprouter/tokens.json)
  - 添加 Token 数据结构和序列化支持

- [x] **Implement secure token generation**

  - 生成 cryptographically secure 随机数
  - 实现 "tok*" + "mcp*" 前缀格式
  - 验证 token 强度和唯一性

- [x] **Add persistent storage operations**

  - 实现 save() 和 load() 异步方法
  - 添加文件权限控制 (chmod 600)
  - 处理文件损坏和错误恢复逻辑

- [x] **Implement core TokenManager methods**
  - create() - 创建新 token (名称、描述、过期时间)
  - list() - 列出所有 token (不含实际值)
  - delete() - 删除指定 token
  - validate_token() - 验证 token 并返回 token_id
  - record_usage() - 记录使用统计
  - cleanup_expired() - 清理过期 token

### 2. Tauri Command Integration

- [x] **Create Tauri command handlers**

  - #[tauri::command] create_token(name, description, expires_in)
  - #[tauri::command] list_tokens() -> Vec<Token>
  - #[tauri::command] delete_token(id)
  - 添加适当的错误处理和 Result 返回类型

- [x] **Register commands in main.rs**

  - 在 Tauri 应用初始化时注册所有 token 相关命令
  - 确保 TokenManager 作为全局状态传递

- [x] **Add TokenManager to app state**
  - 在应用启动时初始化 TokenManager
  - 添加到 Tauri 的 AppState 中
  - 实现优雅关闭时保存数据

### 3. Authentication Middleware Migration

- [x] **Modify aggregator.rs authentication logic**

  - 移除静态 token 验证 (config.bearer_token)
  - 集成 TokenManager 进行动态验证
  - 更新 bearer_auth_middleware() 函数

- [x] **Update McpAggregator to use TokenManager**

  - 修改构造函数接受 TokenManager 参数
  - 更新路由器构建逻辑以使用动态认证
  - 移除 auth 配置字段依赖

- [x] **Add enhanced logging and monitoring**
  - 详细记录认证成功/失败事件
  - 添加客户端 IP 和时间戳日志
  - 实现 token 使用统计的异步记录

### 4. Configuration Migration

- [x] **Update configuration validation**

  - 恢复 ServerConfig 中的 auth 字段为 bool 类型
  - 添加 is_auth_enabled() 方法检查认证状态
  - 根据配置动态启用/禁用认证中间件
  - 设置默认值为 false 以保持向后兼容

- [x] **Remove migration functionality**

  - 删除 migration.rs 文件和相关逻辑
  - 移除所有迁移相关的 Tauri 命令
  - 清理前端迁移相关代码和模态框

### 5. Frontend Token Management Interface

- [x] **Create Token Management page**

  - 新建 `src/pages/TokenManagement.tsx`
  - 实现 token 列表组件 (TokenList, TokenRow)
  - 添加创建和删除按钮

- [x] **Implement Create Token Modal**

  - 创建表单组件 (CreateTokenModal)
  - 添加输入验证 (名称长度、描述限制)
  - 实现过期时间选择器 (1 小时、1 天、30 天、永不过期)

- [x] **Add Delete Token Confirmation**

  - 创建确认对话框 (DeleteTokenModal)
  - 显示使用统计警告
  - 实现二次确认机制

- [x] **Integrate with Tauri commands**
  - 使用 @tauri-apps/api 调用后端命令
  - 添加 loading states 和错误处理
  - 实现乐观更新和错误回滚

### 6. Navigation and UI Integration

- [x] **Add Token Management to navigation**

  - 更新主菜单或侧边栏添加"Token 管理"入口
  - 添加适当的图标和路由
  - 确保响应式设计兼容

- [x] **Update Dashboard with token status**

  - 在主仪表板显示 token 管理状态
  - 添加快速创建 token 的引导
  - 显示认证状态和使用统计摘要

- [x] **Implement responsive design**
  - 确保移动端友好的布局
  - 优化表格/卡片在不同屏幕尺寸的显示
  - 添加适当的触摸交互支持

### 7. User Experience Enhancements

- [x] **Add copy-to-clipboard functionality**

  - 新创建 token 显示复制按钮
  - 实现"已复制"提示消息
  - 支持 HTTPS 和 HTTP 环境

- [x] **Implement token strength indicators**

  - 显示 token 创建时间和过期时间
  - 添加使用统计可视化 (使用次数、最后使用)
  - 提供安全状态指示器

- [x] **Add help and documentation**
  - 创建使用指南和最佳实践
  - 添加工具提示和帮助文本
  - 提供 API 使用示例

### 8. Error Handling and Validation

- [x] **Implement comprehensive error handling**

  - 添加网络错误处理和重试逻辑
  - 实现表单验证和用户友好的错误消息
  - 添加错误恢复机制

- [x] **Add data validation**

  - 前端表单验证 (名称长度、描述长度)
  - 后端数据验证和清理
  - 防止 XSS 和注入攻击

- [x] **Handle edge cases**
  - 空状态处理 (无 token 时的引导)
  - 网络断线时的离线提示
  - 大量 token 时的性能优化

### 9. Testing and Quality Assurance

- [x] **Write unit tests for TokenManager**

  - 测试 token 创建、验证、删除功能
  - 测试文件存储加载和保存
  - 测试并发操作和线程安全

- [x] **Add integration tests**

  - 测试 Tauri 命令的正确性
  - 测试认证中间件的端到端功能
  - 测试配置的各种场景

- [x] **Test frontend components**
  - 组件渲染和交互测试
  - 表单验证和错误处理测试
  - 响应式设计测试

### 10. Documentation and Deployment

- [x] **Update README and documentation**

  - 更新 API 使用文档
  - 添加 Token 管理使用指南
  - 更新配置文件示例

- [x] **Create configuration guide**

  - 提供详细的配置步骤说明
  - 展示 server.auth 字段的使用方法
  - 添加配置示例和最佳实践

- [x] **Add security considerations**
  - 记录安全最佳实践
  - 提供 token 安全管理建议
  - 添加常见安全问题解答

### 11. Performance Optimization

- [x] **Optimize file I/O operations**

  - 实现异步文件操作
  - 添加批量更新支持
  - 优化频繁写入的性能影响

- [x] **Implement caching strategies**
  - 添加内存缓存优化 token 查找
  - 实现智能缓存失效机制
  - 优化大量 token 的性能

### 12. Security Hardening

- [x] **Add security hardening**

  - 实现 constant-time token 比较
  - 添加速率限制防止暴力破解
  - 加强文件权限检查

- [x] **Audit logging enhancement**
  - 详细记录所有认证尝试
  - 添加安全事件告警
  - 实现日志轮转和管理

## Dependencies and Prerequisites

- **Required**: 现有的 aggregator-auth 能力已实现
- **Required**: 基础的 Tauri + React 项目结构
- **Optional**: rand 和 base64 crates (如果还没有)
- **Optional**: chrono crate (用于时间戳处理)

## Validation Criteria

- [x] 所有新功能通过编译和运行时验证
- [x] 手动测试验证用户体验
- [x] 性能测试满足并发要求
- [x] 安全测试通过基本验证
- [x] 应用启动和功能验证完成
- [x] 前后端编译无错误

## Risk Mitigation

- [x] **数据丢失风险**: 实现文件备份和恢复机制
- [x] **配置破坏风险**: 提供详细的配置路径和选项
- [x] **性能风险**: 添加性能监控和优化
- [x] **安全风险**: 进行安全审查和测试
