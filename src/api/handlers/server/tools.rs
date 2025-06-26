// Server tools handlers
// Provides handlers for server tool discovery endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    discovery::{DiscoveryParams, ProcessedToolInfo},
};

use super::common::{
    create_discovery_response, get_database_from_state, get_discovery_service,
    handle_discovery_error, resolve_server_identifier, validate_server_id,
};

/// Query parameters for tools endpoints
#[derive(Debug, serde::Deserialize)]
pub struct ToolsQuery {
    /// Refresh strategy for tool queries
    pub refresh: Option<crate::discovery::RefreshStrategy>,
    /// Response format
    pub format: Option<crate::discovery::ResponseFormat>,
    /// Whether to include metadata
    pub include_meta: Option<bool>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
}

impl ToolsQuery {
    /// Convert to DiscoveryParams
    pub fn to_params(&self) -> Result<DiscoveryParams, ApiError> {
        Ok(DiscoveryParams {
            refresh: self.refresh,
            format: self.format,
            include_meta: self.include_meta,
        })
    }
}

/// Validate tool name format
fn validate_tool_name(tool_name: &str) -> Result<(), ApiError> {
    if tool_name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Tool name cannot be empty".to_string(),
        ));
    }

    if tool_name.len() > 255 {
        return Err(ApiError::BadRequest(
            "Tool name too long (max 255 characters)".to_string(),
        ));
    }

    Ok(())
}

/// List all tools for a specific server
///
/// Returns a list of tools available on the specified server with
/// configurable filtering and formatting options.
///
/// Supports both server_name and server_id as identifier.
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<ToolsQuery>,
) -> Result<Json<crate::discovery::DiscoveryResponse<Vec<ProcessedToolInfo>>>, ApiError> {
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
    let tools_result = tokio::time::timeout(timeout, async {
        discovery_service
            .get_server_tools(&server_info.server_id, params.clone())
            .await
    })
    .await;

    let tools = match tools_result {
        Ok(result) => result.map_err(handle_discovery_error)?,
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Tools request for server '{}' timed out after {}s",
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Create response with metadata
    let response = create_discovery_response(
        tools,
        &params,
        Some(false), // Tools endpoint doesn't use direct caching
        None,        // No capabilities metadata for this endpoint
    );

    tracing::info!(
        "Retrieved {} tools for server '{}' (ID: {}) with strategy {:?}",
        response.data.len(),
        server_info.server_name,
        server_info.server_id,
        params.refresh.unwrap_or_default()
    );

    Ok(Json(response))
}

/// Get detailed information for a specific tool
///
/// Returns complete tool information including input schema, description,
/// and configuration status for the specified tool on the given server.
///
/// Uses tool_name instead of tool_id for clearer semantics.
/// Supports both server_name and server_id as identifier.
pub async fn get_tool_detail(
    State(state): State<Arc<AppState>>,
    Path((identifier, tool_name)): Path<(String, String)>,
    Query(query): Query<ToolsQuery>,
) -> Result<Json<crate::discovery::DiscoveryResponse<ProcessedToolInfo>>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate parameters
    validate_server_id(&server_info.server_id)?;
    validate_tool_name(&tool_name)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Add timeout control
    let timeout = std::time::Duration::from_secs(query.timeout.unwrap_or(30));
    let tool_result = tokio::time::timeout(timeout, async {
        discovery_service
            .get_tool_detail(&server_info.server_id, &tool_name, params.clone())
            .await
    })
    .await;

    let tool_option = match tool_result {
        Ok(result) => result.map_err(handle_discovery_error)?,
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Tool detail request for '{}' on server '{}' timed out after {}s",
                tool_name,
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Check if tool was found
    let tool = tool_option.ok_or_else(|| {
        ApiError::NotFound(format!(
            "Tool '{}' not found on server '{}' (ID: {})",
            tool_name, server_info.server_name, server_info.server_id
        ))
    })?;

    // Create response with metadata
    let response = create_discovery_response(
        tool,
        &params,
        Some(false), // No direct caching for this endpoint
        None,        // No capabilities metadata for this endpoint
    );

    tracing::info!(
        "Retrieved tool '{}' details for server '{}' (ID: {})",
        tool_name,
        server_info.server_name,
        server_info.server_id
    );

    Ok(Json(response))
}

/// Get tool schema information
///
/// Returns only the input schema for a specific tool, useful for
/// form generation and validation purposes.
///
/// Uses tool_name instead of tool_id for clearer semantics.
/// Supports both server_name and server_id as identifier.
pub async fn get_tool_schema(
    State(state): State<Arc<AppState>>,
    Path((identifier, tool_name)): Path<(String, String)>,
    Query(query): Query<ToolsQuery>,
) -> Result<Json<crate::discovery::DiscoveryResponse<ToolSchema>>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate parameters
    validate_server_id(&server_info.server_id)?;
    validate_tool_name(&tool_name)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Get discovery service
    let discovery_service = get_discovery_service(&state).await?;

    // Add timeout control
    let timeout = std::time::Duration::from_secs(query.timeout.unwrap_or(30));
    let tool_result = tokio::time::timeout(timeout, async {
        discovery_service
            .get_tool_detail(&server_info.server_id, &tool_name, params.clone())
            .await
    })
    .await;

    let tool_option = match tool_result {
        Ok(result) => result.map_err(handle_discovery_error)?,
        Err(_) => {
            return Err(ApiError::Timeout(format!(
                "Tool schema request for '{}' on server '{}' timed out after {}s",
                tool_name,
                identifier,
                timeout.as_secs()
            )));
        }
    };

    // Check if tool was found
    let tool = tool_option.ok_or_else(|| {
        ApiError::NotFound(format!(
            "Tool '{}' not found on server '{}' (ID: {})",
            tool_name, server_info.server_name, server_info.server_id
        ))
    })?;

    // Extract schema information
    let schema = ToolSchema {
        tool_name: tool.name,
        input_schema: tool.input_schema,
        description: tool.description,
        unique_name: tool.unique_name,
    };

    // Create response with metadata
    let response = create_discovery_response(
        schema,
        &params,
        Some(false), // No direct caching for this endpoint
        None,        // No capabilities metadata for this endpoint
    );

    tracing::debug!(
        "Retrieved schema for tool '{}' on server '{}' (ID: {})",
        tool_name,
        server_info.server_name,
        server_info.server_id
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
    /// Unique name in configuration
    pub unique_name: Option<String>,
}
