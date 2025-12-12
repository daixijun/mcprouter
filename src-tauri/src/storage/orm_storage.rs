//! 简化的 ORM 存储实现
//!
//! 使用 SeaORM 提供基础的数据库操作

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use crate::entities::{prelude::*, *};
use crate::storage::StorageError;
use crate::types::{McpServerConfig, ServiceTransport, Token};
use tracing::info;
use uuid::Uuid;

/// SeaORM 存储实现
#[derive(Debug)]
pub struct Storage {
    /// 数据库连接
    pub db: DatabaseConnection,
}

impl Storage {
    /// 创建新的 ORM 存储实例
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        info!("Initializing SeaORM storage instance, database path: {}", database_url);

        let db = Database::connect(database_url)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to connect to database: {}", e)))?;

        // 应用性能优化设置
        Self::apply_performance_settings(&db).await?;

        Ok(Self { db })
    }

    /// 应用性能优化设置
    async fn apply_performance_settings(db: &DatabaseConnection) -> Result<(), StorageError> {
        let settings = vec![
            "PRAGMA journal_mode = WAL",
            "PRAGMA synchronous = NORMAL",
            "PRAGMA cache_size = -64000",
            "PRAGMA temp_store = memory",
            "PRAGMA mmap_size = 268435456",
            "PRAGMA busy_timeout = 30000",
            "PRAGMA foreign_keys = true",
        ];

        for setting in settings {
            if let Err(e) = db.execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                setting.to_string(),
            )).await {
                tracing::warn!("Failed to apply database setting {}: {}", setting, e);
            }
        }

        Ok(())
    }

    // ============================================================================
    // 基础 CRUD 操作（简化版）
    // ============================================================================


    /// 获取所有 MCP 服务器
    pub async fn list_mcp_servers(&self) -> Result<Vec<mcp_server::Model>, StorageError> {
        let servers = McpServer::find()
            .order_by_asc(McpServerColumn::Name)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query all MCP servers: {}", e)))?;

        Ok(servers)
    }

    /// 根据名称获取 MCP 服务器
    pub async fn get_mcp_server(&self, name: &str) -> Result<Option<mcp_server::Model>, StorageError> {
        let server = McpServer::find()
            .filter(McpServerColumn::Name.eq(name))
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))?;

        Ok(server)
    }

    /// 添加 MCP 服务器
    pub async fn add_mcp_server(&self, config: &McpServerConfig) -> Result<String, StorageError> {
        let server_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now();

        let server_model = mcp_server::ActiveModel {
            id: Set(server_id.clone()),
            name: Set(config.name.clone()),
            description: Set(config.description.clone()),
            server_type: Set(config.transport.to_string()),
            command: Set(config.command.clone()),
            args: Set(config.args.as_ref().map(|args| serde_json::to_string(args).unwrap_or_default())),
            url: Set(config.url.clone()),
            headers: Set(config.headers.as_ref().map(|headers| serde_json::to_string(headers).unwrap_or_default())),
            env: Set(config.env.as_ref().map(|env| serde_json::to_string(env).unwrap_or_default())),
            enabled: Set(config.enabled),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let result = server_model
            .insert(&self.db)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    StorageError::AlreadyExists(format!("MCP 服务器 '{}' already exists", config.name))
                } else {
                    StorageError::Database(format!("Failed to add: {}", e))
                }
            })?;

        info!("Successfully added MCP server: {}", config.name);
        Ok(result.id)
    }

    /// 更新 MCP 服务器
    pub async fn update_mcp_server(&self, name: &str, config: &McpServerConfig) -> Result<(), StorageError> {
        let now = chrono::Utc::now();

        let server = McpServer::find()
            .filter(McpServerColumn::Name.eq(name))
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to find MCP server: {}", e)))?
            .ok_or_else(|| StorageError::NotFound(format!("MCP 服务器 '{}' not found", name)))?;

        let mut active_server: mcp_server::ActiveModel = server.into();
        active_server.name = Set(config.name.clone());
        active_server.description = Set(config.description.clone());
        active_server.server_type = Set(config.transport.to_string());
        active_server.command = Set(config.command.clone());
        active_server.args = Set(config.args.as_ref().map(|args| serde_json::to_string(args).unwrap_or_default()));
        active_server.url = Set(config.url.clone());
        active_server.headers = Set(config.headers.as_ref().map(|headers| serde_json::to_string(headers).unwrap_or_default()));
        active_server.env = Set(config.env.as_ref().map(|env| serde_json::to_string(env).unwrap_or_default()));
        active_server.enabled = Set(config.enabled);
        active_server.updated_at = Set(now.into());

        active_server
            .update(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to update: {}", e)))?;

        info!("Successfully updated MCP server: {}", config.name);
        Ok(())
    }

    /// 删除 MCP 服务器
    pub async fn delete_mcp_server(&self, name: &str) -> Result<(), StorageError> {
        // 开始事务
        let txn = self.db.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        // 获取服务器 ID
        let server = McpServer::find()
            .filter(McpServerColumn::Name.eq(name))
            .one(&txn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to find MCP server: {}", e)))?
            .ok_or_else(|| StorageError::NotFound(format!("MCP 服务器 '{}' not found", name)))?;

        let server_id = server.id;

        // 级联删除相关数据
        McpTool::delete_many()
            .filter(McpToolColumn::ServerId.eq(&server_id))
            .exec(&txn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete: {}", e)))?;

        McpResource::delete_many()
            .filter(McpResourceColumn::ServerId.eq(&server_id))
            .exec(&txn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete: {}", e)))?;

        McpPrompt::delete_many()
            .filter(McpPromptColumn::ServerId.eq(&server_id))
            .exec(&txn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete: {}", e)))?;

        McpServer::delete_by_id(server_id)
            .exec(&txn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete: {}", e)))?;

        txn.commit().await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        info!("Successfully deleted MCP server: {}", name);
        Ok(())
    }

    // ============================================================================
    // 工具、资源、提示词管理
    // ============================================================================

    /// 获取服务器的所有工具
    pub async fn list_server_tools(&self, server_id: &str) -> Result<Vec<mcp_tool::Model>, StorageError> {
        let tools = McpTool::find()
            .filter(McpToolColumn::ServerId.eq(server_id))
            .filter(McpToolColumn::Enabled.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))?;

        Ok(tools)
    }

    /// 获取服务器的所有资源
    pub async fn list_server_resources(&self, server_id: &str) -> Result<Vec<mcp_resource::Model>, StorageError> {
        let resources = McpResource::find()
            .filter(McpResourceColumn::ServerId.eq(server_id))
            .filter(McpResourceColumn::Enabled.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))?;

        Ok(resources)
    }

    /// 获取服务器的所有提示词
    pub async fn list_server_prompts(&self, server_id: &str) -> Result<Vec<mcp_prompt::Model>, StorageError> {
        let prompts = McpPrompt::find()
            .filter(McpPromptColumn::ServerId.eq(server_id))
            .filter(McpPromptColumn::Enabled.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))?;

        Ok(prompts)
    }

    /// 批量更新服务器工具
    pub async fn upsert_server_tools(&self, _server_id: &str, tools: Vec<mcp_tool::ActiveModel>) -> Result<(), StorageError> {
        let txn = self.db.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        for tool in tools {
            tool.insert(&txn)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to insert: {}", e)))?;
        }

        txn.commit().await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// 批量更新服务器资源
    pub async fn upsert_server_resources(&self, _server_id: &str, resources: Vec<mcp_resource::ActiveModel>) -> Result<(), StorageError> {
        let txn = self.db.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        for resource in resources {
            resource.insert(&txn)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to insert: {}", e)))?;
        }

        txn.commit().await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// 批量更新服务器提示词
    pub async fn upsert_server_prompts(&self, _server_id: &str, prompts: Vec<mcp_prompt::ActiveModel>) -> Result<(), StorageError> {
        let txn = self.db.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        for prompt in prompts {
            prompt.insert(&txn)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to insert: {}", e)))?;
        }

        txn.commit().await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// 删除服务器相关的缓存数据
    pub async fn delete_server_cache(&self, server_id: &str) -> Result<(), StorageError> {
        let txn = self.db.begin().await
            .map_err(|e| StorageError::Database(format!("Failed to begin transaction: {}", e)))?;

        // 删除工具
        McpTool::delete_many()
            .filter(McpToolColumn::ServerId.eq(server_id))
            .exec(&txn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete: {}", e)))?;

        // 删除资源
        McpResource::delete_many()
            .filter(McpResourceColumn::ServerId.eq(server_id))
            .exec(&txn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete: {}", e)))?;

        // 删除提示词
        McpPrompt::delete_many()
            .filter(McpPromptColumn::ServerId.eq(server_id))
            .exec(&txn)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete: {}", e)))?;

        txn.commit().await
            .map_err(|e| StorageError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// 获取数据库统计信息
    pub async fn get_database_stats(&self) -> Result<std::collections::HashMap<String, i64>, StorageError> {
        use sea_orm::PaginatorTrait;
        let mut stats = std::collections::HashMap::new();

        // 获取各表的记录数
        let token_count = token::Entity::find().count(&self.db).await
            .map_err(|e| StorageError::Database(format!("Failed to count tokens: {}", e)))? as i64;
        stats.insert("tokens".to_string(), token_count);

        let permission_count = permission::Entity::find().count(&self.db).await
            .map_err(|e| StorageError::Database(format!("Failed to count permissions: {}", e)))? as i64;
        stats.insert("permissions".to_string(), permission_count);

        let mcp_server_count = mcp_server::Entity::find().count(&self.db).await
            .map_err(|e| StorageError::Database(format!("Failed to count mcp_servers: {}", e)))? as i64;
        stats.insert("mcp_servers".to_string(), mcp_server_count);

        let mcp_tool_count = mcp_tool::Entity::find().count(&self.db).await
            .map_err(|e| StorageError::Database(format!("Failed to count mcp_server_tools: {}", e)))? as i64;
        stats.insert("mcp_server_tools".to_string(), mcp_tool_count);

        let mcp_resource_count = mcp_resource::Entity::find().count(&self.db).await
            .map_err(|e| StorageError::Database(format!("Failed to count mcp_server_resources: {}", e)))? as i64;
        stats.insert("mcp_server_resources".to_string(), mcp_resource_count);

        let mcp_prompt_count = mcp_prompt::Entity::find().count(&self.db).await
            .map_err(|e| StorageError::Database(format!("Failed to count mcp_server_prompts: {}", e)))? as i64;
        stats.insert("mcp_server_prompts".to_string(), mcp_prompt_count);

        Ok(stats)
    }

    // ============================================================================
    // MCP Server Manager 需要的方法
    // ============================================================================

    /// 获取所有启用的 MCP 服务器（用于 MCP Server Manager）
    pub async fn get_enabled_servers(&self) -> Result<Vec<mcp_server::Model>, StorageError> {
        McpServer::find()
            .filter(mcp_server::Column::Enabled.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))
    }

    /// 获取服务器配置（用于 MCP Server Manager）
    pub async fn get_server_config(&self, server_name: &str) -> Result<Option<McpServerConfig>, StorageError> {
        if let Some(server) = self.get_mcp_server(server_name).await? {
            // 转换为配置
            let args = server.args
                .and_then(|args| serde_json::from_str(&args).ok())
                .unwrap_or_default();
            let headers = server.headers
                .and_then(|headers| serde_json::from_str(&headers).ok())
                .unwrap_or_default();
            let env = server.env
                .and_then(|env| serde_json::from_str(&env).ok())
                .unwrap_or_default();
            let transport = server.server_type.parse::<ServiceTransport>()
                .map_err(|e| StorageError::InvalidData(format!("Invalid transport type: {}", e)))?;

            let config = McpServerConfig {
                name: server.name,
                description: server.description,
                command: server.command,
                args: Some(args),
                url: server.url,
                headers: Some(headers),
                env: Some(env),
                transport,
                enabled: server.enabled,
            };
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// 启用/禁用服务器
    pub async fn set_server_enabled(&self, server_name: &str, enabled: bool) -> Result<(), StorageError> {
        use sea_orm::Set;

        // 首先找到要更新的服务器
        let server = McpServer::find()
            .filter(mcp_server::Column::Name.eq(server_name))
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))?;

        if let Some(server) = server {
            let mut active_model: mcp_server::ActiveModel = server.into();
            active_model.enabled = Set(enabled);
            active_model.update(&self.db)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to update: {}", e)))?;
        }

        Ok(())
    }

    // ============================================================================
    // Token Manager 需要的方法
    // ============================================================================

    /// 创建新的 Token
    pub async fn create_token(&self, token: &Token) -> Result<String, StorageError> {
        use sea_orm::Set;
        let now = chrono::Utc::now();

        let token_model = token::ActiveModel {
            id: Set(token.id.clone()),
            name: Set(token.name.clone()),
            token: Set(token.value.clone()),
            description: Set(token.description.clone()),
            enabled: Set(token.enabled),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            last_used_at: Set(token.last_used_at.map(|ts| chrono::DateTime::from_timestamp(ts as i64, 0).unwrap_or(now).into())),
            usage_count: Set(token.usage_count as i32),
            expires_at: Set(token.expires_at.map(|ts| chrono::DateTime::from_timestamp(ts as i64, 0).unwrap_or(now).into())),
            ..Default::default()
        };

        let result = token_model.insert(&self.db)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    StorageError::AlreadyExists(format!("Token '{}' already exists", token.name))
                } else {
                    StorageError::Database(format!("Failed to create: {}", e))
                }
            })?;

        Ok(result.id)
    }

    /// 根据 ID 获取 Token
    pub async fn get_token_by_id(&self, token_id: &str) -> Result<Option<Token>, StorageError> {
        if let Some(token_entity) = token::Entity::find_by_id(token_id.to_string()).one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))? {

            let token = self.convert_entity_to_token(token_entity)?;
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    /// 根据值获取 Token
    pub async fn get_token_by_value(&self, token_value: &str) -> Result<Option<Token>, StorageError> {
        if let Some(token_entity) = token::Entity::find()
            .filter(token::Column::Token.eq(token_value))
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))? {

            let token = self.convert_entity_to_token(token_entity)?;
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    /// 获取所有 Token
    pub async fn get_all_tokens(&self) -> Result<Vec<Token>, StorageError> {
        let tokens = token::Entity::find().all(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))?;

        let result = tokens.into_iter()
            .map(|token_entity| self.convert_entity_to_token(token_entity))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    /// 更新 Token 使用信息
    pub async fn update_token_usage(&self, token_id: &str) -> Result<(), StorageError> {
        use sea_orm::Set;
        let now = chrono::Utc::now();

        if let Some(token_entity) = token::Entity::find_by_id(token_id.to_string()).one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))? {

            let mut active_model: token::ActiveModel = token_entity.into();
            active_model.last_used_at = Set(Some(now.into()));
            active_model.usage_count = Set(active_model.usage_count.unwrap() + 1);
            active_model.updated_at = Set(now.into());

            active_model.update(&self.db)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to update: {}", e)))?;
        }

        Ok(())
    }

    /// 删除 Token
    pub async fn delete_token(&self, token_id: &str) -> Result<(), StorageError> {
        token::Entity::delete_by_id(token_id.to_string()).exec(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to delete: {}", e)))?;

        Ok(())
    }

    /// 启用/禁用 Token
    pub async fn set_token_enabled(&self, token_id: &str, enabled: bool) -> Result<(), StorageError> {
        use sea_orm::Set;

        if let Some(token_entity) = token::Entity::find_by_id(token_id.to_string()).one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))? {

            let mut active_model: token::ActiveModel = token_entity.into();
            active_model.enabled = Set(enabled);
            active_model.updated_at = Set(chrono::Utc::now().into());

            active_model.update(&self.db)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to update: {}", e)))?;
        }

        Ok(())
    }

    // ============================================================================
    // 权限管理方法
    // ============================================================================

    /// 获取Token的所有权限
    pub async fn get_token_permissions(&self, token_id: &str) -> Result<Vec<permission::Model>, StorageError> {
        use sea_orm::ColumnTrait;

        permission::Entity::find()
            .filter(permission::Column::TokenId.eq(token_id))
            .order_by_asc(permission::Column::ResourceType)
            .order_by_asc(permission::Column::ResourcePath)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to query: {}", e)))
    }

    /// 添加权限
    pub async fn add_permission(&self, token_id: &str, resource_type: &str, resource_path: &str) -> Result<(), StorageError> {
        use sea_orm::{Set, ColumnTrait, EntityTrait, QueryFilter};
        use uuid::Uuid;

        // 首先检查权限是否already exists
        let existing_permission = permission::Entity::find()
            .filter(permission::Column::TokenId.eq(token_id))
            .filter(permission::Column::ResourceType.eq(resource_type))
            .filter(permission::Column::ResourcePath.eq(resource_path))
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to check: {}", e)))?;

        if let Some(permission) = existing_permission {
            // 如果permission already exists，更新为允许状态
            let mut active_permission: permission::ActiveModel = permission.into();
            active_permission.allowed = Set(true);
            active_permission.updated_at = Set(chrono::Utc::now().into());

            active_permission
                .update(&self.db)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to update: {}", e)))?;

            tracing::info!("Permission already exists, updated to allowed: token={}, type={}, path={}", token_id, resource_type, resource_path);
        } else {
            // 如果权限not found，创建新权限
            let permission_model = permission::ActiveModel {
                id: Set(Uuid::now_v7().to_string()),
                token_id: Set(token_id.to_string()),
                resource_type: Set(resource_type.to_string()),
                resource_path: Set(resource_path.to_string()),
                allowed: Set(true),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(chrono::Utc::now().into()),
            };

            permission_model
                .insert(&self.db)
                .await
                .map_err(|e| {
                    let error_msg = e.to_string();
                    if error_msg.contains("UNIQUE constraint failed") {
                        StorageError::Database(format!("permission already exists: token={}, type={}, path={}", token_id, resource_type, resource_path))
                    } else {
                        StorageError::Database(format!("Failed to add: {}", error_msg))
                    }
                })?;

            tracing::info!("Successfully created new permission: token={}, type={}, path={}", token_id, resource_type, resource_path);
        }

        Ok(())
    }

    /// 移除权限
    pub async fn remove_permission(&self, token_id: &str, resource_type: &str, resource_path: &str) -> Result<(), StorageError> {
        use sea_orm::{ColumnTrait, Set, EntityTrait, QueryFilter, ActiveModelTrait};

        // 首先检查权限是否存在
        let existing_permission = permission::Entity::find()
            .filter(permission::Column::TokenId.eq(token_id))
            .filter(permission::Column::ResourceType.eq(resource_type))
            .filter(permission::Column::ResourcePath.eq(resource_path))
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to check: {}", e)))?;

        if let Some(permission) = existing_permission {
            // 如果权限存在，设置为不允许状态（软删除）
            let mut active_permission: permission::ActiveModel = permission.into();
            active_permission.allowed = Set(false);
            active_permission.updated_at = Set(chrono::Utc::now().into());

            active_permission
                .update(&self.db)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to update: {}", e)))?;

            tracing::info!("Permission set to denied state: token={}, type={}, path={}", token_id, resource_type, resource_path);
        } else {
            // 如果权限not found，可以选择创建一个不允许的权限记录，或者直接返回
            // 这里我们创建一个不允许的权限记录，以明确表示权限已被移除
            let permission_model = permission::ActiveModel {
                id: Set(uuid::Uuid::now_v7().to_string()),
                token_id: Set(token_id.to_string()),
                resource_type: Set(resource_type.to_string()),
                resource_path: Set(resource_path.to_string()),
                allowed: Set(false),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(chrono::Utc::now().into()),
            };

            permission_model
                .insert(&self.db)
                .await
                .map_err(|e| {
                    let error_msg = e.to_string();
                    if error_msg.contains("UNIQUE constraint failed") {
                        // 如果因为唯一约束失败，说明permission already exists，直接返回成功
                        tracing::info!("Permission removal completed (permission already exists): token={}, type={}, path={}", token_id, resource_type, resource_path);
                        StorageError::Database("Permission removal completed".to_string())
                    } else {
                        StorageError::Database(format!("Failed to create permission record: {}", error_msg))
                    }
                })?;

            tracing::info!("Created denied permission record: token={}, type={}, path={}", token_id, resource_type, resource_path);
        }

        Ok(())
    }

    /// 检查权限
    pub async fn check_permission(&self, token_id: &str, resource_type: &str, resource_path: &str) -> Result<bool, StorageError> {
        use sea_orm::ColumnTrait;

        let permission = permission::Entity::find()
            .filter(permission::Column::TokenId.eq(token_id))
            .filter(permission::Column::ResourceType.eq(resource_type))
            .filter(permission::Column::ResourcePath.eq(resource_path))
            .filter(permission::Column::Allowed.eq(true))
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to check: {}", e)))?;

        Ok(permission.is_some())
    }

    // ============================================================================
    // 兼容性别名方法（为 McpServerManager 和 TokenManager 提供统一接口）
    // ============================================================================

    /// 根据 ID 获取 MCP 服务器（别名方法）
    pub async fn get_mcp_server_by_name(&self, name: &str) -> Result<Option<mcp_server::Model>, StorageError> {
        self.get_mcp_server(name).await
    }

    /// 切换 MCP 服务器启用状态
    pub async fn toggle_mcp_server_enabled(&self, name: &str) -> Result<bool, StorageError> {
        if let Some(server) = self.get_mcp_server(name).await? {
            let new_enabled = !server.enabled;
            self.set_server_enabled(name, new_enabled).await?;
            Ok(new_enabled)
        } else {
            Err(StorageError::NotFound("MCP server not found".to_string()))
        }
    }

    // ============================================================================
    // 辅助方法
    // ============================================================================

    /// 将 token 实体转换为类型 Token
    fn convert_entity_to_token(&self, entity: token::Model) -> Result<Token, StorageError> {
        let _now = chrono::Utc::now().timestamp() as u64;
        let expires_at = entity.expires_at.map(|dt| dt.timestamp() as u64);
        let last_used_at = entity.last_used_at.map(|dt| dt.timestamp() as u64);

        // TODO: Load permissions in a separate async method
        // For now, return empty permission lists
        Ok(Token {
            id: entity.id,
            name: entity.name,
            value: entity.token,
            description: entity.description,
            created_at: entity.created_at.timestamp() as u64,
            enabled: entity.enabled,
            last_used_at,
            usage_count: entity.usage_count as u64,
            expires_at,
            allowed_tools: Some(vec![]),
            allowed_resources: Some(vec![]),
            allowed_prompts: Some(vec![]),
            allowed_prompt_templates: Some(vec![]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm_migration::MigratorTrait;

    async fn create_test_storage() -> Storage {
        // 使用临时文件数据库进行测试
        let _temp_file = std::fs::File::create("/tmp/test_mcprouter.db").unwrap();
        let db_url = "sqlite:/tmp/test_mcprouter.db";

        // 运行迁移
        let db = sea_orm::Database::connect(db_url).await.unwrap();
        crate::migration::Migrator::up(&db, None).await.unwrap();
        drop(db);

        Storage::new(db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_database_stats() {
        let storage = create_test_storage().await;

        // 测试数据库统计功能
        let stats = storage.get_database_stats().await.unwrap();

        // 验证所有表都存在且初始为空
        assert_eq!(stats.get("tokens"), Some(&0));
        assert_eq!(stats.get("permissions"), Some(&0));
        assert_eq!(stats.get("mcp_servers"), Some(&0));
        assert_eq!(stats.get("mcp_server_tools"), Some(&0));
        assert_eq!(stats.get("mcp_server_resources"), Some(&0));
        assert_eq!(stats.get("mcp_server_prompts"), Some(&0));
    }

    #[tokio::test]
    async fn test_crud_operations() {
        let storage = create_test_storage().await;

        // 测试 MCP 服务器的 CRUD 操作
        let config = crate::types::McpServerConfig {
            name: "test-server".to_string(),
            description: Some("Test Server".to_string()),
            transport: crate::types::ServiceTransport::Stdio,
            command: Some("echo".to_string()),
            args: Some(vec!["hello".to_string()]),
            url: None,
            headers: None,
            env: None,
            enabled: true,
        };

        // 创建
        let server_id = storage.add_mcp_server(&config).await.unwrap();
        assert!(!server_id.is_empty());

        // 读取
        let servers = storage.list_mcp_servers().await.unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "test-server");

        let server = storage.get_mcp_server("test-server").await.unwrap();
        assert!(server.is_some());
        assert_eq!(server.unwrap().description, Some("Test Server".to_string()));

        // 更新
        let mut updated_config = config.clone();
        updated_config.description = Some("Updated Server".to_string());
        storage.update_mcp_server("test-server", &updated_config).await.unwrap();

        let updated_server = storage.get_mcp_server("test-server").await.unwrap();
        assert_eq!(updated_server.unwrap().description, Some("Updated Server".to_string()));

        // 删除
        storage.delete_mcp_server("test-server").await.unwrap();
        let servers = storage.list_mcp_servers().await.unwrap();
        assert_eq!(servers.len(), 0);
    }
}