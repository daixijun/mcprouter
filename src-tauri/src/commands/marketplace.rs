// 市场服务命令

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
    env_vars: Option<Vec<(String, String)>>,
) -> Result<McpServerConfig> {
    let service = get_mcp_server_details(service_id.clone()).await?;

    // Convert transport string to ServiceTransport enum
    let service_transport = match service.transport.as_str() {
        "stdio" => ServiceTransport::Stdio,
        "sse" => ServiceTransport::Sse,
        "http" => ServiceTransport::Http,
        _ => {
            return Err(McpError::InvalidConfiguration(format!(
                "Unsupported transport: {}",
                service.transport
            )))
        }
    };

    // Convert env vars into HashMap if provided
    let env_vars_map = env_vars.map(|vars| vars.into_iter().collect::<HashMap<String, String>>());

    // Check if install command is available
    let install_command = service.install_command.ok_or_else(|| {
        McpError::InvalidConfiguration("无法提取安装命令,该服务可能不支持一键安装".to_string())
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
        env_vars: env_vars_map,
        headers: None,
        version: None,
    };

    // Persist into service manager
    SERVICE_MANAGER.add_mcp_server(config.clone()).await?;

    // Try to connect immediately to retrieve version and persist to DB
    match MCP_CLIENT_MANAGER.ensure_connection(&config, false).await {
        Ok(connection) => {
            if let Some(version) = connection.cached_version.clone() {
                use crate::db::repositories::mcp_server_repository::McpServerRepository;
                if let Err(e) =
                    McpServerRepository::update_version(&config.name, Some(version.clone())).await
                {
                    tracing::warn!("Failed to update version in DB for {}: {}", config.name, e);
                } else {
                    tracing::info!(
                        "Persisted version '{}' for service {}",
                        version,
                        config.name
                    );
                }
                // Also update in-memory cache so UI shows immediately
                SERVICE_MANAGER
                    .update_version_cache(&config.name, Some(version.clone()))
                    .await;
            } else {
                tracing::info!(
                    "Service {} connected but did not report version",
                    config.name
                );
            }
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
