# MCP Router 项目规则文档

## 1. 项目概述

MCP Router 是一个基于 Tauri + React + TypeScript 开发的模型上下文协议聚合管理工具，用于管理和聚合多个 MCP 服务。

### 1.1 技术栈

- **前端**: React 19 + TypeScript + Tailwind CSS
- **后端**: Rust + Tauri 2.x
- **构建工具**: Vite
- **包管理**: pnpm

### 1.2 项目结构

```shell
mcprouter/
├── src/                    # React 前端代码
│   ├── components/         # React 组件
│   ├── services/          # API 服务
│   └── types/             # TypeScript 类型定义
├── src-tauri/             # Rust 后端代码
│   └── src/               # Rust 源代码
└── .trae/                 # 项目配置和规则
```

## 2. 开发规范

### 2.1 代码风格

#### TypeScript/React 代码规范

1. **命名规范**:

   - 组件使用 PascalCase: `McpServiceManager`
   - 函数和变量使用 camelCase: `handleServiceChange`
   - 常量使用 UPPER_SNAKE_CASE: `MAX_CONNECTIONS`
   - 类型和接口使用 PascalCase: `McpServerConfig`

2. **文件组织**:

   - 每个组件一个文件，文件名与组件名相同
   - 相关的类型定义放在 `types/index.ts` 中
   - API 调用放在 `services/` 目录下

3. **代码格式**:

   - 使用 2 空格缩进
   - 使用单引号
   - 每行最大长度 120 字符
   - 在语句末尾不使用分号

4. **React 组件规范**:
   - 使用函数组件和 Hooks
   - 组件 Props 必须定义类型接口
   - 使用 TypeScript 严格模式
   - 避免使用 `any` 类型

#### Rust 代码规范

1. **命名规范**:

   - 函数和变量使用 snake_case: `handle_service_change`
   - 类型、结构体和枚举使用 PascalCase: `McpServiceManager`
   - 常量使用 SCREAMING_SNAKE_CASE: `MAX_CONNECTIONS`
   - 模块使用 snake_case: `mcp_manager`

2. **代码组织**:

   - 每个模块一个文件
   - 使用 `mod` 声明模块
   - 公共 API 需要添加文档注释

3. **错误处理**:
   - 使用 `Result<T, Error>` 处理可能失败的操作
   - 使用 `?` 操作符进行错误传播
   - 自定义错误类型实现 `Error` trait

### 2.2 Git 提交规范

1. **提交消息格式**:

   ```shell
   <类型>(<范围>): <描述>

   [可选的正文]

   [可选的脚注]
   ```

2. **类型**:

   - `feat`: 新功能
   - `fix`: 修复 bug
   - `docs`: 文档更新
   - `style`: 代码格式化（不影响功能）
   - `refactor`: 代码重构
   - `test`: 添加或修改测试
   - `chore`: 构建过程或辅助工具的变动

3. **示例**:

   ```shell
   feat(ui): 添加服务管理界面

   - 添加服务列表组件
   - 实现服务启动/停止功能
   - 添加服务状态显示
   ```

### 2.3 分支管理

1. **主分支**:

   - `main`: 主分支，始终保持可发布状态
   - `develop`: 开发分支，集成最新功能

2. **功能分支**:

   - 格式: `feat/功能名称`
   - 从 `develop` 分支创建
   - 完成后合并回 `develop`

3. **修复分支**:
   - 格式: `fix/问题描述`
   - 从 `main` 分支创建
   - 完成后合并回 `main` 和 `develop`

## 3. 代码审查

### 3.1 审查流程

1. 所有代码变更必须通过 Pull Request 进行审查
2. 至少需要一名团队成员批准
3. 必须通过所有自动化检查
4. 解决所有审查意见后才能合并

### 3.2 审查要点

1. **代码质量**:

   - 代码是否符合项目规范
   - 是否有明显的 bug 或逻辑错误
   - 是否有性能问题

2. **设计合理性**:

   - 架构设计是否合理
   - 是否遵循 SOLID 原则
   - 是否考虑了扩展性

3. **测试覆盖**:
   - 是否有足够的测试
   - 测试用例是否覆盖边界情况

## 4. 测试规范

### 4.1 测试类型

1. **单元测试**:

   - 测试单个函数或组件
   - 使用 Jest 进行 React 组件测试
   - 使用 Rust 内置测试框架进行 Rust 代码测试

2. **集成测试**:

   - 测试多个组件或模块的交互
   - 测试前后端 API 交互

3. **端到端测试**:
   - 测试完整的用户流程
   - 使用 Tauri 测试工具

### 4.2 测试要求

1. 新功能必须包含测试
2. 测试覆盖率不低于 80%
3. 关键功能必须有完整的测试覆盖

## 5. 发布流程

### 5.1 版本号规范

使用语义化版本号 (Semantic Versioning):

- `MAJOR.MINOR.PATCH`
- `MAJOR`: 不兼容的 API 变更
- `MINOR`: 向后兼容的功能新增
- `PATCH`: 向后兼容的问题修正

### 5.2 发布步骤

1. 更新版本号
2. 更新 CHANGELOG.md
3. 创建 Git 标签
4. 构建发布包
5. 发布到各平台

## 6. 安全规范

### 6.1 代码安全

1. 不在代码中硬编码敏感信息
2. 使用环境变量存储配置
3. 对用户输入进行验证和清理
4. 使用 HTTPS 进行网络通信

### 6.2 依赖安全

1. 定期更新依赖包
2. 使用安全扫描工具检查漏洞
3. 及时修复已知安全漏洞

## 7. 文档规范

### 7.1 代码文档

1. 所有公共 API 必须有文档注释
2. 复杂逻辑必须有注释说明
3. 使用 JSDoc 格式注释 TypeScript 代码
4. 使用 Rustdoc 格式注释 Rust 代码

### 7.2 项目文档

1. README.md: 项目介绍和快速开始
2. API.md: API 文档
3. CONTRIBUTING.md: 贡献指南
4. CHANGELOG.md: 版本更新记录

## 8. 性能规范

### 8.1 前端性能

1. 使用 React.memo 优化组件渲染
2. 合理使用 useCallback 和 useMemo
3. 避免不必要的重新渲染
4. 优化资源加载

### 8.2 后端性能

1. 使用异步处理提高并发性能
2. 合理使用缓存
3. 优化数据库查询
4. 监控性能指标

## 9. 工具和配置

### 9.1 开发环境

1. **IDE**: 推荐使用 VS Code
2. **扩展**:
   - Rust Analyzer (Rust)
   - Tauri
   - TypeScript and JavaScript Language Features
   - ES7+ React/Redux/React-Native snippets

### 9.2 代码格式化

1. **前端**: 使用 Prettier
2. **后端**: 使用 rustfmt
3. **配置**: 统一的 `.prettierrc` 和 `rustfmt.toml`

### 9.3 代码检查

1. **前端**: ESLint
2. **后端**: Clippy
3. **配置**: 统一的 `.eslintrc` 和 `clippy.toml`

## 10. 协作规范

### 10.1 沟通渠道

1. 日常讨论: 使用团队聊天工具
2. 重要决策: 使用 Issue 进行讨论
3. 代码审查: 使用 Pull Request

### 10.2 会议规范

1. 定期举行代码审查会议
2. 重要设计决策需要会议讨论
3. 会议记录需要共享

## 11. 项目维护

### 11.1 依赖管理

1. 定期检查和更新依赖
2. 使用 pnpm audit 检查安全漏洞
3. 记录重要的依赖变更

### 11.2 技术债务

1. 记录技术债务
2. 定期安排时间处理技术债务
3. 优先处理影响开发效率的技术债务

## 12. 故障处理

### 12.1 Bug 报告

1. 使用 Issue 模板报告 Bug
2. 提供详细的复现步骤
3. 包含环境信息和日志

### 12.2 紧急修复

1. 创建 hotfix 分支
2. 快速修复并测试
3. 合并到主分支并发布

---

## 附录

### A. 有用的链接

- [React 文档](https://react.dev/)
- [Tauri 文档](https://tauri.app/)
- [Rust 文档](https://doc.rust-lang.org/)
- [TypeScript 文档](https://www.typescriptlang.org/)

### B. 模板文件

- PR 模板
- Issue 模板
- 代码审查清单

### C. 常见问题

- 开发环境设置
- 构建和部署
- 调试技巧

---

_此文档会随着项目发展持续更新，所有团队成员都有责任维护和完善。_
