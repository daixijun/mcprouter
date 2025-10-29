use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

use crate::aggregator::McpAggregator;
use crate::config::AppConfig;
use crate::error::{McpError, Result};
use crate::mcp_client::McpClientManager;
use crate::mcp_manager::McpServerManager;

/// Unified application state manager
/// This centralizes all state management and provides a single point of coordination
pub struct AppState {
    /// Core configuration
    config: RwLock<AppConfig>,

    /// MCP server management
    mcp_server_manager: Arc<McpServerManager>,

    /// MCP client connections
    mcp_client_manager: Arc<McpClientManager>,

    /// Aggregator for unified MCP access
    aggregator: Arc<McpAggregator>,

    /// Runtime state cache
    runtime_state: RwLock<RuntimeState>,

    /// Database connection pool
    db_pool: Arc<sqlx::SqlitePool>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &"[RwLock<AppConfig>]")
            .field("mcp_server_manager", &"[Arc<McpServerManager>]")
            .field("mcp_client_manager", &"[Arc<McpClientManager>]")
            .field("aggregator", &"[Arc<McpAggregator>]")
            .field("runtime_state", &self.runtime_state)
            .field("db_pool", &"[Arc<SqlitePool>]")
            .finish()
    }
}

/// Runtime state that changes during application execution
#[derive(Debug, Default)]
pub struct RuntimeState {
    /// Active aggregator task handle
    aggregator_handle: Option<tokio::task::JoinHandle<()>>,

    /// Cached dashboard statistics
    dashboard_stats_cache: Option<DashboardStats>,

    /// Last statistics update time
    last_stats_update: Option<std::time::Instant>,

    /// Connection state tracking
    active_connections: HashMap<String, ConnectionInfo>,
}

/// Information about active connections
/// TODO: Implement connection tracking functionality
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub service_id: String,
    pub connected_at: std::time::SystemTime,
    pub is_active: bool,
}

/// Dashboard statistics for caching
#[derive(Debug, Clone, serde::Serialize)]
pub struct DashboardStats {
    // Basic statistics
    pub total_servers: usize,
    pub enabled_servers: usize,
    pub disabled_servers: usize,
    pub connected_services: usize,
    pub total_tools: u32,
    pub active_clients: usize,
    pub startup_time: String, // ISO 8601 string for JSON serialization

    // System information
    pub os_info: OsInfo,

    // Service details
    pub services: ServiceStats,

    // Tool details
    pub tools: ToolStats,

    // Connection details
    pub connections: ConnectionStats,

    // Aggregator information
    pub aggregator: AggregatorStats,
}

/// Operating system information
#[derive(Debug, Clone, serde::Serialize)]
pub struct OsInfo {
    pub platform: String,
    pub r#type: String, // "type" is a reserved keyword
    pub version: String,
    pub arch: String,
}

/// Service statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceStats {
    pub total: usize,
    pub enabled: usize,
    pub disabled: usize,
}

/// Tool statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolStats {
    pub total_count: u32,
}

/// Connection statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectionStats {
    pub active_clients: usize,
    pub active_services: usize,
}

/// Aggregator statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct AggregatorStats {
    pub endpoint: String,
    pub is_running: bool,
    pub connected_services: usize,
    pub max_connections: usize,
    pub timeout_seconds: u64,
}

impl AppState {
    /// Create new application state with unified initialization
    pub async fn new() -> Result<Self> {
        tracing::info!("Initializing unified application state");

        // Load configuration once and share
        let config = AppConfig::load().map_err(|e| {
            tracing::error!("Failed to load configuration: {}", e);
            e
        })?;

        // Initialize database connection pool
        let db_pool = Self::create_db_pool().await?;

        // Create core managers with shared configuration
        let mcp_server_manager = Arc::new(McpServerManager::new(config.clone()));
        let mcp_client_manager = Arc::new(McpClientManager::new(config.clone()));
        let aggregator = Arc::new(McpAggregator::new(
            mcp_server_manager.clone(), // Use the same McpServerManager instance!
            config.server.clone(),
        ));

        let state = Self {
            config: RwLock::new(config),
            mcp_server_manager,
            mcp_client_manager,
            aggregator,
            runtime_state: RwLock::new(RuntimeState::default()),
            db_pool,
        };

        tracing::info!("Application state initialized successfully");
        Ok(state)
    }

    /// Create new application state with AppHandle for proper database path
    pub async fn new_with_app_handle(app_handle: &tauri::AppHandle) -> Result<Self> {
        tracing::info!("Initializing unified application state with AppHandle");

        // Load configuration once and share
        let config = AppConfig::load().map_err(|e| {
            tracing::error!("Failed to load configuration: {}", e);
            McpError::ConfigError(format!("Failed to load configuration: {}", e))
        })?;

        // Initialize database connection pool using AppHandle
        let db_pool = Self::create_db_pool_with_handle(app_handle).await?;

        // Create core managers with shared configuration
        let mcp_server_manager = Arc::new(McpServerManager::new(config.clone()));
        let mcp_client_manager = Arc::new(McpClientManager::new(config.clone()));
        let aggregator = Arc::new(McpAggregator::new(
            mcp_server_manager.clone(), // Use the same McpServerManager instance!
            config.server.clone(),
        ));

        let state = Self {
            config: RwLock::new(config),
            mcp_server_manager,
            mcp_client_manager,
            aggregator,
            runtime_state: RwLock::new(RuntimeState::default()),
            db_pool,
        };

        tracing::info!("Application state initialized successfully with AppHandle");
        Ok(state)
    }

    /// Create database connection pool for better performance
    async fn create_db_pool() -> Result<Arc<sqlx::SqlitePool>> {
        let db_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".mcprouter")
            .join("mcprouter.db");

        tracing::info!("Database path: {}", db_path.display());

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            tracing::info!("Creating directory: {}", parent.display());
            std::fs::create_dir_all(parent).map_err(|e| {
                McpError::DatabaseInitializationError(format!(
                    "Failed to create database directory: {}",
                    e
                ))
            })?;
        }

        let connection_string = format!("sqlite:{}", db_path.display());
        tracing::info!("Connection string: {}", connection_string);

        // Create connection options with proper SQLite settings
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(&connection_string)
            .map_err(|e| {
                McpError::DatabaseInitializationError(format!("Invalid database URL: {}", e))
            })?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        // Create connection pool with optimal settings
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(10) // Limit concurrent connections
            .min_connections(2) // Keep some connections ready
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(600)) // 10 minutes
            .connect_with(options)
            .await
            .map_err(|e| {
                McpError::DatabaseConnectionError(format!("Failed to create database pool: {}", e))
            })?;

        tracing::info!(
            "Database connection pool created successfully with {} max connections",
            pool.size()
        );
        Ok(Arc::new(pool))
    }

    /// Create database connection pool using AppHandle for proper app data directory
    async fn create_db_pool_with_handle(
        app_handle: &tauri::AppHandle,
    ) -> Result<Arc<sqlx::SqlitePool>> {
        // Get the app data directory (same as working connection.rs)
        let app_data_dir = app_handle.path().app_data_dir().map_err(|e| {
            McpError::DatabaseInitializationError(format!(
                "Failed to get app data directory: {}",
                e
            ))
        })?;

        // Ensure directory exists
        std::fs::create_dir_all(&app_data_dir).map_err(|e| {
            McpError::DatabaseInitializationError(format!(
                "Failed to create app data directory: {}",
                e
            ))
        })?;

        let db_path = app_data_dir.join("mcprouter.db");
        let db_url = format!("sqlite:{}", db_path.display());

        tracing::info!("Database path (with handle): {}", db_path.display());
        tracing::info!("Connection string (with handle): {}", db_url);

        // Create connection options with proper SQLite settings
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| {
                McpError::DatabaseInitializationError(format!("Invalid database URL: {}", e))
            })?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        // Create connection pool with optimal settings
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(10) // Limit concurrent connections
            .min_connections(2) // Keep some connections ready
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(600)) // 10 minutes
            .connect_with(options)
            .await
            .map_err(|e| {
                McpError::DatabaseConnectionError(format!("Failed to create database pool: {}", e))
            })?;

        tracing::info!(
            "Database connection pool created successfully with {} max connections",
            pool.size()
        );
        Ok(Arc::new(pool))
    }

    /// Get configuration reference
    pub async fn get_config(&self) -> AppConfig {
        tracing::debug!("get_config: Attempting to acquire read lock");
        let config_lock = self.config.read().await;
        tracing::debug!("get_config: Successfully acquired read lock");
        let result = config_lock.clone();
        tracing::debug!("get_config: Successfully cloned config, releasing lock");
        result
    }

    /// Update configuration with atomic operation
    pub async fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        tracing::debug!("update_config: Attempting to acquire write lock");
        let mut config = self.config.write().await;
        tracing::debug!("update_config: Successfully acquired write lock");

        // 保存旧配置用于比较
        let old_config = config.clone();
        updater(&mut config);

        // 克隆新配置以在锁外进行其他操作
        let new_config = config.clone();
        tracing::debug!("update_config: Config updated, cloning for sync operations");

        // 尽早释放锁
        drop(config);
        tracing::debug!("update_config: Released write lock");

        // Save to disk (在锁外操作)
        tracing::debug!("update_config: Saving config to disk");
        new_config.save()?;

        // Update dependent managers if needed (在锁外操作，传入旧配置用于比较)
        tracing::debug!("update_config: Starting sync to managers");
        self.sync_config_to_managers_with_old_config(&old_config, &new_config)
            .await?;

        tracing::info!("Configuration updated and synchronized");
        Ok(())
    }

    /// Synchronize configuration changes to all managers
    async fn sync_config_to_managers(&self, config: &AppConfig) -> Result<()> {
        // Note: This method should be avoided as it can cause deadlocks
        // Use sync_config_to_managers_with_old_config instead
        tracing::warn!("sync_config_to_managers: Using potentially deadlock-prone method");
        let current_config = self.get_config().await;
        self.sync_config_to_managers_with_old_config(&current_config, config)
            .await
    }

    /// Synchronize configuration changes to all managers (with old config provided to avoid deadlock)
    async fn sync_config_to_managers_with_old_config(
        &self,
        old_config: &AppConfig,
        new_config: &AppConfig,
    ) -> Result<()> {
        // Update manager configurations
        tracing::info!("Updating manager configurations...");

        // Update McpServerManager configuration
        self.mcp_server_manager
            .update_config(|config| *config = new_config.clone())
            .await?;

        // Note: Aggregator restart is now handled by the caller (save_settings) to avoid duplicate restarts
        if self.needs_aggregator_restart_with_configs(old_config, new_config) {
            tracing::info!(
                "Server configuration changed, aggregator restart will be handled by caller"
            );
        }

        Ok(())
    }

    /// Check if aggregator needs restart due to config changes (avoiding additional lock acquisition)
    fn needs_aggregator_restart_with_configs(
        &self,
        old_config: &AppConfig,
        new_config: &AppConfig,
    ) -> bool {
        old_config.server.host != new_config.server.host
            || old_config.server.port != new_config.server.port
            || old_config.server.max_connections != new_config.server.max_connections
            || old_config.server.timeout_seconds != new_config.server.timeout_seconds
    }

    /// Check if aggregator needs restart due to config changes
    async fn needs_aggregator_restart(&self, new_config: &AppConfig) -> bool {
        let current_config = self.config.read().await;
        current_config.server.host != new_config.server.host
            || current_config.server.port != new_config.server.port
            || current_config.server.max_connections != new_config.server.max_connections
            || current_config.server.timeout_seconds != new_config.server.timeout_seconds
    }

    /// Get MCP server manager
    pub fn mcp_server_manager(&self) -> &Arc<McpServerManager> {
        &self.mcp_server_manager
    }

    /// Get MCP client manager
    pub fn mcp_client_manager(&self) -> &Arc<McpClientManager> {
        &self.mcp_client_manager
    }

    /// Get aggregator
    pub fn aggregator(&self) -> &Arc<McpAggregator> {
        &self.aggregator
    }

    /// Get database connection pool
    pub fn db_pool(&self) -> &Arc<sqlx::SqlitePool> {
        &self.db_pool
    }

    /// Add or update connection tracking
    pub async fn track_connection(&self, service_id: String, is_active: bool) {
        let mut runtime_state = self.runtime_state.write().await;

        let connection_info = ConnectionInfo {
            service_id: service_id.clone(),
            connected_at: std::time::SystemTime::now(),
            is_active,
        };

        runtime_state
            .active_connections
            .insert(service_id.clone(), connection_info);
        tracing::debug!("Tracked connection for service: {}", service_id);
    }

    /// Remove connection tracking
    pub async fn remove_connection_tracking(&self, service_id: &str) {
        let mut runtime_state = self.runtime_state.write().await;
        runtime_state.active_connections.remove(service_id);
        tracing::debug!("Removed connection tracking for service: {}", service_id);
    }

    /// Get active connection statistics
    pub async fn get_connection_stats(&self) -> HashMap<String, ConnectionInfo> {
        let runtime_state = self.runtime_state.read().await;
        runtime_state.active_connections.clone()
    }

    /// Get active connection count
    pub async fn get_active_connection_count(&self) -> usize {
        let runtime_state = self.runtime_state.read().await;
        runtime_state
            .active_connections
            .values()
            .filter(|c| c.is_active)
            .count()
    }

    /// Start aggregator with proper state tracking
    pub async fn start_aggregator(&self) -> Result<()> {
        let mut runtime_state = self.runtime_state.write().await;

        // Stop existing aggregator if running
        if let Some(handle) = runtime_state.aggregator_handle.take() {
            handle.abort();
        }

        // Start new aggregator
        let aggregator = self.aggregator.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = aggregator.start().await {
                tracing::error!("Aggregator failed: {}", e);
            }
        });

        runtime_state.aggregator_handle = Some(handle);
        tracing::info!("Aggregator started successfully");
        Ok(())
    }

    /// Stop aggregator gracefully
    pub async fn stop_aggregator(&self) -> Result<()> {
        let mut runtime_state = self.runtime_state.write().await;

        if let Some(handle) = runtime_state.aggregator_handle.take() {
            handle.abort();
            self.aggregator.trigger_shutdown().await;
            tracing::info!("Aggregator stopped");
        }

        Ok(())
    }

    /// Trigger aggregator shutdown (for config changes)
    pub async fn trigger_aggregator_shutdown(&self) {
        self.aggregator.trigger_shutdown().await;
    }

    /// Get cached dashboard statistics or compute fresh ones
    pub async fn get_dashboard_stats(&self, force_refresh: bool) -> Result<DashboardStats> {
        let mut runtime_state = self.runtime_state.write().await;

        // Check if we need to refresh (cache expired or forced)
        let need_refresh = force_refresh
            || runtime_state.dashboard_stats_cache.is_none()
            || runtime_state.last_stats_update.map_or(true, |update| {
                update.elapsed() > std::time::Duration::from_secs(5)
            });

        if need_refresh {
            tracing::debug!("Computing fresh dashboard statistics");

            // Get services
            let services = self
                .mcp_server_manager
                .list_mcp_servers()
                .await
                .unwrap_or_default();
            let enabled_servers = services.iter().filter(|s| s.enabled).count();
            let total_tools = services
                .iter()
                .map(|s| s.tool_count.unwrap_or(0) as u32)
                .sum::<u32>();

            // Get active connections
            let connections = self.mcp_client_manager.get_connections().await;

            // Get aggregator statistics
            let aggregator_stats = self.aggregator.get_statistics().await;
            let connected_services = aggregator_stats
                .get("connected_services")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let startup_time = {
                let duration_since_epoch = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(
                    duration_since_epoch.as_secs() as i64,
                    0,
                )
                .unwrap_or_default();
                datetime.to_rfc3339()
            };

            // Get system information (will be updated in Tauri command)
            let os_info = OsInfo {
                platform: std::env::consts::OS.to_string(),
                r#type: std::env::consts::FAMILY.to_string(),
                version: "Unknown".to_string(), // Will be updated by Tauri os plugin
                arch: std::env::consts::ARCH.to_string(),
            };

            // Service statistics
            let service_stats = ServiceStats {
                total: services.len(),
                enabled: enabled_servers,
                disabled: services.len() - enabled_servers,
            };

            // Tool statistics
            let tool_stats = ToolStats {
                total_count: total_tools,
            };

            // Connection statistics
            let connection_stats = ConnectionStats {
                active_clients: connections.len(),
                active_services: connected_services,
            };

            // Aggregator statistics
            let aggregator_stats = AggregatorStats {
                endpoint: aggregator_stats
                    .get("endpoint")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                is_running: aggregator_stats
                    .get("is_running")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                connected_services: aggregator_stats
                    .get("connected_services")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                max_connections: aggregator_stats
                    .get("max_connections")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                timeout_seconds: aggregator_stats
                    .get("timeout_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30),
            };

            let stats = DashboardStats {
                total_servers: services.len(),
                enabled_servers,
                disabled_servers: services.len() - enabled_servers,
                connected_services,
                total_tools,
                active_clients: connections.len(),
                startup_time,
                os_info,
                services: service_stats,
                tools: tool_stats,
                connections: connection_stats,
                aggregator: aggregator_stats,
            };

            runtime_state.dashboard_stats_cache = Some(stats.clone());
            runtime_state.last_stats_update = Some(std::time::Instant::now());

            Ok(stats)
        } else {
            // Return cached statistics
            Ok(runtime_state.dashboard_stats_cache.clone().unwrap())
        }
    }

    /// Perform health check on all components
    pub async fn health_check(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut health = HashMap::new();

        // Database health
        match self.db_pool.acquire().await {
            Ok(_) => {
                health.insert(
                    "database".to_string(),
                    serde_json::json!({
                        "status": "healthy",
                        "connections": self.db_pool.size()
                    }),
                );
            }
            Err(e) => {
                health.insert(
                    "database".to_string(),
                    serde_json::json!({
                        "status": "unhealthy",
                        "error": e.to_string()
                    }),
                );
            }
        }

        // MCP Server Manager health
        let services = self
            .mcp_server_manager
            .list_mcp_servers()
            .await
            .unwrap_or_default();
        health.insert(
            "mcp_server_manager".to_string(),
            serde_json::json!({
                "status": "healthy",
                "services": services.len()
            }),
        );

        // MCP Client Manager health
        let connections = self.mcp_client_manager.get_connections().await;
        health.insert(
            "mcp_client_manager".to_string(),
            serde_json::json!({
                "status": "healthy",
                "active_connections": connections.len()
            }),
        );

        // Aggregator health
        let aggregator_stats = self.aggregator.get_statistics().await;
        health.insert(
            "aggregator".to_string(),
            serde_json::json!({
                "status": "healthy",
                "stats": aggregator_stats
            }),
        );

        Ok(health)
    }

    /// Graceful shutdown of all components
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Starting graceful shutdown of application state");

        // Stop aggregator first
        self.stop_aggregator().await?;

        // Disconnect all MCP clients
        let connections = self.mcp_client_manager.get_connections().await;
        for connection in connections {
            if let Err(e) = self
                .mcp_client_manager
                .disconnect_mcp_server(&connection.service_id)
                .await
            {
                tracing::warn!("Failed to disconnect {}: {}", connection.service_id, e);
            }
        }

        // Clear connection tracking
        {
            let mut runtime_state = self.runtime_state.write().await;
            runtime_state.active_connections.clear();
            tracing::info!("Cleared all connection tracking");
        }

        // Close database connection pool
        self.db_pool.close().await;

        tracing::info!("Application state shutdown complete");
        Ok(())
    }

    /// Get system health status
    pub async fn get_health_status(&self) -> HashMap<String, serde_json::Value> {
        let mut health = HashMap::new();

        // Check database connection
        match self.db_pool.acquire().await {
            Ok(_) => {
                health.insert(
                    "database".to_string(),
                    serde_json::json!({
                        "status": "healthy",
                        "connections": self.db_pool.size()
                    }),
                );
            }
            Err(e) => {
                health.insert(
                    "database".to_string(),
                    serde_json::json!({
                        "status": "unhealthy",
                        "error": e.to_string()
                    }),
                );
            }
        }

        // Check MCP connections
        let connections = self.mcp_client_manager.get_connections().await;
        let active_count = connections.len();
        health.insert(
            "mcp_connections".to_string(),
            serde_json::json!({
                "status": if active_count > 0 { "healthy" } else { "idle" },
                "active_connections": active_count
            }),
        );

        // Check aggregator
        let aggregator_stats = self.aggregator.get_statistics().await;
        health.insert(
            "aggregator".to_string(),
            serde_json::json!({
                "status": "running",
                "stats": aggregator_stats
            }),
        );

        // Connection tracking info
        let connection_stats = self.get_connection_stats().await;
        health.insert(
            "connection_tracking".to_string(),
            serde_json::json!({
                "status": "active",
                "tracked_connections": connection_stats.len()
            }),
        );

        health
    }

    /// Cleanup old connections
    pub async fn cleanup_old_connections(&self, max_age: std::time::Duration) -> usize {
        let mut runtime_state = self.runtime_state.write().await;
        let mut removed_count = 0;
        let now = std::time::SystemTime::now();

        runtime_state
            .active_connections
            .retain(|service_id, connection_info| {
                let age = now
                    .duration_since(connection_info.connected_at)
                    .unwrap_or_default();
                if age > max_age {
                    tracing::debug!("Removing old connection tracking for: {}", service_id);
                    removed_count += 1;
                    false
                } else {
                    true
                }
            });

        if removed_count > 0 {
            tracing::info!("Cleaned up {} old connection entries", removed_count);
        }

        removed_count
    }
}

/// Initialize application state and return Arc
pub async fn initialize_app_state() -> Result<Arc<AppState>> {
    let state = Arc::new(AppState::new().await?);
    tracing::info!("Application state initialized and ready");
    Ok(state)
}

/// Initialize application state with AppHandle for proper database path
pub async fn initialize_app_state_with_handle(
    app_handle: &tauri::AppHandle,
) -> Result<Arc<AppState>> {
    let state = Arc::new(AppState::new_with_app_handle(app_handle).await?);
    tracing::info!("Application state initialized and ready");
    Ok(state)
}

/// Global application state instance (will be set during initialization)
static GLOBAL_APP_STATE: std::sync::OnceLock<Arc<AppState>> = std::sync::OnceLock::new();

/// Set global application state (call once during initialization)
pub fn set_global_app_state(state: Arc<AppState>) {
    GLOBAL_APP_STATE
        .set(state)
        .expect("Application state already initialized");
}

/// Get global application state
pub fn get_global_app_state() -> &'static Arc<AppState> {
    GLOBAL_APP_STATE
        .get()
        .expect("Application state not initialized")
}
