//! SeaORM 实体模块
//!
//! 此模块包含所有数据库表的 SeaORM 实体定义，对应现有的 SQLite 表结构

pub mod token;
pub mod permission;
pub mod mcp_server;
pub mod mcp_tool;
pub mod mcp_resource;
pub mod mcp_prompt;

/// Prelude 模块，重新导出常用的 SeaORM 实体和类型
pub mod prelude {
    pub use super::{
        token::Entity as Token,
        permission::Entity as Permission,
        mcp_server::Entity as McpServer,
        mcp_tool::Entity as McpTool,
        mcp_resource::Entity as McpResource,
        mcp_prompt::Entity as McpPrompt,

        token::Column as TokenColumn,
        permission::Column as PermissionColumn,
        mcp_server::Column as McpServerColumn,
        mcp_tool::Column as McpToolColumn,
        mcp_resource::Column as McpResourceColumn,
        mcp_prompt::Column as McpPromptColumn,
    };
}

// 重新导出所有实体
pub use token::Entity as Token;
pub use permission::Entity as Permission;
pub use mcp_server::Entity as McpServer;
pub use mcp_tool::Entity as McpTool;
pub use mcp_resource::Entity as McpResource;
pub use mcp_prompt::Entity as McpPrompt;

pub use token::Model as TokenModel;
pub use permission::Model as PermissionModel;
pub use mcp_server::Model as McpServerModel;
pub use mcp_tool::Model as McpToolModel;
pub use mcp_resource::Model as McpResourceModel;
pub use mcp_prompt::Model as McpPromptModel;

pub use token::ActiveModel as TokenActiveModel;
pub use permission::ActiveModel as PermissionActiveModel;
pub use mcp_server::ActiveModel as McpServerActiveModel;
pub use mcp_tool::ActiveModel as McpToolActiveModel;
pub use mcp_resource::ActiveModel as McpResourceActiveModel;
pub use mcp_prompt::ActiveModel as McpPromptActiveModel;

pub use token::Column as TokenColumn;
pub use permission::Column as PermissionColumn;
pub use mcp_server::Column as McpServerColumn;
pub use mcp_tool::Column as McpToolColumn;
pub use mcp_resource::Column as McpResourceColumn;
pub use mcp_prompt::Column as McpPromptColumn;