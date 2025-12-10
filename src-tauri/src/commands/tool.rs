// Tool Management Commands

use crate::error::Result;
use crate::mcp_manager::McpServerManager;

/// Get MCP server manager from global state
fn get_mcp_manager() -> Result<std::sync::Arc<McpServerManager>> {
    let service_manager = crate::SERVICE_MANAGER.lock().map_err(|e| {
        crate::error::McpError::Internal(format!("Failed to lock SERVICE_MANAGER: {}", e))
    })?;

    service_manager.as_ref()
        .ok_or_else(|| crate::error::McpError::Internal("SERVICE_MANAGER not initialized".to_string()))
        .map(|arc| arc.clone())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn toggle_mcp_server_tool(
    _app_handle: tauri::AppHandle,
    name: String,
    tool_name: String,
    enabled: bool,
) -> Result<String> {
    tracing::info!(
        "Toggling tool {}/{} to enabled state: {}",
        name,
        tool_name,
        enabled
    );

    let mcp_manager = get_mcp_manager()?;
    mcp_manager.toggle_tool_enabled(&name, &tool_name, enabled).await?;

    Ok(format!(
        "Tool '{}' on server '{}' {} successfully",
        tool_name,
        name,
        if enabled { "enabled" } else { "disabled" }
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn enable_all_mcp_server_tools(
    _app_handle: tauri::AppHandle,
    name: String,
) -> Result<String> {
    tracing::info!("Enabling all tools for server: {}", name);

    let mcp_manager = get_mcp_manager()?;
    mcp_manager.enable_all_tools(&name).await?;

    Ok(format!(
        "All tools for server '{}' enabled successfully",
        name
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn disable_all_mcp_server_tools(
    _app_handle: tauri::AppHandle,
    name: String,
) -> Result<String> {
    tracing::info!("Disabling all tools for server: {}", name);

    let mcp_manager = get_mcp_manager()?;
    mcp_manager.disable_all_tools(&name).await?;

    Ok(format!(
        "All tools for server '{}' disabled successfully",
        name
    ))
}
