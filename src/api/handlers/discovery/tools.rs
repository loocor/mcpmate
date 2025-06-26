// Discovery tools handlers
// Provides handlers for server tool discovery endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    discovery::ProcessedToolInfo,
};

use super::{
    DiscoveryQuery, DiscoveryResponse, create_response, get_discovery_service,
    handle_discovery_error, validate_server_id, validate_tool_id,
};

/// List all tools for a specific server
///
/// Returns a list of tools available on the specified server with
/// configurable filtering and formatting options.
pub async fn server_tools(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<Vec<ProcessedToolInfo>>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server tools
    let tools = discovery_service
        .get_server_tools(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Create response with metadata
    let response = create_response(
        tools, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved {} tools for server '{}' with strategy {:?}",
        response.data.len(),
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// Get detailed information for a specific tool
///
/// Returns complete tool information including input schema, description,
/// and configuration status for the specified tool on the given server.
pub async fn get_tool_detail(
    State(state): State<Arc<AppState>>,
    Path((server_id, tool_id)): Path<(String, String)>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<ProcessedToolInfo>>, ApiError> {
    // Validate parameters
    validate_server_id(&server_id)?;
    validate_tool_id(&tool_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get tool detail
    let tool = discovery_service
        .get_tool_detail(&server_id, &tool_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Check if tool was found
    let tool = tool.ok_or_else(|| {
        ApiError::NotFound(format!(
            "Tool '{}' not found on server '{}'",
            tool_id, server_id
        ))
    })?;

    // Create response with metadata
    let response = create_response(
        tool, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved tool '{}' details for server '{}'",
        tool_id,
        server_id
    );

    Ok(Json(response))
}

/// List enabled tools for a specific server
///
/// Returns only the tools that are currently enabled in the configuration.
/// This is useful for getting the active tool set without disabled tools.
pub async fn enabled_server_tools(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<Vec<ProcessedToolInfo>>>, ApiError> {
    // Validate server ID
    validate_server_id(&server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get server tools
    let all_tools = discovery_service
        .get_server_tools(&server_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Filter to only enabled tools
    let enabled_tools: Vec<ProcessedToolInfo> =
        all_tools.into_iter().filter(|tool| tool.enabled).collect();

    // Create response with metadata
    let response = create_response(
        enabled_tools,
        &server_id,
        &params,
        None, // TODO: Add cache hit detection
    );

    tracing::info!(
        "Retrieved {} enabled tools for server '{}' with strategy {:?}",
        response.data.len(),
        server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// Get tool schema information
///
/// Returns only the input schema for a specific tool, useful for
/// form generation and validation purposes.
pub async fn get_tool_schema(
    State(state): State<Arc<AppState>>,
    Path((server_id, tool_id)): Path<(String, String)>,
    Query(query): Query<DiscoveryQuery>,
) -> Result<Json<DiscoveryResponse<ToolSchema>>, ApiError> {
    // Validate parameters
    validate_server_id(&server_id)?;
    validate_tool_id(&tool_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Get tool detail
    let tool = discovery_service
        .get_tool_detail(&server_id, &tool_id, params.clone())
        .await
        .map_err(handle_discovery_error)?;

    // Check if tool was found
    let tool = tool.ok_or_else(|| {
        ApiError::NotFound(format!(
            "Tool '{}' not found on server '{}'",
            tool_id, server_id
        ))
    })?;

    // Extract schema information
    let schema = ToolSchema {
        tool_name: tool.name,
        input_schema: tool.input_schema,
        description: tool.description,
        enabled: tool.enabled,
        unique_name: tool.unique_name,
    };

    // Create response with metadata
    let response = create_response(
        schema, &server_id, &params, None, // TODO: Add cache hit detection
    );

    tracing::debug!(
        "Retrieved schema for tool '{}' on server '{}'",
        tool_id,
        server_id
    );

    Ok(Json(response))
}

/// Tool schema structure for schema-only responses
#[derive(Debug, serde::Serialize)]
pub struct ToolSchema {
    /// Tool name
    pub tool_name: String,
    /// Input schema (JSON object)
    pub input_schema: serde_json::Value,
    /// Tool description
    pub description: Option<String>,
    /// Whether tool is enabled
    pub enabled: bool,
    /// Unique name in configuration
    pub unique_name: Option<String>,
}
