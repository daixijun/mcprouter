// 工具管理命令 - 迁移到配置文件

use crate::error::{McpError, Result};

#[tauri::command(rename_all = "snake_case")]
pub async fn toggle_mcp_server_tool(
    name: String,
    tool_name: String,
    enabled: bool,
) -> Result<String> {
    // TODO: 迁移到配置文件后重新实现
    tracing::warn!(
        "toggle_mcp_server_tool not fully implemented yet for {}/{} (enabled: {})",
        name,
        tool_name,
        enabled
    );

    Err(McpError::ProcessError(
        "Tool management not fully migrated to config-based storage yet".to_string(),
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn enable_all_mcp_server_tools(name: String) -> Result<String> {
    // TODO: 迁移到配置文件后重新实现
    tracing::warn!("enable_all_mcp_server_tools not fully implemented yet for {}", name);

    Err(McpError::ProcessError(
        "Tool management not fully migrated to config-based storage yet".to_string(),
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn disable_all_mcp_server_tools(name: String) -> Result<String> {
    // TODO: 迁移到配置文件后重新实现
    tracing::warn!("disable_all_mcp_server_tools not fully implemented yet for {}", name);

    Err(McpError::ProcessError(
        "Tool management not fully migrated to config-based storage yet".to_string(),
    ))
}
