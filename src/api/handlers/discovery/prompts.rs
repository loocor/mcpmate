// Discovery prompts handlers
// Provides handlers for server prompt discovery endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    discovery::{ProcessedPromptInfo, types::PromptArgument},
};

use super::{
    DiscoveryQuery, DiscoveryResponse, create_response, get_discovery_service,
    handle_discovery_error, validate_server_id,
};

/// List all prompts for a specific server
///
/// Returns a list of prompts available on the specified server with
/// configurable filtering and formatting options.
pub async fn server_prompts(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<Vec<ProcessedPromptInfo>>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server prompts
    let prompts = discovery_service
        .get_server_prompts(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Create response with metadata
    let response = create_response(
        prompts, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved {} prompts for server '{}' with strategy {:?}",
        response.data.len(),
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// List enabled prompts for a specific server
///
/// Returns only the prompts that are currently enabled in the configuration.
/// This is useful for getting the active prompt set without disabled prompts.
pub async fn enabled_server_prompts(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<Vec<ProcessedPromptInfo>>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server prompts
    let all_prompts = discovery_service
        .get_server_prompts(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Filter to only enabled prompts
    let enabled_prompts: Vec<ProcessedPromptInfo> = all_prompts
        .into_iter()
        .filter(|prompt| prompt.enabled)
        .collect();

    // Create response with metadata
    let response = create_response(
        enabled_prompts,
        &server_id,
        &params,
        None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved {} enabled prompts for server '{}' with strategy {:?}",
        response.data.len(),
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// Get prompt summary for a specific server
///
/// Returns a summary of prompt availability including counts and argument analysis.
pub async fn get_server_prompt_summary(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<PromptSummary>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server prompts
    let prompts = discovery_service
        .get_server_prompts(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Analyze prompt arguments
    let mut total_arguments = 0;
    let mut required_arguments = 0;
    let mut prompts_with_args = 0;

    for prompt in &prompts {
        if !prompt.arguments.is_empty() {
            prompts_with_args += 1;
        }
        total_arguments += prompt.arguments.len();
        required_arguments += prompt.arguments.iter().filter(|arg| arg.required).count();
    }

    // Create summary
    let summary = PromptSummary {
        server_id: server_id.clone(),
        total_prompts: prompts.len(),
        enabled_prompts: prompts.iter().filter(|p| p.enabled).count(),
        prompts_with_arguments: prompts_with_args,
        total_arguments,
        required_arguments,
        has_complex_prompts: prompts_with_args > 0,
    };

    // Create response with metadata
    let response = create_response(
        summary, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::debug!(
        "Retrieved prompt summary for server '{}': {} prompts, {} with arguments",
        server_id,
        prompts.len(),
        prompts_with_args
    );

    Ok(Json(response))
}

/// Get detailed prompt argument information
///
/// Returns detailed information about prompt arguments for form generation
/// and validation purposes.
pub async fn get_server_prompt_arguments(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<Vec<PromptArgumentInfo>>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server prompts
    let prompts = discovery_service
        .get_server_prompts(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Extract argument information
    let mut argument_info = Vec::new();
    for prompt in prompts {
        if prompt.enabled && !prompt.arguments.is_empty() {
            argument_info.push(PromptArgumentInfo {
                prompt_name: prompt.name,
                prompt_description: prompt.description,
                arguments: prompt.arguments,
            });
        }
    }

    // Create response with metadata
    let response = create_response(
        argument_info,
        &server_id,
        &params,
        None, // TODO: Add cache hit detection
    );

    tracing::debug!(
        "Retrieved prompt argument information for server '{}': {} prompts with arguments",
        server_id,
        response.data.len()
    );

    Ok(Json(response))
}

/// Prompt summary structure
#[derive(Debug, serde::Serialize)]
pub struct PromptSummary {
    /// Server identifier
    pub server_id: String,
    /// Total number of prompts
    pub total_prompts: usize,
    /// Number of enabled prompts
    pub enabled_prompts: usize,
    /// Number of prompts with arguments
    pub prompts_with_arguments: usize,
    /// Total number of arguments across all prompts
    pub total_arguments: usize,
    /// Number of required arguments
    pub required_arguments: usize,
    /// Whether server has complex prompts (with arguments)
    pub has_complex_prompts: bool,
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
