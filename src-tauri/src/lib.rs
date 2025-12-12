pub mod aggregator;
pub mod auth_context;
pub mod commands;
pub mod config;
pub mod error;
pub mod marketplace;
pub mod mcp_client;
pub mod mcp_manager;
pub mod storage;
pub mod token_manager;
pub mod types;

// SeaORM 实体模块
pub mod entities;
pub mod migration;

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

use crate::mcp_manager::McpServerManager;
use crate::storage::UnifiedStorageManager;
use commands::token_management::TokenManagerState;
use commands::*;
use mcp_client::McpClientManager;

// Re-export types for public use
pub use types::*;

/// Single instance event payload
#[derive(Clone, serde::Serialize)]
struct SingleInstancePayload {
    args: Vec<String>,
    cwd: String,
}

/// Wait for SERVICE_MANAGER to be initialized (with timeout)
pub async fn wait_for_service_manager() -> Result<Arc<McpServerManager>, crate::error::McpError> {
    let mut attempts = 0;
    let max_attempts = 50; // 5 seconds max

    while attempts < max_attempts {
        {
            let guard = SERVICE_MANAGER.lock().map_err(|e| {
                crate::error::McpError::Internal(format!("Failed to lock SERVICE_MANAGER: {}", e))
            })?;

            if let Some(ref manager) = *guard {
                return Ok(manager.clone());
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }

    Err(crate::error::McpError::Internal(
        "SERVICE_MANAGER initialization timeout".to_string(),
    ))
}

/// Wait for TokenManager to be initialized (with timeout)
pub async fn wait_for_token_manager(
) -> Result<Arc<crate::token_manager::TokenManager>, crate::error::McpError> {
    let mut attempts = 0;
    let max_attempts = 50; // 5 seconds max

    while attempts < max_attempts {
        {
            let guard = TOKEN_MANAGER.write().await;
            if let Some(ref manager) = *guard {
                return Ok(manager.clone());
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }

    Err(crate::error::McpError::InternalError(
        "TokenManager initialization timeout".to_string(),
    ))
}

// Global state - use MCP Server Manager
static SERVICE_MANAGER: std::sync::Mutex<Option<Arc<McpServerManager>>> =
    std::sync::Mutex::new(None);

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

#[allow(dead_code)]
static UNIFIED_STORAGE_MANAGER: std::sync::LazyLock<
    std::sync::Mutex<Option<Arc<UnifiedStorageManager>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

// Track application startup time
static STARTUP_TIME: std::sync::LazyLock<SystemTime> = std::sync::LazyLock::new(SystemTime::now);

/// Get localized text for tray menu items
fn get_tray_text(key: &str, language: &str) -> &'static str {
    match (key, language) {
        // Main menu items
        ("show_window", "zh-CN") => "显示主窗口",
        ("show_window", _) => "Show Main Window",
        ("servers", "zh-CN") => "服务器管理",
        ("servers", _) => "Server Management",
        ("marketplace", "zh-CN") => "市场",
        ("marketplace", _) => "Marketplace",
        ("settings", "zh-CN") => "设置",
        ("settings", _) => "Settings",

        // Theme submenu
        ("theme", "zh-CN") => "主题",
        ("theme", _) => "Theme",
        ("theme_auto", "zh-CN") => "自动（跟随系统）",
        ("theme_auto", _) => "Auto (Follow System)",
        ("theme_light", "zh-CN") => "亮色",
        ("theme_light", _) => "Light",
        ("theme_dark", "zh-CN") => "暗色",
        ("theme_dark", _) => "Dark",

        // Language submenu
        ("language", "zh-CN") => "语言",
        ("language", _) => "Language",
        ("language_zh_cn", _) => "简体中文",
        ("language_en_us", _) => "English (US)",

        // Other menu items
        ("about", "zh-CN") => "关于 MCP Router",
        ("about", _) => "About MCP Router",
        ("quit", "zh-CN") => "退出",
        ("quit", _) => "Quit",

        // Fallback
        _ => "",
    }
}

/// Build tray menu with current language and theme settings
/// Returns the menu and cloned menu items for event handling
fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    // Load configuration to get current language and theme
    let config = AppConfig::load().unwrap_or_default();
    let language = config
        .settings
        .as_ref()
        .and_then(|s| s.language.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("zh-CN");
    let theme = config
        .settings
        .as_ref()
        .and_then(|s| s.theme.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("auto");

    // Create theme menu items with correct checked state
    let theme_auto_item =
        tauri::menu::CheckMenuItemBuilder::new(get_tray_text("theme_auto", language))
            .id("theme_auto")
            .checked(theme == "auto")
            .build(app)?;
    let theme_light_item =
        tauri::menu::CheckMenuItemBuilder::new(get_tray_text("theme_light", language))
            .id("theme_light")
            .checked(theme == "light")
            .build(app)?;
    let theme_dark_item =
        tauri::menu::CheckMenuItemBuilder::new(get_tray_text("theme_dark", language))
            .id("theme_dark")
            .checked(theme == "dark")
            .build(app)?;

    // Create language menu items
    let zh_cn_item =
        tauri::menu::CheckMenuItemBuilder::new(get_tray_text("language_zh_cn", language))
            .id("language_zh_cn")
            .checked(language == "zh-CN")
            .build(app)?;
    let en_us_item =
        tauri::menu::CheckMenuItemBuilder::new(get_tray_text("language_en_us", language))
            .id("language_en_us")
            .checked(language == "en-US")
            .build(app)?;

    // Build tray menu
    let menu = tauri::menu::MenuBuilder::new(app)
        .item(
            &tauri::menu::MenuItemBuilder::new(get_tray_text("show_window", language))
                .id("show_window")
                .accelerator("CmdOrCtrl+Shift+M")
                .build(app)?,
        )
        .item(&tauri::menu::PredefinedMenuItem::separator(app)?)
        .item(
            &tauri::menu::MenuItemBuilder::new(get_tray_text("servers", language))
                .id("server_management")
                .build(app)?,
        )
        .item(
            &tauri::menu::MenuItemBuilder::new(get_tray_text("marketplace", language))
                .id("marketplace")
                .build(app)?,
        )
        .item(
            &tauri::menu::MenuItemBuilder::new(get_tray_text("settings", language))
                .id("settings")
                .build(app)?,
        )
        .item(&tauri::menu::PredefinedMenuItem::separator(app)?)
        .item(
            &tauri::menu::SubmenuBuilder::new(app, get_tray_text("theme", language))
                .item(&theme_auto_item)
                .item(&theme_light_item)
                .item(&theme_dark_item)
                .build()?,
        )
        .item(
            &tauri::menu::SubmenuBuilder::new(app, get_tray_text("language", language))
                .item(&zh_cn_item)
                .item(&en_us_item)
                .build()?,
        )
        .item(&tauri::menu::PredefinedMenuItem::separator(app)?)
        .item(
            &tauri::menu::MenuItemBuilder::new(get_tray_text("about", language))
                .id("about")
                .build(app)?,
        )
        .item(&tauri::menu::PredefinedMenuItem::separator(app)?)
        .item(
            &tauri::menu::MenuItemBuilder::new(get_tray_text("quit", language))
                .id("quit")
                .accelerator("CmdOrCtrl+Q")
                .build(app)?,
        )
        .build()?;

    Ok(menu)
}

/// Update existing tray menu safely without rebuilding the tray icon
pub fn update_tray_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main_tray") {
        let new_menu = build_tray_menu(app)?;
        tray.set_menu(Some(new_menu))?;
        tracing::debug!("Tray menu updated successfully");
        Ok(())
    } else {
        tracing::warn!("Tray not found, cannot update menu");
        Ok(()) // Don't error out, just log warning
    }
}

fn build_main_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    // Only build tray if it doesn't exist (startup only)
    if app.tray_by_id("main_tray").is_some() {
        tracing::debug!("Tray already exists, skipping build");
        return Ok(());
    }

    // Build menu using shared function
    let menu = build_tray_menu(app)?;

    // Build tray icon with menu and event handlers
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

                // TODO: 修复配置保存逻辑
                // 暂时注释掉以避免编译错误
                // tokio::spawn(async move {
                //     // 配置处理逻辑需要修复
                // });

                // Update tray menu to reflect new theme
                let app_clone = app.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    if let Err(e) = update_tray_menu(&app_clone) {
                        tracing::error!("Failed to update tray menu after theme change: {}", e);
                    }
                });

                let _ = app.emit("theme-changed", theme);
            }
            "language_zh_cn" | "language_en_us" => {
                let language = if event.id.as_ref() == "language_zh_cn" {
                    "zh-CN"
                } else {
                    "en-US"
                };

                // TODO: 修复语言配置保存逻辑
                // 暂时注释掉以避免编译错误
                // tokio::spawn(async move {
                //     // 语言配置处理逻辑需要修复
                // });

                // Update tray menu to reflect new language
                let app_clone = app.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    if let Err(e) = update_tray_menu(&app_clone) {
                        tracing::error!("Failed to update tray menu after language change: {}", e);
                    }
                });

                // Emit event to frontend
                let _ = app.emit("language-changed", language);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

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
    let (log_level, log_file_name, sql_log_enabled) = if let Some(ref logging) = config.logging {
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

        (level, file_name, logging.sql_log)
    } else {
        (log::LevelFilter::Info, None, false)
    };

    // Use a single, explicit log target
    // Ensure the file name doesn't already have .log extension to avoid duplication
    let final_log_name = log_file_name
        .clone()
        .unwrap_or_else(|| "mcprouter".to_string());

    let log_builder =
        tauri_plugin_log::Builder::new()
            .level(log_level)
            .targets([tauri_plugin_log::Target::new(
                tauri_plugin_log::TargetKind::LogDir {
                    file_name: Some(final_log_name),
                },
            )]);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // 记录第二个实例启动尝试
            tracing::info!(
                "Attempted to start second instance. Args: {:?}, CWD: {}",
                argv,
                cwd
            );

            // 向已运行实例发送事件，携带启动参数
            let payload = SingleInstancePayload {
                args: argv,
                cwd,
            };

            if let Err(e) = app.emit("single-instance", payload) {
                tracing::error!("Failed to emit single-instance event: {}", e);
            }

            // 激活并聚焦到主窗口
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.show();
                let _ = window.unminimize();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(log_builder.build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(move |app| {
            // Clone the log configuration for use in async tasks
            let log_level_for_init = log_level;
            let sql_log_for_init = sql_log_enabled;
            // Initialize tracing-log bridge to capture log crate outputs
            tracing_log::LogTracer::init().ok();



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

            let mcp_client_manager = MCP_CLIENT_MANAGER.clone();
            let server_config = Arc::new(config.server.clone());

            // 2.6) Initialize SeaORM database and managers in background
            let config_dir_for_init = config_dir.clone();
            let mcp_client_manager_for_init = mcp_client_manager.clone();
            let server_config_for_init = server_config.clone();

            // Stage 1: Initialize SeaORM database and managers asynchronously
            let db_path = config_dir_for_init.join("mcprouter.db");
            let storage_config = crate::storage::manager::StorageConfig::with_db_path(db_path);
            let db_url = storage_config.database_url();

            tracing::info!("Initializing SeaORM database at: {}", db_url);

            // Spawn async initialization
            tokio::spawn(async move {
                match crate::storage::UnifiedStorageManager::new(storage_config, sql_log_for_init, log_level_for_init).await {
                    Ok(storage_manager) => {
                        tracing::info!("SeaORM database initialized successfully");

                        // Initialize managers
                        match initialize_managers(storage_manager, mcp_client_manager_for_init, server_config_for_init).await {
                            Ok(_) => {
                                tracing::info!("All managers initialized successfully");
                            }
                            Err(e) => {
                                tracing::error!("Failed to initialize managers: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize SeaORM database: {}", e);
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
            // Legacy Commands
            toggle_mcp_server_tool,
            enable_all_mcp_server_tools,
            disable_all_mcp_server_tools,
            // Settings commands
            get_settings,
            save_settings,
            check_path_validity,
            get_system_command_paths,
            get_dashboard_stats,
            get_local_ip_addresses,
            toggle_autostart,
            get_language_preference,
            save_language_preference,
            // Token Management Commands
            create_token,
            update_token,
            list_tokens,
            delete_token,
            toggle_token,
            get_token_stats,
            cleanup_expired_tokens,
            validate_token,
            get_tokens_for_dashboard,
            // Real-time Token Management Commands (已统一到 update_token_permission)
            // 统一的权限更新命令
            update_token_permission,
            // Permission Management Commands
            list_available_permissions,
            // Language Management Commands (temporarily disabled)
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Initialize managers (split from main run function to prevent stack overflow)
async fn initialize_managers(
    storage_manager: crate::storage::UnifiedStorageManager,
    mcp_client_manager: Arc<McpClientManager>,
    server_config: Arc<ServerConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Stage 2a: Initialize MCP Server Manager
    tracing::info!("Initializing MCP Server Manager");
    let mcp_server_manager: Arc<crate::mcp_manager::McpServerManager> = {
        // Create MCP manager using ORM storage
        match McpServerManager::with_storage_manager(Arc::new(storage_manager.clone())).await {
            Ok(manager) => Arc::new(manager),
            Err(e) => {
                tracing::error!("Failed to create MCP Server Manager: {}", e);
                return Err(format!("Failed to create MCP Server Manager: {}", e).into());
            }
        }
    };

    // Set global SERVICE_MANAGER for backward compatibility
    {
        let mut service_manager_guard = SERVICE_MANAGER.lock().unwrap();
        *service_manager_guard = Some(mcp_server_manager.clone());
    }

    // Stage 2b: Initialize Token Manager
    tracing::info!("Token Manager initialized");
    let token_manager: Arc<crate::token_manager::TokenManager> = {
        // Use the storage manager to create the token manager
        match storage_manager.create_token_manager().await {
            Ok(token_manager) => {
                tracing::info!("Token Manager initialized successfully");
                token_manager
            }
            Err(e) => {
                tracing::error!("Failed to initialize Token Manager: {}", e);
                return Err(format!("Failed to initialize Token Manager: {}", e).into());
            }
        }
    };

    // Initialize global TOKEN_MANAGER state (keep for backward compatibility)
    {
        let mut token_manager_guard = TOKEN_MANAGER.write().await;
        *token_manager_guard = Some(token_manager.clone());
    }

    // Stage 3: Create and start aggregator
    let mcp_server_manager_for_agg = mcp_server_manager.clone();
    let token_manager_for_agg = token_manager.clone();
    tokio::spawn(async move {
        // Give a brief moment for all managers to be fully initialized
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        create_and_start_aggregator(
            mcp_server_manager_for_agg,
            mcp_client_manager,
            server_config,
            token_manager_for_agg,
        )
        .await;
    });

    // Stage 4: Load and connect services (spawned separately)
    tokio::spawn(async move {
        load_and_connect_services(mcp_server_manager).await;
    });

    Ok(())
}

// Create and start the MCP aggregator
async fn create_and_start_aggregator(
    mcp_server_manager: Arc<crate::mcp_manager::McpServerManager>,
    mcp_client_manager: Arc<McpClientManager>,
    server_config: Arc<ServerConfig>,
    token_manager: Arc<crate::token_manager::TokenManager>,
) {
    tracing::info!("Creating and starting MCP aggregator");

    // Create the aggregator instance
    let aggregator = aggregator::McpAggregator::new(
        mcp_server_manager,
        mcp_client_manager,
        server_config,
        token_manager,
    );

    // Store the aggregator in the global variable
    {
        let mut aggregator_guard = AGGREGATOR.lock().unwrap();
        *aggregator_guard = Some(Arc::new(aggregator));
    }

    // Start the aggregator HTTP server
    let aggregator_for_start = {
        let guard = AGGREGATOR.lock().unwrap();
        guard.as_ref().unwrap().clone()
    };

    match aggregator_for_start.start().await {
        Ok(_) => {
            tracing::info!("MCP Aggregator created and started successfully");
        }
        Err(e) => {
            tracing::error!("Failed to start MCP Aggregator: {}", e);
        }
    }
}

/// Load and connect MCP services
async fn load_and_connect_services(mcp_server_manager: Arc<crate::mcp_manager::McpServerManager>) {
    tracing::info!("Loading MCP services from SQLite database");
    match mcp_server_manager.load_mcp_servers().await {
        Ok(_) => {
            tracing::info!("MCP services loaded");

            // Auto-connect all enabled services
            tracing::info!("Auto-connect enabled services");
            if let Err(e) = mcp_server_manager.auto_connect_enabled_services().await {
                tracing::error!("Failed to auto-connect services: {}", e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to load services: {}", e);
        }
    }
}
