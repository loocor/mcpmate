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
    runtime::{ExecutionContext, RuntimeManager, RuntimeType, cli::handle_install_command},
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

// Removed CheckRequest, PathRequest, and PathResponse structs
// Their functionality is now covered by the enhanced list_runtimes endpoint

/// Install a runtime - POST /api/runtime/install
pub async fn install_runtime(
    State(state): State<Arc<AppState>>,
    Json(request): Json<InstallRequest>,
) -> Result<Json<InstallResponse>, ApiError> {
    // Parse runtime type
    let runtime_type: RuntimeType = request
        .runtime_type
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("Invalid runtime type: {}", e)))?;

    // Get database path from state if available
    let database = state.database.as_ref().map(|db| {
        // Get the actual database path from the Database instance
        db.path.to_string_lossy().to_string()
    });

    // Call the existing CLI function with API execution context
    match handle_install_command(
        runtime_type,
        request.version,
        request.timeout.unwrap_or(300),
        request.max_retries.unwrap_or(3),
        request.verbose.unwrap_or(false),
        request.interactive.unwrap_or(false),
        true, // quiet mode for API
        database,
        ExecutionContext::Api,
    )
    .await
    {
        Ok(()) => Ok(Json(InstallResponse {
            success: true,
            message: format!("Successfully installed {}", request.runtime_type),
            runtime_type: request.runtime_type,
        })),
        Err(e) => Ok(Json(InstallResponse {
            success: false,
            message: format!("Installation failed: {}", e),
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
    let manager = RuntimeManager::new()
        .map_err(|e| ApiError::InternalError(format!("Failed to create runtime manager: {}", e)))?;

    let mut runtimes = Vec::new();

    // Determine which runtime types to check
    let runtime_types_to_check = if let Some(runtime_type_str) = &query.runtime_type {
        // Parse and validate the specific runtime type
        let runtime_type: RuntimeType = runtime_type_str
            .parse()
            .map_err(|e| ApiError::BadRequest(format!("Invalid runtime type: {}", e)))?;
        vec![runtime_type]
    } else {
        // Check all runtime types
        vec![RuntimeType::Node, RuntimeType::Uv, RuntimeType::Bun]
    };

    // Check each runtime type
    for runtime_type in runtime_types_to_check {
        let version = query.version.as_deref();

        let available = manager
            .is_runtime_available(runtime_type, version)
            .unwrap_or(false);

        let path = if available {
            manager
                .get_runtime_path(runtime_type, version)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        };

        let version_display = version.unwrap_or("default");
        let message = if available {
            format!("✓ {} ({}) is available", runtime_type, version_display)
        } else {
            format!("✗ {} ({}) is not installed", runtime_type, version_display)
        };

        runtimes.push(RuntimeStatus {
            runtime_type: runtime_type.to_string(),
            available,
            path,
            message,
        });
    }

    Ok(Json(ListResponse { runtimes }))
}

// Removed check_runtime and get_runtime_path functions
// Their functionality is now provided by the enhanced list_runtimes endpoint with query parameters:
// - GET /api/runtime/list?runtime_type=node&version=v16 (replaces check)
// - GET /api/runtime/list?runtime_type=node&version=v16 (replaces path)
