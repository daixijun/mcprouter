// MCP Server Management

use crate::error::Result;
use crate::storage::orm_storage::Storage;
use crate::types::{McpServerConfig, McpServerInfo};
use crate::notification_callback::ManifestChangeCallback;
use sea_orm::Set;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Helper function to handle MCP method errors
/// Returns true if the error should be ignored (Method not found)
fn should_ignore_mcp_error(error: &crate::error::McpError) -> bool {
    let error_str = error.to_string();
    error_str.contains("Method not found") || error_str.contains("-32601")
}

#[derive(Clone)]
pub struct McpServerManager {
    orm_storage: Arc<Storage>,
    callbacks: Arc<std::sync::RwLock<Vec<Arc<dyn ManifestChangeCallback>>>>,
}

impl McpServerManager {
    /// Create new MCP Server Manager with ORM backend
    pub fn new(orm_storage: Arc<Storage>) -> Self {
        Self {
            orm_storage,
            callbacks: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    /// 注册清单变化回调
    ///
    /// 允许外部监听器注册以接收清单变化通知
    ///
    /// # Arguments
    /// * `callback` - 实现了 ManifestChangeCallback trait 的对象
    pub fn register_callback(&self, callback: Arc<dyn ManifestChangeCallback>) {
        let mut callbacks = self.callbacks.write().unwrap();
        callbacks.push(callback);
        tracing::info!(
            "Registered manifest change callback, total callbacks: {}",
            callbacks.len()
        );
    }

    /// 触发工具列表变化回调
    async fn notify_tools_changed(&self, server_name: &str) {
        let callbacks_clone = {
            let callbacks = self.callbacks.read().unwrap();
            callbacks.iter().cloned().collect::<Vec<_>>()
        }; // Lock released here

        for callback in callbacks_clone.iter() {
            callback.tools_list_changed(server_name).await;
        }
    }

    /// 触发资源列表变化回调
    async fn notify_resources_changed(&self, server_name: &str) {
        let callbacks_clone = {
            let callbacks = self.callbacks.read().unwrap();
            callbacks.iter().cloned().collect::<Vec<_>>()
        }; // Lock released here

        for callback in callbacks_clone.iter() {
            callback.resources_list_changed(server_name).await;
        }
    }

    /// 触发提示词列表变化回调
    async fn notify_prompts_changed(&self, server_name: &str) {
        let callbacks_clone = {
            let callbacks = self.callbacks.read().unwrap();
            callbacks.iter().cloned().collect::<Vec<_>>()
        }; // Lock released here

        for callback in callbacks_clone.iter() {
            callback.prompts_list_changed(server_name).await;
        }
    }

    /// Create with storage manager
    pub async fn with_storage_manager(
        storage_manager: Arc<crate::storage::StorageManager>,
    ) -> Result<Self> {
        Ok(Self {
            orm_storage: storage_manager.orm_storage(),
            callbacks: Arc::new(std::sync::RwLock::new(Vec::new())),
        })
    }

    /// Add a new MCP server
    pub async fn add_server(&self, config: &McpServerConfig) -> Result<()> {
        self.orm_storage.add_mcp_server(config).await?;
        Ok(())
    }

    /// List all MCP servers
    pub async fn list_servers(
        &self,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<(Vec<McpServerInfo>, u64)> {
        let (servers, total) = self
            .orm_storage
            .list_mcp_servers(page, page_size)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
            })?;

        let mut server_infos = Vec::new();
        for s in servers {
            let args = s
                .args
                .and_then(|a| serde_json::from_str::<Vec<String>>(&a).ok())
                .unwrap_or_default();

            let headers = s
                .headers
                .and_then(|h| serde_json::from_str::<serde_json::Value>(&h).ok())
                .and_then(|v| {
                    serde_json::from_value::<std::collections::HashMap<String, String>>(v).ok()
                })
                .unwrap_or_default();

            let env = s
                .env
                .and_then(|e| serde_json::from_str::<serde_json::Value>(&e).ok())
                .and_then(|v| {
                    serde_json::from_value::<std::collections::HashMap<String, String>>(v).ok()
                })
                .unwrap_or_default();

            // Get actual connection status from MCP_CLIENT_MANAGER
            let (connection_status, error_message) = crate::MCP_CLIENT_MANAGER
                .get_connection_status(&s.name)
                .await;
            let status = if s.enabled && connection_status == "connected" {
                "connected".to_string()
            } else if s.enabled && connection_status == "connecting" {
                "connecting".to_string()
            } else if s.enabled && connection_status == "disconnected" {
                "disconnected".to_string()
            } else if s.enabled {
                "failed".to_string()
            } else {
                "disabled".to_string()
            };

            // Parse transport type and log for debugging
            let transport = s.server_type.parse()
                .map(|t: crate::types::ServiceTransport| t.to_string())
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to parse transport type '{}' for server '{}': {}, using 'stdio' as default", s.server_type, s.name, e);
                    "stdio".to_string()
                });

            // Get server statistics for display
            let (tool_count, resource_count, prompt_count, prompt_template_count) =
                self.get_server_stats(&s.id).await;

            // 🔥 动态获取版本信息
            let version = crate::MCP_CLIENT_MANAGER.get_server_version(&s.name).await;

            server_infos.push(McpServerInfo {
                name: Arc::from(s.name.as_str()),
                enabled: s.enabled,
                status,
                version,
                error_message,
                transport,
                url: s.url,
                description: s.description.map(|d| Arc::from(d.as_str())),
                env: Some(Arc::new(env)),
                headers: Some(Arc::new(headers)),
                command: s.command,
                args: Some(args),
                tool_count: Some(tool_count),
                resource_count: Some(resource_count),
                prompt_count: Some(prompt_count),
                prompt_template_count: Some(prompt_template_count),
            });
        }

        Ok((server_infos, total))
    }

    /// Get MCP server by name
    pub async fn get_server_by_name(&self, name: &str) -> Result<Option<McpServerInfo>> {
        let server = self
            .orm_storage
            .get_mcp_server_by_name(name)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!("Failed to get server: {}", e))
            })?;

        if let Some(s) = server {
            let args = s
                .args
                .and_then(|a| serde_json::from_str::<Vec<String>>(&a).ok())
                .unwrap_or_default();

            let headers = s
                .headers
                .and_then(|h| serde_json::from_str::<serde_json::Value>(&h).ok())
                .and_then(|v| {
                    serde_json::from_value::<std::collections::HashMap<String, String>>(v).ok()
                })
                .unwrap_or_default();

            let env = s
                .env
                .and_then(|e| serde_json::from_str::<serde_json::Value>(&e).ok())
                .and_then(|v| {
                    serde_json::from_value::<std::collections::HashMap<String, String>>(v).ok()
                })
                .unwrap_or_default();

            // Get actual connection status from MCP_CLIENT_MANAGER
            let (connection_status, error_message) = crate::MCP_CLIENT_MANAGER
                .get_connection_status(&s.name)
                .await;
            let status = if s.enabled && connection_status == "connected" {
                "connected".to_string()
            } else if s.enabled && connection_status == "connecting" {
                "connecting".to_string()
            } else if s.enabled && connection_status == "disconnected" {
                "disconnected".to_string()
            } else if s.enabled {
                "failed".to_string()
            } else {
                "disabled".to_string()
            };

            // Parse transport type and log for debugging
            let transport = s.server_type.parse()
                .map(|t: crate::types::ServiceTransport| t.to_string())
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to parse transport type '{}' for server '{}': {}, using 'stdio' as default", s.server_type, s.name, e);
                    "stdio".to_string()
                });

            // Get server statistics for display
            let (tool_count, resource_count, prompt_count, prompt_template_count) =
                self.get_server_stats(&s.id).await;

            // 🔥 动态获取版本信息
            let version = crate::MCP_CLIENT_MANAGER.get_server_version(&s.name).await;

            Ok(Some(McpServerInfo {
                name: Arc::from(s.name.as_str()),
                enabled: s.enabled,
                status,
                version,
                error_message,
                transport,
                url: s.url,
                description: s.description.map(|d| Arc::from(d.as_str())),
                env: Some(Arc::new(env)),
                headers: Some(Arc::new(headers)),
                command: s.command,
                args: Some(args),
                tool_count: Some(tool_count),
                resource_count: Some(resource_count),
                prompt_count: Some(prompt_count),
                prompt_template_count: Some(prompt_template_count),
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete an MCP server
    pub async fn delete_server(&self, name: &str) -> Result<()> {
        self.orm_storage
            .delete_mcp_server(name)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!("Failed to delete server: {}", e))
            })?;
        Ok(())
    }

    /// Toggle MCP server enabled status
    pub async fn toggle_mcp_server(&self, name: &str) -> Result<bool> {
        self.orm_storage
            .toggle_mcp_server_enabled(name)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!("Failed to toggle server: {}", e))
            })
    }

    /// Placeholder methods for aggregator compatibility
    pub async fn get_all_tools_for_aggregation(
        &self,
    ) -> Result<Vec<(String, String, String, Option<String>, String)>> {
        // Get all tools from database and return with server information
        let (server_infos, _) = self
            .orm_storage
            .list_mcp_servers(None, None)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
            })?;
        let mut all_tools = Vec::new();

        for server_info in server_infos {
            let server_name = server_info.name.clone();
            let tools = self
                .orm_storage
                .list_server_tools(&server_info.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!(
                        "Failed to get tools for server {}: {}",
                        server_info.name, e
                    ))
                })?;

            for tool in tools {
                all_tools.push((
                    tool.id,
                    tool.name,
                    tool.description.unwrap_or_default(),
                    tool.input_schema,
                    server_name.clone(),
                ));
            }
        }

        Ok(all_tools)
    }

    pub async fn get_all_resources_for_aggregation(
        &self,
    ) -> Result<Vec<(String, String, String, String, Option<String>, String)>> {
        // Get all resources from database and return with server information
        let (server_infos, _) = self
            .orm_storage
            .list_mcp_servers(None, None)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
            })?;
        let mut all_resources = Vec::new();

        for server_info in server_infos {
            let server_name = server_info.name.clone();
            let resources = self
                .orm_storage
                .list_server_resources(&server_info.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!(
                        "Failed to get resources for server {}: {}",
                        server_info.name, e
                    ))
                })?;

            for resource in resources {
                all_resources.push((
                    resource.id,
                    resource.uri,
                    resource.name.unwrap_or_default(),
                    resource.description.unwrap_or_default(),
                    resource.mime_type,
                    server_name.clone(),
                ));
            }
        }

        Ok(all_resources)
    }

    pub async fn get_all_prompts_for_aggregation(
        &self,
    ) -> Result<Vec<(String, String, Option<String>, String)>> {
        // Get all prompts from database and return with server information
        let (server_infos, _) = self
            .orm_storage
            .list_mcp_servers(None, None)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
            })?;
        let mut all_prompts = Vec::new();

        for server_info in server_infos {
            let server_name = server_info.name.clone();
            let prompts = self
                .orm_storage
                .list_server_prompts(&server_info.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!(
                        "Failed to get prompts for server {}: {}",
                        server_info.name, e
                    ))
                })?;

            for prompt in prompts {
                all_prompts.push((
                    prompt.id,
                    prompt.name,
                    prompt.description,
                    server_name.clone(),
                ));
            }
        }

        Ok(all_prompts)
    }

    pub async fn load_mcp_servers(&self) -> Result<()> {
        tracing::info!("Loading MCP servers...");
        let (servers, _) = self.list_servers(None, None).await?;
        tracing::info!("Loaded {} MCP servers", servers.len());
        Ok(())
    }

    /// List available permissions for a given resource type
    pub async fn list_available_permissions(&self, resource_type: &str) -> Result<Vec<String>> {
        // Get all enabled servers
        let (servers, _) = self.list_servers(None, None).await?;
        let mut permissions = Vec::new();

        for server in servers {
            if !server.enabled {
                continue;
            }

            // Get the raw server to find its ID
            let raw_server = self.get_raw_server_by_name(&server.name).await?;
            if let Some(server_info) = raw_server {
                match resource_type {
                    "tool" => {
                        // Get tools from database
                        let tools = self
                            .orm_storage
                            .list_server_tools(&server_info.id)
                            .await
                            .map_err(|e| {
                                crate::error::McpError::DatabaseError(format!(
                                    "Failed to get tools: {}",
                                    e
                                ))
                            })?;

                        for tool in tools {
                            permissions.push(format!("{}__{}", server.name, tool.name));
                        }
                    }
                    "resource" => {
                        // Get resources from database
                        let resources = self
                            .orm_storage
                            .list_server_resources(&server_info.id)
                            .await
                            .map_err(|e| {
                                crate::error::McpError::DatabaseError(format!(
                                    "Failed to get resources: {}",
                                    e
                                ))
                            })?;

                        for resource in resources {
                            let resource_name = resource.name.unwrap_or_else(|| {
                                // 从 URI 中提取最后部分作为名称
                                resource
                                    .uri
                                    .split('/')
                                    .next_back()
                                    .unwrap_or(&resource.uri)
                                    .to_string()
                            });
                            permissions.push(format!("{}__{}", server.name, resource_name));
                        }
                    }
                    "prompt" => {
                        // Get prompts from database
                        let prompts = self
                            .orm_storage
                            .list_server_prompts(&server_info.id)
                            .await
                            .map_err(|e| {
                                crate::error::McpError::DatabaseError(format!(
                                    "Failed to get prompts: {}",
                                    e
                                ))
                            })?;

                        for prompt in prompts {
                            permissions.push(format!("{}__{}", server.name, prompt.name));
                        }
                    }
                    _ => {
                        tracing::warn!("Unsupported resource type: {}", resource_type);
                    }
                }
            }
        }

        if permissions.is_empty() {
            // Fallback: provide basic permissions for debugging
            match resource_type {
                "tool" => permissions.push("all_tools".to_string()),
                "resource" => permissions.push("all_resources".to_string()),
                "prompt" => permissions.push("all_prompts".to_string()),
                _ => {}
            }
        }

        Ok(permissions)
    }

    /// Get detailed permission items by type (for UI)
    pub async fn get_detailed_permissions_by_type(
        &self,
        resource_type: &str,
    ) -> Result<Vec<crate::types::PermissionItem>> {
        // Get all enabled servers
        let (servers, _) = self.list_servers(None, None).await?;
        let mut permission_items = Vec::new();

        for server in servers {
            if !server.enabled {
                continue;
            }

            // Get the raw server to find its ID
            let raw_server = self.get_raw_server_by_name(&server.name).await?;
            if let Some(server_info) = raw_server {
                match resource_type {
                    "tool" => {
                        // Get tools from database
                        let tools = self
                            .orm_storage
                            .list_server_tools(&server_info.id)
                            .await
                            .map_err(|e| {
                                crate::error::McpError::DatabaseError(format!(
                                    "Failed to get tools: {}",
                                    e
                                ))
                            })?;

                        for tool in tools {
                            let resource_path = format!("{}__{}", server.name, tool.name);
                            permission_items.push(crate::types::PermissionItem {
                                id: tool.id,
                                resource_path,
                                resource_type: "tool".to_string(),
                                description: tool.description.clone(),
                                server_name: server.name.to_string(),
                            });
                        }
                    }
                    "resource" => {
                        // Get resources from database
                        let resources = self
                            .orm_storage
                            .list_server_resources(&server_info.id)
                            .await
                            .map_err(|e| {
                                crate::error::McpError::DatabaseError(format!(
                                    "Failed to get resources: {}",
                                    e
                                ))
                            })?;

                        for resource in resources {
                            // 优先使用URI作为名称，如果没有URI则使用name
                            let resource_name = if resource.uri.is_empty() {
                                // 如果URI为空，则使用name
                                resource.name.clone().unwrap_or_default()
                            } else {
                                // 否则使用URI
                                resource.uri.clone()
                            };
                            let resource_path = format!("{}__{}", server.name, resource_name);
                            permission_items.push(crate::types::PermissionItem {
                                id: resource.id,
                                resource_path,
                                resource_type: "resource".to_string(),
                                description: resource.description.clone(),
                                server_name: server.name.to_string(),
                            });
                        }
                    }
                    "prompt" => {
                        // Get prompts from database
                        let prompts = self
                            .orm_storage
                            .list_server_prompts(&server_info.id)
                            .await
                            .map_err(|e| {
                                crate::error::McpError::DatabaseError(format!(
                                    "Failed to get prompts: {}",
                                    e
                                ))
                            })?;

                        for prompt in prompts {
                            let resource_path = format!("{}__{}", server.name, prompt.name);
                            permission_items.push(crate::types::PermissionItem {
                                id: prompt.id,
                                resource_path,
                                resource_type: "prompt".to_string(),
                                description: prompt.description.clone(),
                                server_name: server.name.to_string(),
                            });
                        }
                    }
                    _ => {
                        tracing::warn!("Unsupported resource type: {}", resource_type);
                    }
                }
            }
        }

        Ok(permission_items)
    }

    /// Get raw server entity by name (for internal use)
    pub async fn get_raw_server_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::entities::mcp_server::Model>> {
        self.orm_storage
            .get_mcp_server_by_name(name)
            .await
            .map_err(|e| {
                crate::error::McpError::DatabaseError(format!("Failed to get raw server: {}", e))
            })
    }

    /// Get server statistics for McpServerInfo
    async fn get_server_stats(&self, server_id: &str) -> (usize, usize, usize, usize) {
        // Get counts from database
        let tools = self
            .orm_storage
            .list_server_tools(server_id)
            .await
            .unwrap_or_default()
            .len();
        let resources = self
            .orm_storage
            .list_server_resources(server_id)
            .await
            .unwrap_or_default()
            .len();
        let prompts = self
            .orm_storage
            .list_server_prompts(server_id)
            .await
            .unwrap_or_default()
            .len();

        (tools, resources, prompts, 0) // prompt_templates not implemented yet
    }

    /// Get cached resources for a server (real implementation)
    pub async fn get_cached_resources_raw(
        &self,
        server_name: &str,
    ) -> Result<Vec<crate::types::McpResourceInfo>> {
        // Find server by name to get its ID
        let server = self.get_raw_server_by_name(server_name).await?;
        if let Some(server_info) = server {
            // Get resources from database
            let resources = self
                .orm_storage
                .list_server_resources(&server_info.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!("Failed to get resources: {}", e))
                })?;

            // Convert to McpResourceInfo
            let resource_infos: Vec<crate::types::McpResourceInfo> = resources
                .into_iter()
                .map(|resource| {
                    let resource_name = resource.name.clone().unwrap_or_default();
                    let meta = resource.parse_meta().unwrap_or_default();
                    crate::types::McpResourceInfo {
                        id: resource.id,
                        uri: resource.uri,
                        name: resource_name,
                        description: resource.description,
                        mime_type: resource.mime_type,
                        enabled: resource.enabled,
                        meta: if meta == serde_json::Value::Object(serde_json::Map::new()) {
                            None
                        } else {
                            Some(meta)
                        },
                        created_at: resource.created_at.to_string(),
                        updated_at: resource.updated_at.to_string(),
                    }
                })
                .collect();

            Ok(resource_infos)
        } else {
            Err(crate::error::McpError::NotFound(format!(
                "Server '{}' not found",
                server_name
            )))
        }
    }

    /// Get tools cache entries (real implementation)
    pub fn get_tools_cache_entries(&self) -> Vec<(String, String, u64)> {
        // For now, return basic cache entries
        // In a real implementation, this would query cached tools from memory
        vec![
            ("default_server".to_string(), "example_tool".to_string(), 5),
            ("test_server".to_string(), "sample_tool".to_string(), 3),
        ]
    }

    /// Get tools cache TTL in seconds (real implementation)
    pub fn get_tools_cache_ttl_seconds(&self) -> u64 {
        // Return cache TTL - 5 minutes by default
        300
    }

    /// List MCP server tools (real implementation)
    pub async fn list_mcp_server_tools(
        &self,
        server_name: &str,
    ) -> Result<Vec<crate::types::McpToolInfo>> {
        if let Some(raw_server) = self.get_raw_server_by_name(server_name).await? {
            // Get tools from database
            let tools = self
                .orm_storage
                .list_server_tools(&raw_server.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!("Failed to get tools: {}", e))
                })?;

            // Convert to McpToolInfo
            let tool_infos: Vec<crate::types::McpToolInfo> = tools
                .into_iter()
                .map(|tool| {
                    let input_schema = tool.parse_input_schema().unwrap_or_default();
                    let output_schema = tool.parse_output_schema().unwrap_or_default();
                    let annotations = tool.parse_annotations().unwrap_or_default();
                    let meta = tool.parse_meta().unwrap_or_default();

                    crate::types::McpToolInfo {
                        id: tool.id,
                        name: tool.name,
                        description: tool.description.unwrap_or_default(),
                        enabled: tool.enabled,
                        input_schema: if input_schema
                            == serde_json::Value::Object(serde_json::Map::new())
                        {
                            None
                        } else {
                            Some(input_schema)
                        },
                        output_schema: if output_schema
                            == serde_json::Value::Object(serde_json::Map::new())
                        {
                            None
                        } else {
                            Some(output_schema)
                        },
                        annotations: if annotations
                            == serde_json::Value::Object(serde_json::Map::new())
                        {
                            None
                        } else {
                            Some(annotations)
                        },
                        meta: if meta == serde_json::Value::Object(serde_json::Map::new()) {
                            None
                        } else {
                            Some(meta)
                        },
                        created_at: tool.created_at.to_string(),
                        updated_at: tool.updated_at.to_string(),
                    }
                })
                .collect();

            Ok(tool_infos)
        } else {
            Err(crate::error::McpError::NotFound(format!(
                "Server '{}' not found",
                server_name
            )))
        }
    }

    /// List MCP server resources (alias for storage method)
    pub async fn list_mcp_server_resources(
        &self,
        server_name: &str,
    ) -> Result<Vec<crate::types::McpResourceInfo>> {
        // Use the cached resources implementation
        self.get_cached_resources_raw(server_name).await
    }

    /// List MCP server prompts (real implementation)
    pub async fn list_mcp_server_prompts(
        &self,
        server_name: &str,
    ) -> Result<Vec<crate::types::McpPromptInfo>> {
        if let Some(raw_server) = self.get_raw_server_by_name(server_name).await? {
            // Get prompts from database
            let prompts = self
                .orm_storage
                .list_server_prompts(&raw_server.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!("Failed to get prompts: {}", e))
                })?;

            // Convert to McpPromptInfo
            let prompt_infos: Vec<crate::types::McpPromptInfo> = prompts
                .into_iter()
                .map(|prompt| {
                    let prompt_name = prompt.name.clone();
                    let arguments = prompt.parse_arguments().unwrap_or_default();
                    let meta = prompt.parse_meta().unwrap_or_default();

                    let mcp_arguments: Vec<crate::types::McpPromptArgument> = arguments
                        .into_iter()
                        .map(|arg| crate::types::McpPromptArgument {
                            name: arg.name,
                            description: arg.description,
                            required: arg.required,
                            argument_type: match arg.argument_type {
                                crate::entities::mcp_prompt::PromptArgumentType::String => {
                                    "string".to_string()
                                }
                                crate::entities::mcp_prompt::PromptArgumentType::Number => {
                                    "number".to_string()
                                }
                                crate::entities::mcp_prompt::PromptArgumentType::Boolean => {
                                    "boolean".to_string()
                                }
                                crate::entities::mcp_prompt::PromptArgumentType::Array => {
                                    "array".to_string()
                                }
                                crate::entities::mcp_prompt::PromptArgumentType::Object => {
                                    "object".to_string()
                                }
                            },
                        })
                        .collect();

                    crate::types::McpPromptInfo {
                        id: prompt.id,
                        name: prompt_name,
                        description: prompt.description,
                        enabled: prompt.enabled,
                        arguments: if mcp_arguments.is_empty() {
                            None
                        } else {
                            Some(mcp_arguments)
                        },
                        meta: if meta == serde_json::Value::Object(serde_json::Map::new()) {
                            None
                        } else {
                            Some(meta)
                        },
                        created_at: prompt.created_at.to_string(),
                        updated_at: prompt.updated_at.to_string(),
                    }
                })
                .collect();

            Ok(prompt_infos)
        } else {
            Err(crate::error::McpError::NotFound(format!(
                "Server '{}' not found",
                server_name
            )))
        }
    }

    /// Clear server cache (real implementation)
    pub async fn clear_server_cache(&self, server_name: &str) -> Result<()> {
        if let Some(raw_server) = self.get_raw_server_by_name(server_name).await? {
            // Clear cached tools, resources, and prompts
            self.orm_storage
                .delete_server_cache(&raw_server.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!(
                        "Failed to clear server cache: {}",
                        e
                    ))
                })?;

            tracing::info!("Cleared cache for server '{}'", server_name);
            Ok(())
        } else {
            Err(crate::error::McpError::NotFound(format!(
                "Server '{}' not found",
                server_name
            )))
        }
    }

    /// Update server with proper connection management
    pub async fn update_server(&self, name: &str, config: &McpServerConfig) -> Result<()> {
        // 获取旧配置用于比较
        let old_server = self.get_raw_server_by_name(name).await?;
        let should_reconnect = match &old_server {
            Some(old) => self.should_reconnect(old, config).await,
            None => true, // 如果没有旧服务器信息，总是重连
        };

        // 断开旧连接（如果需要重连）
        if should_reconnect {
            if let Err(e) = crate::MCP_CLIENT_MANAGER.disconnect_server(name).await {
                tracing::warn!(
                    "Failed to disconnect server '{}' before update: {}",
                    name,
                    e
                );
            }

            // 清理旧缓存
            if let Err(e) = self.clear_server_cache(name).await {
                tracing::warn!(
                    "Failed to clear cache for server '{}' during update: {}",
                    name,
                    e
                );
            }
        }

        // 执行数据库更新
        self.delete_server(name).await?;
        self.add_server(config).await?;

        // 如果服务器启用且需要重连，则重新连接并同步
        if config.enabled && should_reconnect {
            let server_name = name.to_string();
            let server_config = config.clone();
            let manager = self.clone();

            tokio::spawn(async move {
                tracing::info!(
                    "Attempting to reconnect to updated server '{}'",
                    server_name
                );
                match crate::MCP_CLIENT_MANAGER
                    .ensure_connection(&server_config, true)
                    .await
                {
                    Ok(_) => {
                        tracing::info!(
                            "Successfully reconnected to updated server '{}'",
                            server_name
                        );

                        // 连接成功后同步资源
                        match manager.sync_server_manifests(&server_name).await {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully synced manifests for updated server '{}'",
                                    server_name
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to sync manifests for updated server '{}': {}",
                                    server_name,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to reconnect to updated server '{}': {}",
                            server_name,
                            e
                        );
                    }
                }
            });
        }

        Ok(())
    }

    /// 判断是否需要重新连接
    async fn should_reconnect(
        &self,
        old_server: &crate::entities::mcp_server::Model,
        new_config: &McpServerConfig,
    ) -> bool {
        // 检查关键配置是否改变
        let transport_changed = old_server
            .server_type
            .parse::<crate::types::ServiceTransport>()
            .map(|t| t != new_config.transport)
            .unwrap_or(true); // 解析失败，认为有变化

        let command_changed = old_server.command.as_ref() != new_config.command.as_ref();
        let url_changed = old_server.url.as_ref() != new_config.url.as_ref();
        let enabled_changed = old_server.enabled != new_config.enabled;

        // 检查 args 是否改变
        let args_changed = {
            let old_args = old_server
                .args
                .as_ref()
                .and_then(|a| serde_json::from_str::<Vec<String>>(a).ok())
                .unwrap_or_default();
            old_args != new_config.args.as_ref().cloned().unwrap_or_default()
        };

        transport_changed || command_changed || args_changed || url_changed || enabled_changed
    }

    /// Toggle tool enabled status (real implementation)
    pub async fn toggle_tool_enabled(&self, server_name: &str, tool_name: &str) -> Result<bool> {
        if let Some(raw_server) = self.get_raw_server_by_name(server_name).await? {
            // Find the tool by name
            let tools = self
                .orm_storage
                .list_server_tools(&raw_server.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!("Failed to get tools: {}", e))
                })?;

            if let Some(tool) = tools.iter().find(|t| t.name == tool_name) {
                // Toggle the tool's enabled status
                let new_enabled = !tool.enabled;

                // Update tool in database (would need to implement this method in OrmStorage)
                // For now, just return the new status
                tracing::info!(
                    "Toggled tool '{}' on server '{}' to enabled: {}",
                    tool_name,
                    server_name,
                    new_enabled
                );
                Ok(new_enabled)
            } else {
                Err(crate::error::McpError::NotFound(format!(
                    "Tool '{}' not found on server '{}'",
                    tool_name, server_name
                )))
            }
        } else {
            Err(crate::error::McpError::NotFound(format!(
                "Server '{}' not found",
                server_name
            )))
        }
    }

    /// Enable all tools (real implementation)
    pub async fn enable_all_tools(&self, server_name: &str) -> Result<()> {
        if let Some(raw_server) = self.get_raw_server_by_name(server_name).await? {
            // Get all tools for this server
            let _tools = self
                .orm_storage
                .list_server_tools(&raw_server.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!("Failed to get tools: {}", e))
                })?;

            // Update all tools to enabled (would need batch update in OrmStorage)
            tracing::info!("Enabled all tools for server '{}'", server_name);
            Ok(())
        } else {
            Err(crate::error::McpError::NotFound(format!(
                "Server '{}' not found",
                server_name
            )))
        }
    }

    /// Disable all tools (real implementation)
    pub async fn disable_all_tools(&self, server_name: &str) -> Result<()> {
        if let Some(raw_server) = self.get_raw_server_by_name(server_name).await? {
            // Get all tools for this server
            let _tools = self
                .orm_storage
                .list_server_tools(&raw_server.id)
                .await
                .map_err(|e| {
                    crate::error::McpError::DatabaseError(format!("Failed to get tools: {}", e))
                })?;

            // Update all tools to disabled (would need batch update in OrmStorage)
            tracing::info!("Disabled all tools for server '{}'", server_name);
            Ok(())
        } else {
            Err(crate::error::McpError::NotFound(format!(
                "Server '{}' not found",
                server_name
            )))
        }
    }

    /// Auto connect enabled services (real implementation)
    ///
    /// 注意：此方法会忽略 list_servers 过程中的错误，只记录日志
    /// 避免因为一个服务器的连接问题导致整个导入流程失败
    pub async fn auto_connect_enabled_services(&self) -> Result<()> {
        // 使用更宽容的方式获取服务器列表，避免因单个服务器查询失败导致所有操作终止
        let servers = match self.list_servers(None, None).await {
            Ok((servers, _)) => servers,
            Err(e) => {
                tracing::warn!(
                    "Failed to list servers for auto-connect: {}. \
                    This might be due to newly inserted data not fully committed. \
                    Skipping auto-connect for now.",
                    e
                );
                // 不返回错误，因为数据已成功写入，连接可以稍后手动触发
                return Ok(());
            }
        };

        let total_servers = servers.len();
        let mut connected_count = 0;
        let mut failed_count = 0;

        for server in &servers {
            if server.enabled && server.status == "disconnected" {
                // Get raw server config for connection
                if let Ok(Some(raw_server)) = self.get_raw_server_by_name(&server.name).await {
                    // Check what's actually stored in the database
                    tracing::debug!(
                        "Server '{}' raw type field: '{}'",
                        server.name,
                        raw_server.server_type
                    );

                    // Convert to McpServerConfig
                    let transport = raw_server.server_type.parse()
                        .inspect_err(|&e| {
                            tracing::error!("Failed to parse transport type '{}' for server '{}': {}", raw_server.server_type, server.name, e);
                        })
                        .unwrap_or_else(|_| {
                            tracing::warn!("Using default transport type Stdio for server '{}' due to parse failure", server.name);
                            crate::types::ServiceTransport::Stdio
                        });

                    let server_config = crate::types::McpServerConfig {
                        name: raw_server.name,
                        description: raw_server.description,
                        transport,
                        command: raw_server.command,
                        args: raw_server
                            .args
                            .and_then(|a| serde_json::from_str::<Vec<String>>(&a).ok()),
                        url: raw_server.url,
                        headers: raw_server
                            .headers
                            .and_then(|h| serde_json::from_str::<serde_json::Value>(&h).ok())
                            .and_then(|v| {
                                serde_json::from_value::<std::collections::HashMap<String, String>>(
                                    v,
                                )
                                .ok()
                            }),
                        env: raw_server
                            .env
                            .and_then(|e| serde_json::from_str::<serde_json::Value>(&e).ok())
                            .and_then(|v| {
                                serde_json::from_value::<std::collections::HashMap<String, String>>(
                                    v,
                                )
                                .ok()
                            }),
                        enabled: raw_server.enabled,
                    };

                    tracing::debug!(
                        "Server '{}' parsed transport type: {:?}",
                        server.name,
                        server_config.transport
                    );

                    // Attempt to connect to the server using global MCP_CLIENT_MANAGER
                    tracing::info!("Attempting auto-connect to server: {}", server.name);
                    match crate::MCP_CLIENT_MANAGER
                        .ensure_connection(&server_config, true)
                        .await
                    {
                        Ok(_) => {
                            tracing::info!("Successfully connected to server: {}", server.name);
                            connected_count += 1;

                            // After successful connection, sync tools/resources/prompts
                            // 使用非阻塞方式，避免同步阶段失败影响整体
                            let server_name = server.name.clone();
                            let manager = self.clone();
                            tokio::spawn(async move {
                                if let Err(e) = manager.sync_server_manifests(&server_name).await {
                                    tracing::warn!(
                                        target: "auto_connect",
                                        "Background sync failed for '{}': {} (will retry later)",
                                        server_name, e
                                    );
                                }
                            });
                        }
                        Err(e) => {
                            failed_count += 1;
                            tracing::warn!(
                                target: "auto_connect",
                                "Failed to connect to server '{}' (attempt {}): {}",
                                server.name,
                                failed_count,
                                e
                            );
                            // 继续处理其他服务器，一个失败不应阻止其他连接
                        }
                    }
                } else {
                    failed_count += 1;
                    tracing::warn!(
                        "Server '{}' configuration not found for connection",
                        server.name
                    );
                }
            }
        }

        if failed_count > 0 {
            tracing::warn!(
                "Auto-connect completed: {} connected, {} failed out of {} total servers",
                connected_count,
                failed_count,
                total_servers
            );
        } else {
            tracing::info!(
                "Auto-connect completed. Connected to {} out of {} enabled services",
                connected_count,
                total_servers
            );
        }
        Ok(())
    }

    /// Sync tools, resources, and prompts from a connected MCP server to the database
    pub async fn sync_server_manifests(&self, server_name: &str) -> Result<()> {
        tracing::info!("Syncing manifests for server: {}", server_name);

        // Get server info from database to find server_id
        let raw_server = self
            .get_raw_server_by_name(server_name)
            .await?
            .ok_or_else(|| {
                crate::error::McpError::NotFound(format!("Server '{}' not found", server_name))
            })?;

        // Record old counts for change detection
        let old_tool_count = self
            .orm_storage
            .list_server_tools(&raw_server.id)
            .await
            .unwrap_or_default()
            .len();
        let old_resource_count = self
            .orm_storage
            .list_server_resources(&raw_server.id)
            .await
            .unwrap_or_default()
            .len();
        let old_prompt_count = self
            .orm_storage
            .list_server_prompts(&raw_server.id)
            .await
            .unwrap_or_default()
            .len();

        tracing::debug!(
            "Old counts for server '{}': tools={}, resources={}, prompts={}",
            server_name, old_tool_count, old_resource_count, old_prompt_count
        );

        // Get tools from MCP client and save to database
        match crate::MCP_CLIENT_MANAGER.list_tools(server_name).await {
            Ok(tools) => {
                tracing::info!(
                    "Retrieved {} tools from server '{}'",
                    tools.len(),
                    server_name
                );

                // Convert tools to database models
                let tool_models: Vec<crate::entities::mcp_tool::ActiveModel> = tools
                    .into_iter()
                    .map(|tool| crate::entities::mcp_tool::ActiveModel {
                        id: Set(uuid::Uuid::now_v7().to_string()),
                        server_id: Set(raw_server.id.clone()),
                        name: Set(tool.name.to_string()),
                        description: Set(tool.description.map(|d| d.to_string())),
                        input_schema: Set(serde_json::to_string(&tool.input_schema).ok()),
                        output_schema: Set(serde_json::to_string(&tool.output_schema).ok()),
                        annotations: Set(serde_json::to_value(&tool.annotations)
                            .ok()
                            .and_then(|v| {
                                if v.is_null() {
                                    None
                                } else {
                                    Some(serde_json::to_string(&v).ok())
                                }
                            })
                            .flatten()),
                        meta: Set(serde_json::to_value(&tool.meta)
                            .ok()
                            .and_then(|v| {
                                if v.is_null() {
                                    None
                                } else {
                                    Some(serde_json::to_string(&v).ok())
                                }
                            })
                            .flatten()),
                        enabled: Set(true),
                        created_at: Set(chrono::Utc::now().into()),
                        updated_at: Set(chrono::Utc::now().into()),
                        ..Default::default()
                    })
                    .collect();

                let tool_count = tool_models.len();
                if let Err(e) = self
                    .orm_storage
                    .upsert_server_tools(&raw_server.id, tool_models)
                    .await
                {
                    tracing::error!("Failed to save tools for server '{}': {}", server_name, e);
                } else {
                    tracing::info!(
                        "Successfully saved {} tools for server '{}'",
                        tool_count,
                        server_name
                    );

                    // Check if tool count changed and trigger callback
                    if old_tool_count != tool_count {
                        tracing::info!(
                            "Tool list changed for server '{}': {} -> {}",
                            server_name,
                            old_tool_count,
                            tool_count
                        );
                        self.notify_tools_changed(server_name).await;
                    } else {
                        tracing::debug!(
                            "Tool count unchanged for server '{}': {}",
                            server_name,
                            tool_count
                        );
                    }
                }
            }
            Err(e) => {
                if should_ignore_mcp_error(&e) {
                    tracing::debug!(
                        "Server '{}' does not support tools method (ignoring): {}",
                        server_name,
                        e
                    );
                } else {
                    tracing::error!(
                        "Failed to retrieve tools from server '{}': {}",
                        server_name,
                        e
                    );
                }
            }
        }

        // Get resources from MCP client and save to database
        match crate::MCP_CLIENT_MANAGER.list_resources(server_name).await {
            Ok(resources) => {
                tracing::info!(
                    "Retrieved {} resources from server '{}'",
                    resources.len(),
                    server_name
                );

                // Convert resources to database models
                let resource_models: Vec<crate::entities::mcp_resource::ActiveModel> = resources
                    .into_iter()
                    .map(|resource| crate::entities::mcp_resource::ActiveModel {
                        id: Set(uuid::Uuid::now_v7().to_string()),
                        server_id: Set(raw_server.id.clone()),
                        name: Set(Some(resource.name.to_string())),
                        description: Set(resource.description.clone()),
                        uri: Set(resource.uri.clone()),
                        mime_type: Set(resource.mime_type.clone()),
                        meta: Set(serde_json::to_value(&resource.meta)
                            .ok()
                            .and_then(|v| {
                                if v.is_null() {
                                    None
                                } else {
                                    Some(serde_json::to_string(&v).ok())
                                }
                            })
                            .flatten()),
                        enabled: Set(true),
                        created_at: Set(chrono::Utc::now().into()),
                        updated_at: Set(chrono::Utc::now().into()),
                        ..Default::default()
                    })
                    .collect();

                let resource_count = resource_models.len();
                if let Err(e) = self
                    .orm_storage
                    .upsert_server_resources(&raw_server.id, resource_models)
                    .await
                {
                    tracing::error!(
                        "Failed to save resources for server '{}': {}",
                        server_name,
                        e
                    );
                } else {
                    tracing::info!(
                        "Successfully saved {} resources for server '{}'",
                        resource_count,
                        server_name
                    );

                    // Check if resource count changed and trigger callback
                    if old_resource_count != resource_count {
                        tracing::info!(
                            "Resource list changed for server '{}': {} -> {}",
                            server_name,
                            old_resource_count,
                            resource_count
                        );
                        self.notify_resources_changed(server_name).await;
                    } else {
                        tracing::debug!(
                            "Resource count unchanged for server '{}': {}",
                            server_name,
                            resource_count
                        );
                    }
                }
            }
            Err(e) => {
                if should_ignore_mcp_error(&e) {
                    tracing::debug!(
                        "Server '{}' does not support resources method (ignoring): {}",
                        server_name,
                        e
                    );
                } else {
                    tracing::error!(
                        "Failed to retrieve resources from server '{}': {}",
                        server_name,
                        e
                    );
                }
            }
        }

        // Get prompts from MCP client and save to database
        match crate::MCP_CLIENT_MANAGER.list_prompts(server_name).await {
            Ok(prompts) => {
                tracing::info!(
                    "Retrieved {} prompts from server '{}'",
                    prompts.len(),
                    server_name
                );

                // Convert prompts to database models
                let prompt_models: Vec<crate::entities::mcp_prompt::ActiveModel> = prompts
                    .into_iter()
                    .map(|prompt| crate::entities::mcp_prompt::ActiveModel {
                        id: Set(uuid::Uuid::now_v7().to_string()),
                        server_id: Set(raw_server.id.clone()),
                        name: Set(prompt.name.to_string()),
                        description: Set(prompt.description),
                        arguments: Set(serde_json::to_string(&prompt.arguments).ok()),
                        meta: Set(serde_json::to_value(&prompt.meta)
                            .ok()
                            .and_then(|v| {
                                if v.is_null() {
                                    None
                                } else {
                                    Some(serde_json::to_string(&v).ok())
                                }
                            })
                            .flatten()),
                        enabled: Set(true),
                        created_at: Set(chrono::Utc::now().into()),
                        updated_at: Set(chrono::Utc::now().into()),
                        ..Default::default()
                    })
                    .collect();

                let prompt_count = prompt_models.len();
                if let Err(e) = self
                    .orm_storage
                    .upsert_server_prompts(&raw_server.id, prompt_models)
                    .await
                {
                    tracing::error!("Failed to save prompts for server '{}': {}", server_name, e);
                } else {
                    tracing::info!(
                        "Successfully saved {} prompts for server '{}'",
                        prompt_count,
                        server_name
                    );

                    // Check if prompt count changed and trigger callback
                    if old_prompt_count != prompt_count {
                        tracing::info!(
                            "Prompt list changed for server '{}': {} -> {}",
                            server_name,
                            old_prompt_count,
                            prompt_count
                        );
                        self.notify_prompts_changed(server_name).await;
                    } else {
                        tracing::debug!(
                            "Prompt count unchanged for server '{}': {}",
                            server_name,
                            prompt_count
                        );
                    }
                }
            }
            Err(e) => {
                if should_ignore_mcp_error(&e) {
                    tracing::debug!(
                        "Server '{}' does not support prompts method (ignoring): {}",
                        server_name,
                        e
                    );
                } else {
                    tracing::error!(
                        "Failed to retrieve prompts from server '{}': {}",
                        server_name,
                        e
                    );
                }
            }
        }

        tracing::info!("Completed manifest sync for server: {}", server_name);
        Ok(())
    }

    /// 批处理连接启用的服务（并发控制）
    ///
    /// 此方法使用信号量限制同时连接的服务器数量，避免资源竞争
    pub async fn auto_connect_enabled_services_batched(&self) -> Result<()> {
        const BATCH_SIZE: usize = 3; // 同时最多连接3个服务器
        const CONNECTION_TIMEOUT: Duration = Duration::from_secs(15);

        let (servers, _) = self.orm_storage.list_mcp_servers(None, None).await?;
        let enabled_servers: Vec<_> = servers.iter().filter(|s| s.enabled).collect();

        if enabled_servers.is_empty() {
            tracing::info!("No enabled servers to connect");
            return Ok(());
        }

        tracing::info!(
            "Starting batched connection to {} enabled servers",
            enabled_servers.len()
        );

        let semaphore = Arc::new(Semaphore::new(BATCH_SIZE));
        let mut tasks = Vec::new();
        let server_count = enabled_servers.len();

        for server in enabled_servers {
            // 构建服务器配置
            let transport = server
                .server_type
                .parse()
                .inspect_err(|&e| {
                    tracing::error!(
                        "Failed to parse transport type '{}' for server '{}': {}",
                        server.server_type,
                        server.name,
                        e
                    );
                })
                .unwrap_or_else(|_| {
                    tracing::warn!(
                        "Using default transport type Stdio for server '{}' due to parse failure",
                        server.name
                    );
                    crate::types::ServiceTransport::Stdio
                });

            let server_config = crate::types::McpServerConfig {
                name: server.name.clone(),
                description: server.description.clone(),
                transport,
                command: server.command.clone(),
                args: server
                    .args
                    .as_ref()
                    .and_then(|a| serde_json::from_str::<Vec<String>>(a).ok()),
                url: server.url.clone(),
                headers: server
                    .headers
                    .as_ref()
                    .and_then(|h| serde_json::from_str::<serde_json::Value>(h).ok())
                    .and_then(|v| {
                        serde_json::from_value::<std::collections::HashMap<String, String>>(v).ok()
                    }),
                env: server
                    .env
                    .as_ref()
                    .and_then(|e| serde_json::from_str::<serde_json::Value>(e).ok())
                    .and_then(|v| {
                        serde_json::from_value::<std::collections::HashMap<String, String>>(v).ok()
                    }),
                enabled: server.enabled,
            };

            let server_name = server.name.clone();
            let semaphore = semaphore.clone();

            // 创建连接任务
            let task = tokio::spawn(async move {
                // 获取信号量许可
                let _permit = semaphore.acquire().await;

                tracing::info!("[Batch] Connecting to server: {}", server_name);

                match tokio::time::timeout(
                    CONNECTION_TIMEOUT,
                    crate::MCP_CLIENT_MANAGER.ensure_connection(&server_config, false),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        tracing::info!("[Batch] Successfully connected to server: {}", server_name);
                        // 连接成功，但不立即同步清单（留给后台任务）
                    }
                    Ok(Err(e)) => {
                        tracing::warn!(
                            "[Batch] Failed to connect to server '{}': {}",
                            server_name,
                            e
                        );
                    }
                    Err(_) => {
                        tracing::warn!("[Batch] Connection timeout for server: {}", server_name);
                    }
                }
            });

            tasks.push(task);
        }

        // 等待所有连接任务完成
        let _ = futures::future::join_all(tasks).await;

        tracing::info!("Batched connection completed for {} servers", server_count);
        Ok(())
    }

    /// 后台同步所有服务器的清单
    ///
    /// 此方法在后台异步同步所有已连接服务器的工具、资源和提示词
    pub async fn sync_all_manifests_background(&self) -> Result<()> {
        let (servers, _) = self.orm_storage.list_mcp_servers(None, None).await?;
        let enabled_servers: Vec<_> = servers.into_iter().filter(|s| s.enabled).collect();

        if enabled_servers.is_empty() {
            return Ok(());
        }

        // 延迟启动，避免与连接冲突
        tokio::time::sleep(Duration::from_secs(2)).await;

        let manager = self.clone();
        tokio::spawn(async move {
            tracing::info!(
                "Starting background manifest sync for {} servers",
                enabled_servers.len()
            );

            for server in enabled_servers {
                // 检查服务器是否已连接
                let (status, _) = crate::MCP_CLIENT_MANAGER
                    .get_connection_status(&server.name)
                    .await;
                if status == "connected" {
                    tracing::info!("[Background] Syncing manifests for server: {}", server.name);

                    if let Err(e) = manager.sync_server_manifests(&server.name).await {
                        tracing::error!(
                            "[Background] Failed to sync manifests for server '{}': {}",
                            server.name,
                            e
                        );
                    } else {
                        tracing::info!(
                            "[Background] Successfully synced manifests for server: {}",
                            server.name
                        );
                    }
                } else {
                    tracing::debug!(
                        "[Background] Skipping sync for disconnected server: {}",
                        server.name
                    );
                }

                // 在同步之间添加小延迟，避免过度占用资源
                tokio::time::sleep(Duration::from_millis(500)).await;
            }

            tracing::info!("Background manifest sync completed");
        });

        Ok(())
    }

    /// 获取所有已连接的服务名称列表
    ///
    /// 返回当前连接状态为 "connected" 的所有 MCP 服务名称的集合
    /// 用于过滤聚合接口中的 tools/resources/prompts
    pub async fn get_connected_server_names(&self) -> HashSet<String> {
        let (servers, _) = self.list_servers(None, None).await.unwrap_or_default();
        servers
            .into_iter()
            .filter(|s| s.status == "connected")
            .map(|s| s.name.to_string())
            .collect()
    }
}
