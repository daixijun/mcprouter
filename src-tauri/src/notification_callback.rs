//! 清单变化通知回调 Trait
//!
//! 定义了当 MCP 服务器的 tools/resources/prompts 清单发生变化时的回调接口

use async_trait::async_trait;

/// 清单变化通知回调 Trait
///
/// 当后端 MCP 服务器的工具/资源/提示清单发生变化时,
/// McpServerManager 会通过此 trait 通知注册的监听者
#[async_trait]
pub trait ManifestChangeCallback: Send + Sync {
    /// 当工具列表发生变化时调用
    ///
    /// # Arguments
    /// * `server_name` - 发生变化的服务器名称
    async fn tools_list_changed(&self, server_name: &str);

    /// 当资源列表发生变化时调用
    ///
    /// # Arguments
    /// * `server_name` - 发生变化的服务器名称
    async fn resources_list_changed(&self, server_name: &str);

    /// 当提示词列表发生变化时调用
    ///
    /// # Arguments
    /// * `server_name` - 发生变化的服务器名称
    async fn prompts_list_changed(&self, server_name: &str);
}

/// 用于测试的空实现
///
/// 不会执行任何操作的回调实现,可用于测试或作为占位符
pub struct NullCallback;

#[async_trait]
impl ManifestChangeCallback for NullCallback {
    async fn tools_list_changed(&self, _server_name: &str) {
        tracing::debug!("NullCallback: tools_list_changed called (no-op)");
    }

    async fn resources_list_changed(&self, _server_name: &str) {
        tracing::debug!("NullCallback: resources_list_changed called (no-op)");
    }

    async fn prompts_list_changed(&self, _server_name: &str) {
        tracing::debug!("NullCallback: prompts_list_changed called (no-op)");
    }
}
