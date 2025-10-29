# spec.md

## ADDED Requirements

### Requirement: Service Management Interface Layout

服务管理界面 SHALL 使用表格布局替代卡片列表来展示 MCP 服务器信息，以提升大量服务器时的浏览效率。

#### Scenario: Compact Table Display

- **WHEN** 用户访问服务管理页面
- **THEN** 界面 SHALL 使用 Ant Design Table 组件展示服务器列表
- **AND** 每行显示一个服务器的关键信息（名称、状态、协议、操作等）
- **AND** 表格 SHALL 支持排序和基本的交互功能

#### Scenario: Removed Header Section

- **WHEN** 用户访问服务管理页面
- **THEN** 页面 SHALL 不显示标题 "MCP 服务管理" 和描述文本
- **AND** 垂直空间 SHALL 被重新分配给表格和筛选区域

### Requirement: Status Filter Interface

状态筛选 SHALL 使用按钮组替代下拉选择器，提供更直观的筛选交互。

#### Scenario: Button-based Status Filtering

- **WHEN** 用户需要筛选服务器状态
- **THEN** 界面 SHALL 显示一组紧凑排列的按钮
- **AND** 每个按钮 SHALL 代表一个状态（全部、已连接、连接中、已断开、连接出错）
- **AND** 按钮 SHALL 使用适当的图标和颜色来表示对应状态

#### Scenario: Status Button Icons and Colors

- **WHEN** 状态筛选按钮显示时
- **THEN** "全部" 按钮 SHALL 使用 `List` 或 `Apps` 图标
- **AND** "已连接" 按钮 SHALL 使用 `CheckCircle` 图标和绿色主题
- **AND** "连接中" 按钮 SHALL 使用 `RotateCw` 图标和蓝色主题
- **AND** "已断开" 按钮 SHALL 使用 `XCircle` 图标和灰色主题
- **AND** "连接出错" 按钮 SHALL 使用 `AlertCircle` 图标和红色主题

### Requirement: Responsive Service Actions

服务操作功能 SHALL 在表格环境中保持完整和可访问性。

#### Scenario: Table Row Actions

- **WHEN** 用户查看服务器列表表格
- **THEN** 每行 SHALL 包含操作按钮组（启用/禁用、工具管理、编辑、删除）
- **AND** 所有现有功能 SHALL 保持正常工作
- **AND** 操作按钮 SHALL 有适当的间距和视觉反馈

#### Scenario: Search and Filter Integration

- **WHEN** 用户使用搜索或筛选功能
- **THEN** 表格 SHALL 实时更新显示结果
- **AND** 搜索框 SHALL 保持可访问性
- **AND** 状态筛选按钮 SHALL 清晰显示当前选中的筛选条件
