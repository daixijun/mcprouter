// MCP 服务器管理命令

use crate::error::{McpError, Result};
use crate::mcp_manager::McpServerInfo;
use crate::types::{McpServerConfig, ServiceTransport};
use crate::{MCP_CLIENT_MANAGER, SERVICE_MANAGER};
use serde::Deserialize;
use std::collections::HashMap;

/// MCP 服务器创建请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpServerCreateRequest {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub transport: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub env_vars: Option<Vec<(String, String)>>,
    pub headers: Option<Vec<(String, String)>>,
}

/// MCP 服务器更新请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpServerUpdateRequest {
    pub name: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub transport: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub env_vars: Option<Vec<(String, String)>>,
    pub headers: Option<Vec<(String, String)>>,
    pub enabled: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_mcp_server(app_handle: tauri::AppHandle, request: McpServerCreateRequest) -> Result<String> {
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
    let env_vars_map = request.env_vars.map(|vars| vars.into_iter().collect::<HashMap<String, String>>());
    let headers_map = request.headers.map(|hdrs| hdrs.into_iter().collect::<HashMap<String, String>>());

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
        tracing::info!("Adding HTTP server: {} with URL: {:?}", request.name, request.url);
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
        env_vars: env_vars_map,
        headers: headers_map,
        version: None, // Version will be detected when connecting to the service
    };

    // Add service using the service manager
    tracing::info!("调用 SERVICE_MANAGER.add_mcp_server 添加服务: {}", request.name);
    match SERVICE_MANAGER.add_mcp_server(&app_handle, service_config).await {
        Ok(()) => {
            tracing::info!("服务添加成功: {}", request.name);
            Ok(format!("服务 '{}' 已成功添加", request.name))
        }
        Err(e) => {
            tracing::error!("服务添加失败: {} - {:?}", request.name, e);
            Err(e)
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_mcp_server(app_handle: tauri::AppHandle, request: McpServerUpdateRequest) -> Result<String> {
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
    let env_vars_map = request.env_vars.map(|vars| vars.into_iter().collect::<HashMap<String, String>>());
    let headers_map = request.headers.map(|hdrs| hdrs.into_iter().collect::<HashMap<String, String>>());

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
        env_vars: env_vars_map,
        headers: headers_map,
        version: None, // Version will be preserved/updated when connecting
    };

    // Update service using the service manager
    match SERVICE_MANAGER.update_mcp_server(&app_handle, service_config).await {
        Ok(()) => Ok(format!("服务 '{}' 已成功更新", request.name)),
        Err(e) => Err(e),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn remove_mcp_server(app_handle: tauri::AppHandle, name: String) -> Result<String> {
    match SERVICE_MANAGER.remove_mcp_server(&app_handle, &name).await {
        Ok(()) => Ok(format!("服务 '{}' 已成功删除", name)),
        Err(e) => Err(e),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn check_mcp_server_connectivity(name: String) -> Result<String> {
    match SERVICE_MANAGER.check_service_with_version(&name).await {
        Ok(_) => Ok(format!("服务 '{}' 连接成功", name)),
        Err(e) => {
            tracing::error!("Failed to connect to service {}: {:?}", name, e);
            Err(e)
        }
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
pub async fn list_mcp_servers() -> Vec<McpServerInfo> {
    SERVICE_MANAGER.list_mcp_servers().await.unwrap_or_default()
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
    SERVICE_MANAGER.remove_mcp_server(&app_handle, &name).await?;
    Ok(format!("服务 '{}' 已删除", name))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_mcp_server_tools(app_handle: tauri::AppHandle, name: String) -> Result<Vec<String>> {
    tracing::info!("正在获取服务器 '{}' 的工具列表", name);
    match SERVICE_MANAGER.list_mcp_server_tools(&name, &app_handle).await {
        Ok(tools) => {
            tracing::info!("✅ 成功获取到 {} 个工具", tools.len());
            Ok(tools)
        }
        Err(e) => {
            tracing::error!("❌ 获取工具列表失败: {}", e);
            Err(e)
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn refresh_all_mcp_servers(app_handle: tauri::AppHandle) -> Result<String> {
    tracing::info!("🔄 手动刷新所有MCP服务连接状态...");

    match SERVICE_MANAGER.auto_connect_enabled_services(&app_handle).await {
        Ok(_) => {
            tracing::info!("✅ 所有MCP服务连接状态已刷新");
            Ok("所有MCP服务连接状态已刷新".to_string())
        }
        Err(e) => {
            tracing::error!("❌ 刷新MCP服务连接状态失败: {}", e);
            Err(e)
        }
    }
}
