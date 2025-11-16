use crate::token_manager::Token;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Session信息，包含权限数据
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SessionInfo {
    pub id: String,
    pub token: Token,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub expires_at: Option<Instant>,
}

#[allow(dead_code)]
impl SessionInfo {
    /// 创建新的session
    pub fn new(token: Token, expires_at: Option<Instant>) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            token,
            created_at: now,
            last_accessed: now,
            expires_at,
        }
    }

    /// 检查session是否已过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() > expires_at
        } else {
            // 使用token的过期时间
            self.token.is_expired()
        }
    }

    /// 更新最后访问时间
    pub fn update_access(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// 检查session是否在指定的空闲时间内被访问过
    pub fn is_idle_longer_than(&self, idle_timeout: Duration) -> bool {
        self.last_accessed.elapsed() > idle_timeout
    }
}

/// Session管理器，负责管理连接级的权限缓存
#[derive(Debug)]
#[allow(dead_code)]
pub struct SessionManager {
    sessions: Arc<DashMap<String, SessionInfo>>,
    cleanup_interval: Duration,
    default_idle_timeout: Duration,
}

#[allow(dead_code)]
impl SessionManager {
    /// 创建新的SessionManager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            cleanup_interval: Duration::from_secs(300), // 5分钟清理一次
            default_idle_timeout: Duration::from_secs(3600), // 1小时空闲超时
        }
    }

    /// 创建带有自定义配置的SessionManager
    pub fn new_with_config(cleanup_interval: Duration, idle_timeout: Duration) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            cleanup_interval,
            default_idle_timeout: idle_timeout,
        }
    }

    /// 创建新的session并返回session ID
    pub fn create_session(&self, token: Token) -> String {
        let expires_at = token.expires_at.map(|timestamp| {
            Instant::now()
                + Duration::from_secs(
                    timestamp.saturating_sub(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    ),
                )
        });

        let session = SessionInfo::new(token, expires_at);
        let session_id = session.id.clone();

        tracing::info!(
            "Creating session {} for token {}",
            session_id,
            session.token.id
        );
        self.sessions.insert(session_id.clone(), session);

        // 启动后台清理任务
        self.start_cleanup_task();

        session_id
    }

    /// 获取session信息
    pub fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            if session.is_expired() {
                tracing::info!("Session {} expired, removing", session_id);
                self.sessions.remove(session_id);
                return None;
            }

            session.update_access();
            Some(session.clone())
        } else {
            None
        }
    }

    /// 验证session并检查工具权限
    pub fn check_tool_permission(&self, session_id: &str, tool_name: &str) -> bool {
        if let Some(session) = self.get_session(session_id) {
            session.token.has_tool_permission(tool_name)
        } else {
            false
        }
    }

    /// 验证session并检查资源权限
    pub fn check_resource_permission(&self, session_id: &str, resource_uri: &str) -> bool {
        if let Some(session) = self.get_session(session_id) {
            session.token.has_resource_permission(resource_uri)
        } else {
            false
        }
    }

    /// 验证session并检查提示词权限
    pub fn check_prompt_permission(&self, session_id: &str, prompt_name: &str) -> bool {
        if let Some(session) = self.get_session(session_id) {
            session.token.has_prompt_permission(prompt_name)
        } else {
            false
        }
    }

    /// 移除session
    pub fn remove_session(&self, session_id: &str) -> bool {
        tracing::info!("Removing session {}", session_id);
        self.sessions.remove(session_id).is_some()
    }

    /// 获取活跃session数量
    pub fn active_sessions_count(&self) -> usize {
        self.sessions.len()
    }

    /// 清理过期和空闲的sessions
    pub fn cleanup_expired_sessions(&self) -> usize {
        let mut removed_count = 0;
        let sessions_to_remove: Vec<String> = self
            .sessions
            .iter()
            .filter(|entry| {
                let session = entry.value();
                session.is_expired() || session.is_idle_longer_than(self.default_idle_timeout)
            })
            .map(|entry| entry.key().clone())
            .collect();

        for session_id in sessions_to_remove {
            if self.sessions.remove(&session_id).is_some() {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            tracing::info!("Cleaned up {} expired/idle sessions", removed_count);
        }

        removed_count
    }

    /// 启动后台清理任务
    fn start_cleanup_task(&self) {
        let sessions = Arc::clone(&self.sessions);
        let cleanup_interval = self.cleanup_interval;
        let idle_timeout = self.default_idle_timeout;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                let sessions_to_remove: Vec<String> = sessions
                    .iter()
                    .filter(|entry| {
                        let session = entry.value();
                        session.is_expired() || session.is_idle_longer_than(idle_timeout)
                    })
                    .map(|entry| entry.key().clone())
                    .collect();

                for session_id in sessions_to_remove {
                    sessions.remove(&session_id);
                }
            }
        });
    }

    /// 获取所有活跃session的统计信息
    pub fn get_session_stats(&self) -> SessionStats {
        let mut stats = SessionStats::default();
        let now = Instant::now();

        for session in self.sessions.iter() {
            let session = session.value();
            stats.total_sessions += 1;

            if session.is_expired() {
                stats.expired_sessions += 1;
            } else if session.is_idle_longer_than(self.default_idle_timeout) {
                stats.idle_sessions += 1;
            } else {
                stats.active_sessions += 1;
            }

            let age = now.duration_since(session.created_at);
            stats.average_session_age += age.as_secs();
        }

        if stats.total_sessions > 0 {
            stats.average_session_age /= stats.total_sessions as u64;
        }

        stats
    }
}

/// Session统计信息
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub expired_sessions: usize,
    pub idle_sessions: usize,
    pub average_session_age: u64,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局SessionManager实例
#[allow(dead_code)]
static SESSION_MANAGER: std::sync::LazyLock<SessionManager> =
    std::sync::LazyLock::new(SessionManager::new);

/// 获取全局SessionManager实例
#[allow(dead_code)]
pub fn get_session_manager() -> &'static SessionManager {
    &SESSION_MANAGER
}
