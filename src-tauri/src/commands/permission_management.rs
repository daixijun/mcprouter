use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Item with description
#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionItem {
    pub id: String,
    pub resource_name: String,
    pub description: String,
    pub resource_type: String,
    pub server_id: String,
    pub server_name: String,
}

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
    let service_manager = {
        let guard = crate::SERVICE_MANAGER.lock().unwrap();
        guard.as_ref()
            .ok_or_else(|| crate::error::McpError::InternalError("MCP Server Manager not initialized".to_string()))?
            .clone()
    };

    // Get all available permissions from cached data with descriptions
    let tools_with_desc = service_manager
        .get_all_available_tools_with_descriptions()
        .await;
    let resources_with_desc = service_manager
        .get_all_available_resources_with_descriptions()
        .await;
    let prompts_with_desc = service_manager
        .get_all_available_prompts_with_descriptions()
        .await;
    let prompt_templates_with_desc = service_manager
        .get_all_available_prompt_templates_with_descriptions()
        .await;

    let tools: Vec<PermissionItem> = tools_with_desc
        .into_iter()
        .map(|(id, resource_name, description, server_id, server_name)| PermissionItem {
            id,
            resource_name,
            description,
            resource_type: "tool".to_string(),
            server_id,
            server_name,
        })
        .collect();

    let resources: Vec<PermissionItem> = resources_with_desc
        .into_iter()
        .map(|(id, resource_name, description, server_id, server_name)| PermissionItem {
            id,
            resource_name,
            description,
            resource_type: "resource".to_string(),
            server_id,
            server_name,
        })
        .collect();

    let prompts: Vec<PermissionItem> = prompts_with_desc
        .into_iter()
        .map(|(id, resource_name, description, server_id, server_name)| PermissionItem {
            id,
            resource_name,
            description,
            resource_type: "prompt".to_string(),
            server_id,
            server_name,
        })
        .collect();
    let prompt_templates: Vec<PermissionItem> = prompt_templates_with_desc
        .into_iter()
        .map(|(id, resource_name, description, server_id, server_name)| PermissionItem {
            id,
            resource_name,
            description,
            resource_type: "prompt_template".to_string(),
            server_id,
            server_name,
        })
        .collect();

    Ok(AvailablePermissions {
        tools,
        resources,
        prompts,
        prompt_templates,
    })
}
