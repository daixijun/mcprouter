// Configuration Management Commands

use crate::config as config_mod;
use crate::error::Result;
use crate::SERVICE_MANAGER;
use tauri::Emitter;

#[tauri::command]
pub async fn get_config() -> Result<config_mod::AppConfig> {
    // TODO: Implement config retrieval from database storage
    // For now, return default config
    Ok(config_mod::AppConfig::default())
}

#[tauri::command]
pub async fn get_theme() -> Result<String> {
    let config = config_mod::AppConfig::default();
    let theme = config
        .settings
        .as_ref()
        .and_then(|s| s.theme.as_ref())
        .cloned()
        .unwrap_or_else(|| "auto".to_string());
    Ok(theme)
}

#[tauri::command]
pub async fn set_theme(app: tauri::AppHandle, theme: String) -> Result<()> {
    // Update tray menu to reflect new theme (safe method)
    if let Err(e) = crate::update_tray_menu(&app) {
        tracing::error!("Failed to update tray menu after theme change: {}", e);
    }

    // Emit event to notify frontend
    let _ = app.emit("theme-changed", theme);
    Ok(())
}

#[tauri::command]
pub async fn update_config(config: config_mod::AppConfig) -> Result<String> {
    config.save()?;
    Ok("Config updated".to_string())
}

#[tauri::command]
pub async fn import_mcp_servers_config(
    config_json: serde_json::Value,
) -> Result<String> {
    // Extract mcpServers object from config
    if let Some(mcp_servers) = config_json.get("mcpServers").and_then(|v| v.as_object()) {
        let mut added_servers = Vec::new();

        for (service_name, service_config) in mcp_servers {
            if let Some(service_obj) = service_config.as_object() {
                // Determine transport type based on available fields
                let transport =
                    // Priority 1: If url field exists, it's HTTP type
                    if service_obj.contains_key("url") {
                        crate::types::ServiceTransport::Http
                    }
                    // Priority 2: If command field exists, it's STDIO type
                    else if service_obj.contains_key("command") {
                        crate::types::ServiceTransport::Stdio
                    }
                    // Priority 3: Check explicit type field (for special cases)
                    else if let Some(transport_str) = service_obj.get("type").and_then(|v| v.as_str()) {
                        match transport_str.to_lowercase().as_str() {
                            "http" => crate::types::ServiceTransport::Http,
                            "stdio" => crate::types::ServiceTransport::Stdio,
                            "sse" => {
                                tracing::warn!("SSE transport is deprecated, falling back to HTTP");
                                crate::types::ServiceTransport::Http
                            },
                            _ => {
                                tracing::warn!("Unknown transport type '{}', falling back to STDIO", transport_str);
                                crate::types::ServiceTransport::Stdio
                            }
                        }
                    }
                    // Priority 4: Default to STDIO
                    else {
                        tracing::warn!("No transport information found, defaulting to STDIO");
                        crate::types::ServiceTransport::Stdio
                    };

                // Extract service configuration
                let command = service_obj
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let args = service_obj
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    });
                let url = service_obj
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let description = service_obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Extract environment variables
                let env = service_obj
                    .get("env")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|val| (k.clone(), val.to_string())))
                            .collect::<std::collections::HashMap<String, String>>()
                    });

                // Extract headers
                let headers = service_obj
                    .get("headers")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|val| (k.clone(), val.to_string())))
                            .collect::<std::collections::HashMap<String, String>>()
                    });

                // Create service configuration
                let service_config = crate::types::McpServerConfig {
                    name: service_name.clone(),
                    description,
                    command,
                    args,
                    transport,
                    url,
                    enabled: true,
                    env,
                    headers,
                };

                // Add service
                {
                    let service_manager = {
                        let guard = SERVICE_MANAGER.lock().unwrap();
                        guard.as_ref().unwrap().clone()
                    };
                    match service_manager.add_server(&service_config).await {
                        Ok(()) => added_servers.push(service_name.clone()),
                        Err(e) => {
                            tracing::error!("Failed to import service '{}': {}", service_name, e);
                            // Continue with other services even if one fails
                        }
                    }
                }
            }
        }

        if added_servers.is_empty() {
            return Err(crate::error::McpError::InvalidConfiguration(
                "No valid services found in configuration".to_string(),
            ));
        }

        Ok(format!(
            "Successfully imported {} MCP server(s): {}",
            added_servers.len(),
            added_servers.join(", ")
        ))
    } else {
        Err(crate::error::McpError::InvalidConfiguration(
            "Invalid configuration format. Expected 'mcpServers' object.".to_string(),
        ))
    }
}

