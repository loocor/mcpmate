//! Runtime API handlers - Complete business logic implementation
//!
//! Provides runtime management functionality for UV and Bun package managers,
//! including installation, status monitoring, and cache management.
//!
//! Note: Only manages Runtime Cache (UV/Bun packages), not Capabilities Cache.

use chrono;
use std::fs;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use tokio::process::Command as AsyncCommand;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    common::{RuntimeType, paths::global_paths},
    runtime::{RuntimeInstaller, RuntimeManager},
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
    pub version: Option<String>,
    pub message: String,
}

/// Install response
#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub success: bool,
    pub message: String,
    pub runtime_type: String,
}

/// Runtime status response - only runtime info
#[derive(Debug, Serialize)]
pub struct RuntimeStatusResponse {
    pub uv: RuntimeStatus,
    pub bun: RuntimeStatus,
}

/// Runtime cache response
#[derive(Debug, Serialize)]
pub struct RuntimeCacheResponse {
    pub summary: CacheSummaryInfo,
    pub uv: CacheItem,
    pub bun: CacheItem,
}

#[derive(Debug, Serialize)]
pub struct CacheSummaryInfo {
    pub total_size_bytes: u64,
    pub last_cleanup: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CacheItem {
    pub path: String,
    pub size_bytes: u64,
    pub package_count: u64,
    pub last_modified: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CacheResetQuery {
    #[serde(default = "default_cache_type")]
    pub cache_type: String, // "all" | "uv" | "bun", defaults to "all"
}

fn default_cache_type() -> String {
    "all".to_string()
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
async fn create_runtime_status(runtime_type: RuntimeType) -> RuntimeStatus {
    let manager = RuntimeManager::new();
    let available = manager.is_installed(runtime_type);
    let path = manager.get_executable_path(runtime_type);
    let path_str = path.as_ref().map(|p| p.to_string_lossy().to_string());

    // Get version using async command
    let version = if let Some(path) = &path {
        get_version_from_exec_async(path).await
    } else {
        None
    };

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
        version,
        message,
    }
}

/// Get version from executable asynchronously
async fn get_version_from_exec_async(path: &std::path::Path) -> Option<String> {
    let output = AsyncCommand::new(path).arg("--version").output().await.ok()?;

    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() { Some(s) } else { None }
    } else {
        None
    }
}

/// Calculate directory size recursively
fn calculate_dir_size(dir_path: &std::path::Path) -> u64 {
    if !dir_path.exists() || !dir_path.is_dir() {
        return 0;
    }

    fs::read_dir(dir_path)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        calculate_dir_size(&path)
                    } else {
                        entry.metadata().map(|m| m.len()).unwrap_or(0)
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

/// Count packages in cache directory
fn count_packages_in_cache(cache_path: &std::path::Path) -> u64 {
    if !cache_path.exists() || !cache_path.is_dir() {
        return 0;
    }

    fs::read_dir(cache_path)
        .map(|entries| entries.filter_map(|entry| entry.ok()).count() as u64)
        .unwrap_or(0)
}

/// Get last modified time of directory
fn get_last_modified(path: &std::path::Path) -> Option<String> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map(|time| {
            time.duration_since(UNIX_EPOCH)
                .map(|dur| {
                    let timestamp = dur.as_secs();
                    chrono::DateTime::from_timestamp(timestamp as i64, 0)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| format!("{}", timestamp))
                })
                .unwrap_or_else(|_| "unknown".to_string())
        })
        .ok()
}

/// Get the last cleanup timestamp from .last_cleanup file
fn get_last_cleanup_time() -> Option<String> {
    let paths = global_paths();
    let cleanup_file = paths.cache_dir().join(".last_cleanup");

    if cleanup_file.exists() {
        std::fs::read_to_string(&cleanup_file)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Get cache info for a specific runtime type
async fn get_cache_info(runtime_type: RuntimeType) -> CacheItem {
    let paths = global_paths();
    let cache_path = paths.runtime_cache_dir(runtime_type.as_str());

    let size_bytes = calculate_dir_size(&cache_path);
    let package_count = count_packages_in_cache(&cache_path);
    let last_modified = get_last_modified(&cache_path);

    CacheItem {
        path: cache_path.to_string_lossy().to_string(),
        size_bytes,
        package_count,
        last_modified,
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
        tracing::info!("Runtime {} already installed", runtime_type);

        return Ok(Json(InstallResponse {
            success: true,
            message: format!("Runtime {} already installed", runtime_type),
            runtime_type: request.runtime_type,
        }));
    }

    // Handle new runtime installation
    tracing::info!(
        "Runtime {} not found, proceeding with download and installation",
        runtime_type
    );

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

/// GET /api/runtime/status
pub async fn runtime_status(State(_state): State<Arc<AppState>>) -> Result<Json<RuntimeStatusResponse>, ApiError> {
    // Get runtime information
    let uv_status = create_runtime_status(RuntimeType::Uv).await;
    let bun_status = create_runtime_status(RuntimeType::Bun).await;

    Ok(Json(RuntimeStatusResponse {
        uv: uv_status,
        bun: bun_status,
    }))
}

/// GET /api/runtime/cache - detailed Runtime Cache statistics
/// Focus on space usage and cache management, removed performance metrics
pub async fn runtime_cache(State(_state): State<Arc<AppState>>) -> Result<Json<RuntimeCacheResponse>, ApiError> {
    // Get cache information for both runtime types
    let uv_cache = get_cache_info(RuntimeType::Uv).await;
    let bun_cache = get_cache_info(RuntimeType::Bun).await;

    Ok(Json(RuntimeCacheResponse {
        summary: CacheSummaryInfo {
            total_size_bytes: uv_cache.size_bytes + bun_cache.size_bytes,
            last_cleanup: get_last_cleanup_time(),
        },
        uv: uv_cache,
        bun: bun_cache,
    }))
}

/// Spec: POST /api/runtime/cache/reset?cache_type=all|uv|bun
/// Reset runtime environment cache under ~/.mcpmate/cache.
/// Supports selective clearing: all (default), uv, or bun only.
pub async fn runtime_cache_reset(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<CacheResetQuery>,
) -> Result<Json<ClearCacheResponse>, ApiError> {
    let paths = global_paths();
    let base = paths.cache_dir();

    // Determine which caches to clear based on cache_type parameter
    let cache_paths: Vec<std::path::PathBuf> = match query.cache_type.as_str() {
        "uv" => vec![base.join(RuntimeType::Uv.as_str())],
        "bun" => vec![base.join(RuntimeType::Bun.as_str())],
        _ => vec![
            // "all" or any other value defaults to all
            base.join(RuntimeType::Uv.as_str()),
            base.join(RuntimeType::Bun.as_str()),
        ],
    };

    let mut ok = true;
    for p in cache_paths {
        if p.exists() {
            if let Err(e) = std::fs::remove_dir_all(&p) {
                tracing::warn!("Failed to remove {:?}: {}", p, e);
                ok = false;
            } else {
                tracing::info!("Removed runtime cache dir: {:?}", p);
            }
        }
    }

    // Record cleanup time if successful
    if ok {
        let cleanup_timestamp_path = base.join(".last_cleanup");
        let timestamp = chrono::Utc::now().to_rfc3339();
        if let Err(e) = std::fs::write(&cleanup_timestamp_path, timestamp) {
            tracing::warn!("Failed to write cleanup timestamp: {}", e);
        } else {
            tracing::info!("Recorded cleanup timestamp at {:?}", cleanup_timestamp_path);
        }
    }

    Ok(Json(ClearCacheResponse { success: ok }))
}
