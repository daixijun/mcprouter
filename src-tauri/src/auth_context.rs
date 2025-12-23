// Simple session info structure (in-memory only)
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub token_id: Option<String>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub last_used_at: Option<u64>,
}

impl SessionInfo {
    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|e| {
                    tracing::warn!("SystemTime calculation failed: {}", e);
                    std::time::Duration::ZERO
                })
                .as_secs();
            expires_at < now
        } else {
            false
        }
    }
}
use http::request::Parts as HttpRequestParts;
use rmcp::{service::RequestContext, RoleServer};
use std::sync::Arc;
use std::time::Instant;

/// Session ID extension type for RequestContext
#[derive(Debug, Clone)]
pub struct SessionIdExtension(pub String);

/// Session info extension type for RequestContext
#[derive(Debug, Clone)]
pub struct SessionInfoExtension(pub Arc<SessionInfo>);

/// AuthContext - 包装RequestContext以提供权限信息
///
/// 由于MCP协议层的RequestContext无法访问HTTP头信息，
/// 我们通过session机制将权限信息传递给MCP层
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// 原始的RequestContext
    pub original_context: RequestContext<RoleServer>,
    /// Session信息（如果已认证）
    pub session_info: Option<Arc<SessionInfo>>,
    /// 请求时间
    pub request_time: Instant,
}

impl AuthContext {
    /// 从RequestContext创建AuthContext
    ///
    /// 尝试从session中获取权限信息，如果session_id有效且未过期
    pub fn from_request_context(context: RequestContext<RoleServer>) -> Self {
        let request_time = Instant::now();
        tracing::debug!("=== AuthContext Debug ===");
        tracing::debug!("Creating AuthContext from RequestContext");

        let session_info = Self::extract_session_from_context(&context);

        if let Some(ref session) = session_info {
            tracing::info!("AuthContext created successfully - Session ID: {}, Token ID: {}",
                session.id,
                session.token_id.as_ref().unwrap_or(&"None".to_string()));
        } else {
            tracing::warn!("AuthContext created without session information - this will likely cause authentication failures");
        }

        Self {
            original_context: context,
            session_info,
            request_time,
        }
    }

    /// 从RequestContext中提取session信息
    ///
    /// 直接从RequestContext extensions获取SessionInfo，
    /// 现在我们不再使用SessionManager，而是直接存储Token信息
    fn extract_session_from_context(
        context: &RequestContext<RoleServer>,
    ) -> Option<Arc<SessionInfo>> {
        // 首先尝试从extensions中获取完整的SessionInfo
        if let Some(session_info_ext) = context.extensions.get::<SessionInfoExtension>() {
            tracing::debug!("Found SessionInfoExtension in RequestContext extensions");
            return Some(session_info_ext.0.clone());
        }

        // 当MCP请求通过Streamable HTTP服务时，原始HTTP的Parts会被注入到extensions中
        // 我们需要从parts.extensions里再尝试一次取出session信息
        if let Some(http_parts) = context.extensions.get::<HttpRequestParts>() {
            if let Some(session_info_ext) = http_parts.extensions.get::<SessionInfoExtension>() {
                tracing::debug!("Found SessionInfoExtension inside HTTP request parts extensions");
                return Some(session_info_ext.0.clone());
            }
        }

        tracing::debug!("No session information found in RequestContext extensions");
        None
    }

    /// 检查是否有有效的session
    pub fn has_valid_session(&self) -> bool {
        self.session_info.is_some()
    }

    /// 检查session是否已过期
    pub fn is_session_expired(&self) -> bool {
        if let Some(session) = &self.session_info {
            session.is_expired()
        } else {
            true // 没有session视为过期
        }
    }

    /// 获取session_id
    pub fn session_id(&self) -> Option<&str> {
        self.session_info.as_ref().map(|s| s.id.as_str())
    }

    /// 获取token信息（移除token字段访问，仅保留token_id）
    pub fn token_id(&self) -> Option<&str> {
        self.session_info.as_ref().and_then(|s| s.token_id.as_deref())
    }

    /// 检查工具权限
    pub fn has_tool_permission(&self, _tool_name: &str) -> bool {
        // 简化权限检查 - 如果有有效的session就允许所有操作
        // 真正的权限检查在 aggregator 中的 list_tools_with_auth 方法中进行
        self.session_info.is_some()
    }

    /// 检查资源权限
    pub fn has_resource_permission(&self, _resource_uri: &str) -> bool {
        // 简化权限检查 - 如果有有效的session就允许所有操作
        // 真正的权限检查在 aggregator 中的 list_resources_with_auth 方法中进行
        self.session_info.is_some()
    }

    /// 检查提示词权限
    pub fn has_prompt_permission(&self, _prompt_name: &str) -> bool {
        // 简化权限检查 - 如果有有效的session就允许所有操作
        // 真正的权限检查在 aggregator 中的 list_prompts_with_auth 方法中进行
        self.session_info.is_some()
    }

    /// 检查提示词模板权限
    pub fn has_prompt_template_permission(&self, _template_name: &str) -> bool {
        // 简化权限检查 - 如果有有效的session就允许所有操作
        self.session_info.is_some()
    }

    /// 更新session的最后访问时间
    pub fn update_session_access(&self) {
        if let Some(session) = &self.session_info {
            // 这里需要SessionInfo支持更新访问时间
            // 但由于SessionInfo被Arc包装，我们需要可变引用
            // 可以考虑在SessionManager中添加update_session_access方法
            tracing::debug!("Updating access time for session {}", session.id);
        }
    }
}

/// 权限验证结果的枚举
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionResult {
    /// 允许访问
    Allowed,
    /// 拒绝访问 - 未认证
    NotAuthenticated,
    /// 拒绝访问 - 权限不足
    InsufficientPermissions,
    /// 拒绝访问 - Session过期
    SessionExpired,
}

impl AuthContext {
    /// 验证工具权限并返回详细结果
    pub fn check_tool_permission_with_result(&self, _tool_name: &str) -> PermissionResult {
        if !self.has_valid_session() {
            return PermissionResult::NotAuthenticated;
        }

        if self.is_session_expired() {
            return PermissionResult::SessionExpired;
        }

        PermissionResult::Allowed // 简化实现，真正检查在aggregator中进行
    }

    /// 验证资源权限并返回详细结果
    pub fn check_resource_permission_with_result(&self, _resource_uri: &str) -> PermissionResult {
        if !self.has_valid_session() {
            return PermissionResult::NotAuthenticated;
        }

        if self.is_session_expired() {
            return PermissionResult::SessionExpired;
        }

        PermissionResult::Allowed // 简化实现，真正检查在aggregator中进行
    }

    /// 验证提示词权限并返回详细结果
    pub fn check_prompt_permission_with_result(&self, _prompt_name: &str) -> PermissionResult {
        if !self.has_valid_session() {
            return PermissionResult::NotAuthenticated;
        }

        if self.is_session_expired() {
            return PermissionResult::SessionExpired;
        }

        PermissionResult::Allowed // 简化实现，真正检查在aggregator中进行
    }
}
