// Dashboard Data Commands

use crate::config;
use crate::error::{McpError, Result};
use crate::{AGGREGATOR, MCP_CLIENT_MANAGER, SERVICE_MANAGER, STARTUP_TIME};
use serde::{Deserialize, Serialize};
use std::time::UNIX_EPOCH;

/// Aggregator status enumeration
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregatorStatus {
    Running,
    Stopped,
    Error,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_dashboard_stats(_app_handle: tauri::AppHandle) -> Result<serde_json::Value> {
    // Get actual service statistics
    let service_manager = {
        let guard = SERVICE_MANAGER.lock().unwrap();
        guard.as_ref()
            .ok_or_else(|| McpError::Internal("SERVICE_MANAGER not initialized".to_string()))?
            .clone()
    };
    let services = service_manager.list_mcp_servers().await?;
    let enabled_services = services.iter().filter(|s| s.enabled).count();

    // Calculate healthy services count (connected services)
    let healthy_services = services
        .iter()
        .filter(|s| s.enabled && (s.status == "connected" || s.status == "running"))
        .count();

    // Calculate failed services count (enabled but failed to connect)
    let failed_servers = services
        .iter()
        .filter(|s| {
            s.enabled && (s.status == "failed" || s.status == "disconnected" || s.status == "error")
        })
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
    let service_manager = {
        let guard = SERVICE_MANAGER.lock().unwrap();
        guard.as_ref()
            .ok_or_else(|| crate::error::McpError::Internal("SERVICE_MANAGER not initialized".to_string()))?
            .clone()
    };
    let mcp_servers = service_manager.get_mcp_servers().await?;
    let total_services = mcp_servers.len();

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

    // Determine aggregator status
    let aggregator_status = if aggregator_clone.is_some() {
        // Check if aggregator is actually running by looking at its statistics
        match aggregator_stats.get("status").and_then(|v| v.as_str()) {
            Some("running") | Some("active") => AggregatorStatus::Running,
            Some("stopped") | Some("inactive") => AggregatorStatus::Stopped,
            Some("error") | Some("failed") => AggregatorStatus::Error,
            _ => {
                // If no explicit status, infer from connection count
                let connected_services = aggregator_stats
                    .get("connected_services")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if connected_services > 0 {
                    AggregatorStatus::Running
                } else {
                    AggregatorStatus::Stopped
                }
            }
        }
    } else {
        AggregatorStatus::Stopped
    };

    // Calculate total tools count from all services
    let total_tools = services
        .iter()
        .map(|s| s.tool_count.unwrap_or(0) as u32)
        .sum::<u32>();

    // Calculate total resources count from all services
    let total_resources = services
        .iter()
        .map(|s| s.resource_count.unwrap_or(0) as u32)
        .sum::<u32>();

    // Calculate total prompts count from all services
    let total_prompts = services
        .iter()
        .map(|s| s.prompt_count.unwrap_or(0) as u32)
        .sum::<u32>();

    // Calculate total prompt templates count from all services
    // For now, consider all prompts as templates since MCP prompts are inherently templatable
    let total_prompt_templates = services
        .iter()
        .map(|s| s.prompt_template_count.unwrap_or(s.prompt_count.unwrap_or(0)) as u32)
        .sum::<u32>();

    Ok(serde_json::json!({
        "total_servers": total_services,
        "enabled_servers": enabled_services,
        "failed_servers": failed_servers,
        "healthy_services": healthy_services,
        "total_tools": total_tools,
        "total_resources": total_resources,
        "total_prompts": total_prompts,
        "total_prompt_templates": total_prompt_templates,
        "active_clients": connections.len(),
        "startup_time": startup_time,
        "connections": {
            "active_clients": connections.len(),
            "active_services": aggregator_stats.get("active_connections").and_then(|v| v.as_u64()).unwrap_or(0),
        },
        "aggregator": {
            "endpoint": aggregator_endpoint,
            "status": aggregator_status,
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
