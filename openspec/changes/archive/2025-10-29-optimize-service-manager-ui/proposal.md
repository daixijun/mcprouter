# proposal.md

## Why

当前服务管理页面使用卡片列表展示，在管理大量服务器时存在浏览效率问题。Header 区域占用垂直空间，状态筛选使用 Select 组件不够直观。通过优化界面布局，提升用户体验和操作效率。

## What Changes

- 移除页面的 Header 区域（标题 "MCP 服务管理" 和描述文本）
- 将当前的卡片列表展示改为 Ant Design Table 组件展示
- 状态筛选从 Select 下拉菜单改为 Space + Button 组合，使用紧凑布局
- 为状态筛选按钮添加合适的图标（已连接、连接中、断开、错误等）
- 保留搜索功能和其他操作（添加服务、刷新等）

## Impact

- Affected specs: `ui/service-management`
- Affected code: `src/pages/McpServerManager.tsx`
- 改进用户体验，特别是在管理大量 MCP 服务器时
- 提高状态筛选的直观性和操作效率
- 减少垂直空间占用，为表格展示提供更多空间
