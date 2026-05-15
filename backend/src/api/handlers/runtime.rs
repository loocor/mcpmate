use chrono;
use std::path::Path;
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
    runtime::{CommandResolver, RuntimeInstaller, RuntimeManager, ResolveSource},
};

pub async fn install(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<RuntimeInstallReq>,
) -> Result<Json<RuntimeInstallResp>, StatusCode> {
    let started_at = std::time::Instant::now();
    let runtime_type = request.runtime_type;

    let result = runtime_install_core(&request).await;

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
    data.insert("runtime_type".to_string(), Value::String(runtime_type.to_string()));
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

async fn runtime_install_core(request: &RuntimeInstallReq) -> Result<RuntimeInstallResp, StatusCode> {
    let runtime_type = request.runtime_type;

    let manager = RuntimeManager::new();
    let existing_installation = manager.is_installed(runtime_type);

    tracing::info!(
        "Runtime {} {}",
        runtime_type,
        if existing_installation {
            "already exists, proceeding with repair/reinstall"
        } else {
            "not found, proceeding with download and installation"
        }
    );

    let install_result = perform_installation(request, runtime_type).await;

    let data = match install_result {
        Ok(()) => RuntimeInstallData {
            success: true,
            message: format!("Successfully downloaded and installed {}", runtime_type),
            runtime_type,
        },
        Err(error) => {
            let message = build_install_failure_message(&manager, runtime_type, &error);
            RuntimeInstallData {
                success: false,
                message,
                runtime_type,
            }
        }
    };

    Ok(RuntimeInstallResp::success(data))
}

async fn perform_installation(
    request: &RuntimeInstallReq,
    runtime_type: RuntimeType,
) -> Result<(), anyhow::Error> {
    let max_attempts = request.max_retries.unwrap_or(0).saturating_add(1).clamp(1, 5);
    let mut last_error: Option<anyhow::Error> = None;

    for attempt in 1..=max_attempts {
        match RuntimeInstaller::new()
            .install_runtime(runtime_type, request.version.as_deref())
            .await
        {
            Ok(_) => return Ok(()),
            Err(error) => {
                tracing::warn!(
                    "Runtime {} install attempt {}/{} failed: {:#}",
                    runtime_type,
                    attempt,
                    max_attempts,
                    error
                );

                // Fail fast on deterministic (non-network) errors
                if !looks_like_network_error(&error) {
                    return Err(error);
                }

                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Runtime installation failed")))
}

fn build_install_failure_message(
    manager: &RuntimeManager,
    runtime_type: RuntimeType,
    error: &anyhow::Error,
) -> String {
    let mut message = format!("Installation failed: {error:#}");

    if looks_like_network_error(error) {
        message.push_str(" Network may be unstable. Please check your connection and retry installation.");
    }

    if let Some(path) = manager.get_executable_path(runtime_type) {
        message.push_str(&format!(
            " Existing MCPMate-managed runtime was detected at {}.",
            path.display()
        ));
    }

    message
}

fn looks_like_network_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        let message = cause.to_string().to_ascii_lowercase();
        message.contains("timed out")
            || message.contains("timeout")
            || message.contains("connection reset")
            || message.contains("connection refused")
            || message.contains("network")
            || message.contains("dns")
    })
}

async fn runtime_status_core(_app_state: &AppState) -> Result<RuntimeStatusResp, StatusCode> {
    let (uv_status, bun_status, node_status) = tokio::join!(
        create_runtime_status(RuntimeType::Uv),
        create_runtime_status(RuntimeType::Bun),
        create_runtime_status(RuntimeType::Node)
    );

    let user_home = dirs::home_dir().map(|p| p.to_string_lossy().to_string());

    Ok(RuntimeStatusResp::success(RuntimeStatusData {
        user_home,
        uv: uv_status,
        bun: bun_status,
        node: node_status,
    }))
}

async fn create_runtime_status(runtime_type: RuntimeType) -> RuntimeStatus {
    let manager = RuntimeManager::new();
    let available = manager.is_installed(runtime_type);
    let path = manager.get_executable_path(runtime_type);
    let paths = global_paths().clone();
    let cmd = runtime_type.canonical_command();

    // Use unified resolver for system fallback detection
    let system_fallback_path = if !available {
        CommandResolver::new(&paths)
            .resolve(cmd)
            .filter(|r| r.source == ResolveSource::SystemPath)
            .map(|r| r.path.to_string_lossy().to_string())
    } else {
        None
    };

    let version = if let Some(ref p) = path {
        get_version_from_exec_async(p.as_path()).await
    } else if let Some(ref sys) = system_fallback_path {
        get_version_from_exec_async(Path::new(sys)).await
    } else {
        None
    };

    let message = if available {
        manager
            .list_installed()
            .into_iter()
            .find(|info| info.runtime_type == runtime_type)
            .map(|info| info.message)
            .unwrap_or_else(|| format!("✓ {} is available (MCPMate managed)", runtime_type))
    } else if let Some(sys_path) = system_fallback_path.as_ref() {
        format!("✓ {} is available (system fallback at {})", runtime_type, sys_path)
    } else {
        format!("✗ {} is not installed", runtime_type)
    };

    RuntimeStatus {
        runtime_type,
        available,
        path: path.as_ref().map(|p| p.to_string_lossy().to_string()),
        version,
        system_fallback_path,
        message,
    }
}

async fn runtime_cache_core(_app_state: &AppState) -> Result<RuntimeCacheResp, StatusCode> {
    let (uv_cache, bun_cache, node_cache) = tokio::join!(
        get_cache_info(RuntimeType::Uv),
        get_cache_info(RuntimeType::Bun),
        get_cache_info(RuntimeType::Node)
    );

    Ok(RuntimeCacheResp::success(RuntimeCacheData {
        summary: RuntimeCacheSummary {
            total_size_bytes: uv_cache.size_bytes + bun_cache.size_bytes + node_cache.size_bytes,
            last_cleanup: get_last_cleanup_time(),
        },
        uv: uv_cache,
        bun: bun_cache,
        node: node_cache,
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
        "node" => vec![base.join(RuntimeType::Node.as_str())],
        _ => vec![
            base.join(RuntimeType::Uv.as_str()),
            base.join(RuntimeType::Bun.as_str()),
            base.join(RuntimeType::Node.as_str()),
        ],
    };

    let cleanup_timestamp_path = base.join(".last_cleanup");

    let (all_successful, _removed_count) = task::spawn_blocking(move || {
        let mut removed = 0usize;
        let mut all_ok = true;
        for p in &cache_paths {
            if p.exists() {
                match std::fs::remove_dir_all(p) {
                    Ok(()) => {
                        tracing::info!("Removed runtime cache dir: {:?}", p);
                        removed += 1;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to remove {:?}: {}", p, e);
                        all_ok = false;
                    }
                }
            }
        }

        if all_ok {
            let timestamp = chrono::Utc::now().to_rfc3339();
            match std::fs::write(&cleanup_timestamp_path, timestamp) {
                Ok(()) => tracing::info!("Recorded cleanup timestamp at {:?}", cleanup_timestamp_path),
                Err(e) => tracing::warn!("Failed to write cleanup timestamp: {}", e),
            }
        }

        (all_ok, removed)
    })
    .await
    .unwrap_or((false, 0));

    Ok(RuntimeCacheResetResp::success(RuntimeCacheResetData {
        success: all_successful,
    }))
}

async fn get_version_from_exec_async(path: &std::path::Path) -> Option<String> {
    let output = match AsyncCommand::new(path).arg("--version").output().await {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("Failed to get version from {:?}: {}", path, e);
            return None;
        }
    };

    output.status.success().then(|| {
        let version_string = String::from_utf8_lossy(&output.stdout).trim().to_string();
        normalize_version_output(&version_string)
    })?
}

fn normalize_version_output(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed
        .split_whitespace()
        .map(|segment| segment.trim_matches(|c: char| matches!(c, ',' | ';' | '(' | ')')))
        .find_map(|segment| {
            let candidate = segment.strip_prefix('v').unwrap_or(segment);
            candidate
                .chars()
                .next()
                .filter(|ch| ch.is_ascii_digit())
                .map(|_| candidate.to_string())
        });

    normalized.or_else(|| Some(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::normalize_version_output;

    #[test]
    fn normalizes_plain_node_version_output() {
        assert_eq!(normalize_version_output("v24.15.0"), Some("24.15.0".to_string()));
    }

    #[test]
    fn normalizes_bun_version_output() {
        assert_eq!(normalize_version_output("1.2.15"), Some("1.2.15".to_string()));
    }

    #[test]
    fn normalizes_tool_prefixed_version_output() {
        assert_eq!(
            normalize_version_output("uv 0.7.13 (Homebrew 2025-01-01)"),
            Some("0.7.13".to_string())
        );
    }
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
