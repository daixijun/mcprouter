// Dashboard Data Commands

use crate::config;
use crate::error::{McpError, Result};
use crate::{AGGREGATOR, MCP_CLIENT_MANAGER, SERVICE_MANAGER, STARTUP_TIME};
use std::time::UNIX_EPOCH;

#[tauri::command(rename_all = "snake_case")]
pub async fn get_dashboard_stats(app_handle: tauri::AppHandle) -> Result<serde_json::Value> {
    // Get actual service statistics
    let services = SERVICE_MANAGER.list_mcp_servers(Some(&app_handle)).await?;
    let enabled_services = services.iter().filter(|s| s.enabled).count();
    let disabled_services = services.len() - enabled_services;

    // Calculate healthy services count (connected services)
    let healthy_services = services
        .iter()
        .filter(|s| s.enabled && (s.status == "connected" || s.status == "running"))
        .count();

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
    // 获取聚合器的克隆，避免跨越 await 点持有锁
    let aggregator_clone = {
        let aggregator_guard = AGGREGATOR.lock().unwrap();
        (*aggregator_guard).clone()
    };

    let aggregator_stats = if let Some(ref aggregator) = aggregator_clone {
        aggregator.get_statistics().await
    } else {
        serde_json::json!({
            "status": "not_initialized",
            "message": "Aggregator not initialized"
        })
    };
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
        "healthy_services": healthy_services,
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

#[tauri::command(rename_all = "snake_case")]
pub async fn get_local_ip_addresses() -> Result<Vec<String>> {
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
