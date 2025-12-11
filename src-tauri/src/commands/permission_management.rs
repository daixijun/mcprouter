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

/// Get all available permissions for token configuration
#[tauri::command]
pub async fn get_available_permissions() -> Result<AvailablePermissions> {
    // Use the global SERVICE_MANAGER to get actual cached data
    let mcp_manager = {
        let guard = crate::SERVICE_MANAGER.lock().unwrap();
        guard.as_ref()
            .ok_or_else(|| crate::error::McpError::InternalError("MCP Server Manager not initialized".to_string()))?
            .clone()
    };

    // Get all available permissions using new method
    let permissions = mcp_manager
        .get_available_permissions()
        .await
        .map_err(|e| crate::error::McpError::DatabaseQueryError(format!("Failed to get available permissions: {}", e)))?;

    // Group permissions by type
    let tools: Vec<PermissionItem> = permissions
        .iter()
        .filter(|p| p.resource_type == "tool")
        .cloned()
        .collect();

    let resources: Vec<PermissionItem> = permissions
        .iter()
        .filter(|p| p.resource_type == "resource")
        .cloned()
        .collect();

    let prompts: Vec<PermissionItem> = permissions
        .iter()
        .filter(|p| p.resource_type == "prompt")
        .cloned()
        .collect();

    let prompt_templates: Vec<PermissionItem> = Vec::new(); // Not supported yet

    Ok(AvailablePermissions {
        tools,
        resources,
        prompts,
        prompt_templates,
    })
}

/// Get available permissions by type
#[tauri::command]
pub async fn get_available_permissions_by_type(
    resource_type: String,
) -> Result<Vec<PermissionItem>> {
    // Use the global SERVICE_MANAGER to get actual cached data
    let mcp_manager = {
        let guard = crate::SERVICE_MANAGER.lock().unwrap();
        guard.as_ref()
            .ok_or_else(|| crate::error::McpError::InternalError("MCP Server Manager not initialized".to_string()))?
            .clone()
    };

    mcp_manager
        .get_available_permissions_by_type(&resource_type)
        .await
        .map_err(|e| crate::error::McpError::DatabaseQueryError(format!("Failed to get permissions by type: {}", e)))
}
