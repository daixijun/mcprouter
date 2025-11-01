// 系统设置命令

use crate::config as config_mod;
use crate::error::{McpError, Result};
use crate::{build_main_tray, AGGREGATOR, SERVICE_MANAGER};

#[tauri::command(rename_all = "snake_case")]
pub fn get_settings() -> Result<serde_json::Value> {
    // Load configuration
    let config = config_mod::AppConfig::load()
        .map_err(|e| McpError::ConfigError(format!("Failed to load settings: {}", e)))?;

    // Convert AppConfig to serde_json::Value
    serde_json::to_value(config)
        .map_err(|e| McpError::ConfigError(format!("Failed to convert settings to JSON: {}", e)))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_settings(app: tauri::AppHandle, settings: serde_json::Value) -> Result<String> {
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

                    // 如果系统托盘被禁用,自动禁用"关闭到托盘"功能
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

            // Logging config (support top-level payload; level string and file_name string)
            if let Some(Value::Object(logging_obj)) = settings.get("logging") {
                // Ensure logging exists
                if config.logging.is_none() {
                    config.logging = Some(config_mod::LoggingSettings {
                        level: "info".to_string(),
                        file_name: Some("mcprouter.log".to_string()),
                    });
                }
                let logging_mut = config.logging.as_mut().unwrap();

                // level as string
                if let Some(Value::String(level)) = logging_obj.get("level") {
                    logging_mut.level = level.clone();
                }

                // file_name as string (optional)
                if let Some(Value::String(file_name)) = logging_obj.get("file_name") {
                    logging_mut.file_name = Some(file_name.clone());
                } else if let Some(Value::Null) = logging_obj.get("file_name") {
                    logging_mut.file_name = None;
                }
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

#[tauri::command(rename_all = "snake_case")]
pub async fn is_autostart_enabled(app: tauri::AppHandle) -> Result<bool> {
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
pub async fn toggle_autostart(app: tauri::AppHandle) -> Result<String> {
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
