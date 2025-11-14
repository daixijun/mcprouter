// Tool Management Commands - Migrated to Config Files

use crate::error::Result;

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

    // Deprecated: tool persistence removed; return informational message
    tracing::info!(
        "Tool state management is deprecated; tools are cached in memory from server"
    );
    Ok(format!(
        "Tool '{}' on server '{}' not changed (memory-only listing)",
        tool_name, name
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn enable_all_mcp_server_tools(
    _app_handle: tauri::AppHandle,
    name: String,
) -> Result<String> {
    tracing::info!("Enabling all tools for server: {}", name);

    tracing::info!("Enable-all tools command deprecated; no changes applied");
    Ok(format!("Tools for server '{}' are managed in-memory only", name))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn disable_all_mcp_server_tools(
    _app_handle: tauri::AppHandle,
    name: String,
) -> Result<String> {
    tracing::info!("Disabling all tools for server: {}", name);

    tracing::info!("Disable-all tools command deprecated; no changes applied");
    Ok(format!("Tools for server '{}' are managed in-memory only", name))
}
