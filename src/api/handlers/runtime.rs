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
    runtime::{RuntimeInstaller, RuntimeManager, RuntimeType},
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

/// Handle runtime installation using RuntimeInstaller
async fn perform_runtime_installation(
    _state: &AppState,
    _request: &InstallRequest,
    runtime_type: RuntimeType,
) -> Result<(), anyhow::Error> {
    let installer = RuntimeInstaller::new();
    installer.install_runtime(runtime_type).await.map(|_| ())
}

/// Create runtime status for a specific runtime type
fn create_runtime_status(
    runtime_type: RuntimeType,
    _version: Option<&str>, // Version parameter kept for API compatibility but ignored
) -> RuntimeStatus {
    let manager = RuntimeManager::new();
    let available = manager.is_installed(runtime_type);
    let path = manager.get_executable_path(runtime_type);
    let path_str = path.as_ref().map(|p| p.to_string_lossy().to_string());

    // Use RuntimeManager's enhanced status message with source information
    let runtime_info = manager
        .list_installed()
        .into_iter()
        .find(|info| info.runtime_type == runtime_type);

    let message = runtime_info.map(|info| info.message).unwrap_or_else(|| {
        if available {
            format!("✓ {} is available", runtime_type)
        } else {
            format!("✗ {} is not installed", runtime_type)
        }
    });

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

    let _version = request.version.as_deref(); // Version parameter kept for API compatibility but ignored

    // Check if runtime is already installed
    let manager = RuntimeManager::new();
    let is_already_installed = manager.is_installed(runtime_type);

    // Handle already installed runtime
    if is_already_installed {
        tracing::info!("Runtime {runtime_type} already installed");

        return Ok(Json(InstallResponse {
            success: true,
            message: format!("Runtime {runtime_type} already installed"),
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
        vec![RuntimeType::Uv, RuntimeType::Bun]
    };

    // Check each runtime type and create status
    let runtimes = runtime_types_to_check
        .into_iter()
        .map(|runtime_type| create_runtime_status(runtime_type, query.version.as_deref()))
        .collect();

    Ok(Json(ListResponse { runtimes }))
}
