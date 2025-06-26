// Discovery resources handlers
// Provides handlers for server resource discovery endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    discovery::{ProcessedResourceInfo, ProcessedResourceTemplateInfo},
};

use super::{
    DiscoveryQuery, DiscoveryResponse, create_response, get_discovery_service,
    handle_discovery_error, validate_server_id,
};

/// List all resources for a specific server
///
/// Returns a list of resources available on the specified server with
/// configurable filtering and formatting options.
pub async fn server_resources(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<Vec<ProcessedResourceInfo>>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server resources
    let resources = discovery_service
        .get_server_resources(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Create response with metadata
    let response = create_response(
        resources, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved {} resources for server '{}' with strategy {:?}",
        response.data.len(),
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// List enabled resources for a specific server
///
/// Returns only the resources that are currently enabled in the configuration.
/// This is useful for getting the active resource set without disabled resources.
pub async fn enabled_server_resources(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<Vec<ProcessedResourceInfo>>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server resources
    let all_resources = discovery_service
        .get_server_resources(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Filter to only enabled resources
    let enabled_resources: Vec<ProcessedResourceInfo> = all_resources
        .into_iter()
        .filter(|resource| resource.enabled)
        .collect();

    // Create response with metadata
    let response = create_response(
        enabled_resources,
        &server_id,
        &params,
        None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved {} enabled resources for server '{}' with strategy {:?}",
        response.data.len(),
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// List resource templates for a specific server
///
/// Returns resource templates that define URI patterns for dynamic resources.
/// Templates are used to generate resource URIs with parameters.
pub async fn server_resource_templates(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<Vec<ProcessedResourceTemplateInfo>>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server resource templates
    let templates = discovery_service
        .get_server_resource_templates(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Create response with metadata
    let response = create_response(
        templates, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved {} resource templates for server '{}' with strategy {:?}",
        response.data.len(),
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// Get resource summary for a specific server
///
/// Returns a summary of resource availability including counts and types.
pub async fn get_server_resource_summary(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<ResourceSummary>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server resources and templates
    let resources = discovery_service
        .get_server_resources(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    let templates = discovery_service
        .get_server_resource_templates(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Analyze MIME types
    let mut mime_types = std::collections::HashMap::new();
    for resource in &resources {
        if let Some(mime_type) = &resource.mime_type {
            *mime_types.entry(mime_type.clone()).or_insert(0) += 1;
        }
    }

    // Create summary
    let summary = ResourceSummary {
        server_id: server_id.clone(),
        total_resources: resources.len(),
        enabled_resources: resources.iter().filter(|r| r.enabled).count(),
        total_templates: templates.len(),
        mime_types,
        has_dynamic_resources: !templates.is_empty(),
    };

    // Create response with metadata
    let response = create_response(
        summary, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::debug!(
        "Retrieved resource summary for server '{}': {} resources, {} templates",
        server_id,
        resources.len(),
        templates.len()
    );

    Ok(Json(response))
}

/// Resource summary structure
#[derive(Debug, serde::Serialize)]
pub struct ResourceSummary {
    /// Server identifier
    pub server_id: String,
    /// Total number of resources
    pub total_resources: usize,
    /// Number of enabled resources
    pub enabled_resources: usize,
    /// Total number of resource templates
    pub total_templates: usize,
    /// MIME type distribution
    pub mime_types: std::collections::HashMap<String, usize>,
    /// Whether server supports dynamic resources
    pub has_dynamic_resources: bool,
}
