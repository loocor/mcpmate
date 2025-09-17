//! Server capability handling utilities
//!
//! This module provides comprehensive functionality for managing MCP Server capabilities including:
//! - Database mapping and persistence for tools, prompts, resources, and resource templates
//! - Data enrichment with unique identifiers and database relationships
//! - JSON formatting for API responses
//! - Capability extraction from live server instances
//! - Refresh mechanisms for cache invalidation and temporary instances
//!
//! All capability types (Tools, Prompts, Resources, ResourceTemplates) follow unified patterns
//! for consistent handling across the API.

use axum::Json;
use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, sync::Arc};

use crate::api::handlers::ApiError;
use crate::api::handlers::server::common::{
    InspectParams, RefreshStrategy, ServerIdentification, get_database_from_state,
};
use crate::api::routes::AppState;

#[derive(Debug, Clone, Copy)]
pub enum CapabilityType {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

#[derive(Debug, Clone, Default)]
pub struct ExtractedCapability {
    pub data: Vec<serde_json::Value>,
    pub tools: Vec<crate::core::cache::CachedToolInfo>,
    pub prompts: Vec<crate::core::cache::CachedPromptInfo>,
    pub resources: Vec<crate::core::cache::CachedResourceInfo>,
    pub resource_templates: Vec<crate::core::cache::CachedResourceTemplateInfo>,
}

impl ExtractedCapability {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Load tool name to (id, unique_name) mapping from database
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `server_id` - Server identifier to filter tools
///
/// # Returns
/// HashMap mapping tool names to their (id, unique_name) tuples
pub async fn load_tool_mapping(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> HashMap<String, (String, String)> {
    sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT tool_name, id, unique_name FROM server_tools WHERE server_id = ?"#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(name, id, unique_name)| (name, (id, unique_name)))
    .collect()
}

/// Load prompt name to (id, unique_name) mapping from database
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `server_id` - Server identifier to filter prompts
///
/// # Returns
/// HashMap mapping prompt names to their (id, unique_name) tuples
pub async fn load_prompt_mapping(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> HashMap<String, (String, String)> {
    sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT prompt_name, id, unique_name FROM server_prompts WHERE server_id = ?"#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(name, id, unique_name)| (name, (id, unique_name)))
    .collect()
}

/// Load resource URI to (id, unique_uri) mapping from database
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `server_id` - Server identifier to filter resources
///
/// # Returns
/// HashMap mapping resource URIs to their (id, unique_uri) tuples
pub async fn load_resource_mapping(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> HashMap<String, (String, String)> {
    sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT resource_uri, id, unique_uri FROM server_resources WHERE server_id = ?"#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(uri, id, unique_uri)| (uri, (id, unique_uri)))
    .collect()
}

pub async fn load_resource_template_mapping(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> HashMap<String, (String, String)> {
    sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT uri_template, id, unique_name FROM server_resource_templates WHERE server_id = ?"#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(tpl, id, unique_name)| (tpl, (id, unique_name)))
    .collect()
}

/// Enrich tool items with database identifiers
#[inline]
fn enrich_tool_item(
    item: serde_json::Value,
    mapping: &HashMap<String, (String, String)>,
) -> serde_json::Value {
    let mut item = item;
    if let Some(name) = item.get("name").and_then(|x| x.as_str()) {
        if let Some((id, unique_name)) = mapping.get(name) {
            if let Some(obj) = item.as_object_mut() {
                obj.insert("unique_name".to_string(), serde_json::json!(unique_name));
                obj.insert("id".to_string(), serde_json::json!(id));
            }
        }
    }
    item
}

/// Enrich prompt items with database identifiers
#[inline]
fn enrich_prompt_item(
    item: serde_json::Value,
    mapping: &HashMap<String, (String, String)>,
) -> serde_json::Value {
    let mut item = item;
    if let Some(name) = item.get("name").and_then(|x| x.as_str()) {
        if let Some((id, unique_name)) = mapping.get(name) {
            if let Some(obj) = item.as_object_mut() {
                obj.insert("unique_name".to_string(), serde_json::json!(unique_name));
                obj.insert("id".to_string(), serde_json::json!(id));
            }
        }
    }
    item
}

/// Enrich resource items with database identifiers
#[inline]
fn enrich_resource_item(
    item: serde_json::Value,
    mapping: &HashMap<String, (String, String)>,
) -> serde_json::Value {
    let mut item = item;
    if let Some(uri) = item.get("uri").and_then(|x| x.as_str()) {
        if let Some((id, unique_uri)) = mapping.get(uri) {
            if let Some(obj) = item.as_object_mut() {
                obj.insert("unique_uri".to_string(), serde_json::json!(unique_uri));
                obj.insert("id".to_string(), serde_json::json!(id));
            }
        }
    }
    item
}

/// Enrich resource template items with database identifiers
#[inline]
fn enrich_resource_template_item(
    item: serde_json::Value,
    mapping: &HashMap<String, (String, String)>,
) -> serde_json::Value {
    let mut item = item;
    if let Some(tpl) = item.get("uri_template").and_then(|x| x.as_str()) {
        if let Some((id, unique_name)) = mapping.get(tpl) {
            if let Some(obj) = item.as_object_mut() {
                obj.insert("unique_uri_template".to_string(), serde_json::json!(unique_name));
                obj.insert("id".to_string(), serde_json::json!(id));
            }
        }
    }
    item
}

/// Enrich capability items with database-stored identifiers
///
/// Adds `id` and `unique_name` fields to capability items by looking up
/// the corresponding records in the database.
///
/// # Arguments
/// * `capability_type` - Type of capability (Tools, Prompts, Resources, ResourceTemplates)
/// * `pool` - SQLite connection pool
/// * `server_id` - Server identifier to filter records
/// * `items` - JSON objects representing capability items
///
/// # Returns
/// Enhanced JSON objects with `id` and `unique_name` fields added
pub async fn enrich_capability_items(
    capability_type: CapabilityType,
    pool: &Pool<Sqlite>,
    server_id: &str,
    items: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    match capability_type {
        CapabilityType::Tools => {
            let mapping = load_tool_mapping(pool, server_id).await;
            items.into_iter().map(|item| enrich_tool_item(item, &mapping)).collect()
        }
        CapabilityType::Prompts => {
            let mapping = load_prompt_mapping(pool, server_id).await;
            items
                .into_iter()
                .map(|item| enrich_prompt_item(item, &mapping))
                .collect()
        }
        CapabilityType::Resources => {
            let mapping = load_resource_mapping(pool, server_id).await;
            items
                .into_iter()
                .map(|item| enrich_resource_item(item, &mapping))
                .collect()
        }
        CapabilityType::ResourceTemplates => {
            let mapping = load_resource_template_mapping(pool, server_id).await;
            items
                .into_iter()
                .map(|item| enrich_resource_template_item(item, &mapping))
                .collect()
        }
    }
}

pub fn respond_with_enriched(
    data: Vec<serde_json::Value>,
    cache_hit: bool,
    refresh_strategy: Option<RefreshStrategy>,
    source: &str,
) -> Json<serde_json::Value> {
    crate::api::handlers::server::common::create_inspect_response(data, cache_hit, refresh_strategy, source)
}

/// Create standardized JSON representation of a tool
pub fn tool_json(
    name: &str,
    description: Option<String>,
    input_schema: serde_json::Value,
    unique_name: Option<String>,
    id: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "input_schema": input_schema,
        "unique_name": unique_name,
        "id": id,
    })
}

pub fn tool_json_from_cached(t: &crate::core::cache::CachedToolInfo) -> serde_json::Value {
    let schema = t.input_schema().unwrap_or_else(|_| serde_json::json!({}));
    tool_json(&t.name, t.description.clone(), schema, t.unique_name.clone(), None)
}

pub fn resource_json(
    uri: &str,
    name: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
    unique_uri: Option<String>,
    id: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "uri": uri,
        "name": name,
        "description": description,
        "mime_type": mime_type,
        "unique_uri": unique_uri,
        "id": id,
    })
}

pub fn resource_json_from_cached(r: crate::core::cache::CachedResourceInfo) -> serde_json::Value {
    resource_json(&r.uri, r.name, r.description, r.mime_type, None, None)
}

pub fn resource_template_json(
    uri_template: &str,
    name: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
    unique_uri_template: Option<String>,
    id: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "uri_template": uri_template,
        "name": name,
        "description": description,
        "mime_type": mime_type,
        "unique_uri_template": unique_uri_template,
        "id": id,
    })
}

pub fn resource_template_json_from_cached(t: crate::core::cache::CachedResourceTemplateInfo) -> serde_json::Value {
    resource_template_json(&t.uri_template, t.name, t.description, t.mime_type, None, None)
}

pub fn prompt_json(
    name: &str,
    description: Option<String>,
    arguments: Vec<crate::core::cache::PromptArgument>,
    unique_name: Option<String>,
    id: Option<String>,
) -> serde_json::Value {
    let args: Vec<serde_json::Value> = arguments
        .into_iter()
        .map(|a| {
            serde_json::json!({
                "name": a.name,
                "description": a.description,
                "required": a.required,
            })
        })
        .collect();

    serde_json::json!({
        "name": name,
        "description": description,
        "arguments": args,
        "unique_name": unique_name,
        "id": id,
    })
}

pub fn prompt_json_from_cached(p: crate::core::cache::CachedPromptInfo) -> serde_json::Value {
    prompt_json(&p.name, p.description.clone(), p.arguments.clone(), None, None)
}

pub async fn extract_tools_capability(
    conn: &crate::core::pool::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    let now = chrono::Utc::now();

    let (data, tools): (Vec<_>, Vec<_>) = conn
        .tools
        .iter()
        .map(|t| {
            let schema = t.schema_as_json_value();
            let data_item = serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": schema,
                "unique_name": serde_json::Value::Null,
            });

            let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
            let tool_info = crate::core::cache::CachedToolInfo {
                name: t.name.to_string(),
                description: t.description.clone().map(|d| d.into_owned()),
                input_schema_json,
                unique_name: None,
                enabled: true,
                cached_at: now,
            };

            (data_item, tool_info)
        })
        .unzip();

    Ok(ExtractedCapability {
        data,
        tools,
        prompts: Vec::new(),
        resources: Vec::new(),
        resource_templates: Vec::new(),
    })
}

pub async fn extract_prompts_capability(
    conn: &crate::core::pool::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    if !conn.supports_prompts() {
        return Ok(ExtractedCapability::empty());
    }

    let service = match &conn.service {
        Some(service) => service,
        None => return Ok(ExtractedCapability::empty()),
    };

    let list_result = service
        .list_prompts(None)
        .await
        .map_err(|_| ApiError::InternalError("Failed to list prompts".to_string()))?;

    let now = chrono::Utc::now();
    let (data, prompts): (Vec<_>, Vec<_>) = list_result
        .prompts
        .into_iter()
        .map(|p| {
            let arguments = p.arguments.unwrap_or_default();

            let prompt_info = crate::core::cache::CachedPromptInfo {
                name: p.name,
                description: p.description,
                arguments: arguments
                    .clone()
                    .into_iter()
                    .map(|arg| crate::core::cache::PromptArgument {
                        name: arg.name,
                        description: arg.description,
                        required: arg.required.unwrap_or(false),
                    })
                    .collect(),
                enabled: true,
                cached_at: now,
            };

            let data_item = serde_json::json!({
                "name": prompt_info.name,
                "description": prompt_info.description,
                "arguments": arguments,
            });

            (data_item, prompt_info)
        })
        .unzip();

    Ok(ExtractedCapability {
        data,
        tools: Vec::new(),
        prompts,
        resources: Vec::new(),
        resource_templates: Vec::new(),
    })
}

pub async fn extract_resources_capability(
    conn: &crate::core::pool::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    if !conn.supports_resources() {
        return Ok(ExtractedCapability::empty());
    }

    let service = match &conn.service {
        Some(service) => service,
        None => return Ok(ExtractedCapability::empty()),
    };

    let list_result = service
        .list_resources(None)
        .await
        .map_err(|_| ApiError::InternalError("Failed to list resources".to_string()))?;

    let now = chrono::Utc::now();
    let (data, resources): (Vec<_>, Vec<_>) = list_result
        .resources
        .into_iter()
        .map(|r| {
            let raw = &*r;
            let resource_info = crate::core::cache::CachedResourceInfo {
                uri: raw.uri.clone(),
                name: Some(raw.name.clone()),
                description: raw.description.clone(),
                mime_type: raw.mime_type.clone(),
                enabled: true,
                cached_at: now,
            };

            let data_item = serde_json::json!({
                "uri": resource_info.uri,
                "name": resource_info.name,
                "description": resource_info.description,
                "mime_type": resource_info.mime_type,
            });

            (data_item, resource_info)
        })
        .unzip();

    Ok(ExtractedCapability {
        data,
        tools: Vec::new(),
        prompts: Vec::new(),
        resources,
        resource_templates: Vec::new(),
    })
}

pub async fn extract_resource_templates_capability(
    conn: &crate::core::pool::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    if !conn.supports_resources() {
        return Ok(ExtractedCapability::empty());
    }

    let service = match &conn.service {
        Some(service) => service,
        None => return Ok(ExtractedCapability::empty()),
    };

    let now = chrono::Utc::now();
    let mut all_templates = Vec::new();
    let mut cursor = None;

    // Paginated resource template collection
    loop {
        let list_result = service
            .list_resource_templates(Some(rmcp::model::PaginatedRequestParam { cursor }))
            .await
            .map_err(|_| ApiError::InternalError("Failed to list resource templates".to_string()))?;

        all_templates.extend(list_result.resource_templates);
        cursor = list_result.next_cursor;

        if cursor.is_none() {
            break;
        }
    }

    let (data, resource_templates): (Vec<_>, Vec<_>) = all_templates
        .into_iter()
        .map(|t| {
            let data_item = serde_json::json!({
                "uri_template": t.uri_template,
                "name": t.name,
                "description": t.description,
                "mime_type": t.mime_type,
            });

            let template_info = crate::core::cache::CachedResourceTemplateInfo {
                uri_template: t.uri_template.clone(),
                name: Some(t.name.clone()),
                description: t.description.clone(),
                mime_type: t.mime_type.clone(),
                enabled: true,
                cached_at: now,
            };

            (data_item, template_info)
        })
        .unzip();

    Ok(ExtractedCapability {
        data,
        tools: Vec::new(),
        prompts: Vec::new(),
        resources: Vec::new(),
        resource_templates,
    })
}

/// Create temporary server instance for capability extraction during force refresh
///
/// This function handles force refresh requests by creating a temporary MCP server
/// instance to extract fresh capability data when cache is stale or empty.
///
/// # Arguments
/// * `state` - Application state containing connection pools and cache
/// * `server_info` - Server identification information
/// * `params` - Inspection parameters including refresh strategy
/// * `capability_type` - Type of capability to extract
///
/// # Returns
/// - `Ok(Some(Json))` - Successfully extracted and enriched capability data
/// - `Ok(None)` - No force refresh requested or temporary instance creation failed
/// - `Err(ApiError)` - Database or extraction error occurred
pub async fn create_temporary_instance_for_capability(
    state: &Arc<AppState>,
    server_info: &ServerIdentification,
    params: &InspectParams,
    capability_type: CapabilityType,
    allow_without_force: bool,
) -> Result<Option<Json<serde_json::Value>>, ApiError> {
    if params.refresh != Some(RefreshStrategy::Force) && !allow_without_force {
        return Ok(None);
    }

    // Try to reuse an existing connected instance first
    use crate::api::handlers::server::common::ConnectionPoolManager;
    let mut pool = match ConnectionPoolManager::get_pool_for_capability(state).await {
        Ok(pool) => pool,
        Err(_) => return Ok(None),
    };

    if let Some(instances) = pool.connections.get(&server_info.server_id) {
        if let Some(conn) = instances.values().find(|c| c.is_connected()) {
            let extracted = match capability_type {
                CapabilityType::Tools => extract_tools_capability(conn).await?,
                CapabilityType::Prompts => extract_prompts_capability(conn).await?,
                CapabilityType::Resources => extract_resources_capability(conn).await?,
                CapabilityType::ResourceTemplates => extract_resource_templates_capability(conn).await?,
            };

            if !extracted.data.is_empty() {
                if let Ok(db) = get_database_from_state(state) {
                    let _ = crate::config::server::capabilities::store_dual_write(
                        &db.pool,
                        &state.redb_cache,
                        &server_info.server_id,
                        &server_info.server_name,
                        extracted.tools.clone(),
                        extracted.resources.clone(),
                        extracted.prompts.clone(),
                        extracted.resource_templates.clone(),
                    )
                    .await;
                }
            }

            if let Ok(db) = get_database_from_state(state) {
                let enriched =
                    enrich_capability_items(capability_type, &db.pool, &server_info.server_id, extracted.data).await;
                return Ok(Some(respond_with_enriched(enriched, false, params.refresh, "runtime")));
            }
            return Ok(Some(respond_with_enriched(
                Vec::new(),
                false,
                params.refresh,
                "runtime",
            )));
        }
    }

    // Create temporary validation instance
    match pool
        .get_or_create_validation_instance(&server_info.server_name, "api", std::time::Duration::from_secs(5 * 60))
        .await
    {
        Ok(Some(validation_conn)) => {
            let extracted = match capability_type {
                CapabilityType::Tools => extract_tools_capability(validation_conn).await?,
                CapabilityType::Prompts => extract_prompts_capability(validation_conn).await?,
                CapabilityType::Resources => extract_resources_capability(validation_conn).await?,
                CapabilityType::ResourceTemplates => extract_resource_templates_capability(validation_conn).await?,
            };

            if !extracted.data.is_empty() {
                if let Ok(db) = get_database_from_state(state) {
                    let _ = crate::config::server::capabilities::store_dual_write(
                        &db.pool,
                        &state.redb_cache,
                        &server_info.server_id,
                        &server_info.server_name,
                        extracted.tools.clone(),
                        extracted.resources.clone(),
                        extracted.prompts.clone(),
                        extracted.resource_templates.clone(),
                    )
                    .await;
                }
            }

            let data = extracted.data;
            let items = match get_database_from_state(state) {
                Ok(db) => enrich_capability_items(capability_type, &db.pool, &server_info.server_id, data).await,
                Err(_) => Vec::new(),
            };

            Ok(Some(respond_with_enriched(items, false, params.refresh, "temporary")))
        }
        _ => Ok(None),
    }
}
