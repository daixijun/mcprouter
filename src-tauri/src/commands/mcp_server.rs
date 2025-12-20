// MCP Server Management Commands - SQLite Version

use crate::error::{McpError, Result};
use crate::mcp_manager::McpServerManager;
use crate::types::{
    McpPromptInfo, McpResourceInfo, McpServerConfig, McpServerResult, McpToolInfo, ServiceTransport,
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

// Helper function to get MCP server manager from global state (with wait)
async fn get_mcp_manager() -> Result<Arc<McpServerManager>> {
    crate::wait_for_service_manager().await
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

    let mcp_manager = get_mcp_manager().await?;
    mcp_manager.add_server(&config).await?;

    // 新服务器添加后立即连接并同步
    let server_name = request.name.clone();
    let server_config = config.clone();
    tokio::spawn(async move {
        tracing::info!(
            "Attempting to connect to newly added server '{}'",
            server_name
        );
        // 连接新服务器
        match crate::MCP_CLIENT_MANAGER
            .ensure_connection(&server_config, false)
            .await
        {
            Ok(_) => {
                tracing::info!("Successfully connected to new server '{}'", server_name);

                // 连接成功后同步资源
                match mcp_manager.sync_server_manifests(&server_name).await {
                    Ok(_) => {
                        tracing::info!(
                            "Successfully synced manifests for new server '{}'",
                            server_name
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to sync manifests for new server '{}': {}",
                            server_name,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect to new server '{}': {}", server_name, e);
            }
        }
    });

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

    let mcp_manager = get_mcp_manager().await?;
    mcp_manager.update_server(&request.name, &config).await?;

    Ok(format!(
        "MCP server '{}' updated successfully",
        request.name
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_mcp_server(name: String) -> Result<String> {
    let mcp_manager = get_mcp_manager().await?;
    mcp_manager.delete_server(&name).await?;

    Ok(format!("MCP server '{}' removed successfully", name))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn toggle_mcp_server(name: String) -> Result<bool> {
    let mcp_manager = get_mcp_manager().await?;
    let new_state = mcp_manager.toggle_mcp_server(&name).await?;

    // 根据新状态执行相应操作
    if new_state {
        // 启用：尝试连接并同步
        if let Some(server) = mcp_manager.get_server_by_name(&name).await? {
            let server_config = McpServerConfig {
                name: server.name.clone(),
                description: server.description,
                transport: server.transport.parse().unwrap_or(ServiceTransport::Stdio),
                command: server.command,
                args: server.args,
                url: server.url,
                headers: server.headers,
                env: server.env,
                enabled: true,
            };

            // 异步连接和同步
            let server_name = name.clone();
            tokio::spawn(async move {
                tracing::info!(
                    "Attempting to connect to newly enabled server '{}'",
                    server_name
                );
                match crate::MCP_CLIENT_MANAGER
                    .ensure_connection(&server_config, false)
                    .await
                {
                    Ok(_) => {
                        tracing::info!(
                            "Successfully connected to enabled server '{}'",
                            server_name
                        );

                        // 连接成功后同步资源
                        match mcp_manager.sync_server_manifests(&server_name).await {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully synced manifests for enabled server '{}'",
                                    server_name
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to sync manifests for enabled server '{}': {}",
                                    server_name,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to connect to newly enabled server '{}': {}",
                            server_name,
                            e
                        );
                    }
                }
            });
        }
    } else {
        // 禁用：断开连接
        let server_name = name.clone();
        tokio::spawn(async move {
            tracing::info!("Attempting to disconnect disabled server '{}'", server_name);
            if let Err(e) = crate::MCP_CLIENT_MANAGER
                .disconnect_server(&server_name)
                .await
            {
                tracing::error!("Failed to disconnect server '{}': {}", server_name, e);
            } else {
                tracing::info!("Successfully disconnected server '{}'", server_name);
            }
        });
    }

    Ok(new_state)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_servers() -> Result<McpServerResult> {
    let mcp_manager = get_mcp_manager().await?;
    let (servers, total_count) = mcp_manager.list_servers(None, None).await?;

    Ok(McpServerResult {
        servers,
        total_count,
    })
}

// ============================================================================
// Statistics and Monitoring
// ============================================================================

#[tauri::command(rename_all = "snake_case")]
pub async fn get_mcp_server_statistics() -> Result<serde_json::Value> {
    let mcp_manager = get_mcp_manager().await?;
    let (servers, _) = mcp_manager.list_servers(None, None).await?;

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

/// List MCP server tools
#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_tools(server_name: String) -> Result<Vec<McpToolInfo>> {
    let mcp_manager = get_mcp_manager().await?;
    let tools = mcp_manager.list_mcp_server_tools(&server_name).await?;
    Ok(tools)
}

/// List MCP server resources
#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_resources(server_name: String) -> Result<Vec<McpResourceInfo>> {
    let mcp_manager = get_mcp_manager().await?;
    let resources = mcp_manager.list_mcp_server_resources(&server_name).await?;

    Ok(resources)
}

/// List MCP server prompts
#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_prompts(server_name: String) -> Result<Vec<McpPromptInfo>> {
    let mcp_manager = get_mcp_manager().await?;
    let prompts = mcp_manager.list_mcp_server_prompts(&server_name).await?;
    Ok(prompts)
}
