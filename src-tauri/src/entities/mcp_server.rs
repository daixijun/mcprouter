use crate::types::ServiceTransport;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// MCP 服务器实体
///
/// 对应数据库表 mcp_servers，用于存储 MCP 服务器配置信息
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "mcp_servers")]
pub struct Model {
    /// 主键，UUID v7 格式
    #[sea_orm(primary_key)]
    pub id: String,

    /// 服务器名称（唯一约束）
    #[sea_orm(unique)]
    pub name: String,

    /// 服务器描述
    pub description: Option<String>,

    /// 服务器类型：'stdio' | 'http'
    #[sea_orm(column_name = "type")]
    pub server_type: String,

    /// 命令（stdio 类型）
    pub command: Option<String>,

    /// 参数（JSON 数组）
    pub args: Option<String>,

    /// URL（http 类型）
    pub url: Option<String>,

    /// 请求头（JSON 对象）
    pub headers: Option<String>,

    /// 环境变量（JSON 对象）
    pub env: Option<String>,

    /// 是否启用
    pub enabled: bool,

    /// 服务器版本
    pub version: Option<String>,

    /// 最后版本检查时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub last_version_check: Option<ChronoDateTimeWithTimeZone>,

    /// 创建时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: ChronoDateTimeWithTimeZone,

    /// 更新时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub updated_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 与工具表的一对多关系
    #[sea_orm(has_many = "super::mcp_tool::Entity")]
    Tools,

    /// 与资源表的一对多关系
    #[sea_orm(has_many = "super::mcp_resource::Entity")]
    Resources,

    /// 与提示词表的一对多关系
    #[sea_orm(has_many = "super::mcp_prompt::Entity")]
    Prompts,
}

impl Related<super::mcp_tool::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tools.def()
    }
}

impl Related<super::mcp_resource::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Resources.def()
    }
}

impl Related<super::mcp_prompt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Prompts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 获取服务器类型枚举
    pub fn get_server_type(&self) -> Result<ServiceTransport, strum::ParseError> {
        self.server_type.parse::<ServiceTransport>()
    }

    /// 解析命令行参数
    pub fn parse_args(&self) -> Result<Vec<String>, serde_json::Error> {
        match &self.args {
            Some(args_str) => serde_json::from_str(args_str),
            None => Ok(Vec::new()),
        }
    }

    /// 序列化命令行参数
    pub fn serialize_args(args: &[String]) -> String {
        serde_json::to_string(args).unwrap_or_default()
    }

    /// 解析请求头
    pub fn parse_headers(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, serde_json::Error> {
        match &self.headers {
            Some(headers_str) => serde_json::from_str(headers_str),
            None => Ok(std::collections::HashMap::new()),
        }
    }

    /// 序列化请求头
    pub fn serialize_headers(headers: &std::collections::HashMap<String, String>) -> String {
        serde_json::to_string(headers).unwrap_or_default()
    }

    /// 解析环境变量
    pub fn parse_env(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, serde_json::Error> {
        match &self.env {
            Some(env_str) => serde_json::from_str(env_str),
            None => Ok(std::collections::HashMap::new()),
        }
    }

    /// 序列化环境变量
    pub fn serialize_env(env: &std::collections::HashMap<String, String>) -> String {
        serde_json::to_string(env).unwrap_or_default()
    }

    /// 检查服务器是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 检查是否为 stdio 类型
    pub fn is_stdio_type(&self) -> bool {
        self.get_server_type()
            .map(|t| t == ServiceTransport::Stdio)
            .unwrap_or(false)
    }

    /// 检查是否为 http 类型
    pub fn is_http_type(&self) -> bool {
        self.get_server_type()
            .map(|t| t == ServiceTransport::Http)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ServiceTransport;
    use std::collections::HashMap;

    #[test]
    fn test_mcp_server_model() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin".to_string());

        let model = Model {
            id: "test-id".to_string(),
            name: "test-server".to_string(),
            description: Some("Test MCP server".to_string()),
            server_type: "stdio".to_string(),
            command: Some("node".to_string()),
            args: Some(serde_json::to_string(&vec!["server.js".to_string()]).unwrap()),
            url: None,
            headers: Some(serde_json::to_string(&headers).unwrap()),
            env: Some(serde_json::to_string(&env).unwrap()),
            enabled: true,
            version: Some("1.0.0".to_string()),
            last_version_check: Some(chrono::Utc::now().into()),
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(model.name, "test-server");
        assert_eq!(model.get_server_type().unwrap(), ServiceTransport::Stdio);
        assert!(model.is_enabled());
        assert!(model.is_stdio_type());
        assert!(!model.is_http_type());

        let parsed_args = model.parse_args().unwrap();
        assert_eq!(parsed_args, vec!["server.js"]);

        let parsed_headers = model.parse_headers().unwrap();
        assert_eq!(parsed_headers.get("Authorization").unwrap(), "Bearer token");

        let parsed_env = model.parse_env().unwrap();
        assert_eq!(parsed_env.get("PATH").unwrap(), "/usr/bin");
    }

    #[test]
    fn test_serialization_methods() {
        let args = vec![
            "node".to_string(),
            "server.js".to_string(),
            "--port".to_string(),
            "8080".to_string(),
        ];
        let serialized_args = Model::serialize_args(&args);
        let parsed_args: Vec<String> = serde_json::from_str(&serialized_args).unwrap();
        assert_eq!(args, parsed_args);

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        let serialized_headers = Model::serialize_headers(&headers);
        let parsed_headers: HashMap<String, String> =
            serde_json::from_str(&serialized_headers).unwrap();
        assert_eq!(headers, parsed_headers);
    }

    #[test]
    fn test_server_type_parsing() {
        let stdio_model = Model {
            id: "test-id".to_string(),
            name: "stdio-server".to_string(),
            server_type: "stdio".to_string(),
            ..Default::default()
        };
        assert_eq!(
            stdio_model.get_server_type().unwrap(),
            ServiceTransport::Stdio
        );

        let http_model = Model {
            id: "test-id".to_string(),
            name: "http-server".to_string(),
            server_type: "http".to_string(),
            ..Default::default()
        };
        assert_eq!(
            http_model.get_server_type().unwrap(),
            ServiceTransport::Http
        );

        let invalid_model = Model {
            id: "test-id".to_string(),
            name: "invalid-server".to_string(),
            server_type: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_model.get_server_type().is_err());
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: None,
            server_type: String::new(),
            command: None,
            args: None,
            url: None,
            headers: None,
            env: None,
            enabled: true,
            version: None,
            last_version_check: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        }
    }
}

