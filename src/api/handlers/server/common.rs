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
    // Determine high-level state for client simplicity
    let state = if source.starts_with("capability-") {
        "unsupported"
    } else {
        "ok"
    };
    axum::Json(serde_json::json!({
        "data": data,
        "state": state,
        "meta": {
            "cache_hit": cache_hit,
            "strategy": refresh_strategy.unwrap_or_default(),
            "source": source
        }
    }))
}

