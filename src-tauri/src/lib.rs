mod aggregator;
mod config;
mod db;
mod error;
mod marketplace;
mod mcp_client;
mod mcp_manager;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex as TokioMutex;

use crate::config as config_mod;
use config::{ApiKey, ApiKeyPermissions, AppConfig, McpServerConfig, ServiceTransport};
use error::{McpError, Result};
use mcp_client::McpClientManager;
use mcp_manager::{McpServerInfo, McpServerManager};
use rmcp::model::Tool as RmcpTool;

// Use rmcp Tool model directly instead of custom struct
pub type McpTool = RmcpTool;

// Define MarketplaceService struct since it's used in function signatures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketplaceService {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub transport: String,
    pub install_command: Option<InstallCommand>,
    pub github_stars: Option<u64>,
    pub downloads: u32,
    pub last_updated: String,
    pub platform: String,
    pub logo_url: Option<String>,
    pub license: Option<String>,
    pub is_hosted: Option<bool>,
    pub is_verified: Option<bool>,
    // Fields only in detail view
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation_url: Option<String>,
    pub requirements: Vec<String>,
    pub readme: Option<String>,
    pub server_config: Option<Vec<serde_json::Value>>,
    pub env_schema: Option<EnvSchema>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallCommand {
    pub command: String,
    pub args: Vec<String>,
    pub package_manager: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvSchema {
    #[serde(default)]
    pub properties: HashMap<String, EnvProperty>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(rename = "type")]
    pub schema_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvProperty {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub prop_type: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

// Marketplace service list item without install_command and other unnecessary fields for lighter responses
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketplaceServiceListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub transport: String,
    pub github_stars: Option<u64>,
    pub downloads: u32,
    pub last_updated: String,
    pub platform: String,
    pub logo_url: Option<String>,
    pub license: Option<String>,
    pub is_hosted: Option<bool>,
    pub is_verified: Option<bool>,
}

// Global state
static SERVICE_MANAGER: std::sync::LazyLock<Arc<McpServerManager>> = std::sync::LazyLock::new(
    || {
        let config = AppConfig::load().unwrap_or_else(|e| {
            tracing::error!(
                "\n========================================\nERROR: Failed to load configuration file\n========================================\n{}\n\nThe application cannot start with an invalid configuration.\nPlease fix the config file at: ~/.mcprouter/config.json\nOr delete it to use default settings.\n",
                e
            );
            std::process::exit(1);
        });
        Arc::new(McpServerManager::new(config))
    },
);

static MCP_CLIENT_MANAGER: std::sync::LazyLock<Arc<McpClientManager>> = std::sync::LazyLock::new(
    || {
        // Create a new config instance for MCP_CLIENT_MANAGER
        // We'll sync them later when needed
        let config = AppConfig::load().unwrap_or_else(|e| {
            tracing::error!(
                "\n========================================\nERROR: Failed to load configuration file\n========================================\n{}\n\nThe application cannot start with an invalid configuration.\nPlease fix the config file at: ~/.mcprouter/config.json\nOr delete it to use default settings.\n",
                e
            );
            std::process::exit(1);
        });
        Arc::new(McpClientManager::new(config))
    },
);

static AGGREGATOR: std::sync::LazyLock<Arc<aggregator::McpAggregator>> = std::sync::LazyLock::new(
    || {
        let config = AppConfig::load().unwrap_or_else(|e| {
            tracing::error!(
                "\n========================================\nERROR: Failed to load configuration file\n========================================\n{}\n\nThe application cannot start with an invalid configuration.\nPlease fix the config file at: ~/.mcprouter/config.json\nOr delete it to use default settings.\n",
                e
            );
            std::process::exit(1);
        });
        let mcp_server_manager = Arc::new(mcp_manager::McpServerManager::new(config.clone()));
        Arc::new(aggregator::McpAggregator::new(
            mcp_server_manager,
            config.server.clone(),
        ))
    },
);

// Track application startup time
static STARTUP_TIME: std::sync::LazyLock<SystemTime> = std::sync::LazyLock::new(SystemTime::now);

// Track aggregator task handle
static AGGREGATOR_HANDLE: TokioMutex<Option<tokio::task::JoinHandle<()>>> =
    TokioMutex::const_new(None);

#[tauri::command(rename_all = "snake_case")]
async fn add_mcp_server(
    name: String,
    command: String,
    args: Vec<String>,
    transport: String,
    url: Option<String>,
    description: Option<String>,
    env_vars: Option<Vec<(String, String)>>,
    headers: Option<Vec<(String, String)>>,
) -> Result<String> {
    // Convert transport string to ServiceTransport enum
    let service_transport = match transport.as_str() {
        "stdio" => ServiceTransport::Stdio,
        "sse" => ServiceTransport::Sse,
        "streamablehttp" => ServiceTransport::StreamableHttp,
        _ => {
            return Err(McpError::InvalidConfiguration(
                "Invalid transport. Must be 'stdio', 'sse', or 'streamablehttp'".to_string(),
            ))
        }
    };

    // Convert environment variables and headers
    let env_vars_map = env_vars.map(|vars| vars.into_iter().collect::<HashMap<String, String>>());
    let headers_map = headers.map(|hdrs| hdrs.into_iter().collect::<HashMap<String, String>>());

    // Use the provided name as the service identifier
    tracing::info!(
        "Adding service: {} with transport: {:?}",
        name,
        service_transport
    );

    // Debug logging for HTTP services
    if matches!(
        service_transport,
        ServiceTransport::Sse | ServiceTransport::StreamableHttp
    ) {
        tracing::info!("Adding HTTP server: {} with URL: {:?}", name, url);
        if let Some(ref hdrs) = headers_map {
            tracing::debug!("Headers: {:?}", hdrs);
        }
    }

    // For non-stdio transports, set command and args to None
    let (final_command, final_args) = if matches!(
        service_transport,
        ServiceTransport::Sse | ServiceTransport::StreamableHttp
    ) {
        (None, None)
    } else {
        (Some(command), Some(args))
    };

    // Create service configuration
    let service_config = McpServerConfig {
        name: name.clone(),
        description,
        command: final_command,
        args: final_args,
        transport: service_transport.clone(),
        url: url.clone(),
        enabled: true,
        env_vars: env_vars_map,
        headers: headers_map,
        version: None, // Version will be detected when connecting to the service
    };

    // Add service using the service manager
    match SERVICE_MANAGER.add_mcp_server(service_config).await {
        Ok(()) => Ok(format!("服务 '{}' 已成功添加", name)),
        Err(e) => Err(e),
    }
}

// Configuration commands
#[tauri::command]
async fn get_config() -> Result<config::AppConfig> {
    Ok(SERVICE_MANAGER.get_config().await)
}

#[tauri::command]
async fn get_theme() -> Result<String> {
    let config = SERVICE_MANAGER.get_config().await;
    let theme = config
        .settings
        .as_ref()
        .and_then(|s| s.theme.as_ref())
        .cloned()
        .unwrap_or_else(|| "auto".to_string());
    Ok(theme)
}

#[tauri::command]
async fn set_theme(app: tauri::AppHandle, theme: String) -> Result<()> {
    SERVICE_MANAGER
        .update_config(|config| {
            if config.settings.is_none() {
                config.settings = Some(crate::config::Settings {
                    theme: Some("auto".to_string()),
                    autostart: Some(false),
                    system_tray: Some(crate::config::SystemTraySettings {
                        enabled: Some(true),
                        close_to_tray: Some(false),
                        start_to_tray: Some(false),
                    }),
                    uv_index_url: None,
                    npm_registry: None,
                });
            }
            if let Some(settings) = config.settings.as_mut() {
                settings.theme = Some(theme.clone());
            }
        })
        .await?;
    let _ = app.emit("theme-changed", theme);
    Ok(())
}

// Service Management Commands
#[tauri::command(rename_all = "snake_case")]
async fn remove_mcp_server(name: String) -> Result<String> {
    match SERVICE_MANAGER.remove_mcp_server(&name).await {
        Ok(()) => Ok(format!("服务 '{}' 已成功删除", name)),
        Err(e) => Err(e),
    }
}

// Check service connectivity
#[tauri::command(rename_all = "snake_case")]
async fn check_mcp_server_connectivity(name: String) -> Result<String> {
    match SERVICE_MANAGER.check_service_with_version(&name).await {
        Ok(_) => Ok(format!("服务 '{}' 连接成功", name)),
        Err(e) => {
            tracing::error!("Failed to connect to service {}: {:?}", name, e);
            Err(e)
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn toggle_mcp_server(name: String) -> Result<bool> {
    match SERVICE_MANAGER.toggle_mcp_server(&name).await {
        Ok(new_state) => Ok(new_state),
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn list_mcp_servers() -> Vec<McpServerInfo> {
    match SERVICE_MANAGER.list_mcp_servers().await {
        Ok(services) => services,
        Err(_) => Vec::new(),
    }
}

// Marketplace commands - Platform-specific APIs
#[tauri::command(rename_all = "snake_case")]
async fn get_mcp_server_details(service_id: String) -> Result<MarketplaceService> {
    marketplace::get_service_details(&service_id).await
}

#[tauri::command]
async fn list_marketplace_services(
    query: String,
    page: usize,
    page_size: usize,
) -> Result<marketplace::MarketplaceServiceResult> {
    marketplace::list_marketplace_services(Some(&query), page, page_size).await
}

// Update full AppConfig (persist to ~/.mcprouter/config.json)
#[tauri::command]
async fn update_config(config: config_mod::AppConfig) -> Result<String> {
    config.save()?;
    Ok("Config updated".to_string())
}

#[tauri::command(rename_all = "snake_case")]
async fn install_marketplace_service(
    service_id: String,
    env_vars: Option<Vec<(String, String)>>,
) -> Result<McpServerConfig> {
    let service = get_mcp_server_details(service_id.clone()).await?;

    // Convert transport string to ServiceTransport enum
    let service_transport = match service.transport.as_str() {
        "stdio" => ServiceTransport::Stdio,
        "sse" => ServiceTransport::Sse,
        "streamablehttp" => ServiceTransport::StreamableHttp,
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
        McpError::InvalidConfiguration("无法提取安装命令，该服务可能不支持一键安装".to_string())
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

// MCP Client Management Commands
#[tauri::command(rename_all = "snake_case")]
async fn connect_to_mcp_server(name: String) -> Result<String> {
    // Get service config
    let services_arc = SERVICE_MANAGER.get_mcp_servers().await;
    let services = services_arc.read().await;
    let service_config = services
        .get(&name)
        .ok_or_else(|| McpError::ServiceNotFound(name.clone()))?;

    // Connect to MCP server
    let connection = MCP_CLIENT_MANAGER
        .connect_mcp_server(&service_config)
        .await?;
    let _connection_id = connection.service_id.clone();

    // Version information is now handled automatically by the MCP client
    tracing::info!("Connected to MCP server: {}", name);

    Ok(name)
}

#[tauri::command(rename_all = "snake_case")]
async fn disconnect_from_mcp_server(connection_id: String) -> Result<String> {
    MCP_CLIENT_MANAGER
        .disconnect_mcp_server(&connection_id)
        .await?;
    Ok(format!("已断开连接: {}", connection_id))
}

#[tauri::command(rename_all = "snake_case")]
async fn list_mcp_server_tools(connection_id: String) -> Result<Vec<serde_json::Value>> {
    use crate::db::repositories::mcp_server_repository::McpServerRepository;
    use crate::db::repositories::tool_repository::ToolRepository;

    let server_name = connection_id.split('_').next().unwrap_or(&connection_id);

    // Fetch tools from database only; do not start/connect service
    let server = McpServerRepository::get_by_name(server_name).await?;
    let server_id = match server {
        Some(s) => s.id.unwrap_or_default(),
        None => String::new(),
    };

    let db_tools = if !server_id.is_empty() {
        ToolRepository::get_by_server_id(&server_id)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let result: Vec<serde_json::Value> = db_tools
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description.unwrap_or_default(),
                "enabled": t.enabled,
                "input_schema": serde_json::Value::Null,
                "parameters": serde_json::Value::Null
            })
        })
        .collect();

    Ok(result)
}

#[tauri::command(rename_all = "snake_case")]
async fn call_mcp_tool(
    connection_id: String,
    tool_name: String,
    arguments: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
    let server_name = connection_id.split('_').next().unwrap_or(&connection_id);

    // Ensure connection is established; frontend should not need manual connect
    {
        let services_arc = SERVICE_MANAGER.get_mcp_servers().await;
        let services = services_arc.read().await;
        let service_config = services
            .get(server_name)
            .ok_or_else(|| crate::error::McpError::ServiceNotFound(server_name.to_string()))?
            .clone();
        drop(services);
        let _ = MCP_CLIENT_MANAGER
            .ensure_connection(&service_config, false)
            .await?;
    }

    MCP_CLIENT_MANAGER
        .call_tool(server_name, &tool_name, arguments)
        .await
}

#[tauri::command(rename_all = "snake_case")]
async fn get_mcp_server_info(connection_id: String) -> Result<serde_json::Value> {
    let connection = MCP_CLIENT_MANAGER
        .get_connection(&connection_id)
        .await
        .ok_or_else(|| McpError::ServiceNotFound(connection_id))?;
    Ok(connection.server_info.unwrap_or(serde_json::json!({})))
}

#[tauri::command(rename_all = "snake_case")]
async fn list_mcp_connections() -> Vec<String> {
    MCP_CLIENT_MANAGER
        .get_connections()
        .await
        .into_iter()
        .map(|conn| conn.service_id)
        .collect()
}

// Enhanced Service Management Commands with MCP Client Integration

#[tauri::command(rename_all = "snake_case")]
async fn delete_mcp_server(name: String) -> Result<String> {
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
    SERVICE_MANAGER.remove_mcp_server(&name).await?;
    Ok(format!("服务 '{}' 已删除", name))
}

#[tauri::command(rename_all = "snake_case")]
async fn toggle_mcp_server_tool(name: String, tool_name: String, enabled: bool) -> Result<String> {
    use crate::db::repositories::mcp_server_repository::McpServerRepository;
    use crate::db::repositories::tool_repository::ToolRepository;

    // Get server from database
    let server = McpServerRepository::get_by_name(&name)
        .await?
        .ok_or_else(|| McpError::ServiceNotFound(name.clone()))?;

    let server_id = server
        .id
        .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?;

    // Get tool from database
    let tool = ToolRepository::get_by_name(&server_id, &tool_name)
        .await?
        .ok_or_else(|| McpError::ConfigError(format!("Tool '{}' not found", tool_name)))?;

    // Toggle tool status in database
    let tool_id = tool
        .id
        .ok_or_else(|| McpError::ConfigError("Tool ID not found".to_string()))?;
    ToolRepository::toggle_enabled(&tool_id, enabled).await?;

    let status = if enabled { "enabled" } else { "disabled" };
    Ok(format!(
        "Tool '{}' has been {} for service '{}'",
        tool_name, status, name
    ))
}

#[tauri::command(rename_all = "snake_case")]
async fn enable_all_mcp_server_tools(name: String) -> Result<String> {
    use crate::db::repositories::mcp_server_repository::McpServerRepository;
    use crate::db::repositories::tool_repository::ToolRepository;

    // Get server from database
    let server = McpServerRepository::get_by_name(&name)
        .await?
        .ok_or_else(|| McpError::ServiceNotFound(name.clone()))?;

    let server_id = server
        .id
        .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?;

    // Enable all tools for this server
    ToolRepository::batch_toggle_server_tools(&server_id, true).await?;

    Ok(format!(
        "All tools have been enabled for service '{}'",
        name
    ))
}

#[tauri::command(rename_all = "snake_case")]
async fn disable_all_mcp_server_tools(name: String) -> Result<String> {
    use crate::db::repositories::mcp_server_repository::McpServerRepository;
    use crate::db::repositories::tool_repository::ToolRepository;

    // Get server from database
    let server = McpServerRepository::get_by_name(&name)
        .await?
        .ok_or_else(|| McpError::ServiceNotFound(name.clone()))?;

    let server_id = server
        .id
        .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?;

    // Disable all tools for this server
    ToolRepository::batch_toggle_server_tools(&server_id, false).await?;

    Ok(format!(
        "All tools have been disabled for service '{}'",
        name
    ))
}

#[tauri::command(rename_all = "snake_case")]
async fn get_tools_by_server(
    server_id: Option<String>,
    name: Option<String>,
) -> Result<Vec<serde_json::Value>> {
    use crate::db::repositories::mcp_server_repository::McpServerRepository;
    use crate::db::repositories::tool_repository::ToolRepository;

    // Resolve server_id from either provided server_id or name
    let resolved_server_id = match (server_id, name) {
        (Some(id), _) if !id.is_empty() => id,
        (_, Some(n)) if !n.is_empty() => {
            let server = McpServerRepository::get_by_name(&n)
                .await?
                .ok_or_else(|| McpError::ServiceNotFound(n.clone()))?;
            server
                .id
                .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?
        }
        _ => {
            return Err(McpError::ConfigError(
                "缺少参数：需要 server_id 或 name".to_string(),
            ))
        }
    };

    // Fetch tools from database by server_id
    let tools = ToolRepository::get_by_server_id(&resolved_server_id).await?;

    // Map to JSON matching frontend Tool interface
    let result = tools
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id.unwrap_or_default(),
                "name": t.name,
                "server_id": t.server_id,
                "description": t.description,
                "enabled": t.enabled,
                "created_at": t.created_at.to_rfc3339(),
                "updated_at": t.updated_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(result)
}

#[tauri::command(rename_all = "snake_case")]
async fn get_dashboard_stats() -> Result<serde_json::Value> {
    // Get actual service statistics
    let services = SERVICE_MANAGER.list_mcp_servers().await?;
    let enabled_services = services.iter().filter(|s| s.enabled).count();
    let disabled_services = services.len() - enabled_services;

    // Get active connections from MCP_CLIENT_MANAGER
    let connections = MCP_CLIENT_MANAGER.get_connections().await;

    // Get startup time as ISO 8601 string
    let startup_time = {
        let duration_since_epoch = STARTUP_TIME.duration_since(UNIX_EPOCH).unwrap_or_default();
        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(
            duration_since_epoch.as_secs() as i64,
            0,
        )
        .unwrap_or_default();
        datetime.to_rfc3339()
    };

    // Get the total number of configured services directly from the manager
    let total_services = {
        let mcp_servers = SERVICE_MANAGER.get_mcp_servers().await;
        let servers = mcp_servers.read().await;
        servers.len()
    };

    // Get the current server configuration
    let server_config = {
        let config = config::AppConfig::load()
            .map_err(|e| McpError::ConfigError(format!("Failed to load configuration: {}", e)))?;
        config.server
    };

    // Get aggregator statistics
    let aggregator_stats = AGGREGATOR.get_statistics().await;
    let aggregator_endpoint = format!("http://{}:{}/mcp", server_config.host, server_config.port);

    // Calculate total tools count from all services
    let total_tools = services
        .iter()
        .map(|s| s.tool_count.unwrap_or(0) as u32)
        .sum::<u32>();

    Ok(serde_json::json!({
        "total_servers": total_services,
        "enabled_servers": enabled_services,
        "disabled_servers": disabled_services,
        "connected_services": aggregator_stats.get("connected_services").and_then(|v| v.as_u64()).unwrap_or(0),
        "total_tools": total_tools,
        "active_clients": connections.len(),
        "startup_time": startup_time,
        "os_info": {
            "platform": tauri_plugin_os::platform().to_string(),
            "type": tauri_plugin_os::type_().to_string(),
            "version": match tauri_plugin_os::version() {
                tauri_plugin_os::Version::Semantic(major, minor, patch) =>
                    format!("{}.{}.{}", major, minor, patch),
                _ => format!("{:?}", tauri_plugin_os::version()).replace('"', "").replace("Semantic", ""),
            },
            "arch": tauri_plugin_os::arch().to_string(),
        },
        "connections": {
            "active_clients": connections.len(),
            "active_services": aggregator_stats.get("active_connections").and_then(|v| v.as_u64()).unwrap_or(0),
        },
        "aggregator": {
            "endpoint": aggregator_endpoint,
            "is_running": true,
            "connected_services": aggregator_stats.get("connected_services").and_then(|v| v.as_u64()).unwrap_or(0),
            "max_connections": server_config.max_connections,
            "timeout_seconds": server_config.timeout_seconds,
        }
    }))
}

// System Settings Management Commands
#[tauri::command(rename_all = "snake_case")]
fn get_settings() -> Result<serde_json::Value> {
    // Load configuration
    let config = config::AppConfig::load()
        .map_err(|e| McpError::ConfigError(format!("Failed to load settings: {}", e)))?;

    // Convert AppConfig to serde_json::Value
    serde_json::to_value(config)
        .map_err(|e| McpError::ConfigError(format!("Failed to convert settings to JSON: {}", e)))
}

#[tauri::command(rename_all = "snake_case")]
async fn save_settings(app: tauri::AppHandle, settings: serde_json::Value) -> Result<String> {
    use serde_json::Value;

    // Snapshot old config before update
    let prev_config = SERVICE_MANAGER.get_config().await;
    let tray_old = prev_config
        .settings
        .as_ref()
        .and_then(|s| s.system_tray.as_ref())
        .and_then(|t| t.enabled)
        .unwrap_or(true);

    // Normalize incoming payload: accept { settings: {...} } or pure settings object
    let settings_obj = match settings.get("settings") {
        Some(Value::Object(o)) => Some(o.clone()),
        _ => settings.as_object().cloned(),
    }
    .ok_or_else(|| {
        McpError::ConfigError("Invalid settings payload: expected object".to_string())
    })?;

    // Update tray handling in save_settings to create/hide tray dynamically
    // Apply updates under write-lock to avoid overwriting concurrent changes (e.g., theme)
    SERVICE_MANAGER
        .update_config(|config| {
            // Ensure settings exists
            if config.settings.is_none() {
                config.settings = Some(config_mod::Settings {
                    theme: Some("auto".to_string()),
                    autostart: Some(false),
                    system_tray: Some(config_mod::SystemTraySettings {
                        enabled: Some(true),
                        close_to_tray: Some(false),
                        start_to_tray: Some(false),
                    }),
                    uv_index_url: None,
                    npm_registry: None,
                });
            }
            let settings_mut = config.settings.as_mut().unwrap();

            // Theme
            if let Some(Value::String(theme)) = settings_obj.get("theme") {
                settings_mut.theme = Some(theme.clone());
            }

            // Autostart (flag only; actual OS integration via separate command)
            if let Some(Value::Bool(b)) = settings_obj.get("autostart") {
                settings_mut.autostart = Some(*b);
            }

            // System tray subobject
            if let Some(Value::Object(tray_obj)) = settings_obj.get("system_tray") {
                if settings_mut.system_tray.is_none() {
                    settings_mut.system_tray = Some(config_mod::SystemTraySettings {
                        enabled: Some(true),
                        close_to_tray: Some(false),
                        start_to_tray: Some(false),
                    });
                }
                let tray_mut = settings_mut.system_tray.as_mut().unwrap();

                // Handle enabled status first
                if let Some(Value::Bool(enabled)) = tray_obj.get("enabled") {
                    tray_mut.enabled = Some(*enabled);

                    // 如果系统托盘被禁用，自动禁用"关闭到托盘"功能
                    if !*enabled {
                        tray_mut.close_to_tray = Some(false);
                        tracing::info!(
                            "System tray disabled, automatically disabling close-to-tray feature"
                        );
                    }
                }

                // 只有在系统托盘启用时才允许设置"关闭到托盘"
                let tray_enabled = tray_mut.enabled.unwrap_or(true);
                if tray_enabled {
                    if let Some(Value::Bool(close_to_tray)) = tray_obj.get("close_to_tray") {
                        tray_mut.close_to_tray = Some(*close_to_tray);
                    }
                }

                if let Some(Value::Bool(start_to_tray)) = tray_obj.get("start_to_tray") {
                    tray_mut.start_to_tray = Some(*start_to_tray);
                }
            }

            // Package mirror settings
            if let Some(Value::String(uv_url)) = settings_obj.get("uv_index_url") {
                settings_mut.uv_index_url = Some(uv_url.clone());
            } else if let Some(Value::Null) = settings_obj.get("uv_index_url") {
                settings_mut.uv_index_url = None;
            }

            if let Some(Value::String(npm_reg)) = settings_obj.get("npm_registry") {
                settings_mut.npm_registry = Some(npm_reg.clone());
            } else if let Some(Value::Null) = settings_obj.get("npm_registry") {
                settings_mut.npm_registry = None;
            }

            // Server config (support if provided at top-level payload)
            if let Some(Value::Object(server_obj)) = settings.get("server") {
                if let Some(Value::String(host)) = server_obj.get("host") {
                    config.server.host = host.clone();
                }
                if let Some(Value::Number(port)) = server_obj.get("port") {
                    if let Some(p) = port.as_u64() {
                        config.server.port = p as u16;
                    }
                }
                if let Some(Value::Number(max_conn)) = server_obj.get("max_connections") {
                    if let Some(mc) = max_conn.as_u64() {
                        config.server.max_connections = mc as usize;
                    }
                }
                if let Some(Value::Number(timeout)) = server_obj.get("timeout_seconds") {
                    if let Some(ts) = timeout.as_u64() {
                        config.server.timeout_seconds = ts;
                    }
                }
            }

            // Security config (support top-level payload; auth bool and allowed_hosts)
            if let Some(Value::Object(sec_obj)) = settings.get("security") {
                // Ensure security exists
                if config.security.is_none() {
                    config.security = Some(config_mod::SecuritySettings {
                        allowed_hosts: vec![],
                        auth: true,
                    });
                }
                let sec_mut = config.security.as_mut().unwrap();

                // auth can be boolean (new) or object with enabled (legacy)
                match sec_obj.get("auth") {
                    Some(Value::Bool(b)) => {
                        sec_mut.auth = *b;
                    }
                    Some(Value::Object(auth_obj)) => {
                        if let Some(Value::Bool(b)) = auth_obj.get("enabled") {
                            sec_mut.auth = *b;
                        }
                        // ignore api_key if present (removed from config)
                    }
                    _ => {}
                }

                // allowed_hosts as array of strings
                if let Some(Value::Array(arr)) = sec_obj.get("allowed_hosts") {
                    let hosts: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    sec_mut.allowed_hosts = hosts;
                }
            }
        })
        .await?;

    // Post-save: detect tray visibility change and server restarts
    let config = SERVICE_MANAGER.get_config().await;

    let tray_new = config
        .settings
        .as_ref()
        .and_then(|s| s.system_tray.as_ref())
        .and_then(|t| t.enabled)
        .unwrap_or(true);
    let tray_changed = tray_old != tray_new;

    let server_config_changed = prev_config.server.host != config.server.host
        || prev_config.server.port != config.server.port
        || prev_config.server.max_connections != config.server.max_connections
        || prev_config.server.timeout_seconds != config.server.timeout_seconds;

    if server_config_changed {
        tracing::info!("Server configuration changed (restarting aggregator)...");
        AGGREGATOR.trigger_shutdown().await;
        if tray_changed {
            tracing::info!(
                "System tray configuration changed during server restart, enabled: {}",
                tray_new
            );

            if tray_new {
                if let Some(tray) = app.tray_by_id("main_tray") {
                    let _ = tray.set_visible(true);
                    tracing::info!("Tray visibility updated: visible");
                } else {
                    // Rebuild tray if it was not created at startup
                    if let Err(e) = build_main_tray(&app) {
                        tracing::error!("Failed to rebuild system tray: {}", e);
                    } else {
                        tracing::info!("System tray rebuilt and made visible");
                    }
                }
            } else {
                if let Some(tray) = app.tray_by_id("main_tray") {
                    let _ = tray.set_visible(false);
                }
                tracing::info!("Tray icon hidden after aggregator restart");
            }

            // 立即保存配置到文件以确保变更持久化
            if let Err(e) = config.save() {
                tracing::error!("Failed to save tray configuration to file: {}", e);
            } else {
                tracing::info!("Tray configuration saved to file");
            }
        }
        Ok(format!(
            "Settings saved successfully. Aggregator restarted on {}:{}",
            config.server.host, config.server.port
        ))
    } else {
        if tray_changed {
            tracing::info!("System tray configuration changed, enabled: {}", tray_new);

            if tray_new {
                if let Some(tray) = app.tray_by_id("main_tray") {
                    let _ = tray.set_visible(true);
                    tracing::info!("Tray visibility updated: visible");
                } else {
                    // Rebuild tray if it was not created at startup
                    if let Err(e) = build_main_tray(&app) {
                        tracing::error!("Failed to rebuild system tray: {}", e);
                    } else {
                        tracing::info!("System tray rebuilt and made visible");
                    }
                }
            } else {
                if let Some(tray) = app.tray_by_id("main_tray") {
                    let _ = tray.set_visible(false);
                }
                tracing::info!("Tray icon hidden");
            }

            // 立即保存配置到文件以确保变更持久化
            if let Err(e) = config.save() {
                tracing::error!("Failed to save tray configuration to file: {}", e);
            } else {
                tracing::info!("Tray configuration saved to file");
            }
        }
        Ok("Settings saved successfully".to_string())
    }
}

// Autostart Management Commands
#[tauri::command(rename_all = "snake_case")]
async fn is_autostart_enabled(app: tauri::AppHandle) -> Result<bool> {
    // Use the autostart plugin to check if autostart is enabled
    use tauri_plugin_autostart::ManagerExt;

    match app.autolaunch().is_enabled() {
        Ok(enabled) => Ok(enabled),
        Err(e) => {
            tracing::error!("Failed to check autostart status: {}", e);
            Err(McpError::ConfigError(format!(
                "Failed to check autostart status: {}",
                e
            )))
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn toggle_autostart(app: tauri::AppHandle) -> Result<String> {
    use tauri_plugin_autostart::ManagerExt;

    // Get current autostart status
    let current_enabled = app.autolaunch().is_enabled().unwrap_or(false);

    // Toggle the autostart status based on current state
    if current_enabled {
        // It was enabled, so disable it
        match app.autolaunch().disable() {
            Ok(_) => {
                tracing::info!("Autostart disabled successfully");
                Ok("自动启动已禁用".to_string())
            }
            Err(e) => {
                tracing::error!("Failed to disable autostart: {}", e);
                Err(McpError::ConfigError(format!(
                    "Failed to disable autostart: {}",
                    e
                )))
            }
        }
    } else {
        // It was disabled, so enable it
        match app.autolaunch().enable() {
            Ok(_) => {
                tracing::info!("Autostart enabled successfully");
                Ok("自动启动已启用".to_string())
            }
            Err(e) => {
                tracing::error!("Failed to enable autostart: {}", e);
                Err(McpError::ConfigError(format!(
                    "Failed to enable autostart: {}",
                    e
                )))
            }
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn get_local_ip_addresses() -> Result<Vec<String>> {
    use if_addrs::get_if_addrs;

    let mut ips = Vec::new();

    // Add localhost
    ips.push("localhost".to_string());
    ips.push("127.0.0.1".to_string());

    // Add 0.0.0.0 for all interfaces
    ips.push("0.0.0.0".to_string());

    // Get all network interfaces
    if let Ok(interfaces) = get_if_addrs() {
        for iface in interfaces {
            // Only include IPv4 addresses that are not loopback
            if iface.ip().is_ipv4() && !iface.ip().is_loopback() {
                let ip_str = iface.ip().to_string();
                if !ips.contains(&ip_str) {
                    ips.push(ip_str);
                }
            }
        }
    }

    Ok(ips)
}

// API Key Management Commands

/// Helper function: 从工具级别权限推导出授权的服务器列表
async fn get_allowed_servers_from_tools(api_key_id: &str) -> Result<Vec<String>> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;
    use std::collections::HashSet;

    // 获取所有授权的工具 ID
    let tool_ids = ApiKeyToolRepository::get_tools_by_api_key(api_key_id).await?;

    // 收集所有不重复的 Server
    use sqlx::Row;
    let mut server_ids = HashSet::<String>::new();
    for tool_id in tool_ids {
        // 从 mcp_tools 表查询工具信息以获取 server_id
        if let Ok(rows) = sqlx::query("SELECT server_id FROM mcp_tools WHERE id = ?")
            .bind(&tool_id)
            .fetch_all(&crate::db::get_database().await?)
            .await
        {
            for row in rows {
                if let Ok(server_id) = row.try_get::<String, _>("server_id") {
                    server_ids.insert(server_id);
                }
            }
        }
    }

    // 将 Server ID 转换为 Server 名称
    let mut allowed_servers = Vec::new();
    for server_id in server_ids {
        if let Ok(Some(server)) = McpServerRepository::get_by_id(&server_id).await {
            allowed_servers.push(server.name);
        }
    }

    Ok(allowed_servers)
}

#[tauri::command(rename_all = "snake_case")]
async fn create_api_key(name: String, permissions: ApiKeyPermissions) -> Result<ApiKey> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // Generate a new API key
    let key = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_chars: String = (0..32)
            .map(|_| {
                const CHARSET: &[u8] =
                    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        format!("sk-{}", random_chars)
    };

    // Create API key in database
    let api_key_row = ApiKeyRepository::create(name.clone(), key.clone()).await?;

    // Add tool-level permissions (批量授权 Server 的所有工具)
    for server_name in &permissions.allowed_servers {
        // Get server ID from name
        if let Some(server_row) = McpServerRepository::get_by_name(server_name).await? {
            if let Some(server_id) = server_row.id {
                ApiKeyToolRepository::grant_server_tools(&api_key_row.id, &server_id).await?;
            }
        }
    }

    tracing::info!("Created new API key: {}", api_key_row.name);

    // Return API key in the expected format
    Ok(ApiKey {
        id: api_key_row.id,
        name: api_key_row.name,
        key, // Return the actual key (only time it's shown)
        enabled: api_key_row.enabled,
        created_at: api_key_row.created_at.to_rfc3339(),
        permissions,
    })
}

#[tauri::command(rename_all = "snake_case")]
async fn list_api_keys() -> Result<Vec<serde_json::Value>> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;

    let api_keys = ApiKeyRepository::get_all().await?;

    let mut masked_keys = Vec::new();
    for api_key in api_keys {
        // 从工具权限推导出授权的服务器列表
        let allowed_servers = get_allowed_servers_from_tools(&api_key.id).await?;

        // Mask the key (show first 6 and last 3 characters)
        let masked_key = if api_key.key_hash.len() > 9 {
            format!(
                "sk-{}...{}",
                &api_key.key_hash[..6],
                &api_key.key_hash[api_key.key_hash.len() - 3..]
            )
        } else {
            "sk-****".to_string()
        };

        masked_keys.push(serde_json::json!({
            "id": api_key.id,
            "name": api_key.name,
            "key": masked_key,
            "enabled": api_key.enabled,
            "created_at": api_key.created_at.to_rfc3339(),
            "permissions": {
                "allowed_servers": allowed_servers
            },
        }));
    }

    Ok(masked_keys)
}

#[tauri::command(rename_all = "snake_case")]
async fn get_api_key_details(id: String) -> Result<ApiKey> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;

    let api_key_row = ApiKeyRepository::get_by_id(&id)
        .await?
        .ok_or_else(|| McpError::ConfigError(format!("API key not found: {}", id)))?;

    // 获取已授权的工具 ID 列表
    let tool_ids = ApiKeyToolRepository::get_tools_by_api_key(&id).await?;
    // 通过工具权限推导出服务器名称列表
    let allowed_servers = get_allowed_servers_from_tools(&id).await?;

    Ok(ApiKey {
        id: api_key_row.id,
        name: api_key_row.name,
        key: "***".to_string(),
        enabled: api_key_row.enabled,
        created_at: api_key_row.created_at.to_rfc3339(),
        permissions: ApiKeyPermissions {
            allowed_servers,
            allowed_tools: tool_ids,
        },
    })
}

#[tauri::command(rename_all = "snake_case")]
async fn delete_api_key(id: String) -> Result<String> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;

    // Remove all tool permissions first
    ApiKeyToolRepository::remove_all_permissions(&id).await?;

    // Delete the API key
    let deleted = ApiKeyRepository::delete(&id).await?;

    if !deleted {
        return Err(McpError::ConfigError(format!("API key not found: {}", id)));
    }

    tracing::info!("Deleted API key: {}", id);
    Ok(format!("API key '{}' has been deleted", id))
}

#[tauri::command(rename_all = "snake_case")]
async fn toggle_api_key(id: String) -> Result<bool> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;

    // Get current state
    let api_key = ApiKeyRepository::get_by_id(&id)
        .await?
        .ok_or_else(|| McpError::ConfigError(format!("API key not found: {}", id)))?;

    let new_state = !api_key.enabled;

    // Toggle the state
    ApiKeyRepository::toggle_enabled(&id, new_state).await?;

    tracing::info!(
        "Toggled API key '{}' to {}",
        id,
        if new_state { "enabled" } else { "disabled" }
    );
    Ok(new_state)
}

#[tauri::command(rename_all = "snake_case")]
async fn update_api_key_permissions(id: String, permissions: ApiKeyPermissions) -> Result<String> {
    use crate::db::repositories::api_key_repository::ApiKeyRepository;
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // Verify API key exists
    ApiKeyRepository::get_by_id(&id)
        .await?
        .ok_or_else(|| McpError::ConfigError(format!("API key not found: {}", id)))?;

    // Remove all existing tool permissions
    ApiKeyToolRepository::remove_all_permissions(&id).await?;

    // Add new permissions (tool-level)
    let mut total_granted = 0;

    // Handle allowed_tools if provided
    if !permissions.allowed_tools.is_empty() {
        for tool_id in &permissions.allowed_tools {
            if let Ok(_) = ApiKeyToolRepository::add_tool_permission(&id, tool_id).await {
                total_granted += 1;
            }
        }
    } else {
        // Fallback to server-level permissions for backward compatibility
        for server_name in &permissions.allowed_servers {
            // Get server ID from name
            if let Some(server_row) = McpServerRepository::get_by_name(server_name).await? {
                if let Some(server_id) = server_row.id {
                    // 批量授权该 Server 的所有工具
                    let granted_count =
                        ApiKeyToolRepository::grant_server_tools(&id, &server_id).await?;
                    total_granted += granted_count;
                }
            }
        }
    }

    tracing::info!(
        "Updated permissions for API key: {}, granted {} tools",
        id,
        total_granted
    );
    Ok(format!(
        "Permissions updated for API key '{}': granted {} tools",
        id, total_granted
    ))
}

// Tool-level Permission Management Commands

#[tauri::command(rename_all = "snake_case")]
async fn get_api_key_tools(api_key_id: String) -> Result<Vec<String>> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;

    let tool_ids = ApiKeyToolRepository::get_tools_by_api_key(&api_key_id).await?;
    Ok(tool_ids)
}

#[tauri::command(rename_all = "snake_case")]
async fn add_tool_permission(api_key_id: String, tool_id: String) -> Result<String> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;

    ApiKeyToolRepository::add_tool_permission(&api_key_id, &tool_id).await?;
    tracing::info!("Added tool permission: {} -> {}", api_key_id, tool_id);
    Ok(format!("Tool permission added"))
}

#[tauri::command(rename_all = "snake_case")]
async fn remove_tool_permission(api_key_id: String, tool_id: String) -> Result<String> {
    // First check if the tool exists
    let db = crate::db::get_database().await?;
    let tool_row = sqlx::query("SELECT id FROM mcp_tools WHERE id = ?")
        .bind(&tool_id)
        .fetch_optional(&db)
        .await
        .map_err(McpError::from)?;

    if tool_row.is_none() {
        return Err(McpError::ConfigError(format!(
            "Tool not found: {}",
            tool_id
        )));
    }

    // Remove the permission by deleting the relation
    let result =
        sqlx::query("DELETE FROM api_key_tool_relations WHERE api_key_id = ? AND tool_id = ?")
            .bind(&api_key_id)
            .bind(&tool_id)
            .execute(&db)
            .await
            .map_err(McpError::from)?;

    if result.rows_affected() > 0 {
        tracing::info!("Removed tool permission: {} -> {}", api_key_id, tool_id);
        Ok(format!("Tool permission removed"))
    } else {
        Err(McpError::ConfigError(format!(
            "Permission not found for tool: {}",
            tool_id
        )))
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn grant_server_tools_to_api_key(api_key_id: String, server_name: String) -> Result<String> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // Get server ID from name
    let server = McpServerRepository::get_by_name(&server_name)
        .await?
        .ok_or_else(|| McpError::ServiceNotFound(server_name.clone()))?;

    let server_id = server
        .id
        .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?;

    // Grant all tools in this server
    let granted_count = ApiKeyToolRepository::grant_server_tools(&api_key_id, &server_id).await?;

    tracing::info!(
        "Granted {} tools from server {} to API key {}",
        granted_count,
        server_name,
        api_key_id
    );
    Ok(format!(
        "Granted {} tools from server '{}'",
        granted_count, server_name
    ))
}

#[tauri::command(rename_all = "snake_case")]
async fn revoke_server_tools_from_api_key(
    api_key_id: String,
    server_name: String,
) -> Result<String> {
    use crate::db::repositories::api_key_tool_repository::ApiKeyToolRepository;
    use crate::db::repositories::mcp_server_repository::McpServerRepository;

    // Get server ID from name
    let server = McpServerRepository::get_by_name(&server_name)
        .await?
        .ok_or_else(|| McpError::ServiceNotFound(server_name.clone()))?;

    let server_id = server
        .id
        .ok_or_else(|| McpError::ConfigError("Server ID not found".to_string()))?;

    // Revoke all tools in this server
    let revoked_count = ApiKeyToolRepository::revoke_server_tools(&api_key_id, &server_id).await?;

    tracing::info!(
        "Revoked {} tools from server {} for API key {}",
        revoked_count,
        server_name,
        api_key_id
    );
    Ok(format!(
        "Revoked {} tools from server '{}'",
        revoked_count, server_name
    ))
}

fn build_main_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    // Load configuration to initialize theme menu state
    let config = config::AppConfig::load().unwrap_or_default();

    // Create theme menu items so we can mutate their checked state later
    let theme_auto_item = tauri::menu::CheckMenuItemBuilder::new("自动（跟随系统）")
        .id("theme_auto")
        .checked(true)
        .build(app)?;
    let theme_light_item = tauri::menu::CheckMenuItemBuilder::new("亮色")
        .id("theme_light")
        .build(app)?;
    let theme_dark_item = tauri::menu::CheckMenuItemBuilder::new("暗色")
        .id("theme_dark")
        .build(app)?;

    // Build tray menu
    let menu = tauri::menu::MenuBuilder::new(app)
        .item(
            &tauri::menu::MenuItemBuilder::new("显示主窗口")
                .id("show_window")
                .accelerator("CmdOrCtrl+Shift+M")
                .build(app)?,
        )
        .item(&tauri::menu::PredefinedMenuItem::separator(app)?)
        .item(
            &tauri::menu::MenuItemBuilder::new("服务器管理")
                .id("server_management")
                .build(app)?,
        )
        .item(
            &tauri::menu::MenuItemBuilder::new("市场")
                .id("marketplace")
                .build(app)?,
        )
        .item(
            &tauri::menu::MenuItemBuilder::new("设置")
                .id("settings")
                .build(app)?,
        )
        .item(&tauri::menu::PredefinedMenuItem::separator(app)?)
        .item(
            &tauri::menu::SubmenuBuilder::new(app, "主题")
                .item(&theme_auto_item)
                .item(&theme_light_item)
                .item(&theme_dark_item)
                .build()?,
        )
        .item(&tauri::menu::PredefinedMenuItem::separator(app)?)
        .item(
            &tauri::menu::MenuItemBuilder::new("关于 MCP Router")
                .id("about")
                .build(app)?,
        )
        .item(&tauri::menu::PredefinedMenuItem::separator(app)?)
        .item(
            &tauri::menu::MenuItemBuilder::new("退出")
                .id("quit")
                .accelerator("CmdOrCtrl+Q")
                .build(app)?,
        )
        .build()?;

    // Clone theme items for use inside the event closure
    let theme_auto_item_event = theme_auto_item.clone();
    let theme_light_item_event = theme_light_item.clone();
    let theme_dark_item_event = theme_dark_item.clone();

    let _tray = TrayIconBuilder::<_>::with_id("main_tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("MCP Router")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show_window" => {
                let windows = app.webview_windows();
                if let Some(window) = windows.values().next() {
                    let _ = window.set_focus();
                    let _ = window.show();
                    let _ = window.unminimize();
                }
            }
            "server_management" => {
                let _ = app.emit("navigate-to", "servers");
                let windows = app.webview_windows();
                if let Some(window) = windows.values().next() {
                    let _ = window.set_focus();
                    let _ = window.show();
                    let _ = window.unminimize();
                }
            }
            "marketplace" => {
                let _ = app.emit("navigate-to", "marketplace");
                let windows = app.webview_windows();
                if let Some(window) = windows.values().next() {
                    let _ = window.set_focus();
                    let _ = window.show();
                    let _ = window.unminimize();
                }
            }
            "settings" => {
                let _ = app.emit("navigate-to", "settings");
                let windows = app.webview_windows();
                if let Some(window) = windows.values().next() {
                    let _ = window.set_focus();
                    let _ = window.show();
                    let _ = window.unminimize();
                }
            }
            "about" => {
                let _ = app.emit("show-about-dialog", ());
                let windows = app.webview_windows();
                if let Some(window) = windows.values().next() {
                    let _ = window.set_focus();
                    let _ = window.show();
                    let _ = window.unminimize();
                }
            }
            "theme_auto" | "theme_light" | "theme_dark" => {
                let theme = if event.id.as_ref() == "theme_auto" {
                    "auto"
                } else if event.id.as_ref() == "theme_light" {
                    "light"
                } else {
                    "dark"
                };

                match theme {
                    "auto" => {
                        let _ = theme_auto_item_event.set_checked(true);
                        let _ = theme_light_item_event.set_checked(false);
                        let _ = theme_dark_item_event.set_checked(false);
                    }
                    "light" => {
                        let _ = theme_auto_item_event.set_checked(false);
                        let _ = theme_light_item_event.set_checked(true);
                        let _ = theme_dark_item_event.set_checked(false);
                    }
                    "dark" => {
                        let _ = theme_auto_item_event.set_checked(false);
                        let _ = theme_light_item_event.set_checked(false);
                        let _ = theme_dark_item_event.set_checked(true);
                    }
                    _ => {}
                }

                tokio::spawn(async move {
                    let mut config = SERVICE_MANAGER.get_config().await;
                    if config.settings.is_none() {
                        config.settings = Some(config::Settings {
                            theme: None,
                            autostart: Some(false),
                            system_tray: Some(config::SystemTraySettings {
                                enabled: Some(true),
                                close_to_tray: Some(false),
                                start_to_tray: Some(false),
                            }),
                            uv_index_url: None,
                            npm_registry: None,
                        });
                    }
                    if let Some(settings) = config.settings.as_mut() {
                        settings.theme = Some(theme.to_string());
                    }
                    let _ = config.save();
                });

                let _ = app.emit("theme-changed", theme);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    let theme = config
        .settings
        .as_ref()
        .and_then(|s| s.theme.as_ref())
        .cloned()
        .unwrap_or_else(|| "auto".to_string());
    match theme.as_str() {
        "light" => {
            let _ = theme_auto_item.set_checked(false);
            let _ = theme_light_item.set_checked(true);
            let _ = theme_dark_item.set_checked(false);
        }
        "dark" => {
            let _ = theme_auto_item.set_checked(false);
            let _ = theme_light_item.set_checked(false);
            let _ = theme_dark_item.set_checked(true);
        }
        _ => {
            let _ = theme_auto_item.set_checked(true);
            let _ = theme_light_item.set_checked(false);
            let _ = theme_dark_item.set_checked(false);
        }
    }

    tracing::info!("System tray initialized successfully");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // 1) Load configuration FIRST, fail fast on error
    let config = config::AppConfig::load().unwrap_or_else(|e| {
        eprintln!(
            "Failed to load configuration file: {}\n\
            The application cannot start with an invalid configuration.\n\
            Please fix the config file at: ~/.mcprouter/config.json\n\
            Or delete it to use default settings.",
            e
        );
        std::process::exit(1);
    });

    // 2) Prepare log plugin from config BEFORE any other operations
    let (log_level, log_file_name) = if let Some(ref logging) = config.logging {
        let level = match logging.level.to_lowercase().as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Info,
        };

        let file_name = logging
            .file_name
            .as_ref()
            .filter(|name| !name.is_empty())
            .cloned();

        (level, file_name)
    } else {
        (log::LevelFilter::Info, None)
    };

    // Convert log::LevelFilter to tracing::level_filters::LevelFilter for tracing_subscriber
    let tracing_level = match log_level {
        log::LevelFilter::Off => tracing::level_filters::LevelFilter::OFF,
        log::LevelFilter::Error => tracing::level_filters::LevelFilter::ERROR,
        log::LevelFilter::Warn => tracing::level_filters::LevelFilter::WARN,
        log::LevelFilter::Info => tracing::level_filters::LevelFilter::INFO,
        log::LevelFilter::Debug => tracing::level_filters::LevelFilter::DEBUG,
        log::LevelFilter::Trace => tracing::level_filters::LevelFilter::TRACE,
    };

    let mut log_builder = tauri_plugin_log::Builder::new().level(log_level);

    // Always write logs to log dir (file name from config, or default)
    log_builder = log_builder.target(tauri_plugin_log::Target::new(
        tauri_plugin_log::TargetKind::LogDir {
            file_name: log_file_name.clone(), // Clone for the plugin
        },
    ));

    tauri::Builder::default()
        .plugin(log_builder.build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(move |app| {
            // Get log directory path (same as tauri-plugin-log uses)
            let log_dir = app.path().app_log_dir().expect("Failed to get log directory");
            std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");

            // Create file appender for the log file
            let file_name = log_file_name.unwrap_or_else(|| "mcprouter.log".to_string());
            let file_appender = tracing_appender::rolling::never(&log_dir, file_name);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            // Initialize tracing subscriber with both stdout and file output
            use tracing_subscriber::layer::SubscriberExt;
            use tracing_subscriber::Layer;
            use tracing_subscriber::fmt;

            let subscriber = tracing_subscriber::registry()
                .with(fmt::layer().with_filter(tracing_level)) // Log to stdout/stderr with level filter
                .with(fmt::layer().with_writer(non_blocking).with_ansi(false).with_filter(tracing_level)); // Log to file without ANSI codes with level filter

            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set tracing subscriber");

            // Keep the guard alive for the entire application lifetime
            // This ensures the non-blocking worker continues running
            std::mem::forget(guard);

            // Log after subscriber is initialized
            tracing::info!("Starting MCP Router");

            // 3) Start aggregator AFTER logging is initialized
            let aggregator_clone = AGGREGATOR.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = aggregator_clone.start().await {
                    tracing::error!(
                        "Failed to start MCP aggregator server: {}\n\
                        The application cannot continue without the MCP aggregator service.\n\
                        Please check if the port is already in use or if there are permission issues.",
                        e
                    );
                    std::process::exit(1);
                }
            });

            // Store aggregator handle asynchronously
            tokio::spawn(async move {
                let mut handle_guard = AGGREGATOR_HANDLE.lock().await;
                *handle_guard = Some(handle);
            });

            // 4) Initialize database, then load MCP services sequentially to avoid race conditions
            let app_handle = app.handle().clone();
            tokio::spawn(async move {
                match db::initialize_database(&app_handle).await {
                    Ok(_) => {
                        tracing::info!("Database initialized successfully");
                        match SERVICE_MANAGER.load_mcp_servers().await {
                            Ok(_) => {
                                tracing::info!("MCP services loaded");
                            }
                            Err(e) => {
                                tracing::error!("Failed to load services: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize database: {}", e);
                        tracing::error!("The application may not function correctly without database access.");
                    }
                }
            });

            // Tray helper moved to module scope (build_main_tray)

            // Ensure tray visibility based on config at startup
            let tray_enabled_start = config
                .settings
                .as_ref()
                .and_then(|s| s.system_tray.as_ref())
                .and_then(|t| t.enabled)
                .unwrap_or(true);
            if tray_enabled_start {
                if app.tray_by_id("main_tray").is_none() {
                    let _ = build_main_tray(&app.handle());
                } else if let Some(tray) = app.tray_by_id("main_tray") {
                    let _ = tray.set_visible(true);
                }
            } else if let Some(tray) = app.tray_by_id("main_tray") {
                let _ = tray.set_visible(false);
            }

            // Configure window to minimize to tray on close (runtime-checked)
            if let Some(main_window) = app.get_webview_window("main") {
                let window_clone = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Load latest config synchronously
                        let cfg = config::AppConfig::load().ok();
                        let tray_enabled = cfg
                            .as_ref()
                            .and_then(|c| c.settings.as_ref())
                            .and_then(|s| s.system_tray.as_ref())
                            .and_then(|t| t.enabled)
                            .unwrap_or(true);
                        let minimize_on_close = cfg
                            .as_ref()
                            .and_then(|c| c.settings.as_ref())
                            .and_then(|s| s.system_tray.as_ref())
                            .and_then(|t| t.close_to_tray)
                            .unwrap_or(false);

                        if tray_enabled && minimize_on_close {
                            // Prevent the window from closing and hide instead
                            api.prevent_close();
                            let _ = window_clone.hide();
                            tracing::info!("Window minimized to tray (runtime config)");
                        }
                    }
                });

                // Minimize to tray on start
                let should_minimize_on_start =
                    config
                        .settings
                        .as_ref()
                        .and_then(|s| s.system_tray.as_ref())
                        .and_then(|t| t.start_to_tray)
                        .unwrap_or(false)
                        && tray_enabled_start;
                if should_minimize_on_start {
                    let _ = main_window.hide();
                    tracing::info!("Window hidden on startup due to configuration");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_theme,
            set_theme,
            update_config,
            add_mcp_server,
            remove_mcp_server,
            check_mcp_server_connectivity,
            toggle_mcp_server,
            list_mcp_servers,
            list_marketplace_services,
            get_mcp_server_details,
            install_marketplace_service,
            // MCP Client Commands
            connect_to_mcp_server,
            disconnect_from_mcp_server,
            list_mcp_server_tools,
            call_mcp_tool,
            get_mcp_server_info,
            list_mcp_connections,
            // Enhanced Service Management
            delete_mcp_server,
            // Tool DB Commands
            get_tools_by_server,
            // Legacy Commands
            toggle_mcp_server_tool,
            enable_all_mcp_server_tools,
            disable_all_mcp_server_tools,
            get_settings,
            save_settings,
            get_dashboard_stats,
            get_local_ip_addresses,
            is_autostart_enabled,
            toggle_autostart,
            // API Key Management Commands
            create_api_key,
            list_api_keys,
            get_api_key_details,
            delete_api_key,
            toggle_api_key,
            update_api_key_permissions,
            // Tool-level Permission Management
            get_api_key_tools,
            add_tool_permission,
            remove_tool_permission,
            grant_server_tools_to_api_key,
            revoke_server_tools_from_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
