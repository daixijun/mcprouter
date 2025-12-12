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


