// Server prompts handlers
// Provides handlers for server prompt discovery endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    discovery::{DiscoveryParams, ProcessedPromptInfo, types::PromptArgument},
};

use super::common::{
    create_discovery_response, get_database_from_state, get_discovery_service,
    handle_discovery_error, resolve_server_identifier, validate_server_id,
};

/// Query parameters for prompts endpoints
#[derive(Debug, serde::Deserialize)]
pub struct PromptsQuery {
    /// Refresh strategy for prompt queries
    pub refresh: Option<crate::discovery::RefreshStrategy>,
    /// Response format
    pub format: Option<crate::discovery::ResponseFormat>,
    /// Whether to include metadata
    pub include_meta: Option<bool>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
}

impl PromptsQuery {
    /// Convert to DiscoveryParams
    pub fn to_params(&self) -> Result<DiscoveryParams, ApiError> {
        Ok(DiscoveryParams {
            refresh: self.refresh,
            format: self.format,
            include_meta: self.include_meta,
        })
    }
}

/// List all prompts for a specific server
///
/// Returns a list of prompts available on the specified server with
/// configurable filtering and formatting options.
///
/// Supports both server_name and server_id as identifier.
pub async fn list_prompts(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<PromptsQuery>,
) -> Result<Json<crate::discovery::DiscoveryResponse<Vec<ProcessedPromptInfo>>>, ApiError> {
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
    let prompts_result = tokio::time::timeout(timeout, async {
        discovery_service
            .get_server_prompts(&server_info.server_id, params.clone())
            .await
    })
    .await;

    let prompts = match prompts_result {
        Ok(result) => result.map_err(handle_discovery_error)?,
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Prompts request for server '{}' timed out after {}s",
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Create response with metadata
    let response = create_discovery_response(
        prompts,
        &params,
        Some(false), // No direct caching for this endpoint
        None,        // No capabilities metadata for this endpoint
    );

    tracing::info!(
        "Retrieved {} prompts for server '{}' (ID: {}) with strategy {:?}",
        response.data.len(),
        server_info.server_name,
        server_info.server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// Get detailed prompt argument information
///
/// Returns detailed information about prompt arguments for form generation
/// and validation purposes.
///
/// Supports both server_name and server_id as identifier.
pub async fn get_prompt_arguments(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<PromptsQuery>,
) -> Result<Json<crate::discovery::DiscoveryResponse<Vec<PromptArgumentInfo>>>, ApiError> {
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
    let prompts_result = tokio::time::timeout(timeout, async {
        discovery_service
            .get_server_prompts(&server_info.server_id, params.clone())
            .await
    })
    .await;

    let prompts = match prompts_result {
        Ok(result) => result.map_err(handle_discovery_error)?,
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Prompt arguments request for server '{}' timed out after {}s",
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Extract argument information
    let mut argument_info = Vec::new();
    for prompt in prompts {
        if !prompt.arguments.is_empty() {
            argument_info.push(PromptArgumentInfo {
                prompt_name: prompt.name,
                prompt_description: prompt.description,
                arguments: prompt.arguments,
            });
        }
    }

    // Create response with metadata
    let response = create_discovery_response(
        argument_info,
        &params,
        Some(false), // No direct caching for this endpoint
        None,        // No capabilities metadata for this endpoint
    );

    tracing::debug!(
        "Retrieved prompt argument information for server '{}' (ID: {}): {} prompts with arguments",
        server_info.server_name,
        server_info.server_id,
        response.data.len()
    );

    Ok(Json(response))
}

/// Prompt argument information structure
#[derive(Debug, serde::Serialize)]
pub struct PromptArgumentInfo {
    /// Prompt name
    pub prompt_name: String,
    /// Prompt description
    pub prompt_description: Option<String>,
    /// Prompt arguments
    pub arguments: Vec<PromptArgument>,
}
