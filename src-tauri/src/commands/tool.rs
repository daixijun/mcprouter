// Tool Management Commands

use crate::error::Result;
use crate::mcp_manager::McpServerManager;

/// Get MCP server manager from global state (with wait)
async fn get_mcp_manager() -> Result<std::sync::Arc<McpServerManager>> {
    crate::wait_for_service_manager().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn toggle_mcp_server_tool(
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

    let mcp_manager = get_mcp_manager().await?;
    mcp_manager.toggle_tool_enabled(&name, &tool_name).await?;

    Ok(format!(
        "Tool '{}' on server '{}' {} successfully",
        tool_name,
        name,
        if enabled { "enabled" } else { "disabled" }
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn enable_all_mcp_server_tools(
    name: String,
) -> Result<String> {
    tracing::info!("Enabling all tools for server: {}", name);

    let mcp_manager = get_mcp_manager().await?;
    mcp_manager.enable_all_tools(&name).await?;

    Ok(format!(
        "All tools for server '{}' enabled successfully",
        name
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn disable_all_mcp_server_tools(
    name: String,
) -> Result<String> {
    tracing::info!("Disabling all tools for server: {}", name);

    let mcp_manager = get_mcp_manager().await?;
    mcp_manager.disable_all_tools(&name).await?;

    Ok(format!(
        "All tools for server '{}' disabled successfully",
        name
    ))
}
