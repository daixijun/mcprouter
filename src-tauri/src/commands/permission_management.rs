use crate::error::Result;
use crate::types::PermissionItem;
use serde::{Deserialize, Serialize};

/// Response containing available permissions
#[derive(Debug, Serialize, Deserialize)]
pub struct AvailablePermissions {
    pub tools: Vec<PermissionItem>,
    pub resources: Vec<PermissionItem>,
    pub prompts: Vec<PermissionItem>,
    #[serde(default)]
    pub prompt_templates: Vec<PermissionItem>,
}

/// List available permissions (all types or specific type)
#[tauri::command]
pub async fn list_available_permissions(
    resource_type: Option<String>,
) -> Result<AvailablePermissions> {
    // Use the global SERVICE_MANAGER to get actual cached data
    let mcp_manager = {
        let guard = crate::SERVICE_MANAGER
            .lock()
            .expect("Failed to acquire SERVICE_MANAGER lock");
        guard.as_ref()
            .ok_or_else(|| crate::error::McpError::InternalError("MCP Server Manager not initialized".to_string()))?
            .clone()
    };

    // If resource_type is specified, return only that type (others empty)
    if let Some(resource_type) = resource_type {
        let permissions = mcp_manager
            .get_detailed_permissions_by_type(&resource_type)
            .await
            .map_err(|e| crate::error::McpError::DatabaseQueryError(format!("Failed to get permissions: {}", e)))?;

        // Return only the specified type, others as empty arrays
        match resource_type.as_str() {
            "tool" => Ok(AvailablePermissions {
                tools: permissions,
                resources: Vec::new(),
                prompts: Vec::new(),
                prompt_templates: Vec::new(),
            }),
            "resource" => Ok(AvailablePermissions {
                tools: Vec::new(),
                resources: permissions,
                prompts: Vec::new(),
                prompt_templates: Vec::new(),
            }),
            "prompt" => Ok(AvailablePermissions {
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: permissions,
                prompt_templates: Vec::new(),
            }),
            _ => Err(crate::error::McpError::InvalidInput(format!("Invalid resource_type: {}", resource_type))),
        }
    } else {
        // Return all types
        let tools = mcp_manager
            .get_detailed_permissions_by_type("tool")
            .await
            .map_err(|e| crate::error::McpError::DatabaseQueryError(format!("Failed to get tools permissions: {}", e)))?;

        let resources = mcp_manager
            .get_detailed_permissions_by_type("resource")
            .await
            .map_err(|e| crate::error::McpError::DatabaseQueryError(format!("Failed to get resources permissions: {}", e)))?;

        let prompts = mcp_manager
            .get_detailed_permissions_by_type("prompt")
            .await
            .map_err(|e| crate::error::McpError::DatabaseQueryError(format!("Failed to get prompts permissions: {}", e)))?;

        let prompt_templates: Vec<PermissionItem> = Vec::new(); // Not supported yet

        Ok(AvailablePermissions {
            tools,
            resources,
            prompts,
            prompt_templates,
        })
    }
}


