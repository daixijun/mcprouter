# UI 主题规范增量

## ADDED Requirements

### Requirement: 主题感知的按钮配色系统

应用程序的所有按钮 MUST 支持亮色和暗色主题的自动切换,确保在两种主题下都具有良好的可读性和视觉对比度。

#### Scenario: 主操作按钮在亮色主题下显示

- **WHEN** 用户在亮色主题下查看主操作按钮(如"保存设置"、"创建")
- **THEN** 按钮 MUST 显示蓝色到靛蓝色的渐变背景(`bg-gradient-to-r from-blue-600 to-indigo-600`)
- **AND** 文本颜色 MUST 为白色(`text-white`)
- **AND** hover 状态 MUST 显示更深的渐变(`hover:from-blue-700 hover:to-indigo-700`)

#### Scenario: 主操作按钮在暗色主题下显示

- **WHEN** 用户在暗色主题下查看主操作按钮
- **THEN** 按钮 MUST 保持相同的蓝色到靛蓝色渐变,但亮度适合暗色背景
- **AND** 文本颜色 MUST 保持白色以确保对比度
- **AND** 按钮与暗色背景的对比度 MUST 足够清晰

#### Scenario: 危险操作按钮在两种主题下显示

- **WHEN** 用户查看危险操作按钮(如"删除"、"清除")
- **THEN** 在亮色主题下按钮 MUST 显示红色背景(`bg-red-500`)和白色文本
- **AND** 在暗色主题下按钮 MUST 显示适配的红色背景(`dark:bg-red-600`)和白色文本
- **AND** hover 状态 MUST 显示更深的红色(`hover:bg-red-600 dark:hover:bg-red-700`)

#### Scenario: 次要操作按钮在两种主题下显示

- **WHEN** 用户查看次要操作按钮(如"取消"、"关闭")
- **THEN** 在亮色主题下按钮 MUST 显示浅灰色背景(`bg-gray-200`)和深色文本(`text-gray-700`)
- **AND** 在暗色主题下按钮 MUST 显示深灰色背景(`dark:bg-gray-700`)和浅色文本(`dark:text-gray-200`)
- **AND** 两种主题下的对比度 MUST 都足够清晰

### Requirement: 全局按钮样式类

应用程序 MUST 在全局样式表中提供一套完整的按钮样式类,覆盖所有常见的按钮使用场景,并支持主题自动切换。

#### Scenario: 使用主操作按钮样式类

- **WHEN** 开发者在组件中使用 `.btn-modern .btn-primary-modern` 样式类
- **THEN** 按钮 MUST 自动应用主题感知的蓝色渐变样式
- **AND** 在亮色和暗色主题下都 MUST 正确显示

#### Scenario: 使用危险操作按钮样式类

- **WHEN** 开发者在组件中使用 `.btn-modern .btn-danger-modern` 样式类
- **THEN** 按钮 MUST 自动应用主题感知的红色样式
- **AND** 样式类 MUST 包含基础样式、hover 状态和主题变体

#### Scenario: 使用次要操作按钮样式类

- **WHEN** 开发者在组件中使用 `.btn-modern .btn-secondary-modern` 样式类
- **THEN** 按钮 MUST 自动应用主题感知的灰色样式
- **AND** 在两种主题下 MUST 保持良好的可读性

#### Scenario: 使用成功操作按钮样式类

- **WHEN** 开发者在组件中使用 `.btn-modern .btn-success-modern` 样式类
- **THEN** 按钮 MUST 自动应用主题感知的绿色样式
- **AND** 样式 MUST 适合用于确认、提交等成功类操作

### Requirement: 按钮状态的主题适配

所有按钮的不同状态(normal、hover、focus、disabled)MUST 在亮色和暗色主题下都有清晰的视觉反馈。

#### Scenario: 按钮 hover 状态在两种主题下显示

- **WHEN** 用户将鼠标悬停在按钮上
- **THEN** 按钮 MUST 显示明显的 hover 效果
- **AND** 在亮色主题下 MUST 显示更深或更亮的颜色
- **AND** 在暗色主题下 MUST 显示适配的 hover 颜色
- **AND** hover 效果 MUST 包含轻微的向上移动动画(`-translate-y-0.5`)

#### Scenario: 禁用按钮在两种主题下显示

- **WHEN** 按钮处于禁用状态(`disabled`)
- **THEN** 在亮色主题下按钮 MUST 显示降低透明度的样式(如 `opacity-50`)
- **AND** 在暗色主题下按钮 MUST 同样降低透明度
- **AND** 禁用按钮 MUST 显示不可点击的鼠标指针(`cursor-not-allowed`)

#### Scenario: 按钮 focus 状态的可访问性

- **WHEN** 按钮获得键盘焦点
- **THEN** 按钮 MUST 显示清晰的焦点环(`focus:ring-2`)
- **AND** 焦点环颜色 MUST 在两种主题下都清晰可见
- **AND** 焦点环 MUST 符合无障碍访问标准

### Requirement: 组件中按钮颜色的一致性

应用程序中的所有组件 MUST 使用统一的按钮配色方案,避免使用临时的内联颜色类。

#### Scenario: 设置页面的按钮配色

- **WHEN** 用户查看设置页面(Settings.tsx)
- **THEN** 保存按钮 MUST 使用 `.btn-primary-modern` 样式类
- **AND** 删除主机按钮 MUST 使用 `.btn-danger-modern` 样式类
- **AND** 所有按钮 MUST 在主题切换时正确更新颜色

#### Scenario: API Keys 页面的按钮配色

- **WHEN** 用户查看 API Keys 管理页面
- **THEN** 创建 API Key 按钮 MUST 使用 `.btn-primary-modern` 样式类
- **AND** 复制按钮 MUST 使用适当的主题感知颜色
- **AND** 取消按钮 MUST 使用 `.btn-secondary-modern` 样式类

#### Scenario: 确认对话框的按钮配色

- **WHEN** 用户查看确认对话框(ConfirmModal)
- **THEN** 危险类型对话框的确认按钮 MUST 使用 `.btn-danger-modern` 样式类
- **AND** 信息类型对话框的确认按钮 MUST 使用 `.btn-primary-modern` 样式类
- **AND** 取消按钮 MUST 使用 `.btn-secondary-modern` 样式类
