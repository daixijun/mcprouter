//! SeaORM 数据库迁移模块
//!
//! 使用 SeaORM Migration 管理数据库版本

use sea_orm_migration::prelude::*;

mod m20240101_000001_create_initial_tables;
mod m20241212_000002_add_mcp_metadata_fields;
mod m20250113_remove_version_fields;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_initial_tables::Migration),
            Box::new(m20241212_000002_add_mcp_metadata_fields::Migration),
            Box::new(m20250113_remove_version_fields::Migration),
        ]
    }
}
