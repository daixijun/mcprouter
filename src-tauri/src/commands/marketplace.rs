// Marketplace Service Commands

use crate::error::{McpError, Result};
use crate::types::{MarketplaceService, McpServerConfig, ServiceTransport};
use crate::{marketplace, MCP_CLIENT_MANAGER, SERVICE_MANAGER};
use std::collections::HashMap;

#[tauri::command(rename_all = "snake_case")]
pub async fn get_mcp_server_details(service_id: String) -> Result<MarketplaceService> {
    marketplace::get_service_details(&service_id).await
}

#[tauri::command]
pub async fn list_marketplace_services(
    query: String,
    page: usize,
    page_size: usize,
) -> Result<marketplace::MarketplaceServiceResult> {
    marketplace::list_marketplace_services(Some(&query), page, page_size).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn install_marketplace_service(
    service_id: String,
    env: Option<Vec<(String, String)>>,
) -> Result<McpServerConfig> {
    let service = get_mcp_server_details(service_id.clone()).await?;

    // Convert transport string to ServiceTransport enum
    let service_transport = match service.transport.as_str() {
        "stdio" => ServiceTransport::Stdio,
        "http" => ServiceTransport::Http,
        "sse" => {
            tracing::warn!("SSE transport is no longer supported, falling back to HTTP");
            ServiceTransport::Http
        },
        _ => {
            return Err(McpError::InvalidConfiguration(format!(
                "Unsupported transport: {}",
                service.transport
            )))
        }
    };

    // Convert env vars into HashMap if provided
    let env_map = env.map(|vars| vars.into_iter().collect::<HashMap<String, String>>());

    // Check if install command is available
    let install_command = service.install_command.ok_or_else(|| {
        McpError::InvalidConfiguration("Cannot extract installation command, this service may not support one-click installation".to_string())
    })?;

    // Create service configuration and add to manager (one-click install)
    let config = McpServerConfig {
        name: service.name.clone(),
        description: Some(service.description.clone()),
        command: Some(install_command.command.clone()),
        args: Some(install_command.args.clone()),
        transport: service_transport,
        url: None,
        enabled: true,
        env: env_map,
        headers: None,
    };

    // Persist into service manager
    let service_manager = {
        let guard = SERVICE_MANAGER.lock().unwrap();
        guard.as_ref().unwrap().clone()
    };
    service_manager.add_server(&config).await?;

    // Try to connect immediately to retrieve version (cache only)
    match MCP_CLIENT_MANAGER.ensure_connection(&config, false).await {
        Ok(_connection) => {
            tracing::info!("Service {} connected successfully", config.name);
            // Version is now read directly from config files, no need for in-memory cache
        }
        Err(e) => {
            tracing::warn!(
                "Failed to connect to service {} during install for version capture: {}",
                config.name,
                e
            );
        }
    }

    Ok(config)
}
