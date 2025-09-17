// Common utilities for server API handlers
// Provides shared functions for server identification, validation, and response formatting

use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, sync::Arc};

use crate::{
    api::{
        handlers::ApiError,
        models::server::{InstanceSummary, ServerMetaInfo},
        routes::AppState,
    },
    config::{database::Database, server},
    core::cache::{CacheQuery, FreshnessLevel},
};
use axum::http::StatusCode;

/// Connection pool access manager
///
/// Provides standardized access to the connection pool with timeout handling,
/// following the project's Manager pattern and error handling conventions.
pub struct ConnectionPoolManager;

impl ConnectionPoolManager {
    /// Get connection pool with timeout and proper error handling
    ///
    /// This method provides a standardized way to access the connection pool with:
    /// - Configurable timeout based on operation type
    /// - Consistent error messages using the unified error handling system
    /// - Proper logging and monitoring
    /// - Integration with the project's error handling patterns
    pub async fn get_pool_with_timeout<'a>(
        state: &'a Arc<AppState>,
        timeout_secs: u64,
        operation_context: &str,
    ) -> Result<tokio::sync::MutexGuard<'a, crate::core::pool::UpstreamConnectionPool>, ApiError> {
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            state.connection_pool.lock(),
        )
        .await
        {
            Ok(pool) => {
                tracing::debug!("Successfully acquired connection pool lock for {}", operation_context);
                Ok(pool)
            }
            Err(_) => {
                tracing::warn!(
                    "Connection pool lock timeout for {} ({}s timeout)",
                    operation_context,
                    timeout_secs
                );
                Err(crate::api::handlers::common::errors::internal_error(&format!(
                    "Connection pool access timeout for {} ({}s)",
                    operation_context, timeout_secs
                )))
            }
        }
    }

    /// Get connection pool for API operations (5s timeout)
    pub async fn get_pool_for_api<'a>(
        state: &'a Arc<AppState>
    ) -> Result<tokio::sync::MutexGuard<'a, crate::core::pool::UpstreamConnectionPool>, ApiError> {
        Self::get_pool_with_timeout(state, 5, "API operation").await
    }

    /// Get connection pool for health checks (1s timeout)
    pub async fn get_pool_for_health_check<'a>(
        state: &'a Arc<AppState>
    ) -> Result<tokio::sync::MutexGuard<'a, crate::core::pool::UpstreamConnectionPool>, ApiError> {
        Self::get_pool_with_timeout(state, 1, "health check").await
    }

    /// Get connection pool for capability operations (10s timeout)
    pub async fn get_pool_for_capability<'a>(
        state: &'a Arc<AppState>
    ) -> Result<tokio::sync::MutexGuard<'a, crate::core::pool::UpstreamConnectionPool>, ApiError> {
        Self::get_pool_with_timeout(state, 10, "capability operation").await
    }

    // Removed heavy snapshot getter; prefer pool.get_server_status_summary() or get_snapshot() in read paths
}

/// Server identification result
#[derive(Debug, Clone)]
pub struct ServerIdentification {
    /// Server ID (guaranteed to exist)
    pub server_id: String,
    /// Server name (guaranteed to exist)
    pub server_name: String,
}

/// Helper function to create ServerIdentification from a server record
#[inline]
fn create_server_identification(server: crate::config::models::Server) -> Result<ServerIdentification, ApiError> {
    let server_id = server
        .id
        .ok_or_else(|| ApiError::InternalError("Server found but missing ID".to_string()))?;

    Ok(ServerIdentification {
        server_id,
        server_name: server.name,
    })
}

/// Complete server details for response building
#[derive(Debug, Default)]
pub struct ServerDetails {
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub meta: Option<ServerMetaInfo>,
    pub globally_enabled: bool,
    pub enabled_in_profile: bool,
    pub instances: Vec<InstanceSummary>,
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
        return create_server_identification(server);
    }

    // Try to find server by name
    if let Some(server) = server::get_server(pool, identifier)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {e}")))?
    {
        return create_server_identification(server);
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
            Ok(server_args) if !server_args.is_empty() => {
                let mut sorted_args: Vec<_> = server_args.into_iter().collect();
                sorted_args.sort_by_key(|arg| arg.arg_index);
                details.args = Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect());
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("Failed to get arguments for server '{}': {}", server_name, e),
        }
    }

    // Get server environment variables
    if !server_id.is_empty() {
        match server::get_server_env(pool, server_id).await {
            Ok(env_map) if !env_map.is_empty() => details.env = Some(env_map),
            Ok(_) => {}
            Err(e) => tracing::warn!(
                "Failed to get environment variables for server '{}': {}",
                server_name,
                e
            ),
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
            Err(e) => tracing::warn!("Failed to get metadata for server '{}': {}", server_name, e),
        }
    }

    // Get server global enabled status
    details.globally_enabled = server::get_server_global_status(pool, server_id)
        .await
        .ok()
        .flatten()
        .unwrap_or(true);

    // Get server enabled status in profile
    details.enabled_in_profile = server::is_server_enabled_in_any_profile(pool, server_id)
        .await
        .unwrap_or(false);

    // Get instance information from connection pool
    details.instances = get_server_instances(state, server_id).await;

    details
}

/// Get server instances with timeout protection
///
/// Consolidates the connection pool access and instance retrieval logic
/// that was duplicated across multiple handlers.
pub async fn get_server_instances(
    state: &Arc<AppState>,
    server_id: &str,
) -> Vec<InstanceSummary> {
    let pool = match ConnectionPoolManager::get_pool_for_api(state).await {
        Ok(pool) => pool,
        Err(_) => {
            tracing::warn!("Failed to get connection pool for server '{}'", server_id);
            return Vec::new();
        }
    };

    let instances = match pool.connections.get(server_id) {
        Some(instances) => instances,
        None => return Vec::new(),
    };

    let now = std::time::SystemTime::now();
    instances
        .iter()
        .map(|(id, conn)| {
            let connected_at = if conn.is_connected() {
                Some(chrono::DateTime::<chrono::Utc>::from(now - conn.time_since_last_connection()).to_rfc3339())
            } else {
                None
            };

            let started_at = chrono::DateTime::<chrono::Utc>::from(now - conn.time_since_creation()).to_rfc3339();

            InstanceSummary {
                id: id.clone(),
                status: conn.status_string(),
                started_at: Some(started_at),
                connected_at,
            }
        })
        .collect()
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

/// Get server by ID or return error (unified interface)
///
/// Replaces profile/helpers.rs::get_server_or_error with standardized interface.
/// This function provides a clean, consistent way to get a server by ID.
pub async fn get_server_or_error(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<crate::config::models::Server, ApiError> {
    let server = server::get_server_by_id(pool, server_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    server.ok_or_else(|| ApiError::NotFound(format!("Server with ID '{server_id}' not found")))
}

/// Get server info by ID (unified interface)
///
/// Replaces server/mgmt.rs::get_server_info_by_id with standardized interface.
/// Returns (server_id, server_name) tuple for lightweight operations.
pub async fn get_server_info_by_id(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<(String, String), ApiError> {
    let server = get_server_or_error(pool, server_id).await?;

    let actual_server_id = server
        .id
        .ok_or_else(|| ApiError::InternalError("Server found but missing ID".to_string()))?;

    Ok((actual_server_id, server.name))
}

/// Get server info for inspect endpoints (unified interface)
///
/// Replaces the repeated validation pattern in resources.rs, prompts.rs, and tools.rs.
/// This function consolidates database access, server validation, and info creation.
pub async fn get_server_info_for_inspect(
    app_state: &Arc<AppState>,
    server_id: &str,
    query: &InspectQuery,
) -> Result<
    (
        Arc<crate::config::database::Database>,
        ServerIdentification,
        InspectParams,
    ),
    StatusCode,
> {
    // Get database from app state
    let db = get_database_from_state(app_state).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get server by ID
    let server_row = crate::config::server::get_server_by_id(&db.pool, server_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Create server identification
    let server_info = ServerIdentification {
        server_id: server_id.to_string(),
        server_name: server_row.name.clone(),
    };

    // Validate server ID format
    validate_server_id(&server_info.server_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse query parameters
    let params = query.to_params().map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok((db, server_info, params))
}

/// Check server capability or return error (unified interface)
///
/// Replaces the repeated capability checking pattern in resources.rs, prompts.rs, and tools.rs.
/// Returns Some(response) if the server doesn't support the capability, None if it does support it.
pub async fn check_capability_or_error(
    pool: &Pool<Sqlite>,
    server_info: &ServerIdentification,
    capability: crate::common::capability::CapabilityToken,
    params: &InspectParams,
) -> Option<axum::Json<serde_json::Value>> {
    if let Ok((server_row, _id)) = get_server_by_identifier(pool, &server_info.server_name).await {
        if server_row.capabilities.is_some() && !server_row.has_capability(capability) {
            let strategy = format!("capability-{}-unsupported", capability.as_str().to_lowercase());
            return Some(create_inspect_response(Vec::new(), false, params.refresh, &strategy));
        }
    }
    None
}

/// Get database from application state
///
/// Helper function to extract database connection from AppState
/// with proper error handling.
#[inline]
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
            refresh: Some(self.refresh.unwrap_or_default()),
            format: self.format.clone(),
            include_meta: self.include_meta,
            timeout: self.timeout,
        })
    }
}

/// Map inspect RefreshStrategy to cache FreshnessLevel for Redb
#[inline]
pub fn map_refresh_to_freshness(refresh: RefreshStrategy) -> FreshnessLevel {
    match refresh {
        RefreshStrategy::CacheFirst => FreshnessLevel::Cached,
        RefreshStrategy::RefreshIfStale => FreshnessLevel::RecentlyFresh,
        RefreshStrategy::Force => FreshnessLevel::RealTime,
    }
}

/// Validate a cached snapshot for being non-empty
#[inline]
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
        server_id: server_id.to_owned(),
        freshness_level,
        include_disabled: false,
    }
}

/// Validate server ID format
///
/// Ensures server ID follows expected format patterns
#[inline]
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
#[inline]
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
    let now = chrono::Utc::now();
    crate::core::cache::CachedServerData {
        server_id: server_info.server_id.clone(),
        server_name: server_info.server_name.clone(),
        server_version: None,
        protocol_version: "latest".to_owned(),
        tools,
        resources,
        prompts,
        resource_templates,
        cached_at: now,
        fingerprint: format!("runtime:{}:{}", server_info.server_id, now.timestamp()),
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

/// Try to enrich capability items with database mappings, fallback to plain response if DB unavailable
#[inline]
pub async fn try_enrich_or_fallback(
    state: &Arc<AppState>,
    capability_type: super::capability::CapabilityType,
    server_id: &str,
    processed: Vec<serde_json::Value>,
    cache_hit: bool,
    refresh: Option<RefreshStrategy>,
    strategy: &str,
) -> Result<axum::Json<serde_json::Value>, ApiError> {
    if let Ok(db) = get_database_from_state(state) {
        let enriched =
            super::capability::enrich_capability_items(capability_type, &db.pool, server_id, processed).await;
        Ok(super::capability::respond_with_enriched(
            enriched, cache_hit, refresh, strategy,
        ))
    } else {
        Ok(create_inspect_response(processed, cache_hit, refresh, strategy))
    }
}
