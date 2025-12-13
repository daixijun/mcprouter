use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite 不支持在单个 ALTER TABLE 语句中删除多个列
        // 需要分别执行

        // 先删除 version 列
        manager
            .alter_table(
                Table::alter()
                    .table(McpServers::Table)
                    .drop_column(McpServers::Version)
                    .to_owned(),
            )
            .await?;

        // 再删除 last_version_check 列
        manager
            .alter_table(
                Table::alter()
                    .table(McpServers::Table)
                    .drop_column(McpServers::LastVersionCheck)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite 不支持在单个 ALTER TABLE 语句中添加多个列
        // 需要分别执行

        // 先添加 version 列
        manager
            .alter_table(
                Table::alter()
                    .table(McpServers::Table)
                    .add_column(ColumnDef::new(McpServers::Version).string().null())
                    .to_owned(),
            )
            .await?;

        // 再添加 last_version_check 列
        manager
            .alter_table(
                Table::alter()
                    .table(McpServers::Table)
                    .add_column(ColumnDef::new(McpServers::LastVersionCheck).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum McpServers {
    Table,
    Version,
    LastVersionCheck,
}