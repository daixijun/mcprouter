use crate::error::Result;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// 应用信息结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
}

/// 获取应用完整信息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_app_info(app: AppHandle) -> Result<AppInfo> {
    let package_info = app.package_info();

    Ok(AppInfo {
        version: package_info.version.to_string(),
        name: package_info.name.clone(),
        description: Some(package_info.description.to_string()),
        authors: Vec::new(), // Tauri 2.x doesn't provide authors in package_info
    })
}

/// 获取应用版本号
#[tauri::command(rename_all = "snake_case")]
pub async fn get_app_version(app: AppHandle) -> Result<String> {
    Ok(app.package_info().version.to_string())
}

/// 获取应用名称
#[tauri::command(rename_all = "snake_case")]
pub async fn get_app_name(app: AppHandle) -> Result<String> {
    Ok(app.package_info().name.clone())
}

/// 获取MCP服务器信息
pub fn get_mcp_server_info(app: &AppHandle) -> rmcp::model::Implementation {
    rmcp::model::Implementation {
        name: "MCP Router Aggregator".to_string(),
        version: app.package_info().version.to_string(),
        icons: None,
        title: None,
        website_url: None,
    }
}

/// 获取HTTP User-Agent字符串
pub fn get_user_agent(app: &AppHandle) -> String {
    format!("mcprouter/{}", app.package_info().version)
}

/// 获取HTTP User-Agent字符串（从Cargo.toml版本）
pub fn get_user_agent_static() -> String {
    format!("mcprouter/{}", env!("CARGO_PKG_VERSION"))
}
