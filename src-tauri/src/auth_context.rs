use crate::session_manager::{get_session_manager, SessionInfo};
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
#[allow(dead_code)]
pub struct AuthContext {
    /// 原始的RequestContext
    pub original_context: RequestContext<RoleServer>,
    /// Session信息（如果已认证）
    pub session_info: Option<Arc<SessionInfo>>,
    /// 请求时间
    pub request_time: Instant,
}

#[allow(dead_code)]
impl AuthContext {
    /// 从RequestContext创建AuthContext
    ///
    /// 尝试从session中获取权限信息，如果session_id有效且未过期
    pub fn from_request_context(context: RequestContext<RoleServer>) -> Self {
        let request_time = Instant::now();
        let session_info = Self::extract_session_from_context(&context);

        Self {
            original_context: context,
            session_info,
            request_time,
        }
    }

    /// 从RequestContext中提取session信息
    ///
    /// 通过RequestContext extensions获取session_id，
    /// 然后从SessionManager中获取完整的session信息
    fn extract_session_from_context(
        context: &RequestContext<RoleServer>,
    ) -> Option<Arc<SessionInfo>> {
        // 首先尝试从extensions中获取完整的SessionInfo
        if let Some(session_info_ext) = context.extensions.get::<SessionInfoExtension>() {
            tracing::debug!("Found SessionInfoExtension in RequestContext extensions");
            return Some(session_info_ext.0.clone());
        }

        // 如果没有完整session信息，尝试获取session_id然后查询SessionManager
        if let Some(session_id_ext) = context.extensions.get::<SessionIdExtension>() {
            let session_id = &session_id_ext.0;
            tracing::debug!("Found SessionIdExtension in RequestContext: {}", session_id);

            if let Some(session) = get_session_manager().get_session(session_id) {
                return Some(Arc::new(session));
            }
        }

        // 当MCP请求通过Streamable HTTP服务时，原始HTTP的Parts会被注入到extensions中
        // 我们需要从parts.extensions里再尝试一次取出session信息
        if let Some(http_parts) = context.extensions.get::<HttpRequestParts>() {
            if let Some(session_info_ext) = http_parts.extensions.get::<SessionInfoExtension>() {
                tracing::debug!("Found SessionInfoExtension inside HTTP request parts extensions");
                return Some(session_info_ext.0.clone());
            }

            if let Some(session_id_ext) = http_parts.extensions.get::<SessionIdExtension>() {
                let session_id = &session_id_ext.0;
                tracing::debug!(
                    "Found SessionIdExtension inside HTTP request parts: {}",
                    session_id
                );

                if let Some(session) = get_session_manager().get_session(session_id) {
                    return Some(Arc::new(session));
                }
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

    /// 获取token信息
    pub fn token(&self) -> Option<&crate::token_manager::Token> {
        self.session_info.as_ref().map(|s| &s.token)
    }

    /// 检查工具权限
    pub fn has_tool_permission(&self, tool_name: &str) -> bool {
        if let Some(session) = &self.session_info {
            session.token.has_tool_permission(tool_name)
        } else {
            false
        }
    }

    /// 检查资源权限
    pub fn has_resource_permission(&self, resource_uri: &str) -> bool {
        if let Some(session) = &self.session_info {
            session.token.has_resource_permission(resource_uri)
        } else {
            false
        }
    }

    /// 检查提示词权限
    pub fn has_prompt_permission(&self, prompt_name: &str) -> bool {
        if let Some(session) = &self.session_info {
            session.token.has_prompt_permission(prompt_name)
        } else {
            false
        }
    }

    /// 更新session的最后访问时间
    pub fn update_session_access(&self) {
        if let Some(session) = &self.session_info {
            // 这里需要SessionInfo支持更新访问时间
            // 但由于SessionInfo被Arc包装，我们需要可变引用
            // 可以考虑在SessionManager中添加update_session_access方法
            tracing::debug!("更新session {} 的访问时间", session.id);
        }
    }
}

/// 权限验证结果的枚举
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
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
    pub fn check_tool_permission_with_result(&self, tool_name: &str) -> PermissionResult {
        if !self.has_valid_session() {
            return PermissionResult::NotAuthenticated;
        }

        if self.is_session_expired() {
            return PermissionResult::SessionExpired;
        }

        if self.has_tool_permission(tool_name) {
            PermissionResult::Allowed
        } else {
            PermissionResult::InsufficientPermissions
        }
    }
}
