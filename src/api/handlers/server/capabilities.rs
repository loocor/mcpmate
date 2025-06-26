// Server capabilities handlers
// Provides handlers for server capability overview endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    discovery::{DiscoveryParams, ProcessedCapabilities, ServerCapabilities},
};

use super::common::{
    create_discovery_response, get_database_from_state, get_discovery_service,
    handle_discovery_error, resolve_server_identifier, validate_server_id,
};

/// Query parameters for capabilities endpoints
#[derive(Debug, serde::Deserialize)]
pub struct CapabilitiesQuery {
    /// Refresh strategy for capability queries
    pub refresh: Option<crate::discovery::RefreshStrategy>,
    /// Response format
    pub format: Option<crate::discovery::ResponseFormat>,
    /// Whether to include metadata
    pub include_meta: Option<bool>,
    /// Timeout in seconds (new)
    pub timeout: Option<u64>,
}

impl CapabilitiesQuery {
    /// Convert to DiscoveryParams
    pub fn to_params(&self) -> Result<DiscoveryParams, ApiError> {
        Ok(DiscoveryParams {
            refresh: self.refresh,
            format: self.format,
            include_meta: self.include_meta,
        })
    }
}

/// Get server capabilities overview
///
/// Returns complete capability information for a specific server including
/// tools, resources, prompts, and resource templates with configurable
/// refresh strategy and response format.
///
/// Supports both server_name and server_id as identifier.
pub async fn get_capabilities(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<CapabilitiesQuery>,
) -> Result<Json<crate::discovery::DiscoveryResponse<ProcessedCapabilities>>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Add timeout control for long-running operations
    let timeout = std::time::Duration::from_secs(query.timeout.unwrap_or(30));
    let capabilities_result = tokio::time::timeout(timeout, async {
        discovery_service
            .get_processed_capabilities(&server_info.server_id, params.clone())
            .await
    })
    .await;

    let (capabilities, cache_hit, metadata) = match capabilities_result {
        Ok(result) => {
            let processed = result.map_err(handle_discovery_error)?;
            // Get cache info for this request
            let cache_result = discovery_service
                .get_server_capabilities_with_cache_info(
                    &server_info.server_id,
                    params.refresh.unwrap_or_default(),
                )
                .await
                .map_err(handle_discovery_error)?;
            (
                processed,
                cache_result.cache_hit,
                cache_result.capabilities.metadata,
            )
        }
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Capabilities request for server '{}' timed out after {}s",
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Create response with metadata
    let response =
        create_discovery_response(capabilities, &params, Some(cache_hit), Some(&metadata));

    tracing::info!(
        "Retrieved capabilities for server '{}' (ID: {}) with strategy {:?}, cache_hit: {}",
        server_info.server_name,
        server_info.server_id,
        params.refresh.unwrap_or_default(),
        cache_hit
    );

    Ok(Json(response))
}

/// Get raw server capabilities (internal format)
///
/// This endpoint returns the raw ServerCapabilities structure without
/// processing or filtering. Mainly used for debugging and internal tools.
///
/// Supports both server_name and server_id as identifier.
pub async fn get_raw_capabilities(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<CapabilitiesQuery>,
) -> Result<Json<crate::discovery::DiscoveryResponse<ServerCapabilities>>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Add timeout control
    let timeout = std::time::Duration::from_secs(query.timeout.unwrap_or(30));
    let capabilities_result = tokio::time::timeout(timeout, async {
        discovery_service
            .get_server_capabilities(&server_info.server_id, params.refresh.unwrap_or_default())
            .await
    })
    .await;

    let (capabilities, cache_hit) = match capabilities_result {
        Ok(result) => {
            // Get cache info for this request
            let cache_result = discovery_service
                .get_server_capabilities_with_cache_info(
                    &server_info.server_id,
                    params.refresh.unwrap_or_default(),
                )
                .await
                .map_err(handle_discovery_error)?;
            (
                result.map_err(handle_discovery_error)?,
                cache_result.cache_hit,
            )
        }
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Raw capabilities request for server '{}' timed out after {}s",
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Create response with metadata
    let metadata_clone = capabilities.metadata.clone();
    let response = create_discovery_response(
        capabilities,
        &params,
        Some(cache_hit),
        Some(&metadata_clone),
    );

    tracing::debug!(
        "Retrieved raw capabilities for server '{}' (ID: {}) with strategy {:?}, cache_hit: {}",
        server_info.server_name,
        server_info.server_id,
        params.refresh.unwrap_or_default(),
        cache_hit
    );

    Ok(Json(response))
}
