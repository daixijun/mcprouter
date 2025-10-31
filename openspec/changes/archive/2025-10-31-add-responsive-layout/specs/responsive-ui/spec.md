# responsive-ui Specification

## Purpose
增加响应式布局支持，优化桌面应用在各种窗口大小下的用户体验。当前应用使用固定宽度容器，即使在宽屏显示器上也保持狭窄的显示区域，造成空间浪费。通过实现真正的响应式布局，应用应能根据窗口大小自适应，充分利用屏幕空间。

## ADDED Requirements
### Requirement: 主容器响应式布局

主容器 SHALL 移除固定最大宽度限制，采用更灵活的响应式布局策略，以适应不同屏幕尺寸。

#### Scenario: 移除固定宽度限制

- **GIVEN** 用户将应用窗口调整为宽屏显示器尺寸（如 1920x1080 或更大）
- **WHEN** 应用窗口显示内容
- **THEN** 内容 SHALL 占据更多可用空间，而不是局限于固定的 `max-w-7xl` 宽度
- **AND** 在小窗口（如 1024x768）下，内容 SHALL 保持适当的边距和可读性

#### Scenario: 响应式断点策略

- **GIVEN** 应用需要在不同屏幕尺寸下保持良好的用户体验
- **WHEN** 窗口宽度发生变化
- **THEN** 布局 SHALL 基于 Tailwind CSS 的响应式断点进行自适应
- **AND** 断点应包括：sm (640px), md (768px), lg (1024px), xl (1280px), 2xl (1536px)

### Requirement: Dashboard 统计卡片响应式布局

Dashboard 页面的统计卡片 SHALL 使用响应式列配置，在不同屏幕尺寸下动态调整显示列数。

#### Scenario: 大屏幕统计卡片布局（≥1536px）

- **GIVEN** 用户在 2xl 屏幕（1536px+）下访问 Dashboard
- **WHEN** 统计卡片区域显示
- **THEN** 卡片 SHALL 使用 `span={6}` 显示为每行 4 列布局
- **AND** 卡片之间 SHALL 保持适当的间距（8px）

#### Scenario: 标准桌面屏幕布局（1024px-1535px）

- **GIVEN** 用户在 xl 屏幕（1024px-1535px）下访问 Dashboard
- **WHEN** 统计卡片区域显示
- **THEN** 卡片 SHALL 使用 `span={6}` 显示为每行 4 列布局
- **AND** 保持与大屏幕相同的视觉体验

#### Scenario: 笔记本屏幕布局（768px-1023px）

- **GIVEN** 用户在 lg 屏幕（768px-1023px）下访问 Dashboard
- **WHEN** 统计卡片区域显示
- **THEN** 卡片 SHALL 使用 `span={12}` 显示为每行 2 列布局
- **AND** 卡片内容 SHALL 保持可读性

#### Scenario: 平板屏幕布局（640px-767px）

- **GIVEN** 用户在 md 屏幕（640px-767px）下访问 Dashboard
- **WHEN** 统计卡片区域显示
- **THEN** 卡片 SHALL 使用 `span={24}` 显示为单列布局
- **AND** 卡片 SHALL 填满可用宽度

### Requirement: Dashboard 信息卡片响应式布局

Dashboard 页面的系统信息卡片和聚合接口卡片 SHALL 使用响应式布局，在不同屏幕尺寸下动态调整宽度。

#### Scenario: 大屏幕信息卡片布局（≥1024px）

- **GIVEN** 用户在 lg 屏幕（1024px+）下访问 Dashboard
- **WHEN** 系统信息区域显示
- **THEN** 系统信息卡片 SHALL 使用 `span={12}`（占 50% 宽度）
- **AND** 聚合接口卡片 SHALL 使用 `span={12}`（占 50% 宽度）
- **AND** 两张卡片 SHALL 并排显示在同一行

#### Scenario: 中小屏幕信息卡片布局（<1024px）

- **GIVEN** 用户在 md 屏幕（<1024px）下访问 Dashboard
- **WHEN** 系统信息区域显示
- **THEN** 系统信息卡片 SHALL 使用 `span={24}`（占 100% 宽度）
- **AND** 聚合接口卡片 SHALL 使用 `span={24}`（占 100% 宽度）
- **AND** 两张卡片 SHALL 垂直堆叠显示

### Requirement: 其他页面响应式布局优化

McpServerManager、ApiKeys、Settings、Marketplace 等页面 SHALL 使用 Ant Design 的响应式 Grid 系统，确保表格和表单在不同屏幕尺寸下的可读性和可用性。

#### Scenario: 表格页面响应式列配置

- **GIVEN** 用户在宽屏显示器下访问表格页面（如服务管理、API Keys）
- **WHEN** 表格显示
- **THEN** 表格 SHALL 充分利用可用宽度，显示更多列或更宽的列内容
- **AND** 在窄屏设备上，表格 SHALL 保持水平滚动而非压缩内容

#### Scenario: 表单页面响应式布局

- **GIVEN** 用户在不同屏幕尺寸下访问表单页面（如设置页面）
- **WHEN** 表单显示
- **THEN** 表单元素 SHALL 使用响应式的 `labelCol` 和 `wrapperCol` 配置
- **AND** 在窄屏设备上，标签和输入框 SHALL 垂直堆叠以保持可读性

#### Scenario: MCP 广场列表响应式卡片布局

- **GIVEN** 用户在宽屏显示器下访问 MCP 广场页面
- **WHEN** 服务卡片列表显示
- **THEN** 卡片 SHALL 使用响应式列配置：xl 及以上显示 4 列，lg 显示 3 列，md 显示 2 列，sm 及以下显示 1 列
- **AND** 卡片之间的间距 SHALL 保持一致（16px）
- **AND** 每个卡片 SHALL 填满其容器的全部宽度，保持视觉平衡

### Requirement: 响应式容器组件优化

Layout.tsx 中的 Container 组件 SHALL 增加响应式尺寸选项，以支持更灵活的布局控制。

#### Scenario: 响应式 Container 尺寸选项

- **GIVEN** 需要根据屏幕尺寸动态调整容器最大宽度
- **WHEN** 使用 Container 组件
- **THEN** 组件 SHALL 支持响应式尺寸配置（`responsive: { sm: 'md', lg: 'xl' }` 等）
- **AND** 在不同断点下自动应用对应的最大宽度

### Requirement: 客户端配置代码区域自适应

Dashboard 页面的客户端配置代码显示区域 SHALL 根据可用空间动态调整宽度。

#### Scenario: 配置代码区域响应式显示

- **GIVEN** 用户在宽屏显示器下查看客户端配置
- **WHEN** 配置代码区域显示
- **THEN** 代码块 SHALL 占据更多水平空间，显示更多列内容
- **AND** 在窄屏设备上，代码块 SHALL 保持可读性，通过垂直滚动显示完整内容

