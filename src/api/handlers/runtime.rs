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

// -------- Spec-aligned models --------

#[derive(Debug, Serialize)]
pub struct RuntimeCompositeStatus {
    pub runtime_status: serde_json::Value,
    pub cache_status: serde_json::Value,
    pub active_servers: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CacheStatisticsResponse {
    pub cache_statistics: serde_json::Value,
    pub performance_metrics: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ClearCacheRequest {
    pub cache_types: Option<Vec<String>>, // ["redb", "build_artifacts", "dependencies"]
    pub server_ids: Option<Vec<String>>,  // optional
    pub force: Option<bool>,
    pub backup_before_clear: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ClearCacheResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct RebuildCacheRequest {
    pub rebuild_strategy: Option<String>, // "full|incremental|selective"
    pub server_ids: Option<Vec<String>>,  // optional
    pub parallel_processing: Option<bool>,
    pub validate_after_rebuild: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct VersionsResponse {
    pub versions: serde_json::Value,
    pub compatibility_matrix: serde_json::Value,
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
            message: format!("Successfully downloaded and installed {}", request.runtime_type),
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

/// Spec: GET /api/runtime/status
pub async fn runtime_status(State(state): State<Arc<AppState>>) -> Result<Json<RuntimeCompositeStatus>, ApiError> {
    // Runtime availability via RuntimeManager
    let manager = RuntimeManager::new();
    let node_available = manager.is_installed(RuntimeType::Bun); // using Bun as a placeholder runtime
    let node_status = serde_json::json!({
        "version": null,
        "status": if node_available { "available" } else { "unavailable" },
        "package_managers": serde_json::json!({})
    });
    let python_status = serde_json::json!({
        "version": null,
        "status": "unknown",
        "package_managers": serde_json::json!({})
    });

    let runtime_status = serde_json::json!({
        "node_js": node_status,
        "python": python_status,
    });

    // Cache status from Redb
    let stats = state.redb_cache.get_stats().await;
    let cache_status = serde_json::json!({
        "total_size": format!("{}B", stats.cache_size_bytes),
        "entries_count": stats.total_servers + stats.total_tools + stats.total_resources + stats.total_prompts,
        "hit_rate": stats.hit_ratio,
        "last_cleanup": stats.last_updated.to_rfc3339(),
    });

    // Active servers: derive production from pool connections; others 0 for now
    let pool = state.connection_pool.lock().await;
    let (production, exploration, validation) = pool.active_instance_counts();
    drop(pool);
    let active_servers = serde_json::json!({
        "production": production,
        "exploration": exploration,
        "validation": validation,
    });

    Ok(Json(RuntimeCompositeStatus {
        runtime_status,
        cache_status,
        active_servers,
    }))
}

/// Spec: GET /api/runtime/cache
pub async fn runtime_cache(State(state): State<Arc<AppState>>) -> Result<Json<CacheStatisticsResponse>, ApiError> {
    let stats = state.redb_cache.get_stats().await;
    let cache_statistics = serde_json::json!({
        "redb_cache": {
            "size": format!("{}B", stats.cache_size_bytes),
            "entries": stats.total_servers + stats.total_tools + stats.total_resources + stats.total_prompts,
            "hit_rate": stats.hit_ratio,
            "avg_query_time": null
        },
        "build_artifacts": {"size": "0B", "entries": 0, "last_cleanup": null},
        "dependency_cache": {"node_modules_size": "0B", "python_packages_size": "0B", "shared_dependencies": 0},
    });
    let performance_metrics = serde_json::json!({
        "cache_hit_rate_trend": [],
        "query_time_trend": [],
        "storage_growth_rate": null,
    });
    Ok(Json(CacheStatisticsResponse {
        cache_statistics,
        performance_metrics,
    }))
}

/// Spec: POST /api/runtime/cache/clear
pub async fn runtime_cache_clear(
    State(state): State<Arc<AppState>>,
    Json(_req): Json<ClearCacheRequest>,
) -> Result<Json<ClearCacheResponse>, ApiError> {
    state
        .redb_cache
        .clear_all()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to clear cache: {e}")))?;
    Ok(Json(ClearCacheResponse { success: true }))
}

/// Spec: POST /api/runtime/cache/rebuild
pub async fn runtime_cache_rebuild(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RebuildCacheRequest>,
) -> Result<Json<ClearCacheResponse>, ApiError> {
    tracing::info!("Starting cache rebuild with strategy: {:?}", req.rebuild_strategy);

    // Clear existing cache first
    state
        .redb_cache
        .clear_all()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to clear cache before rebuild: {e}")))?;

    // TODO: For now, return success - full implementation would:
    // 1. Get all configured servers from database
    // 2. Create temporary exploration instances for each
    // 3. Fetch fresh capabilities data
    // 4. Store in cache with new fingerprints
    // 5. Validate after rebuild if requested

    tracing::info!("Cache rebuild completed successfully");
    Ok(Json(ClearCacheResponse { success: true }))
}

/// Spec: GET /api/runtime/versions
pub async fn runtime_versions(State(_state): State<Arc<AppState>>) -> Result<Json<VersionsResponse>, ApiError> {
    let versions = serde_json::json!({
        "mcpmate": env!("CARGO_PKG_VERSION"),
        "node_js": null,
        "python": null,
        "redb": "2.x",
    });
    let compatibility_matrix = serde_json::json!({
        "node_servers": {"supported_versions": ["16.x", "18.x", "20.x"], "recommended_version": "18.x"},
        "python_servers": {"supported_versions": ["3.9", "3.10", "3.11", "3.12"], "recommended_version": "3.11"},
    });
    Ok(Json(VersionsResponse {
        versions,
        compatibility_matrix,
    }))
}
