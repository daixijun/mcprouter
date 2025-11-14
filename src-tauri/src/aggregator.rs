use crate::error::McpError;
use crate::mcp_client::McpClientManager;
use crate::mcp_manager::McpServerManager;
use crate::types::ServerConfig;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, ErrorCode, InitializeRequestParam, InitializeResult,
    ListPromptsResult, ListResourcesResult, ListToolsResult, PaginatedRequestParam,
    ProtocolVersion, Tool as McpTool,
};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig;
use rmcp::transport::streamable_http_server::tower::StreamableHttpService;
use rmcp::ErrorData as RmcpErrorData;
use rmcp::{handler::server::ServerHandler, service::RequestContext, RoleServer};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// MCP Aggregator Server - implements MCP protocol
#[derive(Clone)]
pub struct McpAggregator {
    mcp_server_manager: Arc<McpServerManager>,
    mcp_client_manager: Arc<McpClientManager>,
    config: Arc<ServerConfig>,
    shutdown_signal: Arc<std::sync::Mutex<Option<CancellationToken>>>,
}

impl McpAggregator {
    pub fn new(
        mcp_server_manager: Arc<McpServerManager>,
        mcp_client_manager: Arc<McpClientManager>,
        config: Arc<ServerConfig>,
    ) -> Self {
        Self {
            mcp_server_manager,
            mcp_client_manager,
            config,
            shutdown_signal: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub async fn start(
        self: &Arc<Self>,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("MCP Aggregator server starting...");

        // Build listen address from config
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        tracing::info!("Starting HTTP server on {}", addr);

        // Clone the Arc to pass to the service factory
        let aggregator_for_service = self.clone();

        // Create session manager
        let session_manager = Arc::new(LocalSessionManager::default());

        // Create service factory that returns aggregator handler directly
        let service_factory = move || Ok(aggregator_for_service.as_ref().clone());

        // Create server config
        let server_config = StreamableHttpServerConfig {
            sse_keep_alive: Some(std::time::Duration::from_secs(self.config.timeout_seconds)),
            stateful_mode: false, // 修改为false，与客户端allow_stateless=true保持一致
        };

        // Create StreamableHttpService
        let service = StreamableHttpService::new(service_factory, session_manager, server_config);

        // 暴露 MCP 接口（不再套用鉴权中间件）
        let router = axum::Router::new().nest_service("/mcp", service);

        // Bind TCP listener
        let tcp_listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            e
        })?;

        // Create cancellation token for graceful shutdown
        let ct = CancellationToken::new();

        // Store cancellation token for later use in trigger_shutdown
        {
            let mut shutdown_guard = self.shutdown_signal.lock().unwrap();
            *shutdown_guard = Some(ct.clone());
        }

        // Spawn server task
        let _server_handle = tokio::spawn({
            let ct = ct.clone();
            async move {
                tracing::info!("MCP Aggregator HTTP server running on {}", addr);
                let result = axum::serve(tcp_listener, router)
                    .with_graceful_shutdown(async move {
                        ct.cancelled_owned().await;
                        tracing::info!("MCP Aggregator server shutting down...");
                    })
                    .await;

                if let Err(e) = result {
                    tracing::error!("Server error: {}", e);
                }
            }
        });

        tracing::info!(
            "MCP Aggregator started successfully on {} (timeout: {}s, max_connections: {})",
            addr,
            self.config.timeout_seconds,
            self.config.max_connections
        );

        Ok(())
    }

    pub async fn get_statistics(&self) -> Value {
        let entries = self.mcp_server_manager.tools_cache_entries.read().await;
        let total = entries.len();
        let ttl = self.mcp_server_manager.get_tools_cache_ttl_seconds();
        let updated_count: usize = entries.values().map(|e| e.count).sum();
        let latest = entries.values().map(|e| e.last_updated).max();
        serde_json::json!({
            "status": "running",
            "message": "Aggregator initialized",
            "tool_cache": { "enabled": true, "entries": total, "ttl_seconds": ttl, "tools_total": updated_count, "last_updated": latest.map(|d| d.to_rfc3339()) }
        })
    }

    /// Get tools directly from memory (with optional sync from config file)
    async fn get_tools_from_memory(&self) -> Result<Vec<McpTool>, McpError> {
        let mut aggregated_tools: Vec<McpTool> = Vec::new();
        let servers_lock = self.mcp_server_manager.get_mcp_servers().await;
        let servers = servers_lock.read().await;
        tracing::info!("Found {} MCP servers in memory", servers.len());
        for (name, config) in servers.iter() {
            tracing::info!(
                "Server '{}' - enabled: {}, transport: {:?}, tools: {}",
                name,
                config.enabled,
                config.transport,
                {
                    let tc = self.mcp_server_manager.tools_cache_entries.read().await;
                    tc.get(name).map(|v| v.count).unwrap_or(0)
                }
            );
        }
        for (server_name, server_config) in servers.iter() {
            if !server_config.enabled {
                continue;
            }
            if let Some(cached) = self
                .mcp_server_manager
                .get_raw_cached_tools(server_name)
                .await
            {
                let mut prefixed = Vec::new();
                for mut tool in cached {
                    let original_name = tool.name.clone();
                    tool.name = format!("{}/{}", server_name, original_name).into();
                    if tool.description.is_none() {
                        tool.description = Some("No description".into());
                    }
                    prefixed.push(tool);
                }
                aggregated_tools.extend(prefixed);
            }
        }
        Ok(aggregated_tools)
    }

    pub async fn trigger_shutdown(&self) {
        tracing::info!("Triggering aggregator shutdown...");

        // Get the cancellation token and trigger shutdown
        let shutdown_guard = self.shutdown_signal.lock().unwrap();
        if let Some(ct) = shutdown_guard.as_ref() {
            ct.cancel();
            tracing::info!("Shutdown signal sent to MCP Aggregator server");
        } else {
            tracing::warn!("No active shutdown signal found for MCP Aggregator");
        }
    }
}

// Minimal ServerHandler implementation to allow compilation
// This is a placeholder that will be enhanced later
impl ServerHandler for McpAggregator {
    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, RmcpErrorData> {
        tracing::info!("Initialize request received");

        Ok(InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities: rmcp::model::ServerCapabilities {
                experimental: None,
                logging: None,
                completions: None,
                prompts: None,
                resources: None,
                tools: Some(rmcp::model::ToolsCapability { list_changed: None }),
            },
            server_info: rmcp::model::Implementation {
                name: "MCP Router Aggregator".to_string(),
                version: "1.0.0".to_string(),
                icons: None,
                title: None,
                website_url: None,
            },
            instructions: None,
        })
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, RmcpErrorData> {
        tracing::info!("List tools request received");

        // Enhanced debugging: Check server memory state
        tracing::info!("=== DIAGNOSTIC START ===");
        tracing::info!("MCP Aggregator list_tools diagnostic:");

        // Check total servers in memory
        let servers_lock = self.mcp_server_manager.get_mcp_servers().await;
        let servers = servers_lock.read().await;
        tracing::info!("Total servers in memory: {}", servers.len());

        // Check each server's status
        let mut enabled_servers = 0;
        let mut connected_servers = 0;

        for (name, config) in servers.iter() {
            let status = self.mcp_client_manager.get_connection_status(name).await;
            tracing::info!(
                "Server '{}': enabled={}, transport={:?}, status={}",
                name,
                config.enabled,
                config.transport,
                status.0
            );

            if config.enabled {
                enabled_servers += 1;
                if status.0 == "connected" {
                    connected_servers += 1;
                }
            }
        }

        tracing::info!(
            "Summary: {} total servers, {} enabled servers, {} connected servers",
            servers.len(),
            enabled_servers,
            connected_servers
        );
        tracing::info!("=== DIAGNOSTIC END ===");

        let mut offset = 0usize;
        if let Some(param) = _request {
            if let Some(cursor) = param.cursor {
                if let Ok(v) = cursor.parse::<usize>() {
                    offset = v;
                } else {
                    return Err(RmcpErrorData::new(
                        ErrorCode(400),
                        "Invalid cursor".to_string(),
                        None,
                    ));
                }
            }
        }
        let page_size = 100usize;
        match self.get_tools_from_memory().await {
            Ok(tools) => {
                let total = tools.len();
                let end = std::cmp::min(offset + page_size, total);
                let slice = if offset < end {
                    tools[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next = if end < total {
                    Some(end.to_string())
                } else {
                    None
                };
                tracing::info!("Successfully listed {} tools", total);
                Ok(ListToolsResult {
                    tools: slice,
                    next_cursor: next,
                })
            }
            Err(e) => {
                tracing::error!("Failed to list tools: {}", e);
                Err(RmcpErrorData::new(
                    ErrorCode(500),
                    format!("Failed to list tools: {}", e),
                    None,
                ))
            }
        }
    }

    // Simplified implementations for remaining methods
    async fn call_tool(
        &self,
        _request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, RmcpErrorData> {
        tracing::warn!("Call tool not implemented yet");
        Err(RmcpErrorData::new(
            ErrorCode(501),
            "Tool execution not implemented".to_string(),
            None,
        ))
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, RmcpErrorData> {
        Err(RmcpErrorData::new(
            ErrorCode(501),
            "System prompts are not supported".to_string(),
            None,
        ))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, RmcpErrorData> {
        Err(RmcpErrorData::new(
            ErrorCode(501),
            "Resources are not supported".to_string(),
            None,
        ))
    }
}
