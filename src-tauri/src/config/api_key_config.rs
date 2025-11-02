//! API密钥配置管理
//!
//! 管理API密钥的存储、检索、更新和删除
//! 使用SHA256哈希存储，确保安全性

use super::file_manager::{exists, read_json, write_json_atomic};
use super::{get_api_keys_config_path, ConfigError, Result};
use crate::types::ApiKeyPermissions;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::Manager;

// Note: HashSet<String> serialization/deserialization handled by serde_json
// using flatten and default implementations

/// API密钥存储结构
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ApiKeyConfig {
    pub id: String,
    pub name: String,
    /// SHA256哈希值，不存储明文
    pub key_hash: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 授权的工具ID列表
    #[serde(default)]
    pub authorized_tool_ids: Vec<String>,
}

/// API密钥列表项（用于列表显示，隐藏敏感信息）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ApiKeyListItem {
    pub id: String,
    pub name: String,
    /// 隐藏的密钥（显示前6位和后3位）
    pub key: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub authorized_server_count: u32,
    pub authorized_tool_count: u32,
}

/// API密钥仓库
#[derive(Debug)]
pub struct ApiKeyRepository {
    config_path: PathBuf,
    api_keys: Vec<ApiKeyConfig>,
}

impl ApiKeyRepository {
    /// 创建新的API密钥仓库
    pub async fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|_| ConfigError::Invalid("无法获取应用数据目录".to_string()))?;

        let config_path = get_api_keys_config_path(&app_data_dir);

        // 加载现有配置或创建新的
        let api_keys = if exists(&config_path) {
            read_json(&config_path)?
        } else {
            Vec::new()
        };

        Ok(Self {
            config_path,
            api_keys,
        })
    }

    /// 生成随机API密钥
    fn generate_api_key() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_chars: String = (0..32)
            .map(|_| {
                const CHARSET: &[u8] =
                    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        format!("sk-{}", random_chars)
    }

    /// 计算密钥的SHA256哈希
    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key);
        format!("{:x}", hasher.finalize())
    }

    /// 创建新的API密钥
    pub fn create(
        &mut self,
        name: String,
        permissions: ApiKeyPermissions,
    ) -> (String, ApiKeyConfig) {
        let key = Self::generate_api_key();
        let key_hash = Self::hash_key(&key);
        let now = chrono::Utc::now();

        let api_key = ApiKeyConfig {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            key_hash,
            enabled: true,
            created_at: now,
            updated_at: now,
            last_used_at: None,
            authorized_tool_ids: permissions.allowed_tools,
        };

        self.api_keys.push(api_key.clone());

        (key, api_key)
    }

    /// 获取所有API密钥（列表格式）
    pub fn get_all_list(&self) -> Result<Vec<ApiKeyListItem>> {
        let mut list_items = Vec::new();

        for api_key in &self.api_keys {
            // 隐藏密钥（显示前6位和后3位）
            let masked_key = if api_key.key_hash.len() > 9 {
                format!(
                    "sk-{}...{}",
                    &api_key.key_hash[..6],
                    &api_key.key_hash[api_key.key_hash.len() - 3..]
                )
            } else {
                "sk-****".to_string()
            };

            // 计算授权的服务器和工具数量
            let authorized_server_count =
                self.count_authorized_servers(&api_key.authorized_tool_ids);
            let authorized_tool_count = api_key.authorized_tool_ids.len() as u32;

            list_items.push(ApiKeyListItem {
                id: api_key.id.clone(),
                name: api_key.name.clone(),
                key: masked_key,
                enabled: api_key.enabled,
                created_at: api_key.created_at,
                updated_at: api_key.updated_at,
                authorized_server_count,
                authorized_tool_count,
            });
        }

        Ok(list_items)
    }

    /// 根据ID获取API密钥详情
    pub fn get_by_id(&self, id: &str) -> Option<&ApiKeyConfig> {
        self.api_keys.iter().find(|k| k.id == id)
    }

    /// 根据ID获取API密钥（可修改）
    pub fn get_by_id_mut(&mut self, id: &str) -> Option<&mut ApiKeyConfig> {
        self.api_keys.iter_mut().find(|k| k.id == id)
    }

    /// 验证API密钥
    pub fn verify_key(&mut self, id: &str, key: &str) -> bool {
        if let Some(api_key) = self.get_by_id_mut(id) {
            if !api_key.enabled {
                return false;
            }

            let input_hash = Self::hash_key(key);
            if input_hash == api_key.key_hash {
                api_key.last_used_at = Some(chrono::Utc::now());
                api_key.updated_at = chrono::Utc::now();
                return true;
            }
        }
        false
    }

    /// 切换API密钥启用状态
    pub fn toggle_enabled(&mut self, id: &str) -> Result<bool> {
        if let Some(api_key) = self.get_by_id_mut(id) {
            api_key.enabled = !api_key.enabled;
            api_key.updated_at = chrono::Utc::now();
            Ok(api_key.enabled)
        } else {
            Err(ConfigError::Invalid(format!("API密钥未找到: {}", id)))
        }
    }

    /// 删除API密钥
    pub fn delete(&mut self, id: &str) -> Result<bool> {
        let original_len = self.api_keys.len();
        self.api_keys.retain(|k| k.id != id);
        Ok(self.api_keys.len() < original_len)
    }

    /// 添加工具权限
    pub fn add_tool_permission(&mut self, api_key_id: &str, tool_id: &str) -> Result<()> {
        if let Some(api_key) = self.get_by_id_mut(api_key_id) {
            if !api_key.authorized_tool_ids.iter().any(|id| id == tool_id) {
                api_key.authorized_tool_ids.push(tool_id.to_string());
            }
            api_key.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ConfigError::Invalid(format!(
                "API密钥未找到: {}",
                api_key_id
            )))
        }
    }

    /// 移除工具权限
    pub fn remove_tool_permission(&mut self, api_key_id: &str, tool_id: &str) -> Result<()> {
        if let Some(api_key) = self.get_by_id_mut(api_key_id) {
            api_key.authorized_tool_ids.retain(|id| id != tool_id);
            api_key.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ConfigError::Invalid(format!(
                "API密钥未找到: {}",
                api_key_id
            )))
        }
    }

    /// 获取API密钥的工具列表
    pub fn get_tools_by_api_key(&self, api_key_id: &str) -> Result<Vec<String>> {
        if let Some(api_key) = self.get_by_id(api_key_id) {
            Ok(api_key.authorized_tool_ids.iter().cloned().collect())
        } else {
            Err(ConfigError::Invalid(format!(
                "API密钥未找到: {}",
                api_key_id
            )))
        }
    }

    /// 移除API密钥的所有权限
    pub fn remove_all_permissions(&mut self, api_key_id: &str) -> Result<()> {
        if let Some(api_key) = self.get_by_id_mut(api_key_id) {
            api_key.authorized_tool_ids = Vec::new();
            api_key.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ConfigError::Invalid(format!(
                "API密钥未找到: {}",
                api_key_id
            )))
        }
    }

    /// 批量授权服务器的所有工具
    pub fn grant_server_tools(
        &mut self,
        api_key_id: &str,
        server_tools: &[String],
    ) -> Result<usize> {
        if let Some(api_key) = self.get_by_id_mut(api_key_id) {
            let count_before = api_key.authorized_tool_ids.len();
            for tool_id in server_tools {
                if !api_key.authorized_tool_ids.contains(tool_id) {
                    api_key.authorized_tool_ids.push(tool_id.clone());
                }
            }
            api_key.updated_at = chrono::Utc::now();
            Ok(api_key.authorized_tool_ids.len() - count_before)
        } else {
            Err(ConfigError::Invalid(format!(
                "API密钥未找到: {}",
                api_key_id
            )))
        }
    }

    /// 撤销服务器的所有工具权限
    pub fn revoke_server_tools(
        &mut self,
        api_key_id: &str,
        server_tools: &[String],
    ) -> Result<usize> {
        if let Some(api_key) = self.get_by_id_mut(api_key_id) {
            let count_before = api_key.authorized_tool_ids.len();
            api_key
                .authorized_tool_ids
                .retain(|tool_id| !server_tools.contains(tool_id));
            api_key.updated_at = chrono::Utc::now();
            Ok(count_before - api_key.authorized_tool_ids.len())
        } else {
            Err(ConfigError::Invalid(format!(
                "API密钥未找到: {}",
                api_key_id
            )))
        }
    }

    /// 保存到文件
    pub fn save(&self) -> Result<()> {
        write_json_atomic(&self.config_path, &self.api_keys)
    }

    /// 从工具ID计算授权的服务器数量（简化版本）
    fn count_authorized_servers(&self, tool_ids: &Vec<String>) -> u32 {
        // 由于无法访问MCP服务器信息，简化处理
        // 如果有工具权限，至少算作1个服务器
        if tool_ids.is_empty() {
            0
        } else {
            // 简单估算：每个工具可能属于不同服务器
            // 实际实现中需要从MCP服务器配置中查询
            std::cmp::min(tool_ids.len() as u32, 5) // 最多5个服务器，避免过高估算
        }
    }
}
