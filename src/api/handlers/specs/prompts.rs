// MCP specification-compliant prompt handlers
// Provides handlers for MCP specification-compliant prompt information

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

/// List all MCP specification-compliant prompts
pub async fn list_all(
    State(state): State<Arc<AppState>>
) -> Result<Json<Vec<rmcp::model::Prompt>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, _db) = get_context(&state).await?;

    // Build prompt mapping from all connected servers
    let prompt_mapping =
        crate::core::protocol::prompt::build_prompt_mapping(&proxy.connection_pool).await;

    // Convert prompt mapping to list of prompts (filtering is already done in build_prompt_mapping)
    let prompts: Vec<rmcp::model::Prompt> = prompt_mapping
        .into_values()
        .map(|mapping| mapping.prompt)
        .collect();

    tracing::info!(
        "Returning {} prompts in MCP specification format",
        prompts.len()
    );

    Ok(Json(prompts))
}

/// List MCP specification-compliant prompts for a specific server
pub async fn list_server(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<Vec<rmcp::model::Prompt>>, ApiError> {
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

    // Build prompt mapping from all connected servers
    let prompt_mapping =
        crate::core::protocol::prompt::build_prompt_mapping(&proxy.connection_pool).await;

    // Filter prompts for the specific server
    let prompts: Vec<rmcp::model::Prompt> = prompt_mapping
        .into_values()
        .filter(|mapping| mapping.server_name == server_name)
        .map(|mapping| mapping.prompt)
        .collect();

    tracing::info!(
        "Returning {} prompts from server '{}' in MCP specification format",
        prompts.len(),
        server_name
    );

    Ok(Json(prompts))
}

/// Helper function to get HTTP proxy server and database from application state
///
/// This function extracts the HTTP proxy server and database from the application state,
/// handling common error cases and reducing code duplication.
///
/// # Arguments
/// * `state` - The application state
///
/// # Returns
/// * `Result<(&ProxyServer, &Arc<Database>), ApiError>` - The HTTP proxy server and database, or an error
pub async fn get_context(
    state: &Arc<AppState>
) -> Result<(&ProxyServer, &Arc<crate::config::database::Database>), ApiError> {
    // Get the HTTP proxy server
    let proxy = state
        .http_proxy
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("HTTP proxy server not available".to_string()))?;

    // Get the database
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    Ok((proxy, db))
}
