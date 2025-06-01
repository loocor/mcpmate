// Runtime API handlers - Ultra-lightweight wrappers around CLI functionality
// Directly calls existing CLI functions to avoid any code duplication

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{handlers::ApiError, routes::AppState},
    common::paths::global_paths,
    runtime::{
        RuntimeType, cli::handle_install_command, get_runtime_path, types::ExecutionContext,
    },
};

/// Install runtime request
#[derive(Debug, Deserialize)]
pub struct InstallRequest {
    pub runtime_type: String,
    pub version: Option<String>,
    pub timeout: Option<u64>,
    pub max_retries: Option<u32>,
    pub verbose: Option<bool>,
    pub interactive: Option<bool>,
}

/// Runtime status response
#[derive(Debug, Serialize)]
pub struct RuntimeStatus {
    pub runtime_type: String,
    pub available: bool,
    pub path: Option<String>,
    pub message: String,
}

/// List/query runtime parameters
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub runtime_type: Option<String>,
    pub version: Option<String>,
}

/// List response
#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub runtimes: Vec<RuntimeStatus>,
}

/// Install response
#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub success: bool,
    pub message: String,
    pub runtime_type: String,
}

/// Sync already installed runtime to database
async fn sync_existing_runtime_to_db(
    state: &AppState,
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<(), String> {
    let Some(db) = &state.database else {
        return Ok(()); // No database configured, skip sync
    };

    let runtime_path = get_runtime_path(runtime_type, version)
        .map_err(|e| format!("Failed to get runtime path: {e}"))?;

    // Create relative path for database storage
    let paths = global_paths();
    let mcpmate_dir = paths.base_dir();

    let relative_path = if runtime_path.starts_with(mcpmate_dir) {
        // MCPMate managed runtime - store relative path
        runtime_path
            .strip_prefix(mcpmate_dir)
            .unwrap_or(&runtime_path)
            .to_string_lossy()
            .to_string()
    } else {
        // System runtime - store absolute path
        runtime_path.to_string_lossy().to_string()
    };

    // Save to database
    let version_str = version.unwrap_or_else(|| runtime_type.default_version());
    let config =
        crate::runtime::config::RuntimeConfig::new(runtime_type, version_str, &relative_path);

    crate::runtime::config::save_config(&db.pool, &config)
        .await
        .map_err(|e| format!("Failed to sync runtime config to database: {e}"))?;

    tracing::info!("Successfully synced {runtime_type} runtime to database");
    Ok(())
}

/// Handle runtime installation using CLI
async fn perform_runtime_installation(
    state: &AppState,
    request: &InstallRequest,
    runtime_type: RuntimeType,
) -> Result<(), anyhow::Error> {
    // Get database path from state if available
    let database = state
        .database
        .as_ref()
        .map(|db| db.path.to_string_lossy().to_string());

    // Call the existing CLI function with API execution context
    handle_install_command(
        runtime_type,
        request.version.clone(),
        request.timeout.unwrap_or(300),
        request.max_retries.unwrap_or(3),
        request.verbose.unwrap_or(false),
        request.interactive.unwrap_or(false),
        true,
        database,
        ExecutionContext::Api,
    )
    .await
}

/// Create runtime status for a specific runtime type
fn create_runtime_status(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> RuntimeStatus {
    // Check if runtime is available
    let path = get_runtime_path(runtime_type, version).ok();
    let available = path.as_ref().is_some_and(|p| p.exists());

    let path_str = path.map(|p| p.to_string_lossy().to_string());

    let version_display = version.unwrap_or("default");
    let message = if available {
        format!("✓ {} ({}) is available", runtime_type, version_display)
    } else {
        format!("✗ {} ({}) is not installed", runtime_type, version_display)
    };

    RuntimeStatus {
        runtime_type: runtime_type.to_string(),
        available,
        path: path_str,
        message,
    }
}

/// Install a runtime - POST /api/runtime/install
pub async fn install_runtime(
    State(state): State<Arc<AppState>>,
    Json(request): Json<InstallRequest>,
) -> Result<Json<InstallResponse>, ApiError> {
    // Parse runtime type
    let runtime_type: RuntimeType = request
        .runtime_type
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("Invalid runtime type: {e}")))?;

    let version = request.version.as_deref();

    // Check if runtime is already installed
    let runtime_path = get_runtime_path(runtime_type, version);
    let is_already_installed = runtime_path.as_ref().is_ok_and(|p| p.exists());

    // Handle already installed runtime
    if is_already_installed {
        tracing::info!("Runtime {runtime_type} already installed, syncing to database");

        // Attempt to sync to database, but don't fail if it doesn't work
        if let Err(e) = sync_existing_runtime_to_db(&state, runtime_type, version).await {
            tracing::warn!("Failed to sync runtime to database: {e}");
        }

        return Ok(Json(InstallResponse {
            success: true,
            message: format!("Runtime {runtime_type} already installed and synced to database"),
            runtime_type: request.runtime_type,
        }));
    }

    // Handle new runtime installation
    tracing::info!("Runtime {runtime_type} not found, proceeding with download and installation");

    match perform_runtime_installation(&state, &request, runtime_type).await {
        Ok(()) => Ok(Json(InstallResponse {
            success: true,
            message: format!(
                "Successfully downloaded and installed {}",
                request.runtime_type
            ),
            runtime_type: request.runtime_type,
        })),
        Err(e) => Ok(Json(InstallResponse {
            success: false,
            message: format!("Installation failed: {e}"),
            runtime_type: request.runtime_type,
        })),
    }
}

/// List/query runtimes - GET /api/runtime/list
/// Supports optional query parameters to filter by runtime_type and version
/// Replaces the functionality of /check and /path endpoints
pub async fn list_runtimes(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    // Determine which runtime types to check
    let runtime_types_to_check = if let Some(runtime_type_str) = &query.runtime_type {
        // Parse and validate the specific runtime type
        let runtime_type: RuntimeType = runtime_type_str
            .parse()
            .map_err(|e| ApiError::BadRequest(format!("Invalid runtime type: {e}")))?;
        vec![runtime_type]
    } else {
        // Check all runtime types
        vec![RuntimeType::Node, RuntimeType::Uv, RuntimeType::Bun]
    };

    // Check each runtime type and create status
    let runtimes = runtime_types_to_check
        .into_iter()
        .map(|runtime_type| create_runtime_status(runtime_type, query.version.as_deref()))
        .collect();

    Ok(Json(ListResponse { runtimes }))
}
