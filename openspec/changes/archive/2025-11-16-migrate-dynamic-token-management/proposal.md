# Proposal: migrate-dynamic-token-management

## Why

将聚合接口的 Bearer Token 认证从固定在配置文件的静态模式迁移到支持用户动态创建和管理的模式，以解决当前静态配置的灵活性限制、多用户支持不足、缺乏审计能力等关键问题。

## What Changes

- 实现完整的动态 Token 管理系统，支持创建、查看、删除 token
- 修改聚合器认证中间件，从静态配置验证改为动态 Token 存储验证
- 添加 Token 元数据支持（名称、描述、过期时间、使用统计）
- 提供完整的前端管理界面和 Tauri 命令接口
- 移除静态 `server.bearer_token` 配置，改为通过 `server.auth` 控制认证开关

## Overview

将聚合接口的 Bearer Token 认证从固定在配置文件的静态模式迁移到支持用户动态创建和管理的模式。

## Motivation

### 当前问题

1. **静态 Token 配置限制**:当前系统仅支持在 `config.json` 中配置单个静态 bearer_token,缺乏灵活性
2. **无法多用户/多应用场景**:无法为不同的客户端或应用分配独立的访问凭证
3. **Token 轮换困难**:需要修改配置文件并重启服务才能更新 token
4. **缺乏审计能力**:无法追踪哪个 token 被使用、何时使用、使用频率等信息
5. **无过期机制**:静态 token 永不过期,存在安全隐患

### 期望目标

1. **动态 Token 管理**:用户可以通过 UI 或 API 动态创建、查看、删除 token
2. **Token 元数据**:支持为每个 token 设置名称、描述、过期时间
3. **使用统计**:记录每个 token 的使用次数、最后使用时间等信息
4. **完整的生命周期管理**:支持创建、列表、删除、自动过期等功能
5. **提升安全性**:通过 token 轮换、过期时间、使用审计等机制提升系统安全性

## Scope

### 新增能力

- **token-management**: 完整的 Bearer Token 动态管理系统,包括:
  - 持久化存储 (独立的 `tokens.json` 文件)
  - CRUD 操作 (创建、读取、删除 token)
  - Token 元数据 (名称、描述、创建时间、过期时间、最后使用时间、使用次数)
  - 前端管理界面 (Token 列表、创建对话框、删除确认)
  - Tauri 命令接口 (与前端交互)

### 修改的现有能力

- **aggregator-auth**: 修改认证中间件以支持动态 token 验证
  - 移除静态 `server.bearer_token` 配置
  - 移除 `server.auth` 开关 (始终启用认证)
  - 集成动态 token 存储和验证

## Breaking Changes

⚠️ **重要:此变更包含破坏性修改**

1. **配置文件结构变更**:
   - 移除 `server.auth` 字段 (认证将始终启用)
   - 移除 `server.bearer_token` 字段 (改为动态管理)
   - 旧的配置文件需要迁移:用户需要删除这两个字段,并通过新的 Token 管理界面创建 token

2. **认证行为变更**:
   - 认证将始终启用 (不再有无认证模式)
   - 如果没有创建任何 token,所有请求将被拒绝
   - 用户必须通过 UI 创建至少一个 token 才能使用聚合接口

3. **迁移路径** (可选):
   - 应用启动时检测旧配置
   - 如果发现 `server.bearer_token`:
     - **选项 A (推荐)**: 显示迁移引导，用户可选择手动迁移或忽略
     - **选项 B**: 自动迁移为新动态 token (名称:"Migrated from config")
     - **选项 C**: 仅记录警告，要求用户手动重新创建 token
   - 记录详细的迁移日志和操作指引

## Dependencies

- 无外部依赖
- 依赖现有的 `aggregator-auth` 能力进行修改

## Related Work

- 基于已归档的 `add-bearer-auth` 变更进行重构
- 参考规格:`openspec/specs/aggregator-auth/spec.md`

## Success Criteria

1. 用户可以通过前端界面创建、查看、删除 token
2. 每个 token 包含名称、描述、过期时间等元数据
3. 系统记录每个 token 的使用统计 (使用次数、最后使用时间)
4. 过期的 token 自动失效
5. 为现有 `bearer_token` 配置提供清晰的迁移指引
6. 所有聚合接口请求都需要有效的 token 认证
7. 通过 `openspec validate --strict` 验证

## Open Questions

1. ~~Token 存储位置~~:✅ 已确认使用独立的 `~/.mcprouter/tokens.json` 文件
2. ~~是否保留向后兼容~~:✅ 已确认完全移除静态配置,提供迁移指引
3. ~~前端界面设计~~:✅ 已确认需要完整的前端管理界面
4. Token 生成算法:使用加密安全的随机生成器 (例如 `rand::thread_rng()` + base64 编码,长度 32 字符)
5. ~~迁移策略细节~~:✅ 已确认提供灵活的迁移选项,不强制自动迁移

## Timeline

- Proposal & Design: 1 day
- Implementation: 2-3 days
- Testing & Validation: 1 day
- Documentation: 0.5 day
