// Common utilities for server API handlers
// Provides shared functions for server identification, validation, and response formatting

use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, sync::Arc};

use crate::{
    api::{
        handlers::ApiError,
        models::server::{ServerInstanceSummary, ServerMetaInfo},
        routes::AppState,
    },
    config::{database::Database, server},
    core::cache::{CacheQuery, FreshnessLevel},
    core::pool::UpstreamConnectionPool,
};

/// Server identification result
#[derive(Debug, Clone)]
pub struct ServerIdentification {
    /// Server ID (guaranteed to exist)
    pub server_id: String,
    /// Server name (guaranteed to exist)
    pub server_name: String,
}

/// Complete server details for response building
#[derive(Debug, Default)]
pub struct ServerDetails {
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub meta: Option<ServerMetaInfo>,
    pub globally_enabled: bool,
    pub enabled_in_suits: bool,
    pub instances: Vec<ServerInstanceSummary>,
}

/// Resolve server identifier (name or ID) to complete server information
///
/// This function provides intelligent resolution of server identifiers:
/// - Accepts both server_name and server_id as input
/// - Returns complete server identification information
/// - Handles edge cases and provides clear error messages
pub async fn resolve_server_identifier(
    pool: &Pool<Sqlite>,
    identifier: &str,
) -> Result<ServerIdentification, ApiError> {
    // Validate input
    if identifier.trim().is_empty() {
        return Err(ApiError::BadRequest("Server identifier cannot be empty".to_string()));
    }

    // Try to find server by ID first (more efficient for ID-based lookups)
    if let Some(server) = server::get_server_by_id(pool, identifier)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {e}")))?
    {
        let server_id = server
            .id
            .ok_or_else(|| ApiError::InternalError("Server found but missing ID".to_string()))?;

        return Ok(ServerIdentification {
            server_id,
            server_name: server.name,
        });
    }

    // Try to find server by name
    if let Some(server) = server::get_server(pool, identifier)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {e}")))?
    {
        let server_id = server
            .id
            .ok_or_else(|| ApiError::InternalError("Server found but missing ID".to_string()))?;

        return Ok(ServerIdentification {
            server_id,
            server_name: server.name,
        });
    }

    // Server not found
    Err(ApiError::NotFound(format!(
        "Server '{}' not found. Please check the server name or ID.",
        identifier
    )))
}

/// Get complete server details including args, env, meta, and status information
///
/// This function consolidates all server detail retrieval logic that was
/// previously duplicated across multiple handler functions.
pub async fn get_complete_server_details(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    state: &Arc<AppState>,
) -> ServerDetails {
    let mut details = ServerDetails::default();

    // Get server arguments
    if !server_id.is_empty() {
        match server::get_server_args(pool, server_id).await {
            Ok(server_args) => {
                if !server_args.is_empty() {
                    let mut sorted_args: Vec<_> = server_args.into_iter().collect();
                    sorted_args.sort_by_key(|arg| arg.arg_index);
                    details.args = Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect());
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get arguments for server '{}': {}", server_name, e);
            }
        }
    }

    // Get server environment variables
    if !server_id.is_empty() {
        match server::get_server_env(pool, server_id).await {
            Ok(env_map) => {
                if !env_map.is_empty() {
                    details.env = Some(env_map);
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to get environment variables for server '{}': {}",
                    server_name,
                    e
                );
            }
        }
    }

    // Get server metadata
    if !server_id.is_empty() {
        match server::get_server_meta(pool, server_id).await {
            Ok(Some(server_meta)) => {
                details.meta = Some(ServerMetaInfo {
                    description: server_meta.description,
                    author: server_meta.author,
                    website: server_meta.website,
                    repository: server_meta.repository,
                    category: server_meta.category,
                    recommended_scenario: server_meta.recommended_scenario,
                    rating: server_meta.rating,
                });
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("Failed to get metadata for server '{}': {}", server_name, e);
            }
        }
    }

    // Get server global enabled status
    details.globally_enabled = server::get_server_global_status(pool, server_id)
        .await
        .unwrap_or(Some(true))
        .unwrap_or(true);

    // Get server enabled status in config suits
    details.enabled_in_suits = server::is_server_enabled_in_any_suit(pool, server_id)
        .await
        .unwrap_or(false);

    // Get instance information from connection pool
    details.instances = get_server_instances(state, server_name).await;

    details
}

/// Get server instances with timeout protection
///
/// Consolidates the connection pool access and instance retrieval logic
/// that was duplicated across multiple handlers.
pub async fn get_server_instances(
    state: &Arc<AppState>,
    server_name: &str,
) -> Vec<ServerInstanceSummary> {
    match tokio::time::timeout(std::time::Duration::from_secs(1), state.connection_pool.lock()).await {
        Ok(pool) => {
            if let Some(instances) = pool.connections.get(server_name) {
                instances
                    .iter()
                    .map(|(id, conn)| {
                        let connected_at = if conn.is_connected() {
                            Some(
                                chrono::DateTime::<chrono::Utc>::from(
                                    std::time::SystemTime::now() - conn.time_since_last_connection(),
                                )
                                .to_rfc3339(),
                            )
                        } else {
                            None
                        };

                        ServerInstanceSummary {
                            id: id.clone(),
                            status: conn.status_string(),
                            started_at: Some(
                                chrono::DateTime::<chrono::Utc>::from(
                                    std::time::SystemTime::now() - conn.time_since_creation(),
                                )
                                .to_rfc3339(),
                            ),
                            connected_at,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        }
        Err(_) => {
            tracing::warn!(
                "Timed out waiting for connection pool lock for server '{}'",
                server_name
            );
            Vec::new()
        }
    }
}

/// Get connection pool with timeout protection
///
/// Provides a standardized way to access the connection pool with timeout handling.
pub async fn get_connection_pool_with_timeout(
    state: &Arc<AppState>
) -> Result<tokio::sync::MutexGuard<'_, UpstreamConnectionPool>, ApiError> {
    match tokio::time::timeout(std::time::Duration::from_secs(1), state.connection_pool.lock()).await {
        Ok(pool) => Ok(pool),
        Err(_) => Err(ApiError::InternalError(
            "Timed out waiting for connection pool lock".to_string(),
        )),
    }
}

/// Get server by name or ID with validation
///
/// Consolidates server lookup logic with proper error handling.
pub async fn get_server_by_identifier(
    pool: &Pool<Sqlite>,
    identifier: &str,
) -> Result<(crate::config::models::Server, String), ApiError> {
    let server = server::get_server(pool, identifier)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    let server = server.ok_or_else(|| ApiError::NotFound(format!("Server '{identifier}' not found")))?;

    let server_id = server
        .id
        .clone()
        .ok_or_else(|| ApiError::InternalError(format!("Server '{identifier}' has no ID")))?;

    Ok((server, server_id))
}

/// Note: Validate server access permissions (placeholder for future authorization)
///
/// This function can be extended to include authorization checks,
/// rate limiting, or other access control mechanisms.
pub async fn validate_server_access(
    _pool: &Pool<Sqlite>,
    _server_id: &str,
) -> Result<(), ApiError> {
    // For now, all servers are accessible
    // Add authorization logic here if needed
    Ok(())
}

/// Get database from application state
///
/// Helper function to extract database connection from AppState
/// with proper error handling.
pub fn get_database_from_state(state: &Arc<AppState>) -> Result<Arc<Database>, ApiError> {
    state
        .http_proxy
        .as_ref()
        .and_then(|p| p.database.clone())
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))
}

/// Get inspect service from application state
///
/// Helper function to extract inspect service from AppState
/// with proper error handling.
// Refresh strategy for backward compatibility
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum RefreshStrategy {
    #[default]
    CacheFirst,
    RefreshIfStale,
    Force,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InspectParams {
    pub refresh: Option<RefreshStrategy>,
    pub format: Option<String>,
    pub include_meta: Option<bool>,
    pub timeout: Option<u64>,
}

/// Unified query parameters for inspect endpoints
/// Replaces duplicate ToolsQuery, PromptsQuery, and ResourcesQuery structures
#[derive(Debug, serde::Deserialize)]
pub struct InspectQuery {
    /// Refresh strategy for queries
    pub refresh: Option<RefreshStrategy>,
    /// Response format
    pub format: Option<String>,
    /// Whether to include metadata
    pub include_meta: Option<bool>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
}

impl InspectQuery {
    /// Convert to InspectParams
    pub fn to_params(&self) -> Result<InspectParams, crate::api::handlers::ApiError> {
        Ok(InspectParams {
            refresh: self.refresh.or(Some(RefreshStrategy::CacheFirst)),
            format: self.format.clone(),
            include_meta: self.include_meta,
            timeout: self.timeout,
        })
    }
}

/// Map inspect RefreshStrategy to cache FreshnessLevel for Redb
pub fn map_refresh_to_freshness(refresh: RefreshStrategy) -> FreshnessLevel {
    match refresh {
        RefreshStrategy::CacheFirst => FreshnessLevel::Cached,
        RefreshStrategy::RefreshIfStale => FreshnessLevel::RecentlyFresh,
        RefreshStrategy::Force => FreshnessLevel::RealTime,
    }
}

/// Validate a cached snapshot for being non-empty
pub fn cached_snapshot_has_data(data: &crate::core::cache::CachedServerData) -> bool {
    !(data.tools.is_empty()
        && data.resources.is_empty()
        && data.prompts.is_empty()
        && data.resource_templates.is_empty())
}

/// Build a Redb CacheQuery from server id and Inspect params
pub fn build_cache_query(
    server_id: &str,
    params: &InspectParams,
) -> CacheQuery {
    let refresh = params.refresh.unwrap_or_default();
    let freshness_level = map_refresh_to_freshness(refresh);
    CacheQuery {
        server_id: server_id.to_string(),
        freshness_level,
        include_disabled: false,
    }
}

/// Capability type for temporary instance extraction
#[derive(Debug, Clone)]
pub enum CapabilityType {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

/// Result of capability extraction from temporary instance
pub struct ExtractedCapability {
    pub data: Vec<serde_json::Value>,
    pub tools: Vec<crate::core::cache::CachedToolInfo>,
    pub prompts: Vec<crate::core::cache::CachedPromptInfo>,
    pub resources: Vec<crate::core::cache::CachedResourceInfo>,
    pub resource_templates: Vec<crate::core::cache::CachedResourceTemplateInfo>,
}

impl ExtractedCapability {
    pub fn empty() -> Self {
        Self {
            data: Vec::new(),
            tools: Vec::new(),
            prompts: Vec::new(),
            resources: Vec::new(),
            resource_templates: Vec::new(),
        }
    }
}

/// Extract tools from temporary instance
async fn extract_tools_capability(
    conn: &crate::core::connection::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    let mut result = ExtractedCapability::empty();

    for t in &conn.tools {
        let schema = t.schema_as_json_value();
        result.data.push(serde_json::json!({
            "name": t.name,
            "description": t.description,
            "input_schema": schema,
            "unique_name": serde_json::Value::Null
        }));

        let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
        result.tools.push(crate::core::cache::CachedToolInfo {
            name: t.name.to_string(),
            description: t.description.clone().map(|d| d.into_owned()),
            input_schema_json,
            unique_name: None,
            enabled: true,
            cached_at: chrono::Utc::now(),
        });
    }

    Ok(result)
}

/// Extract prompts from temporary instance
async fn extract_prompts_capability(
    conn: &crate::core::connection::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    let mut result = ExtractedCapability::empty();

    if conn.supports_prompts() {
        if let Some(service) = &conn.service {
            if let Ok(list_result) = service.list_prompts(None).await {
                for p in list_result.prompts {
                    result.data.push(serde_json::json!({
                        "name": p.name,
                        "description": p.description,
                        "arguments": p.arguments.clone().unwrap_or_default(),
                    }));

                    let converted_args = p
                        .arguments
                        .unwrap_or_default()
                        .into_iter()
                        .map(|arg| crate::core::cache::PromptArgument {
                            name: arg.name,
                            description: arg.description,
                            required: arg.required.unwrap_or(false),
                        })
                        .collect();

                    result.prompts.push(crate::core::cache::CachedPromptInfo {
                        name: p.name,
                        description: p.description,
                        arguments: converted_args,
                        enabled: true,
                        cached_at: chrono::Utc::now(),
                    });
                }
            }
        }
    }

    Ok(result)
}

/// Extract resources from temporary instance
async fn extract_resources_capability(
    conn: &crate::core::connection::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    let mut result = ExtractedCapability::empty();

    if conn.supports_resources() {
        if let Some(service) = &conn.service {
            if let Ok(list_result) = service.list_resources(None).await {
                for r in list_result.resources {
                    result.data.push(serde_json::json!({
                        "uri": r.uri,
                        "name": r.name,
                        "description": r.description,
                        "mime_type": r.mime_type,
                    }));

                    result.resources.push(crate::core::cache::CachedResourceInfo {
                        uri: r.uri.clone(),
                        name: Some(r.name.clone()),
                        description: r.description.clone(),
                        mime_type: r.mime_type.clone(),
                        enabled: true,
                        cached_at: chrono::Utc::now(),
                    });
                }
            }
        }
    }

    Ok(result)
}

/// Extract resource templates from temporary instance
async fn extract_resource_templates_capability(
    conn: &crate::core::connection::UpstreamConnection
) -> Result<ExtractedCapability, ApiError> {
    let mut result = ExtractedCapability::empty();

    if conn.supports_resources() {
        if let Some(service) = &conn.service {
            let mut cursor = None;
            while let Ok(list_result) = service
                .list_resource_templates(Some(rmcp::model::PaginatedRequestParam { cursor }))
                .await
            {
                for t in list_result.resource_templates {
                    result.data.push(serde_json::json!({
                        "uri_template": t.uri_template,
                        "name": t.name,
                        "description": t.description,
                        "mime_type": t.mime_type,
                    }));

                    result
                        .resource_templates
                        .push(crate::core::cache::CachedResourceTemplateInfo {
                            uri_template: t.uri_template.clone(),
                            name: Some(t.name.clone()),
                            description: t.description.clone(),
                            mime_type: t.mime_type.clone(),
                            enabled: true,
                            cached_at: chrono::Utc::now(),
                        });
                }
                cursor = list_result.next_cursor;
                if cursor.is_none() {
                    break;
                }
            }
        }
    }

    Ok(result)
}

/// Create temporary instance and extract capability data for refresh=force requests
///
/// This unified function handles capability extraction for all types (tools, prompts, resources, resource templates)
/// when a force refresh is requested. It creates a temporary validation instance and extracts the requested capability data.
pub async fn create_temporary_instance_for_capability(
    state: &Arc<AppState>,
    server_info: &ServerIdentification,
    params: &InspectParams,
    capability_type: CapabilityType,
) -> Result<Option<axum::Json<serde_json::Value>>, ApiError> {
    // Only proceed if refresh=force is requested
    if params.refresh != Some(RefreshStrategy::Force) {
        return Ok(None);
    }

    tracing::debug!(
        "Force refresh requested for server '{}', attempting temporary instance creation for {:?}",
        server_info.server_name,
        capability_type
    );

    // Create temporary instance with timeout
    let pool_result = tokio::time::timeout(std::time::Duration::from_secs(10), state.connection_pool.lock()).await;

    let mut pool = match pool_result {
        Ok(pool) => pool,
        Err(_) => {
            tracing::error!(
                "Timeout acquiring connection pool lock for server '{}'",
                server_info.server_name
            );
            return Ok(None);
        }
    };

    match pool
        .get_or_create_validation_instance(&server_info.server_name, "api", std::time::Duration::from_secs(5 * 60))
        .await
    {
        Ok(Some(validation_conn)) => {
            tracing::debug!("Created temporary instance for server '{}'", server_info.server_name);

            // Extract capabilities based on the requested type
            let extracted = match capability_type {
                CapabilityType::Tools => extract_tools_capability(validation_conn).await?,
                CapabilityType::Prompts => extract_prompts_capability(validation_conn).await?,
                CapabilityType::Resources => extract_resources_capability(validation_conn).await?,
                CapabilityType::ResourceTemplates => extract_resource_templates_capability(validation_conn).await?,
            };

            // Update cache with fresh data (only if we have data)
            if !extracted.data.is_empty() {
                let server_data = crate::core::cache::CachedServerData {
                    server_id: server_info.server_id.clone(),
                    server_name: server_info.server_name.clone(),
                    server_version: None,
                    protocol_version: "latest".to_string(),
                    tools: extracted.tools,
                    resources: extracted.resources,
                    prompts: extracted.prompts,
                    resource_templates: extracted.resource_templates,
                    cached_at: chrono::Utc::now(),
                    fingerprint: format!("temp:{}:{}", server_info.server_id, chrono::Utc::now().timestamp()),
                };
                let _ = state.redb_cache.store_server_data(&server_data).await;
            }

            // Destroy temporary instance (create-use-destroy lifecycle)
            let _ = pool.destroy_validation_instance(&server_info.server_name, "api").await;

            tracing::debug!("Destroyed temporary instance for server '{}'", server_info.server_name);

            // Return formatted response
            Ok(Some(axum::Json(serde_json::json!({
                "data": extracted.data,
                "meta": {
                    "cache_hit": false,
                    "strategy": params.refresh.unwrap_or_default(),
                    "source": "temporary"
                }
            }))))
        }
        Ok(None) => {
            tracing::warn!(
                "Failed to create temporary instance for server '{}' - returned None",
                server_info.server_name
            );
            Ok(None)
        }
        Err(e) => {
            tracing::error!(
                "Error creating temporary instance for server '{}': {:?}",
                server_info.server_name,
                e
            );
            Ok(None)
        }
    }
}

/// Validate server ID format
///
/// Ensures server ID follows expected format patterns
pub fn validate_server_id(server_id: &str) -> Result<(), ApiError> {
    if server_id.trim().is_empty() {
        return Err(ApiError::BadRequest("Server ID cannot be empty".to_string()));
    }

    // Allow both generated IDs (serv_*) and custom names
    // This is flexible to support different ID formats
    if server_id.len() > 255 {
        return Err(ApiError::BadRequest(
            "Server ID too long (max 255 characters)".to_string(),
        ));
    }

    Ok(())
}

/// Handle inspect service errors with appropriate HTTP status codes
///
/// Converts inspect service errors to appropriate API errors
pub fn handle_inspect_error(error: String) -> ApiError {
    ApiError::InternalError(error)
}

/// Create a CachedServerData structure for storing runtime data
///
/// This helper function creates a standardized CachedServerData structure
/// for storing capability data in the cache.
pub fn create_runtime_cache_data(
    server_info: &ServerIdentification,
    tools: Vec<crate::core::cache::CachedToolInfo>,
    resources: Vec<crate::core::cache::CachedResourceInfo>,
    prompts: Vec<crate::core::cache::CachedPromptInfo>,
    resource_templates: Vec<crate::core::cache::CachedResourceTemplateInfo>,
) -> crate::core::cache::CachedServerData {
    crate::core::cache::CachedServerData {
        server_id: server_info.server_id.clone(),
        server_name: server_info.server_name.clone(),
        server_version: None,
        protocol_version: "latest".to_string(),
        tools,
        resources,
        prompts,
        resource_templates,
        cached_at: chrono::Utc::now(),
        fingerprint: format!("runtime:{}:{}", server_info.server_id, chrono::Utc::now().timestamp()),
    }
}

/// Create a standardized JSON response for inspect endpoints
///
/// This helper function creates a consistent response format for all inspect endpoints.
pub fn create_inspect_response(
    data: Vec<serde_json::Value>,
    cache_hit: bool,
    refresh_strategy: Option<RefreshStrategy>,
    source: &str,
) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "data": data,
        "meta": {
            "cache_hit": cache_hit,
            "strategy": refresh_strategy.unwrap_or_default(),
            "source": source
        }
    }))
}

/// Build tool JSON object (shared across handlers)
pub fn tool_json(
    name: &str,
    description: Option<String>,
    input_schema: serde_json::Value,
    unique_name: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "input_schema": input_schema,
        "unique_name": unique_name,
    })
}

/// Build tool JSON from cached model
pub fn tool_json_from_cached(t: &crate::core::cache::CachedToolInfo) -> serde_json::Value {
    let schema = t.input_schema().unwrap_or_else(|_| serde_json::json!({}));
    tool_json(&t.name, t.description.clone(), schema, t.unique_name.clone())
}

/// Build resource JSON object (shared across handlers)
pub fn resource_json(
    uri: &str,
    name: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "uri": uri,
        "name": name,
        "description": description,
        "mime_type": mime_type,
    })
}

/// Build resource JSON from cached model
pub fn resource_json_from_cached(r: crate::core::cache::CachedResourceInfo) -> serde_json::Value {
    resource_json(&r.uri, r.name, r.description, r.mime_type)
}

/// Build resource template JSON object (shared across handlers)
pub fn resource_template_json(
    uri_template: &str,
    name: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "uri_template": uri_template,
        "name": name,
        "description": description,
        "mime_type": mime_type,
    })
}

/// Build resource template JSON from cached model
pub fn resource_template_json_from_cached(t: crate::core::cache::CachedResourceTemplateInfo) -> serde_json::Value {
    resource_template_json(&t.uri_template, t.name, t.description, t.mime_type)
}

/// Build prompt JSON object (shared across handlers)
pub fn prompt_json(
    name: &str,
    description: Option<String>,
    arguments: Vec<crate::core::cache::PromptArgument>,
) -> serde_json::Value {
    use crate::core::cache::PromptArgument;
    let args: Vec<serde_json::Value> = arguments
        .into_iter()
        .map(|a: PromptArgument| {
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
    })
}

/// Build prompt JSON from cached model
pub fn prompt_json_from_cached(p: crate::core::cache::CachedPromptInfo) -> serde_json::Value {
    prompt_json(&p.name, p.description, p.arguments)
}
