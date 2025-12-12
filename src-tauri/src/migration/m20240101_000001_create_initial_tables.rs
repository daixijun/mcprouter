//! 创建初始表结构
//!
//! 对应现有的 sqlx 迁移，使用 SeaORM Migration 重新实现

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 tokens 表
        manager
            .create_table(
                Table::create()
                    .table(Tokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tokens::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tokens::Name).string().not_null())
                    .col(ColumnDef::new(Tokens::Token).string().not_null().unique_key())
                    .col(ColumnDef::new(Tokens::Description).string())
                    .col(
                        ColumnDef::new(Tokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Tokens::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Tokens::Enabled).boolean().not_null().default(true))
                    .col(
                        ColumnDef::new(Tokens::LastUsedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(ColumnDef::new(Tokens::UsageCount).integer().not_null().default(0))
                    .col(
                        ColumnDef::new(Tokens::ExpiresAt)
                            .timestamp_with_time_zone(),
                    )
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // 创建 mcp_servers 表
        manager
            .create_table(
                Table::create()
                    .table(McpServers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(McpServers::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(McpServers::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(McpServers::Description).string())
                    .col(ColumnDef::new(McpServers::Type).string().not_null())
                    .col(ColumnDef::new(McpServers::Command).string())
                    .col(ColumnDef::new(McpServers::Args).string())
                    .col(ColumnDef::new(McpServers::Url).string())
                    .col(ColumnDef::new(McpServers::Headers).string())
                    .col(ColumnDef::new(McpServers::Env).string())
                    .col(ColumnDef::new(McpServers::Enabled).boolean().not_null().default(true))
                    .col(ColumnDef::new(McpServers::Version).string())
                    .col(
                        ColumnDef::new(McpServers::LastVersionCheck)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(McpServers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(McpServers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // 创建 mcp_server_tools 表
        manager
            .create_table(
                Table::create()
                    .table(McpServerTools::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(McpServerTools::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(McpServerTools::ServerId).string().not_null())
                    .col(ColumnDef::new(McpServerTools::Name).string().not_null())
                    .col(ColumnDef::new(McpServerTools::Title).string())
                    .col(ColumnDef::new(McpServerTools::Description).string())
                    .col(
                        ColumnDef::new(McpServerTools::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(McpServerTools::InputSchema).string())
                    .col(
                        ColumnDef::new(McpServerTools::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(McpServerTools::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // 创建 mcp_server_resources 表
        manager
            .create_table(
                Table::create()
                    .table(McpServerResources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(McpServerResources::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(McpServerResources::ServerId).string().not_null())
                    .col(ColumnDef::new(McpServerResources::Uri).string().not_null())
                    .col(ColumnDef::new(McpServerResources::Name).string())
                    .col(ColumnDef::new(McpServerResources::Title).string())
                    .col(ColumnDef::new(McpServerResources::Description).string())
                    .col(ColumnDef::new(McpServerResources::MimeType).string())
                    .col(
                        ColumnDef::new(McpServerResources::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(McpServerResources::IsTemplate)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(McpServerResources::UriTemplate).string())
                    .col(
                        ColumnDef::new(McpServerResources::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(McpServerResources::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // 创建 mcp_server_prompts 表
        manager
            .create_table(
                Table::create()
                    .table(McpServerPrompts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(McpServerPrompts::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(McpServerPrompts::ServerId).string().not_null())
                    .col(ColumnDef::new(McpServerPrompts::Name).string().not_null())
                    .col(ColumnDef::new(McpServerPrompts::Title).string())
                    .col(ColumnDef::new(McpServerPrompts::Description).string())
                    .col(
                        ColumnDef::new(McpServerPrompts::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(McpServerPrompts::Arguments).string())
                    .col(
                        ColumnDef::new(McpServerPrompts::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(McpServerPrompts::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // 创建 permissions 表
        manager
            .create_table(
                Table::create()
                    .table(Permissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Permissions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Permissions::TokenId).string().not_null())
                    .col(ColumnDef::new(Permissions::ResourceType).string().not_null())
                    .col(ColumnDef::new(Permissions::ResourcePath).string().not_null())
                    .col(
                        ColumnDef::new(Permissions::Allowed)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Permissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Permissions::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // 创建索引
        self.create_indexes(manager).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 按依赖关系反向删除表
        manager
            .drop_table(Table::drop().table(Permissions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(McpServerPrompts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(McpServerResources::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(McpServerTools::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(McpServers::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tokens::Table).to_owned())
            .await?;

        Ok(())
    }
}

impl Migration {
    async fn create_indexes(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        // tokens 表索引
        manager
            .create_index(
                Index::create()
                    .name("idx_tokens_name")
                    .table(Tokens::Table)
                    .col(Tokens::Name)
                    .if_not_exists()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // mcp_servers 表索引
        manager
            .create_index(
                Index::create()
                    .name("idx_mcp_servers_enabled")
                    .table(McpServers::Table)
                    .col(McpServers::Enabled)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // mcp_server_tools 表索引
        manager
            .create_index(
                Index::create()
                    .name("idx_mcp_server_tools_server_id")
                    .table(McpServerTools::Table)
                    .col(McpServerTools::ServerId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mcp_server_tools_server_name")
                    .table(McpServerTools::Table)
                    .col(McpServerTools::ServerId)
                    .col(McpServerTools::Name)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // mcp_server_resources 表索引
        manager
            .create_index(
                Index::create()
                    .name("idx_mcp_server_resources_server_id")
                    .table(McpServerResources::Table)
                    .col(McpServerResources::ServerId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mcp_server_resources_server_uri")
                    .table(McpServerResources::Table)
                    .col(McpServerResources::ServerId)
                    .col(McpServerResources::Uri)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // mcp_server_prompts 表索引
        manager
            .create_index(
                Index::create()
                    .name("idx_mcp_server_prompts_server_id")
                    .table(McpServerPrompts::Table)
                    .col(McpServerPrompts::ServerId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mcp_server_prompts_server_name")
                    .table(McpServerPrompts::Table)
                    .col(McpServerPrompts::ServerId)
                    .col(McpServerPrompts::Name)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // permissions 表索引
        manager
            .create_index(
                Index::create()
                    .name("idx_permissions_token_id")
                    .table(Permissions::Table)
                    .col(Permissions::TokenId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_permissions_unique")
                    .table(Permissions::Table)
                    .col(Permissions::TokenId)
                    .col(Permissions::ResourceType)
                    .col(Permissions::ResourcePath)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Tokens {
    Table,
    Id,
    Name,
    Token,
    Description,
    CreatedAt,
    UpdatedAt,
    Enabled,
    LastUsedAt,
    UsageCount,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum McpServers {
    Table,
    Id,
    Name,
    Description,
    Type,
    Command,
    Args,
    Url,
    Headers,
    Env,
    Enabled,
    Version,
    LastVersionCheck,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum McpServerTools {
    Table,
    Id,
    ServerId,
    Name,
    Title,
    Description,
    Enabled,
    InputSchema,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum McpServerResources {
    Table,
    Id,
    ServerId,
    Uri,
    Name,
    Title,
    Description,
    MimeType,
    Enabled,
    IsTemplate,
    UriTemplate,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum McpServerPrompts {
    Table,
    Id,
    ServerId,
    Name,
    Title,
    Description,
    Enabled,
    Arguments,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Permissions {
    Table,
    Id,
    TokenId,
    ResourceType,
    ResourcePath,
    Allowed,
    CreatedAt,
    UpdatedAt,
}