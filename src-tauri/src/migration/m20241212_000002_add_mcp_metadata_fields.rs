use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add output_schema column to mcp_server_tools table
        manager
            .alter_table(
                Table::alter()
                    .table(McpServerTools::Table)
                    .add_column(ColumnDef::new(McpServerTools::OutputSchema).text().null())
                    .to_owned(),
            )
            .await?;

        // Add annotations column to mcp_server_tools table
        manager
            .alter_table(
                Table::alter()
                    .table(McpServerTools::Table)
                    .add_column(ColumnDef::new(McpServerTools::Annotations).text().null())
                    .to_owned(),
            )
            .await?;

        // Add meta column to mcp_server_tools table
        manager
            .alter_table(
                Table::alter()
                    .table(McpServerTools::Table)
                    .add_column(ColumnDef::new(McpServerTools::Meta).text().null())
                    .to_owned(),
            )
            .await?;

        // Add meta column to mcp_server_resources table
        manager
            .alter_table(
                Table::alter()
                    .table(McpServerResources::Table)
                    .add_column(ColumnDef::new(McpServerResources::Meta).text().null())
                    .to_owned(),
            )
            .await?;

        // Add meta column to mcp_server_prompts table
        manager
            .alter_table(
                Table::alter()
                    .table(McpServerPrompts::Table)
                    .add_column(ColumnDef::new(McpServerPrompts::Meta).text().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove columns from mcp_server_tools table (one by one)
        manager
            .alter_table(
                Table::alter()
                    .table(McpServerTools::Table)
                    .drop_column(McpServerTools::OutputSchema)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(McpServerTools::Table)
                    .drop_column(McpServerTools::Annotations)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(McpServerTools::Table)
                    .drop_column(McpServerTools::Meta)
                    .to_owned(),
            )
            .await?;

        // Remove meta column from mcp_server_resources table
        manager
            .alter_table(
                Table::alter()
                    .table(McpServerResources::Table)
                    .drop_column(McpServerResources::Meta)
                    .to_owned(),
            )
            .await?;

        // Remove meta column from mcp_server_prompts table
        manager
            .alter_table(
                Table::alter()
                    .table(McpServerPrompts::Table)
                    .drop_column(McpServerPrompts::Meta)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum McpServerTools {
    Table,
    OutputSchema,
    Annotations,
    Meta,
}

#[derive(DeriveIden)]
enum McpServerResources {
    Table,
    Meta,
}

#[derive(DeriveIden)]
enum McpServerPrompts {
    Table,
    Meta,
}
