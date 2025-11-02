//! MCP服务器配置管理
//!
//! 管理MCP服务器的存储、检索、更新和删除

use super::file_manager::{exists, read_dir, read_json, remove_file, write_json_atomic};
use super::{get_mcp_server_config_path, ConfigError, Result};
use crate::types::{McpServerConfig, ServiceTransport};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;

/// MCP工具存储结构
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct McpToolConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// MCP服务器存储结构（扩展版本）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct McpServerConfigFile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,

    // Stdio-only fields (skip for SSE/HTTP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<HashMap<String, String>>,

    pub transport: ServiceTransport,

    // SSE/HTTP-only fields (skip for stdio)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,

    pub enabled: bool,
    pub version: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tools: Vec<McpToolConfig>,
}

impl McpServerConfigFile {
    /// 清理字段，确保只有与传输类型相关的字段被设置
    pub fn clean_fields(&mut self) {
        match self.transport {
            ServiceTransport::Stdio => {
                // stdio 类型：清理 sse/http 相关字段
                self.url = None;
                self.headers = None;
            }
            ServiceTransport::Sse | ServiceTransport::Http => {
                // sse/http 类型：清理 stdio 相关字段
                self.command = None;
                self.args = None;
                self.env_vars = None;
            }
        }
    }
}

/// MCP服务器仓库
#[derive(Debug)]
pub struct McpServerRepository {
    app_data_dir: PathBuf,
    servers: Vec<McpServerConfigFile>,
}

impl McpServerRepository {
    /// 创建新的MCP服务器仓库
    pub async fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
        tracing::info!("正在创建 McpServerRepository...");
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| {
                tracing::error!("获取应用数据目录失败: {:?}", e);
                ConfigError::Invalid("无法获取应用数据目录".to_string())
            })?;

        tracing::info!("✅ 获取到应用数据目录: {:?}", app_data_dir);

        // 验证目录是否存在或可创建
        if !app_data_dir.exists() {
            tracing::warn!("⚠️ 应用数据目录不存在，尝试创建: {:?}", app_data_dir);
            if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                tracing::error!("❌ 创建应用数据目录失败: {}", e);
            } else {
                tracing::info!("✅ 应用数据目录创建成功");
            }
        }

        let servers = Self::load_all_servers(&app_data_dir)?;
        tracing::info!("✅ 加载到 {} 个服务器配置", servers.len());

        Ok(Self {
            app_data_dir,
            servers,
        })
    }

    /// 加载所有服务器配置
    fn load_all_servers(app_data_dir: &PathBuf) -> Result<Vec<McpServerConfigFile>> {
        let servers_dir = app_data_dir.join("config").join("mcp_servers");

        if !exists(&servers_dir) {
            return Ok(Vec::new());
        }

        let mut servers = Vec::new();
        let config_files = read_dir(&servers_dir)?;

        for config_file in config_files {
            if let Ok(server) = read_json::<_, McpServerConfigFile>(&config_file) {
                servers.push(server);
            }
        }

        Ok(servers)
    }

    /// 获取所有服务器
    pub fn get_all(&self) -> &[McpServerConfigFile] {
        &self.servers
    }

    /// 根据名称获取服务器
    pub fn get_by_name(&self, name: &str) -> Option<&McpServerConfigFile> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// 根据ID获取服务器
    pub fn get_by_id(&self, id: &str) -> Option<&McpServerConfigFile> {
        self.servers.iter().find(|s| s.id == id)
    }

    /// 获取启用的服务器
    pub fn get_enabled(&self) -> Vec<&McpServerConfigFile> {
        self.servers.iter().filter(|s| s.enabled).collect()
    }

    /// 根据ID获取可修改的服务器
    fn get_by_id_mut(&mut self, id: &str) -> Option<&mut McpServerConfigFile> {
        self.servers.iter_mut().find(|s| s.id == id)
    }

    /// 添加新服务器
    pub async fn add(&mut self, config: McpServerConfig) -> Result<String> {
        tracing::info!("开始添加 MCP 服务器: {}", config.name);
        tracing::info!("App data dir: {:?}", self.app_data_dir);

        // 检查名称是否已存在
        if self.get_by_name(&config.name).is_some() {
            tracing::warn!("服务器名称已存在: {}", config.name);
            return Err(ConfigError::Invalid(format!(
                "服务器名称已存在: {}",
                config.name
            )));
        }

        let now = chrono::Utc::now();
        let mut server_file = McpServerConfigFile {
            id: uuid::Uuid::new_v4().to_string(),
            name: config.name.clone(),
            description: config.description,
            command: config.command,
            args: config.args,
            transport: config.transport,
            url: config.url,
            enabled: config.enabled,
            env_vars: config.env_vars,
            headers: config.headers,
            version: config.version,
            created_at: now,
            updated_at: now,
            tools: Vec::new(),
        };

        // 清理字段：根据传输类型移除不相关的字段
        server_file.clean_fields();
        tracing::info!("✅ 字段清理完成，传输类型: {:?}", server_file.transport);

        let server_path = get_mcp_server_config_path(&self.app_data_dir, &config.name);
        tracing::info!("配置文件路径: {:?}", server_path);

        // 保存到文件
        tracing::info!("开始写入配置文件...");
        write_json_atomic(&server_path, &server_file)?;
        tracing::info!("配置文件写入成功");

        self.servers.push(server_file);

        Ok(format!("MCP服务器 '{}' 添加成功", config.name))
    }

    /// 更新服务器
    pub async fn update(&mut self, name: &str, mut config: McpServerConfig) -> Result<String> {
        // 先克隆需要的字段，避免借用冲突
        let server_name = config.name.clone();

        let server_path = get_mcp_server_config_path(&self.app_data_dir, &server_name);

        // 修改数据结构
        {
            let server = self
                .get_by_name_mut(&server_name)
                .ok_or_else(|| ConfigError::Invalid(format!("服务器未找到: {}", name)))?;

            server.name = config.name.clone();
            server.description = config.description;
            server.command = config.command;
            server.args = config.args;
            server.transport = config.transport.clone();
            server.url = config.url;
            server.enabled = config.enabled;
            server.env_vars = config.env_vars;
            server.headers = config.headers;
            server.version = config.version.clone();
            server.updated_at = chrono::Utc::now();

            // 清理字段（根据传输类型）
            server.clean_fields();
        }

        // 保存到文件
        let server = self
            .get_by_name(&server_name)
            .ok_or_else(|| ConfigError::Invalid(format!("服务器未找到: {}", name)))?;

        write_json_atomic(&server_path, server)?;

        Ok(format!("MCP服务器 '{}' 更新成功", server_name))
    }

    /// 删除服务器
    pub async fn delete(&mut self, name: &str) -> Result<String> {
        // 查找服务器
        let server_index = self
            .servers
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| ConfigError::Invalid(format!("服务器未找到: {}", name)))?;

        // 删除配置文件
        let server_path = get_mcp_server_config_path(&self.app_data_dir, name);
        remove_file(&server_path)?;

        // 从内存中移除
        self.servers.remove(server_index);

        Ok(format!("MCP服务器 '{}' 已删除", name))
    }

    /// 切换服务器启用状态
    pub async fn toggle_enabled(&mut self, name: &str) -> Result<bool> {
        // 首先查找服务器
        let server = self
            .get_by_name(name)
            .ok_or_else(|| ConfigError::Invalid(format!("服务器未找到: {}", name)))?;

        let new_state = !server.enabled;

        // 获取可修改的引用并更新
        let server_mut = self.servers.iter_mut().find(|s| s.name == name).unwrap();
        server_mut.enabled = new_state;
        server_mut.updated_at = chrono::Utc::now();

        // 清理字段（根据传输类型）
        server_mut.clean_fields();

        // 保存更改
        let server_path = get_mcp_server_config_path(&self.app_data_dir, name);
        write_json_atomic(&server_path, server_mut)?;

        tracing::info!("✅ 服务器 '{}' 启用状态已更新为: {}", name, new_state);
        Ok(new_state)
    }

    /// 添加工具到服务器
    pub async fn add_tool(&mut self, server_name: &str, tool: McpToolConfig) -> Result<String> {
        let server_path = get_mcp_server_config_path(&self.app_data_dir, server_name);

        // 修改数据结构
        let server = {
            let server = self
                .get_by_name_mut(server_name)
                .ok_or_else(|| ConfigError::Invalid(format!("服务器未找到: {}", server_name)))?;

            // 检查工具是否已存在
            if server.tools.iter().any(|t| t.id == tool.id) {
                return Err(ConfigError::Invalid(format!("工具已存在: {}", tool.id)));
            }

            server.tools.push(tool.clone());
            server.updated_at = chrono::Utc::now();
            server
        };

        // 保存更改
        write_json_atomic(&server_path, server)?;

        Ok(format!("工具已添加到服务器 '{}'", server_name))
    }

    /// 启用/禁用工具
    pub async fn toggle_tool(&mut self, server_name: &str, tool_id: &str) -> Result<bool> {
        let server_path = get_mcp_server_config_path(&self.app_data_dir, server_name);

        // 修改数据结构
        let enabled_state = {
            let server = self
                .get_by_name_mut(server_name)
                .ok_or_else(|| ConfigError::Invalid(format!("服务器未找到: {}", server_name)))?;

            if let Some(tool) = server.tools.iter_mut().find(|t| t.id == tool_id) {
                tool.enabled = !tool.enabled;
                tool.updated_at = chrono::Utc::now();
                server.updated_at = chrono::Utc::now();
                tool.enabled
            } else {
                return Err(ConfigError::Invalid(format!("工具未找到: {}", tool_id)));
            }
        };

        // 保存更改
        let server = self
            .get_by_name(server_name)
            .ok_or_else(|| ConfigError::Invalid(format!("服务器未找到: {}", server_name)))?;

        write_json_atomic(&server_path, server)?;

        Ok(enabled_state)
    }

    /// 获取启用的工具数量
    pub fn get_enabled_tool_count(&self, server_name: &str) -> usize {
        self.get_by_name(server_name)
            .map(|s| s.tools.iter().filter(|t| t.enabled).count())
            .unwrap_or(0)
    }

    /// 根据名称获取可修改的服务器
    fn get_by_name_mut(&mut self, name: &str) -> Option<&mut McpServerConfigFile> {
        self.servers.iter_mut().find(|s| s.name == name)
    }
}
