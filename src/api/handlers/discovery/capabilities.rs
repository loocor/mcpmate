// Discovery capabilities handlers
// Provides handlers for server capability overview endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    discovery::{ProcessedCapabilities, ServerCapabilities},
};

use super::{
    DiscoveryQuery, DiscoveryResponse, create_response, get_discovery_service,
    handle_discovery_error, validate_server_id,
};

/// Get server capabilities overview
///
/// Returns complete capability information for a specific server including
/// tools, resources, prompts, and resource templates with configurable
/// refresh strategy and response format.
pub async fn server_capabilities(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<ProcessedCapabilities>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get processed capabilities
    let capabilities = discovery_service
        .get_processed_capabilities(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Create response with metadata
    let response = create_response(
        capabilities,
        &server_id,
        &params,
        None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved capabilities for server '{}' with strategy {:?}",
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// Get raw server capabilities (internal format)
///
/// This endpoint returns the raw ServerCapabilities structure without
/// processing or filtering. Mainly used for debugging and internal tools.
pub async fn get_raw_server_capabilities(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<ServerCapabilities>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get raw capabilities
    let capabilities = discovery_service
        .get_server_capabilities(&server_id, params.refresh.unwrap_or_default())
        .await
        .map_err(handle_discovery_error)?;

    // Create response with metadata
    let response = create_response(
        capabilities,
        &server_id,
        &params,
        None, // TODO: Add cache hit detection
    );

    tracing::debug!(
        "Retrieved raw capabilities for server '{}' with strategy {:?}",
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// Get server capability summary
///
/// Returns a compact summary of server capabilities including counts
/// of available tools, resources, and prompts.
pub async fn get_server_capability_summary(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<CapabilitySummary>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get processed capabilities
    let capabilities = discovery_service
        .get_processed_capabilities(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Create summary
    let summary = CapabilitySummary {
        server_id: capabilities.server_id.clone(),
        server_name: capabilities.server_name.clone(),
        total_tools: capabilities.tools.len(),
        enabled_tools: capabilities.tools.iter().filter(|t| t.enabled).count(),
        total_resources: capabilities.resources.len(),
        enabled_resources: capabilities.resources.iter().filter(|r| r.enabled).count(),
        total_prompts: capabilities.prompts.len(),
        enabled_prompts: capabilities.prompts.iter().filter(|p| p.enabled).count(),
        total_resource_templates: capabilities.resource_templates.len(),
        last_updated: capabilities.metadata.last_updated,
        cache_ttl: capabilities.metadata.ttl,
    };

    // Create response with metadata
    let response = create_response(
        summary, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::debug!(
        "Retrieved capability summary for server '{}': {} tools, {} resources, {} prompts",
        server_id,
        capabilities.tools.len(),
        capabilities.resources.len(),
        capabilities.prompts.len()
    );

    Ok(Json(response))
}

/// Capability summary structure
#[derive(Debug, serde::Serialize)]
pub struct CapabilitySummary {
    /// Server identifier
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Total number of tools
    pub total_tools: usize,
    /// Number of enabled tools
    pub enabled_tools: usize,
    /// Total number of resources
    pub total_resources: usize,
    /// Number of enabled resources
    pub enabled_resources: usize,
    /// Total number of prompts
    pub total_prompts: usize,
    /// Number of enabled prompts
    pub enabled_prompts: usize,
    /// Total number of resource templates
    pub total_resource_templates: usize,
    /// Last updated timestamp
    pub last_updated: std::time::SystemTime,
    /// Cache TTL
    pub cache_ttl: std::time::Duration,
}
