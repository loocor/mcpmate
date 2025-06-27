// Server resources handlers
// Provides handlers for server resource inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    inspect::{InspectParams, ProcessedResourceInfo, ProcessedResourceTemplateInfo},
};

use super::common::{
    create_inspect_response, get_database_from_state, get_inspect_service,
    handle_inspect_error, resolve_server_identifier, validate_server_id,
};

/// Query parameters for resources endpoints
#[derive(Debug, serde::Deserialize)]
pub struct ResourcesQuery {
    /// Refresh strategy for resource queries
    pub refresh: Option<crate::inspect::RefreshStrategy>,
    /// Response format
    pub format: Option<crate::inspect::ResponseFormat>,
    /// Whether to include metadata
    pub include_meta: Option<bool>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
}

impl ResourcesQuery {
    /// Convert to InspectParams
    pub fn to_params(&self) -> Result<InspectParams, ApiError> {
        Ok(InspectParams {
            refresh: self.refresh,
            format: self.format,
            include_meta: self.include_meta,
        })
    }
}

/// List all resources for a specific server
///
/// Returns a list of resources available on the specified server with
/// configurable filtering and formatting options.
///
/// Supports both server_name and server_id as identifier.
pub async fn list_resources(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<ResourcesQuery>,
) -> Result<Json<crate::inspect::InspectResponse<Vec<ProcessedResourceInfo>>>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get inspect service
    let inspect_service = get_inspect_service(&state).await?;

    // Add timeout control
    let timeout = std::time::Duration::from_secs(query.timeout.unwrap_or(30));
    let resources_result = tokio::time::timeout(timeout, async {
        inspect_service
            .get_server_resources(&server_info.server_id, params.clone())
            .await
    })
    .await;

    let resources = match resources_result {
        Ok(result) => result.map_err(handle_inspect_error)?,
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Resources request for server '{}' timed out after {}s",
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Create response with metadata
    let response = create_inspect_response(
        resources,
        &params,
        Some(false), // No direct caching for this endpoint
        None,        // No capabilities metadata for this endpoint
    );

    tracing::info!(
        "Retrieved {} resources for server '{}' (ID: {}) with strategy {:?}",
        response.data.len(),
        server_info.server_name,
        server_info.server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// List resource templates for a specific server
///
/// Returns resource templates that define URI patterns for dynamic resources.
/// Templates are used to generate resource URIs with parameters.
///
/// Supports both server_name and server_id as identifier.
pub async fn list_resource_templates(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<ResourcesQuery>,
) -> Result<Json<crate::inspect::InspectResponse<Vec<ProcessedResourceTemplateInfo>>>, ApiError>
{
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get inspect service
    let inspect_service = get_inspect_service(&state).await?;

    // Add timeout control
    let timeout = std::time::Duration::from_secs(query.timeout.unwrap_or(30));
    let templates_result = tokio::time::timeout(timeout, async {
        inspect_service
            .get_server_resource_templates(&server_info.server_id, params.clone())
            .await
    })
    .await;

    let templates = match templates_result {
        Ok(result) => result.map_err(handle_inspect_error)?,
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Resource templates request for server '{}' timed out after {}s",
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Create response with metadata
    let response = create_inspect_response(
        templates,
        &params,
        Some(false), // No direct caching for this endpoint
        None,        // No capabilities metadata for this endpoint
    );

    tracing::info!(
        "Retrieved {} resource templates for server '{}' (ID: {}) with strategy {:?}",
        response.data.len(),
        server_info.server_name,
        server_info.server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}
