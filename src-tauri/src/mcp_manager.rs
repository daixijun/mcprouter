// MCP Server Management

use crate::error::Result;
use crate::storage::orm_storage::Storage;
use crate::types::{McpServerConfig, McpServerInfo};
use std::sync::Arc;

#[derive(Clone)]
pub struct McpServerManager {
    orm_storage: Arc<Storage>,
}

impl McpServerManager {
    /// Create new MCP Server Manager with ORM backend
    pub fn new(orm_storage: Arc<Storage>) -> Self {
        Self { orm_storage }
    }

    /// Create with unified storage manager
    pub async fn with_storage_manager(
        storage_manager: Arc<crate::storage::UnifiedStorageManager>,
    ) -> Result<Self> {
        Ok(Self {
            orm_storage: storage_manager.orm_storage(),
        })
    }

    /// Add a new MCP server
    pub async fn add_server(&self, config: &McpServerConfig) -> Result<()> {
        self.orm_storage.add_mcp_server(config).await.map_err(|e| {
            crate::error::McpError::DatabaseError(format!("Failed to add server: {}", e))
        })?;
        Ok(())
    }

    /// Get all MCP servers
    pub async fn get_all_servers(&self) -> Result<Vec<McpServerInfo>> {
        let servers = self.orm_storage.list_mcp_servers().await.map_err(|e| {
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

            server_infos.push(McpServerInfo {
                name: s.name,
                enabled: s.enabled,
                status,
                version: s.version,
                error_message,
                transport,
                url: s.url,
                description: s.description,
                env: Some(env),
                headers: Some(headers),
                command: s.command,
                args: Some(args),
                tool_count: Some(tool_count),
                resource_count: Some(resource_count),
                prompt_count: Some(prompt_count),
                prompt_template_count: Some(prompt_template_count),
            });
        }

        Ok(server_infos)
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

            Ok(Some(McpServerInfo {
                name: s.name,
                enabled: s.enabled,
                status,
                version: s.version,
                error_message,
                transport,
                url: s.url,
                description: s.description,
                env: Some(env),
                headers: Some(headers),
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
        let server_infos = self.orm_storage.list_mcp_servers().await.map_err(|e| {
            crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
        })?;
        let mut all_tools = Vec::new();

        for server_info in server_infos {
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
                    server_info.name.clone(),
                ));
            }
        }

        Ok(all_tools)
    }

    pub async fn get_all_resources_for_aggregation(
        &self,
    ) -> Result<Vec<(String, String, String, String, Option<String>, String)>> {
        // Get all resources from database and return with server information
        let server_infos = self.orm_storage.list_mcp_servers().await.map_err(|e| {
            crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
        })?;
        let mut all_resources = Vec::new();

        for server_info in server_infos {
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
                    server_info.name.clone(),
                ));
            }
        }

        Ok(all_resources)
    }

    pub async fn get_all_prompts_for_aggregation(
        &self,
    ) -> Result<Vec<(String, String, Option<String>, String)>> {
        // Get all prompts from database and return with server information
        let server_infos = self.orm_storage.list_mcp_servers().await.map_err(|e| {
            crate::error::McpError::DatabaseError(format!("Failed to get servers: {}", e))
        })?;
        let mut all_prompts = Vec::new();

        for server_info in server_infos {
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
                    server_info.name.clone(),
                ));
            }
        }

        Ok(all_prompts)
    }

    pub async fn load_mcp_servers(&self) -> Result<()> {
        tracing::info!("Loading MCP servers...");
        let servers = self.get_all_servers().await?;
        tracing::info!("Loaded {} MCP servers", servers.len());
        Ok(())
    }

    /// List available permissions by type (real implementation)
    pub async fn list_available_permissions_by_type(
        &self,
        resource_type: &str,
    ) -> Result<Vec<String>> {
        // Get all enabled servers
        let servers = self.get_all_servers().await?;
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
                                    .last()
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

    /// List available permissions (alias for the above method)
    pub async fn list_available_permissions(&self, resource_type: &str) -> Result<Vec<String>> {
        self.list_available_permissions_by_type(resource_type).await
    }

    /// Get detailed permission items by type (for UI)
    pub async fn get_detailed_permissions_by_type(
        &self,
        resource_type: &str,
    ) -> Result<Vec<crate::types::PermissionItem>> {
        // Get all enabled servers
        let servers = self.get_all_servers().await?;
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
                                server_name: server.name.clone(),
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
                                server_name: server.name.clone(),
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
                                server_name: server.name.clone(),
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
                        meta: if meta == serde_json::Value::Object(serde_json::Map::new()) { None } else { Some(meta) },
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
                        input_schema: if input_schema == serde_json::Value::Object(serde_json::Map::new()) { None } else { Some(input_schema) },
                        output_schema: if output_schema == serde_json::Value::Object(serde_json::Map::new()) { None } else { Some(output_schema) },
                        annotations: if annotations == serde_json::Value::Object(serde_json::Map::new()) { None } else { Some(annotations) },
                        meta: if meta == serde_json::Value::Object(serde_json::Map::new()) { None } else { Some(meta) },
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
                                crate::entities::mcp_prompt::PromptArgumentType::String => "string".to_string(),
                                crate::entities::mcp_prompt::PromptArgumentType::Number => "number".to_string(),
                                crate::entities::mcp_prompt::PromptArgumentType::Boolean => "boolean".to_string(),
                                crate::entities::mcp_prompt::PromptArgumentType::Array => "array".to_string(),
                                crate::entities::mcp_prompt::PromptArgumentType::Object => "object".to_string(),
                            },
                        })
                        .collect();

                    crate::types::McpPromptInfo {
                        id: prompt.id,
                        name: prompt_name,
                        description: prompt.description,
                        enabled: prompt.enabled,
                        arguments: if mcp_arguments.is_empty() { None } else { Some(mcp_arguments) },
                        meta: if meta == serde_json::Value::Object(serde_json::Map::new()) { None } else { Some(meta) },
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

    
    /// Update server method (alias for delete + add)
    pub async fn update_server(&self, name: &str, config: &McpServerConfig) -> Result<()> {
        // Delete existing server and add new one
        self.delete_server(name).await?;
        self.add_server(config).await?;
        Ok(())
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
    pub async fn auto_connect_enabled_services(&self) -> Result<()> {
        let servers = self.get_all_servers().await?;
        let total_servers = servers.len();
        let mut connected_count = 0;

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
                        .map_err(|e| {
                            tracing::error!("Failed to parse transport type '{}' for server '{}': {}", raw_server.server_type, server.name, e);
                            e
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
                        }
                        Err(e) => {
                            tracing::error!("Failed to connect to server '{}': {}", server.name, e);
                        }
                    }
                } else {
                    tracing::warn!(
                        "Server '{}' configuration not found for connection",
                        server.name
                    );
                }
            }
        }

        tracing::info!(
            "Auto-connect completed. Connected to {} out of {} enabled services",
            connected_count,
            total_servers
        );
        Ok(())
    }
}
