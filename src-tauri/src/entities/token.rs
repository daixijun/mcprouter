use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Token 实体
///
/// 对应数据库表 tokens，用于存储 API 认证令牌信息
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tokens")]
pub struct Model {
    /// 主键，UUID v7 格式
    #[sea_orm(primary_key)]
    pub id: String,

    /// 令牌名称
    pub name: String,

    /// 令牌值（唯一约束）
    #[sea_orm(unique)]
    pub token: String,

    /// 令牌描述
    pub description: Option<String>,

    /// 创建时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: ChronoDateTimeWithTimeZone,

    /// 更新时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub updated_at: ChronoDateTimeWithTimeZone,

    /// 是否启用
    pub enabled: bool,

    /// 最后使用时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub last_used_at: Option<ChronoDateTimeWithTimeZone>,

    /// 使用次数
    pub usage_count: i32,

    /// 过期时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub expires_at: Option<ChronoDateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 与权限表的一对多关系
    #[sea_orm(has_many = "super::permission::Entity")]
    Permissions,
}

impl Related<super::permission::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Permissions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ActiveValue::Set;

    #[test]
    fn test_token_model() {
        let model = Model {
            id: "test-id".to_string(),
            name: "Test Token".to_string(),
            token: "test-token".to_string(),
            description: Some("Test token description".to_string()),
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            enabled: true,
            last_used_at: Some(chrono::Utc::now().into()),
            usage_count: 10,
            expires_at: Some(chrono::Utc::now().into()),
        };

        assert_eq!(model.name, "Test Token");
        assert_eq!(model.token, "test-token");
        assert_eq!(model.usage_count, 10);
        assert!(model.enabled);
    }

    #[test]
    fn test_token_active_model() {
        let active_model = ActiveModel {
            id: Set("test-id".to_string()),
            name: Set("Test Token".to_string()),
            token: Set("test-token".to_string()),
            description: Set(Some("Test token description".to_string())),
            ..Default::default()
        };

        assert_eq!(active_model.name.unwrap(), "Test Token");
        assert_eq!(active_model.token.unwrap(), "test-token");
        assert!(active_model.description.unwrap().is_some());
    }
}

