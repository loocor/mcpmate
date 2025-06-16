// MCP specification-compliant resource handlers
// Provides handlers for MCP specification-compliant resource information

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    api::{handlers::ApiError, routes::AppState},
    config::server,
    core::proxy::ProxyServer,
};

/// List all MCP specification-compliant resources
pub async fn list_all(
    State(state): State<Arc<AppState>>
) -> Result<Json<Vec<rmcp::model::Resource>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Build resource mapping from all connected servers
    let resource_mapping =
        crate::core::protocol::resource::build_resource_mapping(&proxy.connection_pool, Some(&db))
            .await;

    // Convert resource mapping to list of resources
    let resources: Vec<rmcp::model::Resource> = resource_mapping
        .into_values()
        .map(|mapping| mapping.resource)
        .collect();

    tracing::info!(
        "Returning {} enabled resources in MCP specification format",
        resources.len()
    );

    Ok(Json(resources))
}

/// List MCP specification-compliant resources for a specific server
pub async fn list_server(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<Vec<rmcp::model::Resource>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = server::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{server_name}' not found"
        )));
    }

    // Build resource mapping from all connected servers
    let resource_mapping =
        crate::core::protocol::resource::build_resource_mapping(&proxy.connection_pool, Some(&db))
            .await;

    // Filter resources for the specific server
    let resources: Vec<rmcp::model::Resource> = resource_mapping
        .into_values()
        .filter(|mapping| mapping.server_name == server_name)
        .map(|mapping| mapping.resource)
        .collect();

    tracing::info!(
        "Returning {} enabled resources from server '{}' in MCP specification format",
        resources.len(),
        server_name
    );

    Ok(Json(resources))
}

/// List all MCP specification-compliant resource templates
pub async fn list_templates(
    State(state): State<Arc<AppState>>
) -> Result<Json<Vec<rmcp::model::ResourceTemplate>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, _db) = get_context(&state).await?;

    // Build resource template mapping from all connected servers
    let resource_template_mapping =
        crate::core::protocol::resource::build_resource_template_mapping(&proxy.connection_pool)
            .await;

    // Convert resource template mapping to list of resource templates
    let resource_templates: Vec<rmcp::model::ResourceTemplate> = resource_template_mapping
        .into_iter()
        .map(|mapping| mapping.resource_template)
        .collect();

    tracing::info!(
        "Returning {} resource templates in MCP specification format",
        resource_templates.len()
    );

    Ok(Json(resource_templates))
}

/// List MCP specification-compliant resource templates for a specific server
pub async fn list_server_templates(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<Vec<rmcp::model::ResourceTemplate>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = server::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{server_name}' not found"
        )));
    }

    // Build resource template mapping from all connected servers
    let resource_template_mapping =
        crate::core::protocol::resource::build_resource_template_mapping(&proxy.connection_pool)
            .await;

    // Filter resource templates for the specific server
    let resource_templates: Vec<rmcp::model::ResourceTemplate> = resource_template_mapping
        .into_iter()
        .filter(|mapping| mapping.server_name == server_name)
        .map(|mapping| mapping.resource_template)
        .collect();

    tracing::info!(
        "Returning {} resource templates from server '{}' in MCP specification format",
        resource_templates.len(),
        server_name
    );

    Ok(Json(resource_templates))
}

/// Helper function to get context (proxy server and database)
async fn get_context(
    state: &AppState
) -> Result<(Arc<ProxyServer>, Arc<crate::config::database::Database>), ApiError> {
    let proxy = state
        .http_proxy
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("HTTP proxy not available".to_string()))?;

    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    Ok((proxy.clone(), db.clone()))
}
