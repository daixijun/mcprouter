pub mod aggregator;
pub mod auth_context;
pub mod commands;
pub mod config;
pub mod error;
pub mod marketplace;
pub mod mcp_client;
pub mod mcp_manager;
pub mod shell_environment;
pub mod storage;
pub mod token_manager;
pub mod tool_manager;
pub mod types;

// SeaORM 实体模块
pub mod entities;
pub mod migration;

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

use crate::mcp_manager::McpServerManager;
use crate::storage::StorageManager;
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

/// Get initialization state instance
async fn get_initialization_state() -> std::sync::Arc<tokio::sync::RwLock<InitializationState>> {
    INIT_STATE
        .get_or_init(|| {
            std::sync::Arc::new(tokio::sync::RwLock::new(InitializationState::NotStarted))
        })
        .clone()
}

/// Update initialization state
async fn update_initialization_state(state: InitializationState) {
    if let Some(init_state) = INIT_STATE.get() {
        *init_state.write().await = state;
        tracing::info!("Initialization state updated: {:?}", state);
    }
}

/// Get service manager (with timeout and progressive waiting)
pub async fn wait_for_service_manager() -> Result<Arc<McpServerManager>, crate::error::McpError> {
    wait_for_service_manager_with_progress().await
}

/// Service manager waiting method with progress feedback
///
/// 优化后的等待逻辑：
/// 1. 首次尝试直接获取，不等待（适用于初始化已完成的场景）
/// 2. 如果服务管理器已存在，立即返回，不检查状态
/// 3. 减少总等待时间，避免接口卡死
pub async fn wait_for_service_manager_with_progress(
) -> Result<Arc<McpServerManager>, crate::error::McpError> {
    // 第一步：立即尝试获取（适用于热启动或已初始化完成的场景）
    if let Some(manager) = get_service_manager_fast()? {
        // 快速路径：管理器已存在，直接返回
        tracing::debug!("Service manager immediately available");
        return Ok(manager);
    }

    let mut last_state = InitializationState::NotStarted;
    let mut progress_logged: std::collections::HashSet<&'static str> = std::collections::HashSet::new();

    // 第二步：最多等待 3 秒（30次尝试 * 100ms）
    // 这比原来的 30 秒短得多，避免阻塞接口太久
    for attempt in 0..30 {
        // 检查初始化状态
        let init_state = get_initialization_state().await;
        let current_state = *init_state.read().await;

        // Log state changes
        if current_state != last_state {
            tracing::info!("Initialization progress: {:?}", current_state);
            last_state = current_state;
        }

        // Provide different status feedback
        match current_state {
            InitializationState::NotStarted => {
                tracing::debug!("Waiting for initialization to start...");
            }
            InitializationState::DatabaseConnecting => {
                if !progress_logged.contains(&"db_connecting") {
                    tracing::info!("⏳ Database connecting...");
                    progress_logged.insert("db_connecting");
                }
            }
            InitializationState::DatabaseMigrating => {
                if !progress_logged.contains(&"db_migrating") {
                    tracing::info!("⏳ Database migrating...");
                    progress_logged.insert("db_migrating");
                }
            }
            InitializationState::ManagersCreated => {
                if !progress_logged.contains(&"managers_created") {
                    tracing::info!("✅ Managers created, basic functions available");
                    progress_logged.insert("managers_created");
                }
            }
            InitializationState::ServicesLoading => {
                if !progress_logged.contains(&"services_loading") {
                    tracing::info!("🔁 Config loading...");
                    progress_logged.insert("services_loading");
                }
            }
            InitializationState::ServicesConnecting => {
                if !progress_logged.contains(&"services_connecting") {
                    tracing::info!("🔌 Services connecting...");
                    progress_logged.insert("services_connecting");
                }
            }
            InitializationState::Completed => {
                if !progress_logged.contains(&"completed") {
                    tracing::info!("🎉 All services initialized");
                    progress_logged.insert("completed");
                }
                // 完全初始化完成，安全返回
                if let Some(manager) = get_service_manager_fast()? {
                    return Ok(manager);
                }
            }
        }

        // 尝试获取服务管理器 - 优化：管理器已存在就返回（不管状态）
        if let Ok(guard) = SERVICE_MANAGER.lock() {
            if let Some(ref manager) = *guard {
                // 激进优化：只要管理器存在就返回
                // 后台任务可以继续，但对于调用者来说功能已可用
                if current_state == InitializationState::ManagersCreated
                    || current_state == InitializationState::Completed
                    || (attempt >= 10
                        && !matches!(
                            current_state,
                            InitializationState::DatabaseConnecting
                                | InitializationState::NotStarted
                        ))
                {
                    if !progress_logged.contains(&"early_return") {
                        tracing::warn!("⚠️  Manager already exists, returning immediately (background tasks continue)");
                        progress_logged.insert("early_return");
                    }
                    return Ok(manager.clone());
                }
            }
        }

        // 循环等待 - 保持快速响应
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // 第三步：3秒后仍然没有，检查是否 manager 已存在（部分可用）
    tracing::warn!("3 seconds timeout, checking for partially available manager");
    if let Some(manager) = get_service_manager_fast()? {
        tracing::info!(
            "✅ Got partially available service manager (some functions may be unavailable)"
        );
        return Ok(manager);
    }

    // 第四步：完全失败
    Err(crate::error::McpError::Internal(
        "服务管理器初始化失败：3秒内未可用".to_string(),
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

/// Helper function to get service manager quickly (clone version)
fn get_service_manager_fast() -> Result<Option<Arc<McpServerManager>>, crate::error::McpError> {
    SERVICE_MANAGER
        .lock()
        .map_err(|_| {
            crate::error::McpError::Internal("Failed to acquire service manager lock".to_string())
        })?
        .clone()
        .map(Ok)
        .transpose()
}

// Global initialization state tracking
static INIT_STATE: std::sync::OnceLock<std::sync::Arc<tokio::sync::RwLock<InitializationState>>> =
    std::sync::OnceLock::new();

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
static STORAGE_MANAGER: std::sync::LazyLock<std::sync::Mutex<Option<Arc<StorageManager>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

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
        .icon(app.default_window_icon()
            .expect("Application must have a default window icon")
            .clone())
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
    let (log_level, _, sql_log_enabled) = if let Some(ref logging) = config.logging {
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

    // Use a fixed log file name
    let final_log_name = "mcprouter".to_string();

    let log_builder = tauri_plugin_log::Builder::new()
        .level(log_level)
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
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
            let app_for_init = app.handle().clone();

            // Stage 1: Initialize SeaORM database and managers asynchronously
            let db_path = config_dir_for_init.join("mcprouter.db");
            let storage_config = crate::storage::manager::StorageConfig::with_db_path(db_path);
            let db_url = storage_config.database_url();

            tracing::info!("Initializing SeaORM database at: {}", db_url);

            // Spawn async initialization
            tokio::spawn(async move {
                let db_init_start = std::time::Instant::now();  // 耗时追踪开始

                // 更新状态：开始数据库连接
                update_initialization_state(crate::types::InitializationState::DatabaseConnecting).await;

                match crate::storage::StorageManager::new(storage_config, sql_log_for_init, log_level_for_init).await {
                    Ok(storage_manager) => {
                        tracing::info!("✅ SeaORM database initialized successfully, took {:?}", db_init_start.elapsed());

                        // Initialize managers - 耗时追踪
                        let manager_init_start = std::time::Instant::now();
                        match initialize_managers(storage_manager, mcp_client_manager_for_init, server_config_for_init, app_for_init).await {
                            Ok(_) => {
                                tracing::info!("✅ All managers initialized successfully, took {:?}", manager_init_start.elapsed());
                                tracing::info!("🎉 Complete initialization took {:?} total", db_init_start.elapsed());
                            }
                            Err(e) => {
                                tracing::error!("❌ Manager initialization failed: {}
Initialization process may be partially completed, set managers are still usable", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Database initialization failed: {}", e);
                        tracing::error!("System will run in degraded mode (no data persistence available)");
                    }
                }
            });


            // Tray helper moved to module scope (build_main_tray)

            // Add TokenManager to Tauri app state (will be populated async)
            app.manage(TOKEN_MANAGER.clone());

            // Add AGGREGATOR to Tauri app state for permission management
            app.manage(AGGREGATOR.clone());

            // Add ToolManager to Tauri app state
            app.manage(commands::tool_manager::ToolManagerState::new());

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
            // App Info Commands
            get_app_info,
            get_app_version,
            get_app_name,
            // Config Commands
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
            batch_update_token_permissions,
            // Permission Management Commands
            list_available_permissions,
            // Tool Manager Commands
            get_tools_info,
            get_tool_info,
            install_all_tools,
            install_tool,
            check_python_runtime,
            get_tool_startup_status,
            // Language Management Commands (temporarily disabled)
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Initialize managers (split from main run function to prevent stack overflow)
async fn initialize_managers(
    storage_manager: crate::storage::StorageManager,
    mcp_client_manager: Arc<McpClientManager>,
    server_config: Arc<ServerConfig>,
    app: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 更新状态：开始数据库连接（实际上已经连接，但开始迁移）
    update_initialization_state(crate::types::InitializationState::DatabaseMigrating).await;

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
        let mut service_manager_guard = SERVICE_MANAGER
            .lock()
            .expect("Failed to acquire SERVICE_MANAGER lock");
        *service_manager_guard = Some(mcp_server_manager.clone());
    }

    // 更新状态：管理器已创建
    update_initialization_state(crate::types::InitializationState::ManagersCreated).await;

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

    // Stage 3: Create and start aggregator immediately (no delay)
    let mcp_server_manager_for_agg = mcp_server_manager.clone();
    let token_manager_for_agg = token_manager.clone();
    let mcp_client_manager_for_agg = mcp_client_manager.clone();
    let server_config_for_agg = server_config.clone();
    let app_for_agg = app.clone();
    tokio::spawn(async move {
        // 立即启动聚合接口，无需等待服务连接完成
        tracing::info!("Starting aggregator initialization immediately");
        create_and_start_aggregator(
            mcp_server_manager_for_agg,
            mcp_client_manager_for_agg,
            server_config_for_agg,
            token_manager_for_agg,
            app_for_agg.clone(),
        )
        .await;
    });

    // Stage 4: Load and connect services in background (fully non-blocking)
    tokio::spawn(async move {
        load_and_connect_services(mcp_server_manager).await;
    });

    // Mark initialization as completed at the manager level
    // Individual services will update their own status as they complete
    update_initialization_state(crate::types::InitializationState::Completed).await;
    tracing::info!("🎉 Manager initialization completed");

    Ok(())
}

// Create and start the MCP aggregator
async fn create_and_start_aggregator(
    mcp_server_manager: Arc<crate::mcp_manager::McpServerManager>,
    mcp_client_manager: Arc<McpClientManager>,
    server_config: Arc<ServerConfig>,
    token_manager: Arc<crate::token_manager::TokenManager>,
    app: tauri::AppHandle,
) {
    tracing::info!("🚀 Creating and starting MCP aggregator");

    // Log aggregator configuration
    tracing::info!("📋 Aggregator configuration:");
    tracing::info!("   - Host: {}", server_config.host);
    tracing::info!("   - Port: {}", server_config.port);
    tracing::info!("   - Auth enabled: {}", server_config.is_auth_enabled());
    tracing::info!("   - Timeout: {}s", server_config.timeout_seconds);
    tracing::info!("   - Max connections: {}", server_config.max_connections);

    // Create the aggregator instance
    tracing::info!("🔧 Creating aggregator instance...");
    let aggregator = aggregator::McpAggregator::new(
        mcp_server_manager,
        mcp_client_manager,
        server_config,
        token_manager,
        app,
    );

    // Store the aggregator in the global variable
    tracing::info!("💾 Storing aggregator instance in global state...");
    {
        let mut aggregator_guard = AGGREGATOR
            .lock()
            .expect("Failed to acquire AGGREGATOR lock");
        *aggregator_guard = Some(Arc::new(aggregator));
    }

    // Start the aggregator HTTP server
    let aggregator_for_start = {
        let guard = AGGREGATOR
            .lock()
            .expect("Failed to acquire AGGREGATOR lock");
        guard.as_ref()
            .expect("AGGREGATOR should contain a value after initialization")
            .clone()
    };

    tracing::info!("🎯 Starting aggregator HTTP server...");
    match aggregator_for_start.start().await {
        Ok(_) => {
            tracing::info!("🎉 MCP Aggregator created and started successfully");

            // Log startup statistics
            let stats = aggregator_for_start.get_statistics().await;
            tracing::info!("📊 Aggregator statistics: {}", stats);
        }
        Err(e) => {
            tracing::error!("❌ Failed to start MCP Aggregator: {}", e);
        }
    }
}

/// Load and connect MCP services
async fn load_and_connect_services(mcp_server_manager: Arc<crate::mcp_manager::McpServerManager>) {
    tracing::info!("🚀 Starting services initialization");

    // Update initialization state: start service loading
    update_initialization_state(crate::types::InitializationState::ServicesLoading).await;

    // Phase 1: Load service configuration (non-blocking)
    let manager_for_load = mcp_server_manager.clone();
    tokio::spawn(async move {
        tracing::info!("📋 Loading service configurations...");
        match manager_for_load.load_mcp_servers().await {
            Ok(_) => {
                tracing::info!("✅ Service configuration loaded successfully");

                // Get server list to track individual service status
                let (servers, _) = manager_for_load
                    .list_servers(None, None)
                    .await
                    .unwrap_or_default();
                tracing::info!(
                    "📊 Found {} MCP servers ({} enabled)",
                    servers.len(),
                    servers.iter().filter(|s| s.enabled).count()
                );

                // Log individual server details
                for server in &servers {
                    tracing::debug!(
                        "   - {}: {} (enabled: {}, transport: {})
",
                        server.name,
                        server.status,
                        server.enabled,
                        server.transport
                    );
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to load services: {}", e);
                return;
            }
        }

        // Update initialization state: start service connection
        update_initialization_state(crate::types::InitializationState::ServicesConnecting).await;

        // Phase 2: Batched connection (fully async execution)
        let manager_for_connection = manager_for_load.clone();
        tokio::spawn(async move {
            tracing::info!("🔌 Phase 1: Starting batched connection to enabled services");

            // Get enabled servers count before connection attempt
            let (servers_before, _) = manager_for_connection
                .list_servers(None, None)
                .await
                .unwrap_or_default();
            let enabled_servers = servers_before.iter().filter(|s| s.enabled).count();
            tracing::info!(
                "   Attempting to connect to {} enabled servers",
                enabled_servers
            );

            if let Err(e) = manager_for_connection
                .auto_connect_enabled_services_batched()
                .await
            {
                // Ignore "Method not found" errors as they are normal for some MCP services
                let error_str = e.to_string();
                if error_str.contains("Method not found") || error_str.contains("-32601") {
                    tracing::debug!(
                        "   ⚠️  Method not found error during batched connection (ignoring): {}",
                        e
                    );
                } else {
                    tracing::error!("   ❌ Batched connection failed: {}", e);
                }
            } else {
                // Log connection results
                let (servers_after, _) = manager_for_connection
                    .list_servers(None, None)
                    .await
                    .unwrap_or_default();
                let connected_count = servers_after
                    .iter()
                    .filter(|s| s.status == "connected")
                    .count();
                tracing::info!(
                    "   ✅ Phase 1 completed: {} of {} services connected",
                    connected_count,
                    enabled_servers
                );
            }
        });

        // Phase 3: Background manifest sync (delayed start, low priority)
        let manager_for_sync = manager_for_load.clone();
        tokio::spawn(async move {
            // Wait for connection tasks to make progress before starting sync
            tracing::info!(
                "⏳ Phase 2: Waiting 3 seconds before starting background manifest sync..."
            );
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            tracing::info!("🔄 Phase 2: Starting background manifest sync");
            if let Err(e) = manager_for_sync.sync_all_manifests_background().await {
                // Ignore "Method not found" errors as they are normal for some MCP services
                let error_str = e.to_string();
                if error_str.contains("Method not found") || error_str.contains("-32601") {
                    tracing::debug!(
                        "   ⚠️  Method not found error during background sync (ignoring): {}",
                        e
                    );
                } else {
                    tracing::error!("   ❌ Background manifest sync startup failed: {}", e);
                }
            } else {
                tracing::info!("   ✅ Phase 2 completed: Manifest sync running in background");
            }
        });
    });

    // Mark initialization as managers created (aggregator can start independently)
    update_initialization_state(crate::types::InitializationState::ManagersCreated).await;
    tracing::info!(
        "✅ Initialization completed at manager level - aggregator can start independently"
    );
}
