# 实施任务清单

## 1. 更新全局按钮样式类

- [x] 1.1 在 `src/App.css` 中添加 `.btn-danger-modern` 样式类,支持亮色和暗色主题
- [x] 1.2 在 `src/App.css` 中添加 `.btn-success-modern` 样式类,支持亮色和暗色主题
- [x] 1.3 在 `src/App.css` 中添加 `.btn-secondary-modern` 样式类,支持亮色和暗色主题
- [x] 1.4 验证现有的 `.btn-primary-modern` 样式在暗色主题下的表现

## 2. 更新页面组件的按钮

- [x] 2.1 更新 `src/pages/Settings.tsx` 中的按钮样式
  - [x] 2.1.1 将"添加主机"按钮改为使用 `.btn-primary-modern`
  - [x] 2.1.2 将"删除"按钮改为使用 `.btn-danger-modern`
  - [x] 2.1.3 将"保存设置"按钮改为使用 `.btn-primary-modern`
- [x] 2.2 更新 `src/pages/ApiKeys.tsx` 中的按钮样式
  - [x] 2.2.1 将"创建API Key"按钮改为使用 `.btn-primary-modern`
  - [x] 2.2.2 将"复制"按钮改为使用支持主题的蓝色样式
  - [x] 2.2.3 将"取消"按钮改为使用 `.btn-secondary-modern`
  - [x] 2.2.4 将"创建"按钮改为使用 `.btn-primary-modern`
  - [x] 2.2.5 将"关闭"按钮改为使用 `.btn-primary-modern`

## 3. 更新对话框组件的按钮

- [x] 3.1 更新 `src/components/ConfirmModal.tsx` 中的按钮样式
  - [x] 3.1.1 将危险类型对话框的确认按钮改为使用 `.btn-danger-modern`
  - [x] 3.1.2 将信息类型对话框的确认按钮改为使用 `.btn-primary-modern`
  - [x] 3.1.3 将取消按钮改为使用 `.btn-secondary-modern`
- [x] 3.2 更新 `src/components/InstallConfirmModal.tsx` 中的按钮样式
  - [x] 3.2.1 更新"下一步"按钮使用主题感知的蓝色样式
  - [x] 3.2.2 更新"上一步"和"取消"按钮使用 `.btn-secondary-modern`
  - [x] 3.2.3 更新"安装"按钮使用 `.btn-success-modern` 或 `.btn-primary-modern`

## 4. 更新其他功能组件的按钮

- [x] 4.1 更新 `src/components/CacheManager.tsx` 中的按钮样式
  - [x] 4.1.1 将"清除缓存"按钮改为使用 `.btn-danger-modern` 或主题感知的红色
  - [x] 4.1.2 将"刷新"按钮改为使用 `.btn-primary-modern` 或主题感知的蓝色
- [x] 4.2 检查 `src/components/ToolManager.tsx` 中的切换开关样式
  - [x] 4.2.1 确保切换开关在暗色主题下的绿色和灰色都适配良好
  - [x] 4.2.2 如需要,更新开关的背景色为主题感知的颜色

## 5. 类型检查和验证

- [x] 5.1 运行 `npx tsc --noEmit` 检查前端类型错误
- [x] 5.2 检查 IDE 诊断,确保没有遗留的类型或样式问题
- [x] 5.3 手动测试亮色主题下的所有按钮显示效果
- [x] 5.4 手动测试暗色主题下的所有按钮显示效果
- [x] 5.5 测试按钮的 hover、focus、disabled 状态在两种主题下的表现

## 6. 文档更新

- [x] 6.1 在项目文档中说明新增的按钮样式类及其用途
- [x] 6.2 如有需要,更新组件使用指南

## 验收标准

- 所有按钮在亮色和暗色主题下都有良好的对比度和可读性
- 按钮的 hover、focus、disabled 状态在两种主题下都清晰可见
- 没有遗留使用固定颜色类的按钮(除非有特殊需求)
- 类型检查通过,无编译错误
- 手动测试覆盖所有主要页面和对话框的按钮
