use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Item with description
#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionItem {
    pub id: String,
    pub description: String,
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
    let service_manager = &crate::SERVICE_MANAGER;

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
        .map(|(id, description)| PermissionItem { id, description })
        .collect();

    let resources: Vec<PermissionItem> = resources_with_desc
        .into_iter()
        .map(|(id, description)| PermissionItem { id, description })
        .collect();

    let prompts: Vec<PermissionItem> = prompts_with_desc
        .into_iter()
        .map(|(id, description)| PermissionItem { id, description })
        .collect();
    let prompt_templates: Vec<PermissionItem> = prompt_templates_with_desc
        .into_iter()
        .map(|(id, description)| PermissionItem { id, description })
        .collect();

    Ok(AvailablePermissions {
        tools,
        resources,
        prompts,
        prompt_templates,
    })
}
