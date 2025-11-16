// System Settings Commands

use crate::aggregator::McpAggregator;
use crate::config as config_mod;
use crate::error::{McpError, Result};
use crate::{
    build_main_tray, types, AGGREGATOR, MCP_CLIENT_MANAGER, SERVICE_MANAGER, TOKEN_MANAGER,
};
use serde::Serialize;
use std::sync::Arc;

#[tauri::command(rename_all = "snake_case")]
pub async fn get_settings(app: tauri::AppHandle) -> Result<serde_json::Value> {
    // Load configuration
    let mut config = config_mod::AppConfig::load()
        .map_err(|e| McpError::ConfigError(format!("Failed to load settings: {}", e)))?;

    // Get actual autostart status from the system
    use tauri_plugin_autostart::ManagerExt;
    match app.autolaunch().is_enabled() {
        Ok(enabled) => {
            if let Some(ref mut settings) = config.settings {
                settings.autostart = Some(enabled);
            } else {
                config.settings = Some(types::Settings {
                    autostart: Some(enabled),
                    ..Default::default()
                });
            }
        }
        Err(e) => {
            tracing::warn!(
                "Failed to check actual autostart status, using config value: {}",
                e
            );
            // Keep the config value if we can't check the system status
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    struct ServerOut {
        host: String,
        port: u16,
        max_connections: usize,
        timeout_seconds: u64,
        #[serde(default)]
        auth: bool,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    struct LoggingOut {
        level: String,
        file_name: Option<String>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    struct TrayOut {
        #[serde(default)]
        enabled: Option<bool>,
        #[serde(default)]
        close_to_tray: Option<bool>,
        #[serde(default)]
        start_to_tray: Option<bool>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    struct SettingsOut {
        #[serde(default)]
        theme: Option<String>,
        #[serde(default)]
        autostart: Option<bool>,
        #[serde(default)]
        system_tray: Option<TrayOut>,
        #[serde(default)]
        uv_index_url: Option<String>,
        #[serde(default)]
        npm_registry: Option<String>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    struct SystemSettingsOut {
        server: ServerOut,
        logging: Option<LoggingOut>,
        settings: Option<SettingsOut>,
    }

    let out = SystemSettingsOut {
        server: ServerOut {
            host: config.server.host,
            port: config.server.port,
            max_connections: config.server.max_connections,
            timeout_seconds: config.server.timeout_seconds,
            auth: config.server.auth,
        },
        logging: config.logging.as_ref().map(|l| LoggingOut {
            level: l.level.clone(),
            file_name: l.file_name.clone(),
        }),
        settings: config.settings.as_ref().map(|s| SettingsOut {
            theme: s.theme.clone(),
            autostart: s.autostart,
            system_tray: s.system_tray.as_ref().map(|t| TrayOut {
                enabled: t.enabled,
                close_to_tray: t.close_to_tray,
                start_to_tray: t.start_to_tray,
            }),
            uv_index_url: s.uv_index_url.clone(),
            npm_registry: s.npm_registry.clone(),
        }),
    };

    serde_json::to_value(out)
        .map_err(|e| McpError::ConfigError(format!("Failed to convert settings to JSON: {}", e)))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_settings(app: tauri::AppHandle, settings: serde_json::Value) -> Result<String> {
    use serde_json::Value;

    // Debug: Log the received settings
    tracing::info!(
        "save_settings called with data: {}",
        serde_json::to_string_pretty(&settings)
            .unwrap_or_else(|_| "Failed to serialize".to_string())
    );

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
                config.settings = Some(crate::Settings {
                    theme: Some("auto".to_string()),
                    autostart: Some(false),
                    system_tray: Some(crate::SystemTraySettings {
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
                    settings_mut.system_tray = Some(crate::SystemTraySettings {
                        enabled: Some(true),
                        close_to_tray: Some(false),
                        start_to_tray: Some(false),
                    });
                }
                let tray_mut = settings_mut.system_tray.as_mut().unwrap();

                // Handle enabled status first
                if let Some(Value::Bool(enabled)) = tray_obj.get("enabled") {
                    tray_mut.enabled = Some(*enabled);

                    // If system tray is disabled, automatically disable "close to tray" feature
                    if !*enabled {
                        tray_mut.close_to_tray = Some(false);
                        tracing::info!(
                            "System tray disabled, automatically disabling close-to-tray feature"
                        );
                    }
                }

                // Only allow setting "close to tray" when system tray is enabled
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
                    config.logging = Some(crate::LoggingSettings {
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
                tracing::info!("Processing server config: {:?}", server_obj);

                if let Some(Value::String(host)) = server_obj.get("host") {
                    config.server.host = host.clone();
                    tracing::info!("Updated host: {}", host);
                }
                if let Some(Value::Number(port)) = server_obj.get("port") {
                    if let Some(p) = port.as_u64() {
                        config.server.port = p as u16;
                        tracing::info!("Updated port: {}", p);
                    }
                }
                if let Some(Value::Number(max_conn)) = server_obj.get("max_connections") {
                    if let Some(mc) = max_conn.as_u64() {
                        config.server.max_connections = mc as usize;
                        tracing::info!("Updated max_connections: {}", mc);
                    }
                }
                if let Some(Value::Number(timeout)) = server_obj.get("timeout_seconds") {
                    if let Some(ts) = timeout.as_u64() {
                        config.server.timeout_seconds = ts;
                        tracing::info!("Updated timeout_seconds: {}", ts);
                    }
                }
                if let Some(Value::Bool(auth)) = server_obj.get("auth") {
                    config.server.auth = *auth;
                    tracing::info!("Updated auth: {}", auth);
                } else {
                    tracing::warn!("auth field not found in server config or not a boolean");
                }
            } else {
                tracing::warn!("No server config found in settings payload");
            }

            // Security settings removed
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
        || prev_config.server.timeout_seconds != config.server.timeout_seconds
        || prev_config.server.auth != config.server.auth;

    if server_config_changed {
        tracing::info!("Server configuration changed (restarting aggregator with new config)...");

        // 停止现有的聚合器
        let aggregator_clone = {
            let aggregator_guard = AGGREGATOR.lock().unwrap();
            (*aggregator_guard).clone()
        };

        if let Some(aggregator) = &aggregator_clone {
            tracing::info!("Shutting down existing aggregator...");
            aggregator.trigger_shutdown().await;
        }

        // 等待一段时间确保完全关闭
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // 从最新配置创建新的聚合器
        let new_config = SERVICE_MANAGER.get_config().await;
        let server_config = Arc::new(new_config.server.clone());

        // 获取 TokenManager
        let token_manager = {
            let token_manager_guard = TOKEN_MANAGER.read().await;
            (*token_manager_guard)
                .as_ref()
                .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
                .clone()
        };

        // 创建新的聚合器实例
        tracing::info!(
            "Creating new aggregator with updated configuration (auth: {})",
            server_config.auth
        );
        let new_aggregator = Arc::new(McpAggregator::new(
            SERVICE_MANAGER.clone(),
            MCP_CLIENT_MANAGER.clone(),
            server_config,
            token_manager,
        ));

        // 更新全局聚合器状态
        {
            let mut aggregator_guard = AGGREGATOR.lock().unwrap();
            *aggregator_guard = Some(new_aggregator.clone());
        }

        // 重新启动聚合器
        tracing::info!("Starting new aggregator with new configuration...");
        if let Err(e) = new_aggregator.start().await {
            tracing::error!("Failed to start new aggregator: {}", e);
            return Err(McpError::InternalError(format!(
                "Failed to start aggregator: {}",
                e
            )));
        } else {
            tracing::info!("Aggregator restarted successfully with new configuration");
        }
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

            // Config has been automatically saved to file via SERVICE_MANAGER.update_config
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

            // Config has been automatically saved to file via SERVICE_MANAGER.update_config
        }
        Ok("Settings saved successfully".to_string())
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
                Ok("Auto-startup disabled".to_string())
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
                Ok("Auto-startup enabled".to_string())
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
