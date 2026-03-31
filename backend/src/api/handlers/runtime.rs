use chrono;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Map, Value};
use tokio::{process::Command as AsyncCommand, task};
use walkdir::WalkDir;

use crate::{
    api::{models::runtime::*, routes::AppState},
    audit::{AuditAction, AuditStatus},
    common::{RuntimeType, paths::global_paths},
    runtime::{RuntimeInstaller, RuntimeManager},
};

pub async fn install(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<RuntimeInstallReq>,
) -> Result<Json<RuntimeInstallResp>, StatusCode> {
    let started_at = std::time::Instant::now();
    let runtime_type = request.runtime_type.clone();

    let result = runtime_install_core(&request, &app_state).await;

    let (audit_status, audit_error) = match &result {
        Ok(resp) => {
            if resp.data.as_ref().map(|d| d.success).unwrap_or(false) {
                (AuditStatus::Success, None)
            } else {
                (
                    AuditStatus::Failed,
                    resp.data
                        .as_ref()
                        .and_then(|d| if d.success { None } else { Some(d.message.clone()) }),
                )
            }
        }
        Err(e) => (AuditStatus::Failed, Some(e.to_string())),
    };

    let mut data = Map::new();
    data.insert("runtime_type".to_string(), Value::String(runtime_type.clone()));
    if let Ok(ref resp) = result {
        if let Some(ref inner) = resp.data {
            data.insert("success".to_string(), Value::Bool(inner.success));
            data.insert("message".to_string(), Value::String(inner.message.clone()));
        }
    }

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            AuditAction::RuntimeInstall,
            audit_status,
            "POST",
            "/api/runtime/install",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            None,
            Some(data),
            audit_error,
        ),
    )
    .await;

    result.map(Json)
}

pub async fn status(State(app_state): State<Arc<AppState>>) -> Result<Json<RuntimeStatusResp>, StatusCode> {
    let result = runtime_status_core(&app_state).await?;
    Ok(Json(result))
}

pub async fn cache(State(app_state): State<Arc<AppState>>) -> Result<Json<RuntimeCacheResp>, StatusCode> {
    let result = runtime_cache_core(&app_state).await?;
    Ok(Json(result))
}

pub async fn reset_cache(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<RuntimeCacheResetReq>,
) -> Result<Json<RuntimeCacheResetResp>, StatusCode> {
    let started_at = std::time::Instant::now();
    let cache_type = request.cache_type.clone();

    let result = reset_cache_core(&request, &app_state).await;

    let (audit_status, audit_error) = match &result {
        Ok(resp) => {
            if resp.data.as_ref().map(|d| d.success).unwrap_or(false) {
                (AuditStatus::Success, None)
            } else {
                (AuditStatus::Failed, None)
            }
        }
        Err(e) => (AuditStatus::Failed, Some(e.to_string())),
    };

    let mut data = Map::new();
    data.insert("cache_type".to_string(), Value::String(cache_type));
    if let Ok(ref resp) = result {
        if let Some(ref inner) = resp.data {
            data.insert("success".to_string(), Value::Bool(inner.success));
        }
    }

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            AuditAction::RuntimeCacheReset,
            audit_status,
            "POST",
            "/api/runtime/cache/reset",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            None,
            Some(data),
            audit_error,
        ),
    )
    .await;

    result.map(Json)
}

async fn runtime_install_core(
    request: &RuntimeInstallReq,
    app_state: &AppState,
) -> Result<RuntimeInstallResp, StatusCode> {
    let runtime_type: RuntimeType = request.runtime_type.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let manager = RuntimeManager::new();

    // Early return if already installed
    if manager.is_installed(runtime_type) {
        tracing::info!("Runtime {} already installed", runtime_type);

        return Ok(RuntimeInstallResp::success(RuntimeInstallData {
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
            RuntimeInstallResp::success(RuntimeInstallData {
                success: true,
                message: format!("Successfully downloaded and installed {}", request.runtime_type),
                runtime_type: request.runtime_type.clone(),
            })
        })
        .or_else(|e| {
            Ok(RuntimeInstallResp::success(RuntimeInstallData {
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

async fn runtime_status_core(_app_state: &AppState) -> Result<RuntimeStatusResp, StatusCode> {
    let (uv_status, bun_status) = tokio::join!(
        create_runtime_status(RuntimeType::Uv),
        create_runtime_status(RuntimeType::Bun)
    );

    Ok(RuntimeStatusResp::success(RuntimeStatusData {
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

async fn runtime_cache_core(_app_state: &AppState) -> Result<RuntimeCacheResp, StatusCode> {
    let (uv_cache, bun_cache) = tokio::join!(get_cache_info(RuntimeType::Uv), get_cache_info(RuntimeType::Bun));

    Ok(RuntimeCacheResp::success(RuntimeCacheData {
        summary: RuntimeCacheSummary {
            total_size_bytes: uv_cache.size_bytes + bun_cache.size_bytes,
            last_cleanup: get_last_cleanup_time(),
        },
        uv: uv_cache,
        bun: bun_cache,
    }))
}

async fn get_cache_info(runtime_type: RuntimeType) -> RuntimeCacheItem {
    let paths = global_paths();
    let cache_path = paths.runtime_cache_dir(runtime_type.as_str());
    let cache_path_string = cache_path.to_string_lossy().to_string();

    let stats = task::spawn_blocking(move || collect_cache_stats(&cache_path))
        .await
        .unwrap_or_default();

    RuntimeCacheItem {
        path: cache_path_string,
        size_bytes: stats.size_bytes,
        package_count: stats.package_count,
        last_modified: stats.last_modified,
    }
}

async fn reset_cache_core(
    request: &RuntimeCacheResetReq,
    _app_state: &AppState,
) -> Result<RuntimeCacheResetResp, StatusCode> {
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

    Ok(RuntimeCacheResetResp::success(RuntimeCacheResetData {
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

#[derive(Default)]
struct CacheStats {
    size_bytes: u64,
    package_count: u64,
    last_modified: Option<String>,
}

fn collect_cache_stats(cache_path: &std::path::Path) -> CacheStats {
    if !cache_path.exists() {
        return CacheStats::default();
    }

    let mut size_bytes = 0u64;
    let mut package_count = 0u64;
    let mut latest_modified: Option<SystemTime> = None;

    for entry in WalkDir::new(cache_path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let depth = entry.depth();
        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => continue,
        };

        if metadata.is_file() {
            size_bytes = size_bytes.saturating_add(metadata.len());
        }

        if depth == 1 {
            package_count = package_count.saturating_add(1);
        }

        if let Ok(modified) = metadata.modified() {
            latest_modified = match latest_modified {
                Some(existing) if existing > modified => Some(existing),
                _ => Some(modified),
            };
        }
    }

    let last_modified = latest_modified.and_then(|time| {
        time.duration_since(UNIX_EPOCH).ok().map(|dur| {
            chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| dur.as_secs().to_string())
        })
    });

    CacheStats {
        size_bytes,
        package_count,
        last_modified,
    }
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
