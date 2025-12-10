// System Settings Commands

use crate::config as config_mod;
use crate::error::{McpError, Result};
use crate::{build_main_tray, types, AGGREGATOR, MCP_CLIENT_MANAGER, SERVICE_MANAGER, TOKEN_MANAGER};
use serde::Serialize;
use std::sync::Arc;
use tauri::Emitter;

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
    struct ServerOut {
        host: String,
        port: u16,
        max_connections: usize,
        timeout_seconds: u64,
        #[serde(default)]
        auth: bool,
    }

    #[derive(Serialize)]
    struct LoggingOut {
        level: String,
        file_name: Option<String>,
    }

    #[derive(Serialize)]
    struct TrayOut {
        #[serde(default)]
        enabled: Option<bool>,
        #[serde(default)]
        close_to_tray: Option<bool>,
        #[serde(default)]
        start_to_tray: Option<bool>,
    }

    #[derive(Serialize)]
    struct SettingsOut {
        #[serde(default)]
        theme: Option<String>,
        #[serde(default)]
        language: Option<String>,
        #[serde(default)]
        autostart: Option<bool>,
        #[serde(default)]
        system_tray: Option<TrayOut>,
        #[serde(default)]
        uv_index_url: Option<String>,
        #[serde(default)]
        npm_registry: Option<String>,
        #[serde(default)]
        command_paths: std::collections::HashMap<String, String>,
    }

    #[derive(Serialize)]
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
            language: s.language.clone(),
            autostart: s.autostart,
            system_tray: s.system_tray.as_ref().map(|t| TrayOut {
                enabled: t.enabled,
                close_to_tray: t.close_to_tray,
                start_to_tray: t.start_to_tray,
            }),
            uv_index_url: s.uv_index_url.clone(),
            npm_registry: s.npm_registry.clone(),
            command_paths: s.command_paths.clone(),
        }),
    };

    serde_json::to_value(out)
        .map_err(|e| McpError::ConfigError(format!("Failed to convert settings to JSON: {}", e)))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_settings(app: tauri::AppHandle, settings: serde_json::Value) -> Result<String> {
    use serde_json::Value;

    // Debug: Log the received settings
    tracing::debug!(
        "save_settings called with data: {}",
        serde_json::to_string_pretty(&settings)
            .unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // Load current config
    let mut config = config_mod::AppConfig::load()
        .map_err(|e| McpError::ConfigError(format!("Failed to load config: {}", e)))?;

    // Snapshot old config before update
    let prev_config = config.clone();
    let tray_old = prev_config
        .settings
        .as_ref()
        .and_then(|s| s.system_tray.as_ref())
        .and_then(|t| t.enabled)
        .unwrap_or(true);

    // Parse the JSON payload
    let settings_obj = match settings.as_object() {
        Some(obj) => obj,
        None => {
            return Err(McpError::ConfigError("Invalid settings payload: not an object".to_string()));
        }
    };

    // Update configuration
    {
        // Ensure settings exists
        if config.settings.is_none() {
            config.settings = Some(types::Settings {
                theme: Some("auto".to_string()),
                language: Some("en-US".to_string()),
                autostart: Some(false),
                system_tray: Some(types::SystemTraySettings {
                    enabled: Some(true),
                    close_to_tray: Some(false),
                    start_to_tray: Some(false),
                }),
                uv_index_url: None,
                npm_registry: None,
                command_paths: std::collections::HashMap::new(),
            });
        }
        let settings_mut = config.settings.as_mut().unwrap();

        // Theme
        if let Some(Value::String(theme)) = settings_obj.get("theme") {
            settings_mut.theme = Some(theme.clone());
        }

        // Language
        if let Some(Value::String(language)) = settings_obj.get("language") {
            settings_mut.language = Some(language.clone());
        }

        // Autostart (flag only; actual OS integration via separate command)
        if let Some(Value::Bool(b)) = settings_obj.get("autostart") {
            settings_mut.autostart = Some(*b);
        }

        // System tray subobject
        if let Some(Value::Object(tray_obj)) = settings_obj.get("system_tray") {
            if settings_mut.system_tray.is_none() {
                settings_mut.system_tray = Some(types::SystemTraySettings {
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
                    tracing::debug!(
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

        // Command paths settings
        if let Some(Value::Object(cmd_paths)) = settings_obj.get("command_paths") {
            let mut new_command_paths = std::collections::HashMap::new();
            for (key, value) in cmd_paths {
                if let Value::String(path) = value {
                    new_command_paths.insert(key.clone(), path.clone());
                }
            }
            tracing::debug!("Updating command_paths: {:?}", new_command_paths);
            settings_mut.command_paths = new_command_paths;
        }

        // Logging config
        if let Some(Value::Object(logging_obj)) = settings.get("logging") {
            // Ensure logging exists
            if config.logging.is_none() {
                config.logging = Some(types::LoggingSettings {
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

        // Server config
        if let Some(Value::Object(server_obj)) = settings.get("server") {
            tracing::debug!("Processing server config: {:?}", server_obj);

            if let Some(Value::String(host)) = server_obj.get("host") {
                config.server.host = host.clone();
                tracing::debug!("Updated host: {}", host);
            }
            if let Some(Value::Number(port)) = server_obj.get("port") {
                if let Some(p) = port.as_u64() {
                    config.server.port = p as u16;
                    tracing::debug!("Updated port: {}", p);
                }
            }
            if let Some(Value::Number(max_conn)) = server_obj.get("max_connections") {
                if let Some(mc) = max_conn.as_u64() {
                    config.server.max_connections = mc as usize;
                    tracing::debug!("Updated max_connections: {}", mc);
                }
            }
            if let Some(Value::Number(timeout)) = server_obj.get("timeout_seconds") {
                if let Some(ts) = timeout.as_u64() {
                    config.server.timeout_seconds = ts;
                    tracing::debug!("Updated timeout_seconds: {}", ts);
                }
            }
            if let Some(Value::Bool(auth)) = server_obj.get("auth") {
                config.server.auth = *auth;
                tracing::debug!("Updated auth: {}", auth);
            } else {
                tracing::warn!("auth field not found in server config or not a boolean");
            }
        }
    }

    // Save configuration
    config.save()
        .map_err(|e| McpError::ConfigError(format!("Failed to save config: {}", e)))?;

    // Post-save: detect tray visibility change and server restarts
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

        // Stop existing aggregator
        let aggregator_clone = {
            let aggregator_guard = AGGREGATOR.lock().unwrap();
            (*aggregator_guard).clone()
        };

        if let Some(aggregator) = &aggregator_clone {
            tracing::debug!("Shutting down existing aggregator...");
            aggregator.trigger_shutdown().await;
        }

        // Wait for shutdown
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Create new aggregator from latest config
        let server_config = Arc::new(config.server.clone());

        // Get TokenManager
        let token_manager = {
            let token_manager_guard = TOKEN_MANAGER.read().await;
            (*token_manager_guard)
                .as_ref()
                .ok_or_else(|| McpError::InternalError("TokenManager not initialized".to_string()))?
                .clone()
        };

        // Get service manager
        let service_manager = SERVICE_MANAGER.lock().unwrap().as_ref()
            .ok_or_else(|| McpError::Internal("SERVICE_MANAGER not initialized".to_string()))?
            .clone();

        // Create new aggregator instance
        tracing::debug!(
            "Creating new aggregator with updated configuration (auth: {})",
            server_config.auth
        );
        let new_aggregator = Arc::new(crate::aggregator::McpAggregator::new(
            service_manager,
            MCP_CLIENT_MANAGER.clone(),
            server_config,
            token_manager,
        ));

        // Update global aggregator state
        {
            let mut aggregator_guard = AGGREGATOR.lock().unwrap();
            *aggregator_guard = Some(new_aggregator.clone());
        }

        // Restart aggregator
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
    }

    // Handle tray changes
    if tray_changed {
        tracing::debug!("System tray configuration changed, enabled: {}", tray_new);

        if tray_new {
            if let Some(tray) = app.tray_by_id("main_tray") {
                let _ = tray.set_visible(true);
                tracing::debug!("Tray visibility updated: visible");
            } else {
                // Rebuild tray if it was not created at startup
                if let Err(e) = build_main_tray(&app) {
                    tracing::error!("Failed to rebuild system tray: {}", e);
                } else {
                    tracing::debug!("System tray rebuilt and made visible");
                }
            }
        } else {
            if let Some(tray) = app.tray_by_id("main_tray") {
                let _ = tray.set_visible(false);
            }
            tracing::debug!("Tray icon hidden");
        }
    }

    if server_config_changed {
        Ok(format!(
            "Settings saved successfully. Aggregator restarted on {}:{}",
            config.server.host, config.server.port
        ))
    } else {
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

#[tauri::command(rename_all = "snake_case")]
pub async fn get_language_preference() -> Result<Option<String>> {
    let config = config_mod::AppConfig::load()
        .map_err(|e| McpError::ConfigError(format!("Failed to load settings: {}", e)))?;

    Ok(config.settings.and_then(|s| s.language))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_language_preference(app: tauri::AppHandle, language: String) -> Result<String> {
    // Validate language
    if !["zh-CN", "en-US"].contains(&language.as_str()) {
        return Err(McpError::ConfigError(format!(
            "Unsupported language: {}",
            language
        )));
    }

    // Load current config
    let mut config = config_mod::AppConfig::load()
        .map_err(|e| McpError::ConfigError(format!("Failed to load config: {}", e)))?;

    // Update language preference
    {
        // Ensure settings exists
        if config.settings.is_none() {
            config.settings = Some(types::Settings {
                theme: Some("auto".to_string()),
                language: Some(language.clone()),
                autostart: Some(false),
                system_tray: Some(types::SystemTraySettings {
                    enabled: Some(true),
                    close_to_tray: Some(false),
                    start_to_tray: Some(false),
                }),
                uv_index_url: None,
                npm_registry: None,
                command_paths: Default::default(),
            });
        } else {
            config.settings.as_mut().unwrap().language = Some(language.clone());
        }
    }

    // Save configuration
    config.save()
        .map_err(|e| McpError::ConfigError(format!("Failed to save config: {}", e)))?;

    // Update tray menu to reflect new language (safe method)
    if let Err(e) = crate::update_tray_menu(&app) {
        tracing::error!("Failed to update tray menu after language change: {}", e);
    }

    // Emit event to frontend
    let _ = app.emit("language-changed", language.clone());

    Ok(format!("Language preference saved: {}", language))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn check_path_validity(path: String) -> Result<serde_json::Value> {
    use std::fs;
    use std::path::Path;

    let path_obj = Path::new(&path);
    let exists = path_obj.exists();
    let is_executable = if exists {
        if let Ok(metadata) = fs::metadata(path_obj) {
            let permissions = metadata.permissions();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                permissions.mode() & 0o111 != 0
            }
            #[cfg(windows)]
            {
                // On Windows, we just check if the file exists
                true
            }
        } else {
            false
        }
    } else {
        false
    };

    Ok(serde_json::json!({
        "exists": exists,
        "is_executable": is_executable
    }))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_system_command_paths(command: String) -> Result<Vec<String>> {
    use std::collections::HashSet;

    let mut paths = HashSet::new();

    // Use which crate to find all occurrences of the command in PATH
    if let Ok(found_paths) = which::which_all_global(&command) {
        for path in found_paths {
            // Use original path without resolving symlinks
            if let Some(path_str) = path.to_str() {
                paths.insert(path_str.to_string());
            }
        }
    }

    // Convert to sorted Vec
    let mut result: Vec<String> = paths.into_iter().collect();
    result.sort();

    Ok(result)
}