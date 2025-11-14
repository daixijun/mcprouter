// MCP Server Management Commands

use crate::error::{McpError, Result};
use crate::mcp_manager::McpServerInfo;
use crate::types::{McpServerConfig, ServiceTransport};
use crate::{MCP_CLIENT_MANAGER, SERVICE_MANAGER};
use serde::Deserialize;
use std::collections::HashMap;

/// MCP Server Create Request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpServerCreateRequest {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
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

#[tauri::command(rename_all = "snake_case")]
pub async fn add_mcp_server(
    app_handle: tauri::AppHandle,
    request: McpServerCreateRequest,
) -> Result<String> {
    // Convert transport string to ServiceTransport enum
    let service_transport = match request.transport.as_str() {
        "stdio" => ServiceTransport::Stdio,
        "sse" => ServiceTransport::Sse,
        "http" => ServiceTransport::Http,
        _ => {
            return Err(McpError::InvalidConfiguration(
                "Invalid transport. Must be 'stdio', 'sse', or 'http'".to_string(),
            ))
        }
    };

    // Convert environment variables and headers
    let env_vars_map = request
        .env
        .map(|vars| vars.into_iter().collect::<HashMap<String, String>>());
    let headers_map = request
        .headers
        .map(|hdrs| hdrs.into_iter().collect::<HashMap<String, String>>());

    // Use the provided name as the service identifier
    tracing::info!(
        "Adding service: {} with transport: {:?}",
        request.name,
        service_transport
    );

    // Debug logging for HTTP services
    if matches!(
        service_transport,
        ServiceTransport::Sse | ServiceTransport::Http
    ) {
        tracing::info!(
            "Adding HTTP server: {} with URL: {:?}",
            request.name,
            request.url
        );
        if let Some(ref hdrs) = headers_map {
            tracing::debug!("Headers: {:?}", hdrs);
        }
    }

    // For non-stdio transports, set command and args to None
    let (final_command, final_args) = if matches!(
        service_transport,
        ServiceTransport::Sse | ServiceTransport::Http
    ) {
        (None, None)
    } else {
        (Some(request.command), Some(request.args))
    };

    // Create service configuration
    let service_config = McpServerConfig {
        name: request.name.clone(),
        description: request.description,
        command: final_command,
        args: final_args,
        transport: service_transport.clone(),
        url: request.url.clone(),
        enabled: true,
        env: env_vars_map,
        headers: headers_map,
    };

    // Add service using the service manager
    tracing::info!(
        "Calling SERVICE_MANAGER.add_mcp_server to add service: {}",
        request.name
    );
    match SERVICE_MANAGER
        .add_mcp_server(&app_handle, service_config)
        .await
    {
        Ok(()) => {
            tracing::info!("Service added successfully: {}", request.name);
            Ok(format!("Service '{}' added successfully", request.name))
        }
        Err(e) => {
            tracing::error!("Service addition failed: {} - {:?}", request.name, e);
            Err(e)
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_mcp_server(
    app_handle: tauri::AppHandle,
    request: McpServerUpdateRequest,
) -> Result<String> {
    // Convert transport string to ServiceTransport enum
    let service_transport = match request.transport.as_str() {
        "stdio" => ServiceTransport::Stdio,
        "sse" => ServiceTransport::Sse,
        "http" => ServiceTransport::Http,
        _ => {
            return Err(McpError::InvalidConfiguration(
                "Invalid transport. Must be 'stdio', 'sse', or 'http'".to_string(),
            ))
        }
    };

    // Convert environment variables and headers
    let env_vars_map = request
        .env
        .map(|vars| vars.into_iter().collect::<HashMap<String, String>>());
    let headers_map = request
        .headers
        .map(|hdrs| hdrs.into_iter().collect::<HashMap<String, String>>());

    tracing::info!(
        "Updating service: {} with transport: {:?}",
        request.name,
        service_transport
    );

    // Create service configuration
    let service_config = McpServerConfig {
        name: request.name.clone(),
        description: request.description,
        command: request.command,
        args: request.args,
        transport: service_transport,
        url: request.url,
        enabled: request.enabled,
        env: env_vars_map,
        headers: headers_map,
    };

    // Update service using the service manager
    match SERVICE_MANAGER
        .update_mcp_server(&app_handle, service_config)
        .await
    {
        Ok(()) => Ok(format!("Service '{}' updated successfully", request.name)),
        Err(e) => Err(e),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn toggle_mcp_server(app_handle: tauri::AppHandle, name: String) -> Result<bool> {
    match SERVICE_MANAGER.toggle_mcp_server(&app_handle, &name).await {
        Ok(new_state) => Ok(new_state),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn list_mcp_servers(app_handle: tauri::AppHandle) -> Vec<McpServerInfo> {
    SERVICE_MANAGER
        .list_mcp_servers(Some(&app_handle))
        .await
        .unwrap_or_default()
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_mcp_server(app_handle: tauri::AppHandle, name: String) -> Result<String> {
    // First disconnect any active connections
    let connections = MCP_CLIENT_MANAGER.get_connections().await;
    for connection in connections {
        if connection.service_id.starts_with(&format!("{}_", name)) {
            let _ = MCP_CLIENT_MANAGER
                .disconnect_mcp_server(&connection.service_id)
                .await;
        }
    }

    // Remove the service
    SERVICE_MANAGER
        .remove_mcp_server(&app_handle, &name)
        .await?;
    Ok(format!("Service '{}' deleted", name))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_tools(
    app_handle: tauri::AppHandle,
    name: String,
) -> Result<Vec<crate::types::McpToolInfo>> {
    tracing::info!("Getting tool list for server '{}'", name);
    match SERVICE_MANAGER
        .list_mcp_server_tools(&name, &app_handle)
        .await
    {
        Ok(tools) => {
            tracing::info!("Successfully retrieved {} tools", tools.len());
            Ok(tools)
        }
        Err(e) => {
            tracing::error!("Failed to get tool list: {}", e);
            Err(e)
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn refresh_all_mcp_servers(app_handle: tauri::AppHandle) -> Result<String> {
    tracing::info!("Manually refreshing all MCP service connection status...");

    match SERVICE_MANAGER
        .auto_connect_enabled_services(&app_handle)
        .await
    {
        Ok(_) => {
            tracing::info!("All MCP service connection status refreshed");
            Ok("All MCP service connection status refreshed".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to refresh MCP service connection status: {}", e);
            Err(e)
        }
    }
}
