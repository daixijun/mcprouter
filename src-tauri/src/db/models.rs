use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// MCP 服务器数据库模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRow {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>, // 从 JSON 字符串解析
    pub transport: String,
    pub url: Option<String>,
    pub enabled: bool,
    pub env_vars: Option<std::collections::HashMap<String, String>>, // 从 JSON 字符串解析
    pub headers: Option<std::collections::HashMap<String, String>>,  // 从 JSON 字符串解析
    pub version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 工具数据库模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRow {
    pub id: Option<String>,
    pub name: String,
    pub server_id: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API 密钥数据库模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRow {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// API 密钥-服务器关联模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyServerRelationRow {
    pub id: Option<String>,
    pub api_key_id: String,
    pub server_id: String,
    pub created_at: DateTime<Utc>,
}

impl ApiKeyRow {
    pub fn new(name: String, key_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            key_hash,
            enabled: true,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        }
    }
}

impl ApiKeyServerRelationRow {
    pub fn new(api_key_id: String, server_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Some(uuid::Uuid::new_v4().to_string()),
            api_key_id,
            server_id,
            created_at: now,
        }
    }
}
