# 更新按钮配色以适配亮色和暗色主题

## Why

目前应用中的按钮配色方案存在以下问题:

1. 某些按钮使用固定的颜色值(如 `bg-blue-500`、`bg-red-500`、`bg-gray-200` 等),在暗色主题下缺乏对应的深色变体
2. 按钮的颜色对比度在不同主题下不够一致,影响可读性和用户体验
3. 全局按钮样式类(`.btn-primary-modern`)未完全覆盖所有按钮使用场景,导致样式不统一

用户在暗色模式下使用应用时,部分按钮的配色与背景对比度不佳,或者颜色过于刺眼,影响视觉体验。

## What Changes

更新所有按钮组件的配色方案,确保在亮色和暗色主题下都有良好的视觉表现:

1. **统一主要操作按钮配色**
   - 主操作按钮(Primary):使用蓝色到靛蓝色的渐变,添加暗色模式适配
   - 次要操作按钮(Secondary):使用中性灰色,根据主题切换明暗
   - 危险操作按钮(Danger):使用红色系,添加暗色模式支持
   - 成功操作按钮(Success):使用绿色系,添加暗色模式支持

2. **更新现有按钮颜色**
   - 将所有使用固定颜色的按钮改为使用支持主题切换的颜色类
   - 确保按钮的 hover、focus、disabled 状态在两种主题下都清晰可见
   - 统一按钮的文本颜色,确保足够的对比度

3. **扩展全局按钮样式类**
   - 在 `App.css` 中添加更多按钮变体样式类
   - 提供 `.btn-danger-modern`、`.btn-success-modern`、`.btn-secondary-modern` 等样式类
   - 所有样式类都支持亮色和暗色主题自动切换

4. **更新受影响的组件**
   - Settings.tsx - 设置页面的保存按钮、删除按钮
   - ApiKeys.tsx - API Key 管理页面的操作按钮
   - ConfirmModal.tsx - 确认对话框的按钮
   - InstallConfirmModal.tsx - 安装确认对话框的按钮
   - CacheManager.tsx - 缓存管理的清除按钮
   - 其他使用按钮的组件

## Impact

- **受影响的规范**: `ui-theme` (新增能力)
- **受影响的代码**:
  - `src/App.css` - 全局样式定义,添加新的按钮样式类
  - `src/pages/Settings.tsx` - 更新保存按钮和其他操作按钮
  - `src/pages/ApiKeys.tsx` - 更新 API Key 管理页面的按钮
  - `src/components/ConfirmModal.tsx` - 更新确认对话框按钮
  - `src/components/InstallConfirmModal.tsx` - 更新安装对话框按钮
  - `src/components/CacheManager.tsx` - 更新缓存管理按钮
  - `src/components/ToolManager.tsx` - 更新工具管理按钮(切换开关)
  - 其他包含按钮的组件

- **用户影响**:
  - 提升暗色主题下的视觉体验
  - 保持亮色主题下的一致性
  - 改善按钮的可访问性和对比度
  - 无破坏性变更,所有现有功能保持不变
