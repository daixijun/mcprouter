use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 权限实体
///
/// 对应数据库表 permissions，用于存储细粒度权限控制信息
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "permissions")]
pub struct Model {
    /// 主键，UUID v7 格式
    #[sea_orm(primary_key)]
    pub id: String,

    /// 关联的令牌ID
    pub token_id: String,

    /// 资源类型：'tool' | 'resource' | 'prompt'
    pub resource_type: String,

    /// 资源路径，格式：server__resource
    pub resource_path: String,

    /// 是否允许访问
    pub allowed: bool,

    /// 创建时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: ChronoDateTimeWithTimeZone,

    /// 更新时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub updated_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 与令牌表的多对一关系
    #[sea_orm(
        belongs_to = "super::token::Entity",
        from = "Column::TokenId",
        to = "super::token::Column::Id"
    )]
    Token,
}

impl Related<super::token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Token.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 资源类型枚举，用于类型安全的资源类型定义
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "lowercase")]
pub enum ResourceType {
    /// 工具权限
    Tool,
    /// 资源权限
    Resource,
    /// 提示词权限
    Prompt,
}

impl Model {
    /// 检查权限是否允许
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// 获取资源类型枚举
    pub fn get_resource_type_enum(&self) -> Result<ResourceType, strum::ParseError> {
        self.resource_type.parse::<ResourceType>()
    }

    /// 解析资源路径，返回 (server_name, resource_name)
    pub fn parse_resource_path(&self) -> Option<(&str, &str)> {
        let parts: Vec<&str> = self.resource_path.split("__").collect();
        if parts.len() >= 2 {
            Some((parts[0], parts[1]))
        } else {
            None
        }
    }

    /// 从 server_name 和 resource_name 构建资源路径
    pub fn build_resource_path(server_name: &str, resource_name: &str) -> String {
        format!("{}__{}", server_name, resource_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_model() {
        let model = Model {
            id: "test-id".to_string(),
            token_id: "token-id".to_string(),
            resource_type: "tool".to_string(),
            resource_path: "test-server__test-tool".to_string(),
            allowed: true,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(model.token_id, "token-id");
        assert_eq!(model.resource_type, "tool");
        assert!(model.is_allowed());

        let (server, resource) = model.parse_resource_path().unwrap();
        assert_eq!(server, "test-server");
        assert_eq!(resource, "test-tool");
    }

    #[test]
    fn test_resource_path_operations() {
        let path = Model::build_resource_path("test-server", "test-tool");
        assert_eq!(path, "test-server__test-tool");

        let model = Model {
            id: "test-id".to_string(),
            token_id: "token-id".to_string(),
            resource_type: "tool".to_string(),
            resource_path: path.clone(),
            allowed: true,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        let (server, resource) = model.parse_resource_path().unwrap();
        assert_eq!(server, "test-server");
        assert_eq!(resource, "test-tool");
    }

    #[test]
    fn test_resource_type_enum() {
        assert_eq!(ResourceType::Tool.to_string(), "tool");
        assert_eq!(ResourceType::Resource.to_string(), "resource");
        assert_eq!(ResourceType::Prompt.to_string(), "prompt");

        assert_eq!("tool".parse::<ResourceType>().unwrap(), ResourceType::Tool);
        assert_eq!(
            "resource".parse::<ResourceType>().unwrap(),
            ResourceType::Resource
        );
        assert_eq!(
            "prompt".parse::<ResourceType>().unwrap(),
            ResourceType::Prompt
        );

        assert!("invalid".parse::<ResourceType>().is_err());
    }
}

