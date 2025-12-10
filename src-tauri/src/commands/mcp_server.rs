// MCP Server Management Commands - SQLite Version

use crate::error::{McpError, Result};
use crate::mcp_manager::McpServerManager;
use crate::types::{
    McpPromptInfo, McpResourceInfo, McpServerConfig, McpServerInfo, McpToolInfo, ServiceTransport,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// MCP Server Create Request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpServerCreateRequest {
    pub name: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub transport: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub env: Option<Vec<(String, String)>>,
    pub headers: Option<Vec<(String, String)>>,
}

/// MCP Server Update Request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpServerUpdateRequest {
    pub name: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub transport: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub env: Option<Vec<(String, String)>>,
    pub headers: Option<Vec<(String, String)>>,
    pub enabled: bool,
}

// Helper function to get MCP server manager from global state
fn get_mcp_manager() -> Result<Arc<McpServerManager>> {
    let service_manager = crate::SERVICE_MANAGER.lock().map_err(|e| {
        McpError::Internal(format!("Failed to lock SERVICE_MANAGER: {}", e))
    })?;

    service_manager.as_ref().cloned().ok_or_else(|| {
        McpError::Internal("SERVICE_MANAGER not initialized".to_string())
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_mcp_server(
    _app_handle: tauri::AppHandle,
    request: McpServerCreateRequest,
) -> Result<String> {
    // Convert transport string to ServiceTransport enum
    let service_transport = match request.transport.as_str() {
        "stdio" => ServiceTransport::Stdio,
        "http" => ServiceTransport::Http,
        _ => {
            return Err(McpError::InvalidInput(format!(
                "Invalid transport type: {}",
                request.transport
            )))
        }
    };

    // Convert env and headers from Vec<(String, String)> to HashMap<String, String>
    let env_map = request
        .env
        .map(|env_list| env_list.into_iter().collect::<HashMap<String, String>>());

    let headers_map = request
        .headers
        .map(|header_list| header_list.into_iter().collect::<HashMap<String, String>>());

    let config = McpServerConfig {
        name: request.name.clone(),
        description: request.description,
        command: request.command,
        args: request.args,
        transport: service_transport,
        url: request.url,
        headers: headers_map,
        env: env_map,
        enabled: true, // Default to enabled when adding
    };

    let mcp_manager = get_mcp_manager()?;
    mcp_manager.add_mcp_server(config).await?;

    Ok(format!("MCP server '{}' added successfully", request.name))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_mcp_server(
    _app_handle: tauri::AppHandle,
    request: McpServerUpdateRequest,
) -> Result<String> {
    // Convert transport string to ServiceTransport enum
    let service_transport = match request.transport.as_str() {
        "stdio" => ServiceTransport::Stdio,
        "http" => ServiceTransport::Http,
        _ => {
            return Err(McpError::InvalidInput(format!(
                "Invalid transport type: {}",
                request.transport
            )))
        }
    };

    // Convert env and headers from Vec<(String, String)> to HashMap<String, String>
    let env_map = request
        .env
        .map(|env_list| env_list.into_iter().collect::<HashMap<String, String>>());

    let headers_map = request
        .headers
        .map(|header_list| header_list.into_iter().collect::<HashMap<String, String>>());

    let config = McpServerConfig {
        name: request.name.clone(),
        description: request.description,
        command: request.command,
        args: request.args,
        transport: service_transport,
        url: request.url,
        headers: headers_map,
        env: env_map,
        enabled: request.enabled,
    };

    let mcp_manager = get_mcp_manager()?;
    mcp_manager.update_mcp_server(config).await?;

    Ok(format!(
        "MCP server '{}' updated successfully",
        request.name
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_mcp_server(_app_handle: tauri::AppHandle, name: String) -> Result<String> {
    let mcp_manager = get_mcp_manager()?;
    mcp_manager.remove_mcp_server(&name).await?;

    Ok(format!("MCP server '{}' removed successfully", name))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn toggle_mcp_server(_app_handle: tauri::AppHandle, name: String) -> Result<bool> {
    let mcp_manager = get_mcp_manager()?;
    let new_state = mcp_manager.toggle_mcp_server(&name).await?;

    Ok(new_state)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_servers(
    _app_handle: tauri::AppHandle,
) -> Result<Vec<McpServerInfo>> {
    let mcp_manager = get_mcp_manager()?;
    let servers = mcp_manager.list_mcp_servers().await?;

    Ok(servers)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_mcp_server_tools(
    _app_handle: tauri::AppHandle,
    server_name: String,
) -> Result<Vec<McpToolInfo>> {
    let mcp_manager = get_mcp_manager()?;
    let tools = mcp_manager.get_mcp_server_tools(&server_name).await?;

    Ok(tools)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_mcp_server_resources(
    _app_handle: tauri::AppHandle,
    server_name: String,
) -> Result<Vec<McpResourceInfo>> {
    let mcp_manager = get_mcp_manager()?;
    let resources = mcp_manager.get_mcp_server_resources(&server_name).await?;

    Ok(resources)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_mcp_server_prompts(
    _app_handle: tauri::AppHandle,
    server_name: String,
) -> Result<Vec<McpPromptInfo>> {
    let mcp_manager = get_mcp_manager()?;
    let prompts = mcp_manager.get_mcp_server_prompts(&server_name).await?;

    Ok(prompts)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn refresh_mcp_server_capabilities(
    _app_handle: tauri::AppHandle,
    server_name: String,
) -> Result<String> {
    let mcp_manager = get_mcp_manager()?;

    // Clear existing cache
    mcp_manager.clear_server_cache(&server_name).await?;

    // Sync capabilities from service
    mcp_manager.sync_server_capabilities(&server_name).await?;

    Ok(format!(
        "Capabilities for MCP server '{}' refreshed successfully",
        server_name
    ))
}


// ============================================================================
// Statistics and Monitoring
// ============================================================================

#[tauri::command(rename_all = "snake_case")]
pub async fn get_mcp_server_statistics(_app_handle: tauri::AppHandle) -> Result<serde_json::Value> {
    let mcp_manager = get_mcp_manager()?;
    let servers = mcp_manager.list_mcp_servers().await?;

    let mut total_tools = 0;
    let mut total_resources = 0;
    let mut total_prompts = 0;
    let mut enabled_count = 0;
    let mut connected_count = 0;
    let servers_count = servers.len();

    for server in servers {
        if server.enabled {
            enabled_count += 1;
        }
        if server.status == "connected" {
            connected_count += 1;
        }
        total_tools += server.tool_count.unwrap_or(0);
        total_resources += server.resource_count.unwrap_or(0);
        total_prompts += server.prompt_count.unwrap_or(0);
    }

    let stats = serde_json::json!({
        "total_servers": servers_count,
        "enabled_servers": enabled_count,
        "connected_servers": connected_count,
        "total_tools": total_tools,
        "total_resources": total_resources,
        "total_prompts": total_prompts,
        "connection_rate": if servers_count > 0 {
            (connected_count as f64 / servers_count as f64 * 100.0).round()
        } else { 0.0 }
    });

    Ok(stats)
}

// Alias functions for command compatibility

/// List MCP server tools (alias for get_mcp_server_tools)
#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_tools(
    _app_handle: tauri::AppHandle,
    server_name: String,
) -> Result<Vec<McpToolInfo>> {
    get_mcp_server_tools(_app_handle, server_name).await
}

/// List MCP server resources (alias for get_mcp_server_resources)
#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_resources(
    _app_handle: tauri::AppHandle,
    server_name: String,
) -> Result<Vec<McpResourceInfo>> {
    get_mcp_server_resources(_app_handle, server_name).await
}

/// List MCP server prompts (alias for get_mcp_server_prompts)
#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_prompts(
    _app_handle: tauri::AppHandle,
    server_name: String,
) -> Result<Vec<McpPromptInfo>> {
    get_mcp_server_prompts(_app_handle, server_name).await
}

/// Refresh all MCP servers (alias for refresh_mcp_server_capabilities)
#[tauri::command(rename_all = "snake_case")]
pub async fn refresh_all_mcp_servers(
    _app_handle: tauri::AppHandle,
    server_name: String,
) -> Result<String> {
    refresh_mcp_server_capabilities(_app_handle, server_name).await
}
