use crate::error::{McpError, Result};
use crate::marketplace::MarketplaceSource;
use crate::{InstallCommand, MarketplaceService, MarketplaceServiceListItem};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::MarketplaceClient;

// ----- ModelScope OpenAPI typed structures -----
#[derive(Debug, Serialize, Default, Clone)]
struct ModelScopeFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_hosted: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ModelScopeListRequest {
    direction: i32,
    filter: ModelScopeFilter,
    page_number: u32,
    page_size: u32,
    search: String,
}

#[derive(Debug, Deserialize)]
struct ModelScopeListResponse {
    success: bool,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    data: Option<ModelScopeListData>,
}

#[derive(Debug, Deserialize)]
struct ModelScopeListData {
    #[serde(default)]
    mcp_server_list: Vec<ModelScopeServer>,
    #[serde(default)]
    total_count: u64,
}

#[derive(Debug, Deserialize)]
struct LocaleFields {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    readme: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Locales {
    #[serde(default)]
    zh: Option<LocaleFields>,
    #[serde(default)]
    en: Option<LocaleFields>,
}

#[derive(Debug, Deserialize)]
struct OperationalUrl {
    #[serde(default)]
    transport_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct EnvSchema {
    #[serde(default)]
    properties: std::collections::HashMap<String, EnvProperty>,
    #[serde(default)]
    required: Vec<String>,
    #[serde(rename = "type")]
    schema_type: String,
}

#[derive(Debug, Deserialize, Clone)]
struct EnvProperty {
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "type")]
    prop_type: String,
    #[serde(default)]
    default: Option<serde_json::Value>,
    #[serde(default, rename = "enum")]
    enum_values: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct ModelScopeServer {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    chinese_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    categories: Option<Vec<String>>,
    #[serde(default)]
    locales: Option<Locales>,
    #[serde(default)]
    view_count: Option<u64>,
    #[serde(default)]
    publisher: Option<String>,
    #[serde(default)]
    logo_url: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    github_stars: Option<u64>,
    #[serde(default)]
    is_hosted: Option<bool>,
    #[serde(default)]
    is_verified: Option<bool>,
    #[serde(default)]
    operational_urls: Option<Vec<OperationalUrl>>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    readme: Option<String>,
    #[serde(default)]
    server_config: Option<Vec<Value>>,
    #[serde(default)]
    env_schema: Option<EnvSchema>,
}

#[derive(Debug, Deserialize)]
struct ModelScopeDetailResponse {
    success: bool,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    data: ModelScopeServer,
}

/// Extract installation command from ModelScope server_config
/// server_config structure: [{"mcpServers": {"service-name": {"command": "...", "args": [...]}}}]
/// Returns None if installation command cannot be extracted
fn extract_install_command_from_server_config(
    server_config: &Option<Vec<Value>>,
    service_id: &str,
) -> Option<InstallCommand> {
    // Return None if server_config is missing
    let Some(config_array) = server_config else {
        tracing::debug!(
            "No server_config found for service {}, cannot extract install command",
            service_id
        );
        return None;
    };

    // Try to extract command from the first config object
    if let Some(first_config) = config_array.first() {
        // Navigate to mcpServers -> service-name -> command/args
        if let Some(mcp_servers) = first_config.get("mcpServers").and_then(|v| v.as_object()) {
            // Find the first service entry (we don't know the service name)
            if let Some((_, service_config)) = mcp_servers.iter().next() {
                if let Some(service_config_obj) = service_config.as_object() {
                    let command = service_config_obj.get("command").and_then(|v| v.as_str());
                    let args = service_config_obj.get("args").and_then(|v| v.as_array());

                    match (command, args) {
                        (Some(cmd), Some(args_array)) => {
                            let args_strs: Vec<String> = args_array
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect();

                            tracing::debug!(
                                "Extracted install command for service {}: {} {}",
                                service_id,
                                cmd,
                                args_strs.join(" ")
                            );

                            return Some(InstallCommand {
                                command: cmd.to_string(),
                                args: args_strs,
                                package_manager: cmd.to_string(),
                            });
                        }
                        (Some(cmd), None) => {
                            tracing::debug!(
                                "Extracted command without args for service {}: {}",
                                service_id,
                                cmd
                            );
                            return Some(InstallCommand {
                                command: cmd.to_string(),
                                args: vec![],
                                package_manager: cmd.to_string(),
                            });
                        }
                        _ => {
                            tracing::debug!(
                                "Invalid command/args in server_config for service {}, using default npx",
                                service_id
                            );
                        }
                    }
                }
            }
        }
    }

    tracing::debug!(
        "Failed to parse server_config for service {}, cannot extract install command",
        service_id
    );

    None
}

/// Convert ModelScope server JSON to MarketplaceServiceListItem (for list responses)
/// Note: This function should only include fields needed for list view to reduce payload size
fn modelscope_server_to_list_item(s: &ModelScopeServer) -> MarketplaceServiceListItem {
    let id = s.id.clone();

    let name = s
        .locales
        .as_ref()
        .and_then(|loc| loc.en.as_ref())
        .and_then(|en| en.name.as_ref())
        .and_then(|name| if name.is_empty() { None } else { Some(name) })
        .map(|s| s.to_string())
        .or_else(|| {
            // 如果英文名称不存在或为空，尝试中文名称
            s.locales
                .as_ref()
                .and_then(|loc| loc.zh.as_ref())
                .and_then(|zh| zh.name.as_ref())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .cloned()
        })
        .or_else(|| {
            // 尝试直接的名称字段（过滤空字符串）
            s.name.as_ref().and_then(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(name.clone())
                }
            })
        })
        .or_else(|| {
            // 尝试直接的中文名称字段（过滤空字符串）
            s.chinese_name.as_ref().and_then(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(name.clone())
                }
            })
        })
        .unwrap_or_else(|| id.clone());

    let description = s
        .locales
        .as_ref()
        .and_then(|loc| loc.zh.as_ref())
        .and_then(|zh| zh.description.as_ref())
        .cloned()
        .or_else(|| s.description.clone())
        .unwrap_or_default();

    let categories = s.categories.clone().unwrap_or_default();
    let category = categories
        .first()
        .cloned()
        .unwrap_or_else(|| "utilities".to_string());

    let tags = s.tags.clone().unwrap_or_default();
    let view_count = s.view_count.unwrap_or(0) as u32;
    // Prefer author (display name); fallback to owner (username), then publisher
    let author_name = s
        .author
        .clone()
        .or_else(|| s.owner.clone())
        .or_else(|| s.publisher.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let logo_url = s.logo_url.clone();

    // Infer transport from operational_urls if available, otherwise default to stdio
    let transport = s
        .operational_urls
        .as_ref()
        .and_then(|list| list.iter().find_map(|ou| ou.transport_type.clone()))
        .unwrap_or_else(|| "stdio".to_string());

    MarketplaceServiceListItem {
        id,
        name,
        description,
        author: author_name,
        category,
        tags,
        transport,
        github_stars: s.github_stars,
        downloads: view_count,
        last_updated: chrono::Utc::now().to_rfc3339(),
        platform: "魔搭社区".to_string(),
        logo_url,
        license: None,
        is_hosted: s.is_hosted,
        is_verified: s.is_verified,
    }
}

/// Convert ModelScope server JSON to MarketplaceService (for detail responses)
fn modelscope_server_to_marketplace(s: &ModelScopeServer) -> MarketplaceService {
    let id = s.id.clone();

    let name = s
        .locales
        .as_ref()
        .and_then(|loc| loc.en.as_ref())
        .and_then(|en| en.name.as_ref())
        .and_then(|name| if name.is_empty() { None } else { Some(name) })
        .map(|s| s.to_string())
        .or_else(|| {
            // 如果英文名称不存在或为空，尝试中文名称
            s.locales
                .as_ref()
                .and_then(|loc| loc.zh.as_ref())
                .and_then(|zh| zh.name.as_ref())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .cloned()
        })
        .or_else(|| {
            // 尝试直接的名称字段（过滤空字符串）
            s.name.as_ref().and_then(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(name.clone())
                }
            })
        })
        .or_else(|| {
            // 尝试直接的中文名称字段（过滤空字符串）
            s.chinese_name.as_ref().and_then(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(name.clone())
                }
            })
        })
        .unwrap_or_else(|| id.clone());

    let description = s
        .locales
        .as_ref()
        .and_then(|loc| loc.zh.as_ref())
        .and_then(|zh| zh.description.as_ref())
        .cloned()
        .or_else(|| s.description.clone())
        .unwrap_or_default();

    let categories = s.categories.clone().unwrap_or_default();
    let category = categories
        .first()
        .cloned()
        .unwrap_or_else(|| "utilities".to_string());

    let tags = s.tags.clone().unwrap_or_default();
    let view_count = s.view_count.unwrap_or(0) as u32;
    // Prefer author (display name); fallback to owner (username), then publisher
    let author_name = s
        .author
        .clone()
        .or_else(|| s.owner.clone())
        .or_else(|| s.publisher.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let logo_url = s.logo_url.clone();
    let homepage = s.source_url.clone();

    // Infer transport from operational_urls if available, otherwise default to stdio
    let transport = s
        .operational_urls
        .as_ref()
        .and_then(|list| list.iter().find_map(|ou| ou.transport_type.clone()))
        .unwrap_or_else(|| "stdio".to_string());

    let install_command = extract_install_command_from_server_config(&s.server_config, &id);

    // Prefer localized readme (zh) if available
    let readme = s
        .locales
        .as_ref()
        .and_then(|loc| loc.zh.as_ref())
        .and_then(|zh| zh.readme.clone())
        .or_else(|| s.readme.clone());

    MarketplaceService {
        id,
        name,
        description,
        author: author_name,
        category,
        tags,
        transport,
        install_command,
        github_stars: s.github_stars,
        downloads: view_count,
        last_updated: chrono::Utc::now().to_rfc3339(),
        platform: "魔搭社区".to_string(),
        logo_url,
        license: None,
        is_hosted: s.is_hosted,
        is_verified: s.is_verified,
        // Fields only in detail view
        homepage,
        repository: None,
        documentation_url: None,
        requirements: vec![],
        readme,
        server_config: s.server_config.clone(),
        env_schema: s.env_schema.clone().map(|schema| {
            let properties = schema
                .properties
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        crate::EnvProperty {
                            description: v.description,
                            prop_type: v.prop_type,
                            default: v.default,
                            enum_values: v.enum_values,
                        },
                    )
                })
                .collect();

            crate::EnvSchema {
                properties,
                required: schema.required,
                schema_type: schema.schema_type,
            }
        }),
    }
}

/// Fetch services from ModelScope API (uses PUT with JSON body)
/// Returns (services, total_count)
/// Implements pagination to fetch all available services (max 100 per request)
pub(crate) async fn fetch_modelscope_services(
    client: &MarketplaceClient,
    source: &MarketplaceSource,
    query: Option<&str>,
    page: usize,
    page_size: usize,
) -> Result<(Vec<MarketplaceServiceListItem>, usize)> {
    let url = Url::parse(&source.url).map_err(|e| {
        McpError::MarketplaceError(format!("Invalid source URL '{}': {}", source.url, e))
    })?;

    tracing::info!(
        "fetch_modelscope_services called with: query={:?}, page={}, page_size={}",
        query,
        page,
        page_size
    );

    // Typed filter
    let filter = ModelScopeFilter::default();
    // filter.is_hosted = Some(true);

    // ModelScope API has a hard limit: can only fetch first 100 items
    // Frontend should use page_size=100 to minimize number of requests
    let page_size = page_size as u32;
    let page_number = page as u32;

    tracing::info!(
        "ModelScope API limits: max_items=100, using page_size={}, page={}",
        page_size,
        page
    );

    let request_body = ModelScopeListRequest {
        direction: 1,
        filter: filter.clone(),
        page_number,
        page_size,
        search: query.unwrap_or("").to_string(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Some(h) = &source.headers {
        for (k, v) in h.iter() {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(k.as_bytes()),
                HeaderValue::from_str(v),
            ) {
                headers.insert(name, value);
            }
        }
    }

    tracing::debug!(
        "Fetching ModelScope services page {} with size {} (PUT) with typed body",
        page_number,
        page_size
    );

    let resp = client
        .http
        .put(url.clone())
        .headers(headers.clone())
        .json(&request_body)
        .send()
        .await
        .map_err(|e| McpError::NetworkError(format!("ModelScope PUT failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        // 尝试解析响应体以获取详细错误信息
        let error_text = resp.text().await.unwrap_or_default();

        // 尝试解析 JSON 错误响应
        if let Ok(error_resp) = serde_json::from_str::<ModelScopeListResponse>(&error_text) {
            let code = error_resp.code.unwrap_or_default();
            let message = error_resp.message.unwrap_or_default();
            let request_id = error_resp.request_id.unwrap_or_default();
            tracing::error!(
                "ModelScope API error: status={}, code={}, message={}, request_id={}",
                status,
                code,
                message,
                request_id
            );
            return Err(McpError::NetworkError(format!(
                "ModelScope API error (status: {}, code: {}, message: {}, request_id: {})",
                status, code, message, request_id
            )));
        }

        tracing::error!("ModelScope API returned status {}: {}", status, error_text);
        return Err(McpError::NetworkError(format!(
            "ModelScope API returned status {}: {}",
            status, error_text
        )));
    }

    let list_resp: ModelScopeListResponse = resp.json().await?;

    if !list_resp.success {
        let code = list_resp.code.unwrap_or_default();
        let message = list_resp.message.unwrap_or_default();
        let request_id = list_resp.request_id.unwrap_or_default();
        tracing::error!(
            "ModelScope API returned success=false: code={}, message={}, request_id={}",
            code,
            message,
            request_id
        );
        return Err(McpError::NetworkError(format!(
            "ModelScope API returned success=false (code: {}, message: {}, request_id: {})",
            code, message, request_id
        )));
    }

    // Handle missing data field
    let data = list_resp.data.ok_or_else(|| {
        McpError::NetworkError(
            "ModelScope API returned success=true but missing data field".to_string(),
        )
    })?;

    tracing::info!(
        "ModelScope API response: success={}, total_count={}, list_len={}, requested_page={}, requested_page_size={}",
        list_resp.success,
        data.total_count,
        data.mcp_server_list.len(),
        page_number,
        page_size
    );

    let total_count = data.total_count as usize;
    let services: Vec<MarketplaceServiceListItem> = data
        .mcp_server_list
        .iter()
        .map(modelscope_server_to_list_item)
        .collect();

    tracing::info!(
        "Fetched {} services from ModelScope page {} (total_count from API: {})",
        services.len(),
        page_number,
        total_count
    );

    // Check if we got fewer items than requested - might indicate end of data
    if services.is_empty() || (services.len() < page_size as usize && page_number > 1) {
        tracing::warn!(
            "ModelScope returned {} services but requested {}, this might be the last page",
            services.len(),
            page_size
        );
    }

    Ok((services, total_count))
}

/// Fetch service detail from ModelScope API (uses GET with ID in path)
pub(crate) async fn fetch_modelscope_service_detail(
    client: &MarketplaceClient,
    _source: &MarketplaceSource,
    service_id: &str,
) -> Result<MarketplaceService> {
    // ModelScope detail API: GET https://www.modelscope.cn/openapi/v1/mcp/servers/{id}
    let detail_url = format!(
        "https://www.modelscope.cn/openapi/v1/mcp/servers/{}",
        service_id
    );

    tracing::debug!("Fetching ModelScope service detail: {}", detail_url);

    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let resp = client.http.get(&detail_url).headers(headers).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        // 尝试解析响应体以获取详细错误信息
        let error_text = resp.text().await.unwrap_or_default();

        if let Ok(error_resp) = serde_json::from_str::<ModelScopeDetailResponse>(&error_text) {
            let code = error_resp.code.unwrap_or_default();
            let message = error_resp.message.unwrap_or_default();
            let request_id = error_resp.request_id.unwrap_or_default();
            tracing::error!(
                "ModelScope detail API error: status={}, code={}, message={}, request_id={}",
                status,
                code,
                message,
                request_id
            );
            return Err(McpError::NetworkError(format!(
                "ModelScope detail API error (status: {}, code: {}, message: {}, request_id: {})",
                status, code, message, request_id
            )));
        }

        tracing::error!(
            "ModelScope detail API returned status {}: {}",
            status,
            error_text
        );
        return Err(McpError::NetworkError(format!(
            "ModelScope detail API returned status {}: {}",
            status, error_text
        )));
    }

    let typed: ModelScopeDetailResponse = resp.json().await?;
    tracing::debug!(
        "ModelScope typed detail: success={} id={}",
        typed.success,
        typed.data.id
    );
    Ok(modelscope_server_to_marketplace(&typed.data))
}

// ----- Provider Implementation removed: using direct function calls -----
