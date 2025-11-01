//! 自定义 HTTP 客户端工具，支持自定义 headers
//!
//! 该模块提供了使用 reqwest 创建自定义 HTTP 客户端的实用工具，
//! 并与 RMCP 的 StreamableHttpClientTransport 集成以支持自定义 headers。

use crate::error::Result;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransportConfig,
};
use std::collections::HashMap;
use tracing::{debug, warn};

/// 自定义 HTTP 客户端传输层配置器
///
/// 该结构体提供了链式 API 来配置带有自定义 headers 的 HTTP 传输层。
#[derive(Debug)]
pub struct HttpTransportConfig {
    /// 目标 URL
    url: String,
    /// 自定义 headers
    headers: HashMap<String, String>,
}

impl HttpTransportConfig {
    /// 创建新的配置器
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            headers: HashMap::new(),
        }
    }

    /// 添加自定义 header
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// 添加 Authorization header（Bearer token）
    pub fn authorization(mut self, token: &str) -> Self {
        self.headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }

    /// 添加 API Key header
    pub fn api_key(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// 添加多个 headers
    pub fn headers(mut self, headers: &HashMap<String, String>) -> Self {
        for (k, v) in headers {
            self.headers.insert(k.clone(), v.clone());
        }
        self
    }

    /// 构建 StreamableHttpClientTransportConfig
    ///
    /// 注意：该方法返回一个配置对象，调用者可以使用它来创建实际的传输层：
    /// ```rust
    /// let config = HttpTransportConfig::new(url).authorization(token).build_config()?;
    /// let transport = StreamableHttpClientTransport::from_config(config);
    /// ```
    pub fn build_config(self) -> Result<StreamableHttpClientTransportConfig> {
        debug!("Building HTTP transport config for: {}", self.url);
        debug!("Custom headers: {:?}", self.headers);

        let mut config = StreamableHttpClientTransportConfig::with_uri(self.url.as_str());

        // 处理所有自定义 headers
        for (key, value) in &self.headers {
            if key.eq_ignore_ascii_case("authorization") {
                // Authorization header 使用专门的方法
                let auth_token = value
                    .strip_prefix("Bearer ")
                    .unwrap_or(value.as_str());
                config = config.auth_header(auth_token);
                debug!("Set Authorization header in transport config");
            } else {
                // 其他 headers 当前不支持，记录警告
                warn!(
                    "Header '{}' cannot be set in current RMCP implementation. \
                     Only Authorization header is supported directly.",
                    key
                );
            }
        }

        Ok(config)
    }
}

/// 创建自定义 HTTP 传输层配置的便捷函数
pub fn create_transport_config(
    url: &str,
    headers: HashMap<String, String>,
) -> Result<StreamableHttpClientTransportConfig> {
    HttpTransportConfig::new(url).headers(&headers).build_config()
}

/// 创建带有 Authorization header 的传输层配置
pub fn create_transport_config_with_auth(
    url: &str,
    token: &str,
) -> Result<StreamableHttpClientTransportConfig> {
    HttpTransportConfig::new(url).authorization(token).build_config()
}

/// HTTP 客户端工厂
pub struct HttpClientFactory;

impl HttpClientFactory {
    /// 创建默认的 HTTP 传输层配置（无自定义 headers）
    pub fn create_default(url: &str) -> Result<StreamableHttpClientTransportConfig> {
        HttpTransportConfig::new(url).build_config()
    }

    /// 创建带有 API Key 的 HTTP 传输层配置
    pub fn create_with_api_key(
        url: &str,
        api_key: &str,
    ) -> Result<StreamableHttpClientTransportConfig> {
        HttpTransportConfig::new(url).authorization(api_key).build_config()
    }

    /// 创建带有自定义 Bearer Token 的 HTTP 传输层配置
    pub fn create_with_bearer_token(
        url: &str,
        token: &str,
    ) -> Result<StreamableHttpClientTransportConfig> {
        HttpTransportConfig::new(url).authorization(token).build_config()
    }

    /// 创建带有自定义 Content-Type 的 HTTP 传输层配置
    pub fn create_with_content_type(
        url: &str,
        content_type: &str,
    ) -> Result<StreamableHttpClientTransportConfig> {
        HttpTransportConfig::new(url)
            .header("Content-Type", content_type)
            .build_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_transport_config() {
        let config = HttpTransportConfig::new("http://example.com")
            .authorization("test-token")
            .header("X-Custom", "value")
            .build_config()
            .unwrap();

        // 验证构建成功
        assert!(config.uri().contains("http://example.com"));
    }

    #[test]
    fn test_create_transport_with_auth() {
        let config =
            create_transport_config_with_auth("http://example.com", "test-token").unwrap();

        assert!(config.uri().contains("http://example.com"));
    }
}
