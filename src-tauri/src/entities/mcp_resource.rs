use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// MCP 资源实体
///
/// 对应数据库表 mcp_server_resources，用于缓存 MCP 服务器提供的资源信息
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "mcp_server_resources")]
pub struct Model {
    /// 主键，UUID v7 格式
    #[sea_orm(primary_key)]
    pub id: String,

    /// 关联的服务器ID
    pub server_id: String,

    /// 资源URI
    pub uri: String,

    /// 资源名称
    pub name: Option<String>,

    /// 资源标题
    pub title: Option<String>,

    /// 资源描述
    pub description: Option<String>,

    /// MIME 类型
    pub mime_type: Option<String>,

    /// 是否启用
    pub enabled: bool,

    /// 是否为模板资源
    pub is_template: bool,

    /// URI 模板（用于模板资源）
    pub uri_template: Option<String>,

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
    /// 检查资源是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 检查是否为模板资源
    pub fn is_template_resource(&self) -> bool {
        self.is_template
    }

    /// 获取资源显示名称（优先级：title > name > uri 最后部分）
    pub fn get_display_name(&self) -> &str {
        self.title
            .as_deref()
            .or(self.name.as_deref())
            .unwrap_or_else(|| {
                // 从 URI 中提取最后部分作为显示名称
                self.uri.split('/').next_back().unwrap_or(&self.uri)
            })
    }

    /// 获取资源的完整路径（server__resource 格式）
    pub fn get_full_path(&self) -> String {
        // 注意：这里我们需要 server 的名称，但这需要 join 查询
        // 在实际使用中，应该通过 join 查询来获取
        format!("__{}", self.uri)
    }

    /// 获取资源描述信息
    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
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
            uri: String::new(),
            name: None,
            title: None,
            description: None,
            mime_type: None,
            enabled: true,
            is_template: false,
            uri_template: None,
            meta: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        }
    }
}
