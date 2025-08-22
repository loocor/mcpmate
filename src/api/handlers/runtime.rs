use chrono;
use std::fs;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use axum::{Json, extract::State, http::StatusCode};

use tokio::process::Command as AsyncCommand;

use crate::{
    api::{
        models::{runtime::*},
        routes::AppState,
    },
    common::{RuntimeType, paths::global_paths},
    runtime::{RuntimeInstaller, RuntimeManager},
};

pub async fn install(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<RuntimeInstallReq>,
) -> Result<Json<RuntimeInstallApiResp>, StatusCode> {
    let result = runtime_install_core(&request, &app_state).await?;
    Ok(Json(result))
}

pub async fn status(
    State(app_state): State<Arc<AppState>>
) -> Result<Json<RuntimeStatusApiResp>, StatusCode> {
    let result = runtime_status_core(&app_state).await?;
    Ok(Json(result))
}

pub async fn cache(State(app_state): State<Arc<AppState>>) -> Result<Json<RuntimeCacheApiResp>, StatusCode> {
    let result = runtime_cache_core(&app_state).await?;
    Ok(Json(result))
}

pub async fn reset_cache(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<RuntimeCacheResetReq>,
) -> Result<Json<RuntimeCacheResetApiResp>, StatusCode> {
    let result = reset_cache_core(&request, &app_state).await?;
    Ok(Json(result))
}

async fn runtime_install_core(
    request: &RuntimeInstallReq,
    app_state: &AppState,
) -> Result<RuntimeInstallApiResp, StatusCode> {
    let runtime_type: RuntimeType = request.runtime_type.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let manager = RuntimeManager::new();

    // Early return if already installed
    if manager.is_installed(runtime_type) {
        tracing::info!("Runtime {} already installed", runtime_type);

        return Ok(RuntimeInstallApiResp::success(RuntimeInstallResp {
            success: true,
            message: format!("Runtime {} already installed", runtime_type),
            runtime_type: request.runtime_type.clone(),
        }));
    }

    // Proceed with installation
    tracing::info!(
        "Runtime {} not found, proceeding with download and installation",
        runtime_type
    );

    perform_installation(app_state, request, runtime_type)
        .await
        .map(|_| {
            RuntimeInstallApiResp::success(RuntimeInstallResp {
                success: true,
                message: format!("Successfully downloaded and installed {}", request.runtime_type),
                runtime_type: request.runtime_type.clone(),
            })
        })
        .or_else(|e| {
            Ok(RuntimeInstallApiResp::success(RuntimeInstallResp {
                success: false,
                message: format!("Installation failed: {e}"),
                runtime_type: request.runtime_type.clone(),
            }))
        })
}

async fn perform_installation(
    _state: &AppState,
    _request: &RuntimeInstallReq,
    runtime_type: RuntimeType,
) -> Result<(), anyhow::Error> {
    RuntimeInstaller::new().install_runtime(runtime_type).await.map(|_| ())
}

async fn runtime_status_core(_app_state: &AppState) -> Result<RuntimeStatusApiResp, StatusCode> {
    let (uv_status, bun_status) = tokio::join!(
        create_runtime_status(RuntimeType::Uv),
        create_runtime_status(RuntimeType::Bun)
    );

    Ok(RuntimeStatusApiResp::success(RuntimeStatusResp {
        uv: uv_status,
        bun: bun_status,
    }))
}

async fn create_runtime_status(runtime_type: RuntimeType) -> RuntimeStatus {
    let manager = RuntimeManager::new();
    let available = manager.is_installed(runtime_type);
    let path = manager.get_executable_path(runtime_type);

    let version = match &path {
        Some(p) => get_version_from_exec_async(p).await,
        None => None,
    };

    let message = manager
        .list_installed()
        .into_iter()
        .find(|info| info.runtime_type == runtime_type)
        .map(|info| info.message)
        .unwrap_or_else(|| match available {
            true => format!("✓ {} is available", runtime_type),
            false => format!("✗ {} is not installed", runtime_type),
        });

    RuntimeStatus {
        runtime_type: runtime_type.to_string(),
        available,
        path: path.as_ref().map(|p| p.to_string_lossy().to_string()),
        version,
        message,
    }
}

async fn runtime_cache_core(_app_state: &AppState) -> Result<RuntimeCacheApiResp, StatusCode> {
    let (uv_cache, bun_cache) = tokio::join!(get_cache_info(RuntimeType::Uv), get_cache_info(RuntimeType::Bun));

    Ok(RuntimeCacheApiResp::success(RuntimeCacheResp {
        summary: CacheSummaryInfo {
            total_size_bytes: uv_cache.size_bytes + bun_cache.size_bytes,
            last_cleanup: get_last_cleanup_time(),
        },
        uv: uv_cache,
        bun: bun_cache,
    }))
}

async fn get_cache_info(runtime_type: RuntimeType) -> CacheItem {
    let paths = global_paths();
    let cache_path = paths.runtime_cache_dir(runtime_type.as_str());

    CacheItem {
        path: cache_path.to_string_lossy().to_string(),
        size_bytes: calculate_dir_size(&cache_path),
        package_count: count_packages_in_cache(&cache_path),
        last_modified: get_last_modified(&cache_path),
    }
}

async fn reset_cache_core(
    request: &RuntimeCacheResetReq,
    _app_state: &AppState,
) -> Result<RuntimeCacheResetApiResp, StatusCode> {
    let paths = global_paths();
    let base = paths.cache_dir();

    let cache_paths = match request.cache_type.as_str() {
        "uv" => vec![base.join(RuntimeType::Uv.as_str())],
        "bun" => vec![base.join(RuntimeType::Bun.as_str())],
        _ => vec![
            base.join(RuntimeType::Uv.as_str()),
            base.join(RuntimeType::Bun.as_str()),
        ],
    };

    let removal_results = cache_paths
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| {
            std::fs::remove_dir_all(&p)
                .map(|_| {
                    tracing::info!("Removed runtime cache dir: {:?}", p);
                    true
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to remove {:?}: {}", p, e);
                    false
                })
        })
        .collect::<Vec<_>>();

    let all_successful = removal_results.iter().all(|&success| success);

    // Record cleanup time if successful
    if all_successful {
        let cleanup_timestamp_path = base.join(".last_cleanup");
        let timestamp = chrono::Utc::now().to_rfc3339();

        std::fs::write(&cleanup_timestamp_path, timestamp)
            .map(|_| tracing::info!("Recorded cleanup timestamp at {:?}", cleanup_timestamp_path))
            .unwrap_or_else(|e| tracing::warn!("Failed to write cleanup timestamp: {}", e));
    }

    Ok(RuntimeCacheResetApiResp::success(RuntimeCacheResetResp {
        success: all_successful,
    }))
}

async fn get_version_from_exec_async(path: &std::path::Path) -> Option<String> {
    let output = AsyncCommand::new(path).arg("--version").output().await.ok()?;

    output.status.success().then(|| {
        let version_string = String::from_utf8_lossy(&output.stdout).trim().to_string();

        (!version_string.is_empty()).then_some(version_string)
    })?
}

fn calculate_dir_size(dir_path: &std::path::Path) -> u64 {
    fs::read_dir(dir_path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| {
                    let path = entry.path();
                    match path.is_dir() {
                        true => calculate_dir_size(&path),
                        false => entry.metadata().map(|m| m.len()).unwrap_or(0),
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

fn count_packages_in_cache(cache_path: &std::path::Path) -> u64 {
    fs::read_dir(cache_path)
        .map(|entries| entries.filter_map(Result::ok).count() as u64)
        .unwrap_or(0)
}

fn get_last_modified(path: &std::path::Path) -> Option<String> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map(|time| {
            time.duration_since(UNIX_EPOCH)
                .map(|dur| {
                    chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| dur.as_secs().to_string())
                })
                .unwrap_or_else(|_| "unknown".to_string())
        })
        .ok()
}

fn get_last_cleanup_time() -> Option<String> {
    let paths = global_paths();
    let cleanup_file = paths.cache_dir().join(".last_cleanup");

    cleanup_file.exists().then(|| {
        std::fs::read_to_string(&cleanup_file)
            .ok()
            .map(|s| s.trim().to_string())
    })?
}
