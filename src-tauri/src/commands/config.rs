// 配置管理命令

use crate::config as config_mod;
use crate::error::Result;
use crate::SERVICE_MANAGER;
use tauri::Emitter;

#[tauri::command]
pub async fn get_config() -> Result<config_mod::AppConfig> {
    Ok(SERVICE_MANAGER.get_config().await)
}

#[tauri::command]
pub async fn get_theme() -> Result<String> {
    let config = SERVICE_MANAGER.get_config().await;
    let theme = config
        .settings
        .as_ref()
        .and_then(|s| s.theme.as_ref())
        .cloned()
        .unwrap_or_else(|| "auto".to_string());
    Ok(theme)
}

#[tauri::command]
pub async fn set_theme(app: tauri::AppHandle, theme: String) -> Result<()> {
    SERVICE_MANAGER
        .update_config(|config| {
            if config.settings.is_none() {
                config.settings = Some(crate::config::Settings {
                    theme: Some("auto".to_string()),
                    autostart: Some(false),
                    system_tray: Some(crate::config::SystemTraySettings {
                        enabled: Some(true),
                        close_to_tray: Some(false),
                        start_to_tray: Some(false),
                    }),
                    uv_index_url: None,
                    npm_registry: None,
                });
            }
            if let Some(settings) = config.settings.as_mut() {
                settings.theme = Some(theme.clone());
            }
        })
        .await?;
    let _ = app.emit("theme-changed", theme);
    Ok(())
}

#[tauri::command]
pub async fn update_config(config: config_mod::AppConfig) -> Result<String> {
    config.save()?;
    Ok("Config updated".to_string())
}
