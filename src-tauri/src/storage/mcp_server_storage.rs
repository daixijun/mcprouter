#![allow(dead_code)]

use super::{Result, StorageError};
use crate::types::{
    McpPromptInfo, McpResourceInfo, McpServerConfig, McpToolInfo, ServiceTransport,
};
use chrono::Utc;
use serde_json;
use sqlx::{Row, SqlitePool};
use uuid;

/// MCP Server Storage implementation using SQLite
pub struct McpServerStorage {
    pub pool: SqlitePool,
}

impl McpServerStorage {
    /// Create new MCP server storage with database pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize database tables for MCP server management
    /// Note: Tables should be created using SQLx migrations, this method only applies optimizations
    pub async fn init(&self) -> Result<()> {
        // Apply SQLite performance optimizations
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set WAL mode: {}", e)))?;

        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to set synchronous mode: {}", e))
            })?;

        sqlx::query("PRAGMA cache_size = 10000")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set cache size: {}", e)))?;

        sqlx::query("PRAGMA temp_store = memory")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set temp store: {}", e)))?;

        sqlx::query("PRAGMA mmap_size = 268435456")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to set mmap size: {}", e)))?;

        tracing::info!("✅ MCP server storage initialized with performance optimizations");
        Ok(())
    }

    // ============================================================================
    // MCP Server Configuration Management
    // ============================================================================

    /// Get all MCP server configurations
    pub async fn get_all_servers(&self) -> Result<Vec<McpServerConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, type, command, args, url, headers, env, enabled
            FROM mcp_servers
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to fetch all MCP servers: {}", e)))?;

        let mut servers = Vec::new();
        for row in rows {
            let transport_str: String = row.get("type");
            let transport = match transport_str.as_str() {
                "stdio" => ServiceTransport::Stdio,
                "http" => ServiceTransport::Http,
                _ => {
                    return Err(StorageError::InvalidData(format!(
                        "Invalid transport type: {}",
                        transport_str
                    )))
                }
            };

            let args: Option<String> = row.get("args");
            let args = args.map(|s| serde_json::from_str(&s).unwrap_or_default());

            let headers: Option<String> = row.get("headers");
            let headers = headers.map(|s| serde_json::from_str(&s).unwrap_or_default());

            let env: Option<String> = row.get("env");
            let env = env.map(|s| serde_json::from_str(&s).unwrap_or_default());

            servers.push(McpServerConfig {
                name: row.get("name"),
                description: row.get("description"),
                command: row.get("command"),
                args,
                env,
                transport,
                url: row.get("url"),
                headers,
                enabled: row.get("enabled"),
            });
        }

        Ok(servers)
    }

    /// Get MCP server configuration by name
    pub async fn get_server_by_name(&self, name: &str) -> Result<Option<McpServerConfig>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, type, command, args, url, headers, env, enabled
            FROM mcp_servers
            WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch MCP server '{}': {}", name, e))
        })?;

        if let Some(row) = row {
            let transport_str: String = row.get("type");
            let transport = match transport_str.as_str() {
                "stdio" => ServiceTransport::Stdio,
                "http" => ServiceTransport::Http,
                _ => {
                    return Err(StorageError::InvalidData(format!(
                        "Invalid transport type: {}",
                        transport_str
                    )))
                }
            };

            let args: Option<String> = row.get("args");
            let args = args.map(|s| serde_json::from_str(&s).unwrap_or_default());

            let headers: Option<String> = row.get("headers");
            let headers = headers.map(|s| serde_json::from_str(&s).unwrap_or_default());

            let env: Option<String> = row.get("env");
            let env = env.map(|s| serde_json::from_str(&s).unwrap_or_default());

            Ok(Some(McpServerConfig {
                name: row.get("name"),
                description: row.get("description"),
                command: row.get("command"),
                args,
                env,
                transport,
                url: row.get("url"),
                headers,
                enabled: row.get("enabled"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get server ID by server name
    pub async fn get_server_id_by_name(&self, name: &str) -> Result<String> {
        let row = sqlx::query(
            r#"
            SELECT id FROM mcp_servers WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch server ID for '{}': {}", name, e))
        })?;

        if let Some(row) = row {
            Ok(row.get("id"))
        } else {
            Err(StorageError::NotFound(format!("Server '{}' not found", name)))
        }
    }

    /// Add new MCP server configuration
    pub async fn add_server(&self, config: &McpServerConfig) -> Result<String> {
        let server_id = uuid::Uuid::new_v4().to_string();
        let transport_str = match config.transport {
            ServiceTransport::Stdio => "stdio",
            ServiceTransport::Http => "http",
        };

        let args_json = config
            .args
            .as_ref()
            .map(|a| serde_json::to_string(a).unwrap_or_default());
        let headers_json = config
            .headers
            .as_ref()
            .map(|h| serde_json::to_string(h).unwrap_or_default());
        let env_json = config
            .env
            .as_ref()
            .map(|e| serde_json::to_string(e).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT INTO mcp_servers (id, name, description, type, command, args, url, headers, env, enabled)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&server_id)
        .bind(&config.name)
        .bind(&config.description)
        .bind(transport_str)
        .bind(&config.command)
        .bind(args_json)
        .bind(&config.url)
        .bind(headers_json)
        .bind(env_json)
        .bind(config.enabled)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                StorageError::AlreadyExists(format!("MCP server '{}' already exists", config.name))
            } else {
                StorageError::Database(format!("Failed to add MCP server '{}': {}", config.name, e))
            }
        })?;

        tracing::info!(
            "✅ MCP server '{}' added successfully with ID: {}",
            config.name,
            server_id
        );
        Ok(server_id)
    }

    /// Update MCP server configuration
    pub async fn update_server(&self, name: &str, config: &McpServerConfig) -> Result<String> {
        let transport_str = match config.transport {
            ServiceTransport::Stdio => "stdio",
            ServiceTransport::Http => "http",
        };

        let args_json = config
            .args
            .as_ref()
            .map(|a| serde_json::to_string(a).unwrap_or_default());
        let headers_json = config
            .headers
            .as_ref()
            .map(|h| serde_json::to_string(h).unwrap_or_default());
        let env_json = config
            .env
            .as_ref()
            .map(|e| serde_json::to_string(e).unwrap_or_default());

        let result = sqlx::query(
            r#"
            UPDATE mcp_servers
            SET name = ?, description = ?, type = ?, command = ?, args = ?,
                url = ?, headers = ?, env = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP
            WHERE name = ?
            "#,
        )
        .bind(&config.name)
        .bind(&config.description)
        .bind(transport_str)
        .bind(&config.command)
        .bind(args_json)
        .bind(&config.url)
        .bind(headers_json)
        .bind(env_json)
        .bind(config.enabled)
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to update MCP server '{}': {}", name, e))
        })?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "MCP server '{}' not found",
                name
            )));
        }

        // Get server ID
        let row = sqlx::query("SELECT id FROM mcp_servers WHERE name = ?")
            .bind(&config.name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to get server ID: {}", e)))?;
        let server_id: String = row.get("id");

        tracing::info!("✅ MCP server '{}' updated successfully", name);
        Ok(server_id)
    }

    /// Delete MCP server configuration with application-level cascade delete
    pub async fn delete_server(&self, name: &str) -> Result<()> {
        // Begin transaction for cascade delete
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                StorageError::Database(format!("Failed to begin transaction: {}", e))
            })?;

        // Get server ID first
        let server_row = sqlx::query("SELECT id FROM mcp_servers WHERE name = ?")
            .bind(name)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to get server ID for '{}': {}", name, e))
            })?;

        if let Some(row) = server_row {
            let server_id: String = row.get("id");

            // Delete associated records from all child tables (cascade delete)
            sqlx::query("DELETE FROM mcp_server_tools WHERE server_id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StorageError::Database(format!(
                        "Failed to delete tools for server '{}': {}",
                        name, e
                    ))
                })?;

            sqlx::query("DELETE FROM mcp_server_resources WHERE server_id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StorageError::Database(format!(
                        "Failed to delete resources for server '{}': {}",
                        name, e
                    ))
                })?;

            sqlx::query("DELETE FROM mcp_server_prompts WHERE server_id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StorageError::Database(format!(
                        "Failed to delete prompts for server '{}': {}",
                        name, e
                    ))
                })?;

            // Delete permissions associated with this server
            sqlx::query("DELETE FROM permissions WHERE mcp_server_id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StorageError::Database(format!(
                        "Failed to delete permissions for server '{}': {}",
                        name, e
                    ))
                })?;

            // Delete the server itself
            let result = sqlx::query("DELETE FROM mcp_servers WHERE id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to delete MCP server '{}': {}", name, e))
                })?;

            if result.rows_affected() == 0 {
                return Err(StorageError::NotFound(format!(
                    "MCP server '{}' not found",
                    name
                )));
            }

            // Commit transaction
            tx.commit().await.map_err(|e| {
                StorageError::Database(format!("Failed to commit delete transaction: {}", e))
            })?;

            tracing::info!(
                "✅ MCP server '{}' and all associated data deleted successfully",
                name
            );
        } else {
            return Err(StorageError::NotFound(format!(
                "MCP server '{}' not found",
                name
            )));
        }

        Ok(())
    }

    /// Toggle MCP server enabled status
    pub async fn toggle_server_enabled(&self, name: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE mcp_servers
            SET enabled = NOT enabled, updated_at = CURRENT_TIMESTAMP
            WHERE name = ?
            "#,
        )
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to toggle MCP server '{}': {}", name, e))
        })?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "MCP server '{}' not found",
                name
            )));
        }

        // Get the new state
        let row = sqlx::query("SELECT enabled FROM mcp_servers WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to fetch updated state for '{}': {}",
                    name, e
                ))
            })?;

        let new_state: bool = row.get("enabled");
        tracing::info!(
            "✅ MCP server '{}' enabled state updated to: {}",
            name,
            new_state
        );
        Ok(new_state)
    }

    // ============================================================================
    // MCP Server Cache Management
    // ============================================================================

    /// Update server version
    pub async fn update_server_version(
        &self,
        server_name: &str,
        version: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE mcp_servers
            SET version = ?, last_version_check = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
            WHERE name = ?
            "#,
        )
        .bind(version)
        .bind(server_name)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to update version for '{}': {}",
                server_name, e
            ))
        })?;

        Ok(())
    }

    /// Get server version
    pub async fn get_server_version(&self, server_name: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT version FROM mcp_servers WHERE name = ?")
            .bind(server_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to fetch version for '{}': {}",
                    server_name, e
                ))
            })?;

        Ok(row.and_then(|r| r.get("version")))
    }

    /// Cache server tools using UPSERT strategy to maintain UUID stability
    pub async fn cache_server_tools(
        &self,
        server_name: &str,
        tools: &[rmcp::model::Tool],
    ) -> Result<()> {
        // Get server ID first
        let server_row = sqlx::query("SELECT id FROM mcp_servers WHERE name = ?")
            .bind(server_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to get server ID for '{}': {}",
                    server_name, e
                ))
            })?;
        let server_id: String = server_row.get("id");

        // Begin transaction for atomic operation
        let mut tx = self.pool.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        // Stage 1: Get existing tools for this server
        let existing_tools = sqlx::query(
            "SELECT id, name FROM mcp_server_tools WHERE server_id = ?"
        )
        .bind(&server_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to fetch existing tools: {}", e)))?;

        let existing_tool_names: std::collections::HashSet<String> = existing_tools
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        let incoming_tool_names: std::collections::HashSet<String> = tools
            .iter()
            .map(|tool| tool.name.to_string())
            .collect();

        // Stage 2: UPSERT incoming tools
        for tool in tools {
            let input_schema = serde_json::to_string(&tool.input_schema).unwrap_or_default();

            // Check if tool exists to decide whether to generate new ID or use existing
            let tool_id = if existing_tool_names.contains(&tool.name.to_string()) {
                // Find existing tool ID
                existing_tools
                    .iter()
                    .find(|row| row.get::<String, _>("name") == tool.name.to_string())
                    .map(|row| row.get::<String, _>("id"))
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
            } else {
                // New tool - generate new UUID
                uuid::Uuid::new_v4().to_string()
            };

            sqlx::query(
                r#"
                INSERT INTO mcp_server_tools (
                    id, server_id, name, title, description, enabled, input_schema, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(server_id, name) DO UPDATE SET
                    title = excluded.title,
                    description = excluded.description,
                    input_schema = excluded.input_schema,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = mcp_server_tools.id
                "#
            )
            .bind(&tool_id)
            .bind(&server_id)
            .bind(&tool.name)
            .bind(tool.title.as_ref().map(|t| t.to_string()).unwrap_or_else(|| tool.name.to_string()))
            .bind(&tool.description)
            .bind(true) // enabled
            .bind(&input_schema)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to upsert tool '{}': {}", tool.name, e)))?;
        }

        // Stage 3: Delete tools that no longer exist in the server
        let tools_to_delete: Vec<String> = existing_tool_names
            .difference(&incoming_tool_names)
            .cloned()
            .collect();

        if !tools_to_delete.is_empty() {
            tracing::debug!("Cleaning up {} deleted tools for server '{}': {}",
                tools_to_delete.len(), server_name,
                tools_to_delete.join(", ")
            );

            for tool_name in &tools_to_delete {
                sqlx::query("DELETE FROM mcp_server_tools WHERE server_id = ? AND name = ?")
                    .bind(&server_id)
                    .bind(tool_name)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to delete tool '{}': {}", tool_name, e)))?;
            }

            // Clean up associated permissions - execute individually to avoid borrowing issues

            // Collect tool IDs first to avoid lifetime issues
            let mut tool_ids = Vec::new();
            for tool_name in &tools_to_delete {
                let tool_id_row = sqlx::query("SELECT id FROM mcp_server_tools WHERE server_id = ? AND name = ?")
                    .bind(&server_id)
                    .bind(tool_name)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to get tool ID for deletion: {}", e)))?;

                let tool_id: String = tool_id_row.get("id");
                tool_ids.push(tool_id);
            }

            if !tool_ids.is_empty() {
                // Execute each deletion individually to avoid borrowing issues
                for tool_id in tool_ids {
                    sqlx::query("DELETE FROM permissions WHERE resource_type = 'tool' AND resource_id = ?")
                        .bind(&tool_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| StorageError::Database(format!("Failed to clean up permissions: {}", e)))?;
                }
            }
        }

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        let updated_count = tools.len() - tools_to_delete.len();
        let new_count = existing_tool_names.intersection(&incoming_tool_names).count();

        tracing::info!(
            "✅ UPSERTED {} tools for server '{}' (updated: {}, new: {}, deleted: {})",
            tools.len(),
            server_name,
            updated_count - new_count,
            new_count,
            tools_to_delete.len()
        );
        Ok(())
    }

    /// Get cached server tools
    pub async fn get_cached_server_tools(&self, server_name: &str) -> Result<Vec<McpToolInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT t.id, t.name, t.title, t.description, t.enabled
            FROM mcp_server_tools t
            JOIN mcp_servers s ON t.server_id = s.id
            WHERE s.name = ?
            ORDER BY t.name
            "#,
        )
        .bind(server_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to fetch cached tools for '{}': {}",
                server_name, e
            ))
        })?;

        let mut tools = Vec::new();
        for row in rows {
            tools.push(McpToolInfo {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                enabled: row.get("enabled"),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            });
        }

        Ok(tools)
    }

    /// Cache server resources using UPSERT strategy to maintain UUID stability
    pub async fn cache_server_resources(
        &self,
        server_name: &str,
        resources: &[rmcp::model::Resource],
    ) -> Result<()> {
        // Get server ID first
        let server_row = sqlx::query("SELECT id FROM mcp_servers WHERE name = ?")
            .bind(server_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to get server ID for '{}': {}",
                    server_name, e
                ))
            })?;
        let server_id: String = server_row.get("id");

        // Begin transaction for atomic operation
        let mut tx = self.pool.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        // Stage 1: Get existing resources for this server
        let existing_resources = sqlx::query(
            "SELECT id, uri FROM mcp_server_resources WHERE server_id = ?"
        )
        .bind(&server_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to fetch existing resources: {}", e)))?;

        let existing_resource_uris: std::collections::HashSet<String> = existing_resources
            .iter()
            .map(|row| row.get::<String, _>("uri"))
            .collect();

        let incoming_resource_uris: std::collections::HashSet<String> = resources
            .iter()
            .map(|resource| resource.uri.to_string())
            .collect();

        // Stage 2: UPSERT incoming resources
        for resource in resources {
            let is_template = resource.uri.contains("{") || resource.uri.contains(":");

            // Get resource name, description and mime_type from Resource itself
            let resource_name = if !resource.name.is_empty() {
                resource.name.clone()
            } else {
                resource.uri.clone()
            };

            let resource_description = resource.description.clone();
            let mime_type = resource.mime_type.clone();

            // Check if resource exists to decide whether to generate new ID or use existing
            let resource_id = if existing_resource_uris.contains(&resource.uri.to_string()) {
                // Find existing resource ID
                existing_resources
                    .iter()
                    .find(|row| row.get::<String, _>("uri") == resource.uri.to_string())
                    .map(|row| row.get::<String, _>("id"))
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
            } else {
                // New resource - generate new UUID
                uuid::Uuid::new_v4().to_string()
            };

            sqlx::query(
                r#"
                INSERT INTO mcp_server_resources (
                    id, server_id, uri, name, title, description, mime_type,
                    enabled, is_template, uri_template, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(server_id, uri) DO UPDATE SET
                    name = excluded.name,
                    title = excluded.title,
                    description = excluded.description,
                    mime_type = excluded.mime_type,
                    is_template = excluded.is_template,
                    uri_template = excluded.uri_template,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = mcp_server_resources.id
                "#
            )
            .bind(&resource_id)
            .bind(&server_id)
            .bind(&resource.uri)
            .bind(&resource_name)
            .bind(&resource_name) // Use name as title if no title field
            .bind(&resource_description)
            .bind(&mime_type)
            .bind(true) // enabled
            .bind(is_template)
            .bind(if is_template { Some(&resource.uri) } else { None })
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to upsert resource '{}': {}", resource_name, e)))?;
        }

        // Stage 3: Delete resources that no longer exist in the server
        let resources_to_delete: Vec<String> = existing_resource_uris
            .difference(&incoming_resource_uris)
            .cloned()
            .collect();

        if !resources_to_delete.is_empty() {
            tracing::debug!("Cleaning up {} deleted resources for server '{}': {}",
                resources_to_delete.len(), server_name,
                resources_to_delete.join(", ")
            );

            for resource_uri in &resources_to_delete {
                sqlx::query("DELETE FROM mcp_server_resources WHERE server_id = ? AND uri = ?")
                    .bind(&server_id)
                    .bind(resource_uri)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to delete resource '{}': {}", resource_uri, e)))?;
            }

            // Clean up associated permissions - execute individually to avoid borrowing issues

            // Collect resource IDs first to avoid lifetime issues
            let mut resource_ids = Vec::new();
            for resource_uri in &resources_to_delete {
                let resource_id_row = sqlx::query("SELECT id FROM mcp_server_resources WHERE server_id = ? AND uri = ?")
                    .bind(&server_id)
                    .bind(resource_uri)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to get resource ID for deletion: {}", e)))?;

                let resource_id: String = resource_id_row.get("id");
                resource_ids.push(resource_id);
            }

            // Execute each deletion individually to avoid borrowing issues
            for resource_id in resource_ids {
                sqlx::query("DELETE FROM permissions WHERE resource_type = 'resource' AND resource_id = ?")
                    .bind(&resource_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to clean up permissions: {}", e)))?;
            }
        }

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        let updated_count = resources.len() - resources_to_delete.len();
        let new_count = existing_resource_uris.intersection(&incoming_resource_uris).count();

        tracing::info!(
            "✅ UPSERTED {} resources for server '{}' (updated: {}, new: {}, deleted: {})",
            resources.len(),
            server_name,
            updated_count - new_count,
            new_count,
            resources_to_delete.len()
        );
        Ok(())
    }

    /// Get cached server resources
    pub async fn get_cached_server_resources(
        &self,
        server_name: &str,
    ) -> Result<Vec<McpResourceInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT r.id, r.uri, r.name, r.title, r.description, r.mime_type, r.enabled
            FROM mcp_server_resources r
            JOIN mcp_servers s ON r.server_id = s.id
            WHERE s.name = ?
            ORDER BY r.name
            "#,
        )
        .bind(server_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to fetch cached resources for '{}': {}",
                server_name, e
            ))
        })?;

        let mut resources = Vec::new();
        for row in rows {
            resources.push(McpResourceInfo {
                id: row.get("id"),
                uri: row.get("uri"),
                name: row.get("name"),
                description: row.get("description"),
                mime_type: row.get("mime_type"),
                enabled: row.get("enabled"),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            });
        }

        Ok(resources)
    }

    /// Cache server prompts using UPSERT strategy to maintain UUID stability
    pub async fn cache_server_prompts(
        &self,
        server_name: &str,
        prompts: &[rmcp::model::Prompt],
    ) -> Result<()> {
        // Get server ID first
        let server_row = sqlx::query("SELECT id FROM mcp_servers WHERE name = ?")
            .bind(server_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to get server ID for '{}': {}",
                    server_name, e
                ))
            })?;
        let server_id: String = server_row.get("id");

        // Begin transaction for atomic operation
        let mut tx = self.pool.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        // Stage 1: Get existing prompts for this server
        let existing_prompts = sqlx::query(
            "SELECT id, name FROM mcp_server_prompts WHERE server_id = ?"
        )
        .bind(&server_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to fetch existing prompts: {}", e)))?;

        let existing_prompt_names: std::collections::HashSet<String> = existing_prompts
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        let incoming_prompt_names: std::collections::HashSet<String> = prompts
            .iter()
            .map(|prompt| prompt.name.to_string())
            .collect();

        // Stage 2: UPSERT incoming prompts
        for prompt in prompts {
            let arguments_json = prompt
                .arguments
                .as_ref()
                .map(|args| serde_json::to_string(args).unwrap_or_default());

            // Check if prompt exists to decide whether to generate new ID or use existing
            let prompt_id = if existing_prompt_names.contains(&prompt.name.to_string()) {
                // Find existing prompt ID
                existing_prompts
                    .iter()
                    .find(|row| row.get::<String, _>("name") == prompt.name.to_string())
                    .map(|row| row.get::<String, _>("id"))
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
            } else {
                // New prompt - generate new UUID
                uuid::Uuid::new_v4().to_string()
            };

            sqlx::query(
                r#"
                INSERT INTO mcp_server_prompts (
                    id, server_id, name, title, description, enabled, arguments, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(server_id, name) DO UPDATE SET
                    title = excluded.title,
                    description = excluded.description,
                    arguments = excluded.arguments,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = mcp_server_prompts.id
                "#
            )
            .bind(&prompt_id)
            .bind(&server_id)
            .bind(&prompt.name)
            .bind(prompt.title.as_ref().cloned().unwrap_or_else(|| prompt.name.clone()))
            .bind(&prompt.description)
            .bind(true) // enabled
            .bind(arguments_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to upsert prompt '{}': {}", prompt.name, e)))?;
        }

        // Stage 3: Delete prompts that no longer exist in the server
        let prompts_to_delete: Vec<String> = existing_prompt_names
            .difference(&incoming_prompt_names)
            .cloned()
            .collect();

        if !prompts_to_delete.is_empty() {
            tracing::debug!("Cleaning up {} deleted prompts for server '{}': {}",
                prompts_to_delete.len(), server_name,
                prompts_to_delete.join(", ")
            );

            for prompt_name in &prompts_to_delete {
                sqlx::query("DELETE FROM mcp_server_prompts WHERE server_id = ? AND name = ?")
                    .bind(&server_id)
                    .bind(prompt_name)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to delete prompt '{}': {}", prompt_name, e)))?;
            }

            // Clean up associated permissions (both regular prompts and templates) - execute individually to avoid borrowing issues

            // Collect prompt IDs first to avoid lifetime issues
            let mut prompt_ids = Vec::new();
            for prompt_name in &prompts_to_delete {
                let prompt_id_row = sqlx::query("SELECT id FROM mcp_server_prompts WHERE server_id = ? AND name = ?")
                    .bind(&server_id)
                    .bind(prompt_name)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to get prompt ID for deletion: {}", e)))?;

                let prompt_id: String = prompt_id_row.get("id");
                prompt_ids.push(prompt_id);
            }

            // Execute each deletion individually to avoid borrowing issues
            for prompt_id in prompt_ids {
                sqlx::query("DELETE FROM permissions WHERE resource_type IN ('prompt', 'prompt_template') AND resource_id = ?")
                    .bind(&prompt_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| StorageError::Database(format!("Failed to clean up permissions: {}", e)))?;
            }
        }

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        let updated_count = prompts.len() - prompts_to_delete.len();
        let new_count = existing_prompt_names.intersection(&incoming_prompt_names).count();

        tracing::info!(
            "✅ UPSERTED {} prompts for server '{}' (updated: {}, new: {}, deleted: {})",
            prompts.len(),
            server_name,
            updated_count - new_count,
            new_count,
            prompts_to_delete.len()
        );
        Ok(())
    }

    /// Get cached server prompts
    pub async fn get_cached_server_prompts(&self, server_name: &str) -> Result<Vec<McpPromptInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.name, p.title, p.description, p.enabled, p.arguments
            FROM mcp_server_prompts p
            JOIN mcp_servers s ON p.server_id = s.id
            WHERE s.name = ?
            ORDER BY p.name
            "#,
        )
        .bind(server_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to fetch cached prompts for '{}': {}",
                server_name, e
            ))
        })?;

        let mut prompts = Vec::new();
        for row in rows {
            prompts.push(McpPromptInfo {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                enabled: row.get("enabled"),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            });
        }

        Ok(prompts)
    }

    /// Get cached server prompts with template info
    pub async fn get_cached_server_prompts_with_templates(
        &self,
        server_name: &str,
    ) -> Result<Vec<(McpPromptInfo, bool)>> {
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.name, p.title, p.description, p.enabled, p.arguments
            FROM mcp_server_prompts p
            JOIN mcp_servers s ON p.server_id = s.id
            WHERE s.name = ?
            ORDER BY p.name
            "#,
        )
        .bind(server_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to fetch cached prompts for '{}': {}",
                server_name, e
            ))
        })?;

        let mut prompts = Vec::new();
        for row in rows {
            let arguments: Option<String> = row.get("arguments");
            let is_template = arguments.is_some() && !arguments.unwrap_or_default().is_empty();

            prompts.push((
                McpPromptInfo {
                    id: row.get("id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    enabled: row.get("enabled"),
                    created_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                },
                is_template,
            ));
        }

        Ok(prompts)
    }

    /// Clear all cached data for a server
    pub async fn clear_server_cache(&self, server_name: &str) -> Result<()> {
        // Get server ID
        let server_row = sqlx::query("SELECT id FROM mcp_servers WHERE name = ?")
            .bind(server_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to get server ID for '{}': {}",
                    server_name, e
                ))
            })?;

        if let Some(row) = server_row {
            let server_id: String = row.get("id");

            // Begin transaction
            let mut tx = self.pool.begin().await.map_err(|e| {
                StorageError::Database(format!("Failed to begin transaction: {}", e))
            })?;

            // Clear from all cache tables
            sqlx::query("DELETE FROM mcp_server_tools WHERE server_id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to clear tools cache: {}", e))
                })?;

            sqlx::query("DELETE FROM mcp_server_resources WHERE server_id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to clear resources cache: {}", e))
                })?;

            sqlx::query("DELETE FROM mcp_server_prompts WHERE server_id = ?")
                .bind(&server_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to clear prompts cache: {}", e))
                })?;

            // Commit transaction
            tx.commit().await.map_err(|e| {
                StorageError::Database(format!("Failed to commit transaction: {}", e))
            })?;
        }

        // Clear version information
        sqlx::query(
            r#"
            UPDATE mcp_servers
            SET version = NULL, last_version_check = NULL
            WHERE name = ?
            "#,
        )
        .bind(server_name)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to clear version cache: {}", e)))?;

        tracing::info!("✅ Cleared all cache data for server '{}'", server_name);
        Ok(())
    }

    // ============================================================================
    // Permission Management Integration
    // ============================================================================

    /// Get all tools for permission management
    pub async fn get_all_tools_for_permissions(&self) -> Result<Vec<(String, String, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT t.id, t.name, s.name as server_name
            FROM mcp_server_tools t
            JOIN mcp_servers s ON t.server_id = s.id
            WHERE t.enabled = 1 AND s.enabled = 1
            ORDER BY s.name, t.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch tools for permissions: {}", e))
        })?;

        let mut tools = Vec::new();
        for row in rows {
            tools.push((row.get("id"), row.get("name"), row.get("server_name")));
        }

        Ok(tools)
    }

    /// Get all resources for permission management
    pub async fn get_all_resources_for_permissions(&self) -> Result<Vec<(String, String, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT r.id, r.name, s.name as server_name
            FROM mcp_server_resources r
            JOIN mcp_servers s ON r.server_id = s.id
            WHERE r.enabled = 1 AND s.enabled = 1
            ORDER BY s.name, r.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch resources for permissions: {}", e))
        })?;

        let mut resources = Vec::new();
        for row in rows {
            resources.push((row.get("id"), row.get("name"), row.get("server_name")));
        }

        Ok(resources)
    }

    /// Get tool ID by server name and tool name
    pub async fn get_tool_id_by_name(&self, server_name: &str, tool_name: &str) -> Result<String> {
        let row = sqlx::query(
            r#"
            SELECT t.id
            FROM mcp_server_tools t
            JOIN mcp_servers s ON t.server_id = s.id
            WHERE s.name = ? AND t.name = ?
            LIMIT 1
            "#,
        )
        .bind(server_name)
        .bind(tool_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to get tool ID by name: {}", e))
        })?;

        match row {
            Some(row) => Ok(row.get("id")),
            None => Err(StorageError::NotFound(format!(
                "Tool '{}' not found in server '{}'",
                tool_name, server_name
            ))),
        }
    }

    /// Get resource ID by server name and resource URI
    pub async fn get_resource_id_by_uri(&self, server_name: &str, resource_uri: &str) -> Result<String> {
        let row = sqlx::query(
            r#"
            SELECT r.id
            FROM mcp_server_resources r
            JOIN mcp_servers s ON r.server_id = s.id
            WHERE s.name = ? AND r.uri = ?
            LIMIT 1
            "#,
        )
        .bind(server_name)
        .bind(resource_uri)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to get resource ID by URI: {}", e))
        })?;

        match row {
            Some(row) => Ok(row.get("id")),
            None => Err(StorageError::NotFound(format!(
                "Resource '{}' not found in server '{}'",
                resource_uri, server_name
            ))),
        }
    }

    /// Get all prompts for permission management
    pub async fn get_all_prompts_for_permissions(&self) -> Result<Vec<(String, String, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.name, s.name as server_name
            FROM mcp_server_prompts p
            JOIN mcp_servers s ON p.server_id = s.id
            WHERE p.enabled = 1 AND s.enabled = 1
            ORDER BY s.name, p.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch prompts for permissions: {}", e))
        })?;

        let mut prompts = Vec::new();
        for row in rows {
            prompts.push((row.get("id"), row.get("name"), row.get("server_name")));
        }

        Ok(prompts)
    }

    /// Get all prompt templates for permission management
    pub async fn get_all_prompt_templates_for_permissions(&self) -> Result<Vec<(String, String, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.name, s.name as server_name
            FROM mcp_server_prompts p
            JOIN mcp_servers s ON p.server_id = s.id
            WHERE p.enabled = 1 AND s.enabled = 1
            AND p.arguments IS NOT NULL AND p.arguments != '[]'
            ORDER BY s.name, p.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch prompt templates for permissions: {}", e))
        })?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push((row.get("id"), row.get("name"), row.get("server_name")));
        }

        Ok(templates)
    }

    
    /// Export all MCP server configurations to JSON format
    pub async fn export_to_json(&self) -> Result<Vec<McpServerConfig>> {
        self.get_all_servers().await
    }

    // ============================================================================
    // Data Integrity Validation
    // ============================================================================

    /// Validate server exists before adding child records
    pub async fn validate_server_exists(&self, server_id: &str) -> Result<bool> {
        let count = sqlx::query("SELECT COUNT(*) as count FROM mcp_servers WHERE id = ?")
            .bind(server_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to validate server exists: {}", e))
            })?;

        let count: i64 = count.get("count");
        Ok(count > 0)
    }

    /// Clean up orphaned records
    pub async fn cleanup_orphaned_records(&self) -> Result<usize> {
        let mut total_cleaned = 0;

        // Clean up orphaned tools
        let tools_deleted = sqlx::query(
            "DELETE FROM mcp_server_tools WHERE server_id NOT IN (SELECT id FROM mcp_servers)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to cleanup orphaned tools: {}", e)))?;

        total_cleaned += tools_deleted.rows_affected();

        // Clean up orphaned resources
        let resources_deleted = sqlx::query(
            "DELETE FROM mcp_server_resources WHERE server_id NOT IN (SELECT id FROM mcp_servers)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to cleanup orphaned resources: {}", e))
        })?;

        total_cleaned += resources_deleted.rows_affected();

        // Clean up orphaned prompts
        let prompts_deleted = sqlx::query(
            "DELETE FROM mcp_server_prompts WHERE server_id NOT IN (SELECT id FROM mcp_servers)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to cleanup orphaned prompts: {}", e))
        })?;

        total_cleaned += prompts_deleted.rows_affected();

        // Clean up orphaned permissions
        let permissions_deleted = sqlx::query(
            "DELETE FROM permissions WHERE mcp_server_id NOT IN (SELECT id FROM mcp_servers) AND mcp_server_id IS NOT NULL"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to cleanup orphaned permissions: {}", e)))?;

        total_cleaned += permissions_deleted.rows_affected();

        if total_cleaned > 0 {
            tracing::info!("✅ Cleaned up {} orphaned records", total_cleaned);
        }

        Ok(total_cleaned.try_into().unwrap())
    }

    /// Validate UUID format
    pub fn validate_uuid(&self, uuid_str: &str) -> bool {
        uuid::Uuid::parse_str(uuid_str).is_ok()
    }

    /// Validate data integrity on startup
    pub async fn validate_data_integrity(&self) -> Result<Vec<String>> {
        let mut issues = Vec::new();

        // Check for orphaned tools
        let orphaned_tools = sqlx::query(
            "SELECT COUNT(*) as count FROM mcp_server_tools WHERE server_id NOT IN (SELECT id FROM mcp_servers)"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to check orphaned tools: {}", e)))?;

        let count: i64 = orphaned_tools.get("count");
        if count > 0 {
            issues.push(format!("Found {} orphaned tool records", count));
        }

        // Check for orphaned resources
        let orphaned_resources = sqlx::query(
            "SELECT COUNT(*) as count FROM mcp_server_resources WHERE server_id NOT IN (SELECT id FROM mcp_servers)"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to check orphaned resources: {}", e)))?;

        let count: i64 = orphaned_resources.get("count");
        if count > 0 {
            issues.push(format!("Found {} orphaned resource records", count));
        }

        // Check for orphaned prompts
        let orphaned_prompts = sqlx::query(
            "SELECT COUNT(*) as count FROM mcp_server_prompts WHERE server_id NOT IN (SELECT id FROM mcp_servers)"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to check orphaned prompts: {}", e)))?;

        let count: i64 = orphaned_prompts.get("count");
        if count > 0 {
            issues.push(format!("Found {} orphaned prompt records", count));
        }

        // Check for invalid UUIDs
        let invalid_uuids = sqlx::query(
            "SELECT COUNT(*) as count FROM mcp_servers WHERE length(id) != 36 OR id NOT LIKE '%-%'",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to check invalid UUIDs: {}", e)))?;

        let count: i64 = invalid_uuids.get("count");
        if count > 0 {
            issues.push(format!("Found {} records with invalid UUID format", count));
        }

        if issues.is_empty() {
            tracing::info!("✅ Data integrity validation passed");
        } else {
            tracing::warn!("⚠️  Data integrity issues found: {:?}", issues);
        }

        Ok(issues)
    }

    // ============================================================================
    // Enhanced Permission Management Methods with Full Information
    // ============================================================================

    /// Get all tools for permission management with full information
    pub async fn get_all_tools_for_permissions_full(&self) -> Result<Vec<(String, String, Option<String>, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT t.id, t.name, t.description, s.name as server_name
            FROM mcp_server_tools t
            JOIN mcp_servers s ON t.server_id = s.id
            WHERE t.enabled = 1 AND s.enabled = 1
            ORDER BY s.name, t.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch tools for permissions: {}", e))
        })?;

        let mut tools = Vec::new();
        for row in rows {
            tools.push((
                row.get("id"),
                row.get("name"),
                row.get("description"),
                row.get("server_name")
            ));
        }

        Ok(tools)
    }

    /// Get all tools for MCP aggregation with input schema
    pub async fn get_all_tools_for_aggregation(&self) -> Result<Vec<(String, String, Option<String>, Option<String>, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT t.id, t.name, t.description, t.input_schema, s.name as server_name
            FROM mcp_server_tools t
            JOIN mcp_servers s ON t.server_id = s.id
            WHERE t.enabled = 1 AND s.enabled = 1
            ORDER BY s.name, t.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch tools for aggregation: {}", e))
        })?;

        let mut tools = Vec::new();
        for row in rows {
            let input_schema: String = row.get("input_schema");
            let input_schema_opt = if input_schema.is_empty() { None } else { Some(input_schema) };
            let server_name: String = row.get("server_name");

            tools.push((
                row.get("id"),
                row.get("name"),
                row.get("description"),
                input_schema_opt,
                server_name.clone(), // 克隆以避免移动
            ));
        }

        Ok(tools)
    }

    /// Get all resources for permission management with full information
    pub async fn get_all_resources_for_permissions_full(&self) -> Result<Vec<(String, String, Option<String>, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT r.id, r.uri, r.description, s.name as server_name
            FROM mcp_server_resources r
            JOIN mcp_servers s ON r.server_id = s.id
            WHERE r.enabled = 1 AND s.enabled = 1
            ORDER BY s.name, r.uri
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch resources for permissions: {}", e))
        })?;

        let mut resources = Vec::new();
        for row in rows {
            resources.push((
                row.get("id"),
                row.get("uri"),
                row.get("description"),
                row.get("server_name")
            ));
        }

        Ok(resources)
    }

    /// Get all prompts for permission management with full information
    pub async fn get_all_prompts_for_permissions_full(&self) -> Result<Vec<(String, String, Option<String>, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.name, p.description, s.name as server_name
            FROM mcp_server_prompts p
            JOIN mcp_servers s ON p.server_id = s.id
            WHERE p.enabled = 1 AND s.enabled = 1
            ORDER BY s.name, p.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to fetch prompts for permissions: {}", e))
        })?;

        let mut prompts = Vec::new();
        for row in rows {
            prompts.push((
                row.get("id"),
                row.get("name"),
                row.get("description"),
                row.get("server_name")
            ));
        }

        Ok(prompts)
    }
}
