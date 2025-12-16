use crate::error::Result;
use crate::types::ToolInfo;
use std::sync::Arc;
use tauri::State;

#[derive(Debug)]
pub struct ToolManagerState {
    pub manager: Arc<crate::tool_manager::ToolManager>,
}

impl Default for ToolManagerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolManagerState {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(crate::tool_manager::ToolManager::new()),
        }
    }
}

/// Get all managed tools information
#[tauri::command(rename_all = "snake_case")]
pub async fn get_tools_info(
    state: State<'_, ToolManagerState>,
) -> Result<Vec<ToolInfo>> {
    state.manager.get_tools_info().await
}

/// Get a specific tool's information by name
#[tauri::command(rename_all = "snake_case")]
pub async fn get_tool_info(
    state: State<'_, ToolManagerState>,
    tool_name: String,
) -> Result<Option<ToolInfo>> {
    let tools = state.manager.get_tools_info().await?;
    Ok(tools.into_iter().find(|tool| tool.name.to_lowercase() == tool_name.to_lowercase()))
}

/// Install all required tools
#[tauri::command(rename_all = "snake_case")]
pub async fn install_all_tools(
    state: State<'_, ToolManagerState>,
) -> Result<()> {
    state.manager.install_all_tools().await
}

/// Install a specific tool
#[tauri::command(rename_all = "snake_case")]
pub async fn install_tool(
    state: State<'_, ToolManagerState>,
    tool_name: String,
) -> Result<()> {
    state.manager.install_tool(&tool_name).await
}

/// Check Python runtime compatibility
#[tauri::command(rename_all = "snake_case")]
pub async fn check_python_runtime(
    state: State<'_, ToolManagerState>,
) -> Result<(bool, Option<String>)> {
    state.manager.check_python_runtime().await
}


/// Get tool startup status for application boot check
#[tauri::command(rename_all = "snake_case")]
pub async fn get_tool_startup_status(
    state: State<'_, ToolManagerState>,
) -> Result<crate::types::ToolStartupStatus> {
    state.manager.get_startup_tool_status().await
}
