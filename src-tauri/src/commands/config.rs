// Configuration Management Commands

use crate::config as config_mod;
use crate::error::Result;
use crate::SERVICE_MANAGER;
use tauri::Emitter;

#[tauri::command]
pub async fn get_config() -> Result<config_mod::AppConfig> {
    Ok(SERVICE_MANAGER.get_config().await)
}

#[tauri::command]
pub async fn get_theme() -> Result<String> {
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
pub async fn set_theme(app: tauri::AppHandle, theme: String) -> Result<()> {
    SERVICE_MANAGER
        .update_config(|config| {
            if config.settings.is_none() {
                config.settings = Some(crate::Settings {
                    language: Some("zh-CN".to_string()),
                    theme: Some("auto".to_string()),
                    autostart: Some(false),
                    system_tray: None,
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

#[tauri::command]
pub async fn update_config(config: config_mod::AppConfig) -> Result<String> {
    config.save()?;
    Ok("Config updated".to_string())
}

#[tauri::command]
pub async fn import_mcp_servers_config(
    app_handle: tauri::AppHandle,
    config_json: serde_json::Value,
) -> Result<String> {
    // Extract mcpServers object from config
    if let Some(mcp_servers) = config_json.get("mcpServers").and_then(|v| v.as_object()) {
        let mut added_servers = Vec::new();

        for (service_name, service_config) in mcp_servers {
            if let Some(service_obj) = service_config.as_object() {
                // Determine transport type based on available fields
                let transport =
                    if let Some(transport_str) = service_obj.get("type").and_then(|v| v.as_str()) {
                        // Explicit transport field takes priority
                        match transport_str {
                            "sse" => crate::types::ServiceTransport::Sse,
                            "http" => crate::types::ServiceTransport::Http,
                            "stdio" => crate::types::ServiceTransport::Stdio,
                            _ => crate::types::ServiceTransport::Stdio, // Default to stdio for unknown values
                        }
                    } else if let Some(url) = service_obj.get("url").and_then(|v| v.as_str()) {
                        // Check URL pattern to determine SSE vs HTTP
                        if url.trim_end_matches('/').ends_with("/sse") {
                            crate::types::ServiceTransport::Sse
                        } else {
                            crate::types::ServiceTransport::Http
                        }
                    } else if service_obj.contains_key("command") {
                        // Default to stdio for command-based services
                        crate::types::ServiceTransport::Stdio
                    } else {
                        // Fallback to stdio if no clear indicators
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
                match SERVICE_MANAGER
                    .add_mcp_server(&app_handle, service_config)
                    .await
                {
                    Ok(()) => added_servers.push(service_name.clone()),
                    Err(e) => {
                        tracing::error!("Failed to import service '{}': {}", service_name, e);
                        // Continue with other services even if one fails
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
