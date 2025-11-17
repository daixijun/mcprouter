mod aggregator;
mod auth_context;
mod commands;
mod config;
mod error;
mod marketplace;
mod mcp_client;
mod mcp_manager;
mod session_manager;
mod token_manager;
mod types;

use std::sync::Arc;
use std::time::SystemTime;
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

use commands::token_management::{init_token_manager, TokenManagerState};
use commands::*;
use mcp_client::McpClientManager;
use mcp_manager::McpServerManager;

// Re-export types for public use
pub use types::*;

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

pub static AGGREGATOR: std::sync::LazyLock<
    Arc<std::sync::Mutex<Option<Arc<aggregator::McpAggregator>>>>,
> = std::sync::LazyLock::new(|| Arc::new(std::sync::Mutex::new(None)));

static TOKEN_MANAGER: std::sync::LazyLock<TokenManagerState> =
    std::sync::LazyLock::new(|| Arc::new(tokio::sync::RwLock::new(None)));

// Track application startup time
static STARTUP_TIME: std::sync::LazyLock<SystemTime> = std::sync::LazyLock::new(SystemTime::now);

fn build_main_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    // Load configuration to initialize theme menu state
    let config = AppConfig::load().unwrap_or_default();

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
                        config.settings = Some(Settings {
                            theme: None,
                            language: Some("zh-CN".to_string()),
                            autostart: Some(false),
                            system_tray: Some(SystemTraySettings {
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

    tracing::debug!("System tray initialized successfully");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // 1) Load configuration FIRST, fail fast on error
    let config = AppConfig::load().unwrap_or_else(|e| {
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
                .with(
                    fmt::layer()
                        .with_target(false)
                        .with_level(false)
                        .with_file(false)
                        .with_line_number(false)
                        .event_format(fmt::format().compact())
                        .with_filter(tracing_level),
                )
                .with(
                    fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(false)
                        .with_target(false)
                        .with_level(false)
                        .with_file(false)
                        .with_line_number(false)
                        .event_format(fmt::format().compact())
                        .with_filter(tracing_level),
                );

            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set tracing subscriber");

            // Keep the guard alive for the entire application lifetime
            // This ensures the non-blocking worker continues running
            std::mem::forget(guard);

            // Log after subscriber is initialized
            tracing::info!("Starting MCP Router");

            // 2.5) Initialize TokenManager (async task)
            // Use ~/.mcprouter as the configuration directory for consistency
            let home_dir = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            let config_dir = std::path::PathBuf::from(format!("{}/.mcprouter", home_dir));

            let config = AppConfig::load().unwrap_or_else(|e| {
                tracing::error!(
                    "\n========================================\nERROR: Failed to load configuration file\n========================================\n{}\n\nThe application cannot start with an invalid configuration.\nPlease fix the config file at: ~/.mcprouter/config.json\nOr delete it to use default settings.\n",
                    e
                );
                std::process::exit(1);
            });

            let mcp_server_manager = SERVICE_MANAGER.clone();
            let mcp_client_manager = MCP_CLIENT_MANAGER.clone();
            let server_config = Arc::new(config.server.clone());

            // 2.6) Initialize TokenManager and start aggregator in sequence
            tokio::spawn(async move {
                // Initialize TokenManager
                let token_manager = match init_token_manager(&config_dir).await {
                    Ok(manager) => {
                        tracing::info!("TokenManager initialized successfully");
                        manager
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize TokenManager: {}", e);
                        std::process::exit(1);
                    }
                };

                // Initialize global TOKEN_MANAGER state
                {
                    let mut token_manager_guard = TOKEN_MANAGER.write().await;
                    *token_manager_guard = Some(token_manager.clone());
                }

                // Create aggregator with TokenManager
                let aggregator = Arc::new(aggregator::McpAggregator::new(
                    mcp_server_manager,
                    mcp_client_manager,
                    server_config,
                    token_manager,
                ));

                // Store aggregator in global variable
                {
                    let mut agg_guard = AGGREGATOR.lock().unwrap();
                    *agg_guard = Some(aggregator.clone());
                }

                // Start aggregator
                if let Err(e) = aggregator.start().await {
                    tracing::error!(
                        "Failed to start MCP aggregator server: {}\n\
                        The application cannot continue without the MCP aggregator service.\n\
                        Please check if the port is already in use or if there are permission issues.",
                        e
                    );
                    std::process::exit(1);
                }
            });

            // 4) Load MCP services from configuration files
            let app_handle = app.handle().clone();
            tokio::spawn(async move {
                tracing::info!("Loading MCP services from configuration files");
                match SERVICE_MANAGER.load_mcp_servers(&app_handle).await {
                    Ok(_) => {
                        tracing::info!("MCP services loaded");

                        // 5) 自动连接所有启用的服务
                        tracing::info!("Auto-connect enabled services");
                        if let Err(e) = SERVICE_MANAGER.auto_connect_enabled_services(&app_handle).await {
                            tracing::error!("Failed to auto-connect services: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to load services: {}", e);
                    }
                }
            });

            // Tray helper moved to module scope (build_main_tray)

            // Add TokenManager to Tauri app state (will be populated async)
            app.manage(TOKEN_MANAGER.clone());

            // Add AGGREGATOR to Tauri app state for permission management
            app.manage(AGGREGATOR.clone());

            // Ensure tray visibility based on config at startup
            let tray_enabled_start = config
                .settings
                .as_ref()
                .and_then(|s| s.system_tray.as_ref())
                .and_then(|t| t.enabled)
                .unwrap_or(true);
            if tray_enabled_start {
                if app.tray_by_id("main_tray").is_none() {
                    let _ = build_main_tray(app.handle());
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
                        let cfg = AppConfig::load().ok();
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
                            tracing::debug!("Window minimized to tray (runtime config)");
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
                    tracing::debug!("Window hidden on startup due to configuration");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_theme,
            set_theme,
            update_config,
            import_mcp_servers_config,
            add_mcp_server,
            update_mcp_server,
            toggle_mcp_server,
            list_mcp_servers,
            list_marketplace_services,
            get_mcp_server_details,
            install_marketplace_service,
            // Enhanced Service Management
            delete_mcp_server,
            // Tool DB Commands
            list_mcp_server_tools,
            list_mcp_server_resources,
            list_mcp_server_prompts,
            refresh_all_mcp_servers,
            // Legacy Commands
            toggle_mcp_server_tool,
            enable_all_mcp_server_tools,
            disable_all_mcp_server_tools,
            get_settings,
            save_settings,
            get_dashboard_stats,
            get_local_ip_addresses,
            toggle_autostart,
            // Token Management Commands
            create_token,
            update_token,
            list_tokens,
            delete_token,
            toggle_token,
            get_token_stats,
            cleanup_expired_tokens,
            validate_token,
            // Permission Management Commands
            get_available_permissions,
            // Language Management Commands
            get_language_preference,
            save_language_preference,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
