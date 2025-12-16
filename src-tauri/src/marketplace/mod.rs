use crate::error::{McpError, Result};
use crate::types::{MarketplaceService, MarketplaceServiceListItem};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub(crate) mod providers;

// Local MarketplaceSource struct since we removed it from config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSource {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

// Platform-specific result types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceServiceResult {
    pub services: Vec<MarketplaceServiceListItem>,
    pub total_count: usize,
    pub has_more: bool,
}

/// Marketplace client to fetch MCP services from multiple sources
#[derive(Clone)]
pub struct MarketplaceClient {
    pub(crate) http: Client,
}

impl Default for MarketplaceClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketplaceClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("McpRouter/0.1 (+https://github.com)")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { http: client }
    }

    /// Return all enabled sources (using hardcoded defaults)
    pub fn enabled_sources(&self) -> Vec<MarketplaceSource> {
        default_sources()
    }
}

/// Default integrated marketplace sources
pub fn default_sources() -> Vec<MarketplaceSource> {
    vec![MarketplaceSource {
        name: "modelscope.cn".to_string(),
        url: "https://www.modelscope.cn/openapi/v1/mcp/servers".to_string(),
        enabled: true,
        headers: None,
    }]
}

/// Get detailed info for a service id from ModelScope
pub async fn get_service_details(service_id: &str) -> Result<MarketplaceService> {
    let client = MarketplaceClient::new();

    // Find ModelScope source
    let source = client
        .enabled_sources()
        .into_iter()
        .find(|s| s.name == "modelscope.cn")
        .ok_or_else(|| McpError::MarketplaceError("ModelScope source not found".to_string()))?;

    // Call ModelScope provider directly
    providers::modelscope::fetch_modelscope_service_detail(&client, &source, service_id).await
}

// Popular services endpoint removed per product requirements

// ---- Provider trait removed: direct provider functions are used ----

/// Unified marketplace services listing with offset-based pagination
///
/// Parameters:
/// - query: Search query string
/// - page: Page number (1-based)
/// - page_size: Items per page
///
/// Returns MarketplaceServiceResult with services, total_count, has_more
pub async fn list_marketplace_services(
    query: Option<&str>,
    page: usize,
    page_size: usize,
) -> Result<MarketplaceServiceResult> {
    let client = MarketplaceClient::new();

    // Find ModelScope source
    let source = client
        .enabled_sources()
        .into_iter()
        .find(|s| s.name == "modelscope.cn")
        .ok_or_else(|| McpError::MarketplaceError("ModelScope source not found".to_string()))?;

    // Call ModelScope provider directly
    let (services, total_count) =
        providers::modelscope::fetch_modelscope_services(&client, &source, query, page, page_size)
            .await?;

    let has_more = (page * page_size) < total_count;

    Ok(MarketplaceServiceResult {
        services,
        total_count,
        has_more,
    })
}
