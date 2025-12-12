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
