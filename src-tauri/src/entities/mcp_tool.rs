use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// MCP 工具实体
///
/// 对应数据库表 mcp_server_tools，用于缓存 MCP 服务器提供的工具信息
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "mcp_server_tools")]
pub struct Model {
    /// 主键，UUID v7 格式
    #[sea_orm(primary_key)]
    pub id: String,

    /// 关联的服务器ID
    pub server_id: String,

    /// 工具名称
    pub name: String,

    /// 工具标题
    pub title: Option<String>,

    /// 工具描述
    pub description: Option<String>,

    /// 是否启用
    pub enabled: bool,

    /// 输入架构（JSON schema）
    pub input_schema: Option<String>,

    /// 输出架构（JSON schema）
    pub output_schema: Option<String>,

    /// 工具注解（JSON object）
    pub annotations: Option<String>,

    /// 元数据（JSON object）
    pub meta: Option<String>,

    /// 创建时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: ChronoDateTimeWithTimeZone,

    /// 更新时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub updated_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 与服务器表的多对一关系
    #[sea_orm(
        belongs_to = "super::mcp_server::Entity",
        from = "Column::ServerId",
        to = "super::mcp_server::Column::Id"
    )]
    Server,
}

impl Related<super::mcp_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Server.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 解析输入架构
    pub fn parse_input_schema(&self) -> Result<serde_json::Value, serde_json::Error> {
        match &self.input_schema {
            Some(schema_str) => serde_json::from_str(schema_str),
            None => Ok(serde_json::Value::Object(serde_json::Map::new())),
        }
    }

    /// 序列化输入架构
    pub fn serialize_input_schema(schema: &serde_json::Value) -> String {
        serde_json::to_string(schema).unwrap_or_default()
    }

    /// 解析输出架构
    pub fn parse_output_schema(&self) -> Result<serde_json::Value, serde_json::Error> {
        match &self.output_schema {
            Some(schema_str) => serde_json::from_str(schema_str),
            None => Ok(serde_json::Value::Object(serde_json::Map::new())),
        }
    }

    /// 序列化输出架构
    pub fn serialize_output_schema(schema: &serde_json::Value) -> String {
        serde_json::to_string(schema).unwrap_or_default()
    }

    /// 解析注解
    pub fn parse_annotations(&self) -> Result<serde_json::Value, serde_json::Error> {
        match &self.annotations {
            Some(annotations_str) => serde_json::from_str(annotations_str),
            None => Ok(serde_json::Value::Object(serde_json::Map::new())),
        }
    }

    /// 序列化注解
    pub fn serialize_annotations(annotations: &serde_json::Value) -> String {
        serde_json::to_string(annotations).unwrap_or_default()
    }

    /// 解析元数据
    pub fn parse_meta(&self) -> Result<serde_json::Value, serde_json::Error> {
        match &self.meta {
            Some(meta_str) => serde_json::from_str(meta_str),
            None => Ok(serde_json::Value::Object(serde_json::Map::new())),
        }
    }

    /// 序列化元数据
    pub fn serialize_meta(meta: &serde_json::Value) -> String {
        serde_json::to_string(meta).unwrap_or_default()
    }

    /// 检查工具是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取工具显示名称（优先使用 title，其次是 name）
    pub fn get_display_name(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.name)
    }

    /// 获取完整的工具路径（server__tool 格式）
    pub fn get_full_path(&self) -> String {
        // 注意：这里我们需要 server 的名称，但这需要 join 查询
        // 在实际使用中，应该通过 join 查询来获取
        format!("__{}", self.name)
    }

    /// 检查是否有输入架构
    pub fn has_input_schema(&self) -> bool {
        self.input_schema.is_some()
    }

    /// 获取输入架构类型
    pub fn get_input_schema_type(&self) -> Option<String> {
        if let Some(schema_str) = &self.input_schema {
            if let Ok(schema) = serde_json::from_str::<serde_json::Value>(schema_str) {
                if let Some(obj) = schema.as_object() {
                    if let Some(type_val) = obj.get("type") {
                        if let Some(type_str) = type_val.as_str() {
                            return Some(type_str.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// 检查是否有输出架构
    pub fn has_output_schema(&self) -> bool {
        self.output_schema.is_some()
    }

    /// 获取输出架构类型
    pub fn get_output_schema_type(&self) -> Option<String> {
        if let Some(schema_str) = &self.output_schema {
            if let Ok(schema) = serde_json::from_str::<serde_json::Value>(schema_str) {
                if let Some(obj) = schema.as_object() {
                    if let Some(type_val) = obj.get("type") {
                        if let Some(type_str) = type_val.as_str() {
                            return Some(type_str.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// 检查是否有注解
    pub fn has_annotations(&self) -> bool {
        self.annotations.is_some()
    }

    /// 检查是否有元数据
    pub fn has_meta(&self) -> bool {
        self.meta.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_model() {
        let input_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to send"
                }
            },
            "required": ["message"]
        });

        let output_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "success": {
                    "type": "boolean",
                    "description": "Whether the message was sent successfully"
                },
                "message_id": {
                    "type": "string",
                    "description": "The ID of the sent message"
                }
            }
        });

        let annotations = serde_json::json!({
            "experimental": false,
            "deprecated": false,
            "author": "MCP Team"
        });

        let meta = serde_json::json!({
            "category": "communication",
            "tags": ["message", "send"]
        });

        let model = Model {
            id: "test-id".to_string(),
            server_id: "server-id".to_string(),
            name: "send_message".to_string(),
            title: Some("Send Message".to_string()),
            description: Some("Sends a message to the specified recipient".to_string()),
            enabled: true,
            input_schema: Some(serde_json::to_string(&input_schema).unwrap()),
            output_schema: Some(serde_json::to_string(&output_schema).unwrap()),
            annotations: Some(serde_json::to_string(&annotations).unwrap()),
            meta: Some(serde_json::to_string(&meta).unwrap()),
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(model.name, "send_message");
        assert_eq!(model.get_display_name(), "Send Message");
        assert!(model.is_enabled());
        assert!(model.has_input_schema());

        let parsed_schema = model.parse_input_schema().unwrap();
        assert_eq!(parsed_schema["type"], "object");
        assert_eq!(parsed_schema["required"][0], "message");

        assert_eq!(model.get_input_schema_type(), Some("object".to_string()));
        assert!(model.has_output_schema());
        assert_eq!(model.get_output_schema_type(), Some("object".to_string()));
        assert!(model.has_annotations());
        assert!(model.has_meta());

        let parsed_output_schema = model.parse_output_schema().unwrap();
        assert_eq!(parsed_output_schema["type"], "object");
        assert!(parsed_output_schema["properties"]["success"].is_object());

        let parsed_annotations = model.parse_annotations().unwrap();
        assert_eq!(parsed_annotations["experimental"], false);
        assert_eq!(parsed_annotations["author"], "MCP Team");

        let parsed_meta = model.parse_meta().unwrap();
        assert_eq!(parsed_meta["category"], "communication");
        assert!(parsed_meta["tags"].is_array());
    }

    #[test]
    fn test_tool_without_title() {
        let model = Model {
            id: "test-id".to_string(),
            server_id: "server-id".to_string(),
            name: "simple_tool".to_string(),
            title: None,
            description: None,
            enabled: true,
            input_schema: None,
            output_schema: None,
            annotations: None,
            meta: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(model.get_display_name(), "simple_tool");
        assert!(!model.has_input_schema());
        assert!(model.get_input_schema_type().is_none());
    }

    #[test]
    fn test_serialization_methods() {
        let schema = serde_json::json!({
            "type": "string",
            "description": "A simple string input"
        });

        let serialized = Model::serialize_input_schema(&schema);
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(schema, parsed);

        // 测试空架构
        let empty_schema = serde_json::Value::Object(serde_json::Map::new());
        let serialized_empty = Model::serialize_input_schema(&empty_schema);
        assert!(!serialized_empty.is_empty());
    }

    #[test]
    fn test_full_path() {
        let model = Model {
            id: "test-id".to_string(),
            server_id: "server-id".to_string(),
            name: "test_tool".to_string(),
            title: None,
            description: None,
            enabled: true,
            input_schema: None,
            output_schema: None,
            annotations: None,
            meta: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // 注意：这只是一个基础路径，实际使用中需要通过 join 查询获取服务器名称
        let path = model.get_full_path();
        assert_eq!(path, "__test_tool");
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: String::new(),
            server_id: String::new(),
            name: String::new(),
            title: None,
            description: None,
            enabled: true,
            input_schema: None,
            output_schema: None,
            annotations: None,
            meta: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        }
    }
}
