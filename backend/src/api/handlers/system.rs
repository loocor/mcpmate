// MCP Proxy API handlers for system management
// Contains handler functions for system endpoints

use std::{collections::HashMap, sync::Arc};

use axum::{Json, extract::State};
use serde_json::{Map, Value};

use super::ApiError;
use crate::api::models::system::ManagementActionResp;
use crate::api::{
    models::system::{
        SystemMetricsResp, SystemSettingsData, SystemSettingsResp, SystemSettingsUpdateReq, SystemStatusResp,
    },
    routes::AppState,
};
use crate::audit::{AuditAction, AuditStatus};
use crate::system::config::{
    api_url_from_port, bind_socket_addr, get_runtime_port_config, init_port_config, mcp_http_url_from_port,
};

const PROXY_NOT_AVAILABLE_ERROR: &str = "Proxy server not available";

fn system_settings_data(settings: crate::system::settings::SystemSettings) -> SystemSettingsData {
    SystemSettingsData {
        api_port: settings.api_port,
        mcp_port: settings.mcp_port,
        api_url: api_url_from_port(settings.api_port),
        mcp_http_url: mcp_http_url_from_port(settings.mcp_port),
        first_contact_behavior: settings.first_contact_behavior.as_str().to_string(),
        onboarding_policy: settings.onboarding_policy().as_str().to_string(),
        inspector_timeout_ms: settings.inspector_timeout_ms,
        default_config_mode: settings.default_config_mode,
    }
}

fn build_management_action_data(
    operation: &str,
    errors: &[String],
) -> Map<String, Value> {
    let mut data = Map::new();
    data.insert("operation".to_string(), Value::String(operation.to_string()));
    data.insert("error_count".to_string(), Value::from(errors.len() as u64));
    if !errors.is_empty() {
        data.insert(
            "errors".to_string(),
            Value::Array(errors.iter().cloned().map(Value::String).collect()),
        );
    }
    data
}

fn audit_status_for_errors(errors: &[String]) -> AuditStatus {
    if errors.is_empty() {
        AuditStatus::Success
    } else {
        AuditStatus::Failed
    }
}

fn joined_errors(errors: &[String]) -> Option<String> {
    (!errors.is_empty()).then(|| errors.join("; "))
}

/// Get system status
pub async fn get_status(State(state): State<Arc<AppState>>) -> Result<Json<SystemStatusResp>, ApiError> {
    // Get all servers count (including disabled)
    let mut total_servers = 0;
    if let Some(http_proxy) = &state.http_proxy {
        if let Some(db) = &http_proxy.database {
            // Use database connection to get server count
            match crate::config::server::get_all_servers(&db.pool).await {
                Ok(servers) => {
                    total_servers = servers.len();
                }
                Err(e) => {
                    tracing::error!("Failed to get servers from database: {}", e);
                    // Don't update total_servers if it fails
                }
            }
        }
    }

    // Use lightweight server status summary to avoid heavy cloning
    let summary = match tokio::time::timeout(std::time::Duration::from_millis(500), state.connection_pool.lock()).await
    {
        Ok(pool) => pool.get_server_status_summary(),
        Err(_) => {
            tracing::warn!("Connection pool status summary timeout (500ms), returning empty summary");
            HashMap::new()
        }
    };

    // If we can't get the server count from the database, use the number of servers in summary
    if total_servers == 0 {
        total_servers = summary.len();
    }

    let connected_servers = summary.values().filter(|(_, ready, _)| *ready > 0).count();

    Ok(Json(SystemStatusResp {
        status: "running".to_string(),
        uptime: get_uptime_seconds(),
        total_servers,
        connected_servers,
    }))
}

pub async fn get_settings(State(state): State<Arc<AppState>>) -> Result<Json<SystemSettingsResp>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".into()))?;

    let settings = crate::system::settings::get_settings(&db.pool)
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    Ok(Json(SystemSettingsResp::success(system_settings_data(settings))))
}

pub async fn set_settings(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SystemSettingsUpdateReq>,
) -> Result<Json<SystemSettingsResp>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".into()))?;

    let mut settings = crate::system::settings::get_settings(&db.pool)
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    let previous_ports = (settings.api_port, settings.mcp_port);

    if let Some(api_port) = request.api_port {
        settings.api_port = api_port;
    }

    if let Some(mcp_port) = request.mcp_port {
        settings.mcp_port = mcp_port;
    }

    if let Some(first_contact_behavior) = request.first_contact_behavior {
        settings.first_contact_behavior = first_contact_behavior
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid first contact behavior".to_string()))?;
    }

    if let Some(inspector_timeout_ms) = request.inspector_timeout_ms {
        settings.inspector_timeout_ms = inspector_timeout_ms;
    }

    if let Some(default_config_mode) = request.default_config_mode {
        settings.default_config_mode = default_config_mode;
    }

    let settings = crate::system::settings::set_settings(&db.pool, &settings)
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    if previous_ports != (settings.api_port, settings.mcp_port) {
        init_port_config(settings.api_port, settings.mcp_port);
    }

    let mut data = Map::new();
    data.insert("api_port".to_string(), Value::from(settings.api_port));
    data.insert("mcp_port".to_string(), Value::from(settings.mcp_port));
    data.insert(
        "first_contact_behavior".to_string(),
        Value::String(settings.first_contact_behavior.as_str().to_string()),
    );
    data.insert(
        "inspector_timeout_ms".to_string(),
        Value::from(settings.inspector_timeout_ms),
    );
    data.insert(
        "default_config_mode".to_string(),
        Value::String(settings.default_config_mode.clone()),
    );

    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            AuditAction::ClientSettingsUpdate,
            AuditStatus::Success,
            "POST",
            "/api/system/settings",
            None,
            None,
            None,
            Some(data),
            None,
        ),
    )
    .await;

    Ok(Json(SystemSettingsResp::success(system_settings_data(settings))))
}

/// Get system metrics
pub async fn get_metrics(State(state): State<Arc<AppState>>) -> Result<Json<SystemMetricsResp>, ApiError> {
    // We'll get metrics directly from sysinfo instead of the metrics collector

    // Get connection pool metrics
    let pool = state.connection_pool.lock().await;

    // Count instances by status
    let mut total_instances_count = 0;
    let mut ready_instances_count = 0;
    let mut error_instances_count = 0;
    let mut initializing_instances_count = 0;
    let mut busy_instances_count = 0;
    let mut shutdown_instances_count = 0;
    let mut total_tools_count = 0;
    let mut unique_tools = std::collections::HashSet::new();

    // Count connected servers
    let mut connected_servers_count = 0;

    // Iterate through all instances
    for instances in pool.connections.values() {
        let mut server_has_ready_instance = false;

        for conn in instances.values() {
            total_instances_count += 1;

            // Count by status
            if conn.is_connected() {
                ready_instances_count += 1;
                server_has_ready_instance = true;
            } else {
                // Use string representation for simplicity
                match conn.status_string().as_str() {
                    "error" => error_instances_count += 1,
                    "initializing" => initializing_instances_count += 1,
                    "busy" => busy_instances_count += 1,
                    "shutdown" => shutdown_instances_count += 1,
                    _ => {} // Unknown status
                }
            }

            // Count tools
            total_tools_count += conn.tools.len();
            for tool in &conn.tools {
                unique_tools.insert(tool.name.clone());
            }
        }

        // Count connected servers
        if server_has_ready_instance {
            connected_servers_count += 1;
        }
    }

    // Get system metrics using sysinfo
    let mut system = sysinfo::System::new();
    system.refresh_all();

    // Get current process ID
    let pid = std::process::id();

    // Get process metrics
    let (cpu_usage, memory_usage) = if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
        (Some(process.cpu_usage()), Some(process.memory()))
    } else {
        (None, None)
    };

    // Get system metrics
    let system_cpu_usage = Some(system.global_cpu_info().cpu_usage());
    let system_memory_usage = Some(system.used_memory());
    let system_memory_total = Some(system.total_memory());

    // Get current timestamp
    let timestamp = chrono::Local::now().to_rfc3339();

    // Get uptime
    let uptime_seconds = get_uptime_seconds();

    // Get configuration application status
    let config_application_status = state.config_application_state.get_current_status().await;

    Ok(Json(SystemMetricsResp {
        uptime_seconds,
        timestamp,
        connected_servers_count,
        total_instances_count,
        ready_instances_count,
        error_instances_count,
        initializing_instances_count,
        busy_instances_count,
        shutdown_instances_count,
        total_tools_count,
        unique_tools_count: unique_tools.len(),
        cpu_usage,
        memory_usage,
        system_cpu_usage,
        system_memory_usage,
        system_memory_total,
        config_application_status,
    }))
}

/// Management: graceful shutdown (delegates to management handlers)
pub async fn shutdown(State(state): State<Arc<AppState>>) -> Result<Json<ManagementActionResp>, ApiError> {
    let started_at = std::time::Instant::now();

    let Some(proxy) = state.http_proxy.clone() else {
        crate::audit::interceptor::emit_event(
            state.audit_service.as_ref(),
            crate::audit::interceptor::build_rest_event(
                AuditAction::LocalCoreServiceStop,
                AuditStatus::Failed,
                "POST",
                "/api/system/shutdown",
                Some(started_at.elapsed().as_millis() as u64),
                None,
                None,
                None,
                Some(PROXY_NOT_AVAILABLE_ERROR.to_string()),
            ),
        )
        .await;
        return Err(ApiError::InternalError(PROXY_NOT_AVAILABLE_ERROR.into()));
    };

    let mut errors = Vec::new();

    if let Err(err) = proxy.initiate_shutdown().await {
        tracing::warn!(error = %err, "Failed to initiate proxy shutdown");
        errors.push(format!("initiate_shutdown: {err}"));
    }
    if let Err(err) = proxy.complete_shutdown().await {
        tracing::warn!(error = %err, "Failed to complete proxy shutdown");
        errors.push(format!("complete_shutdown: {err}"));
    }

    let data = build_management_action_data("shutdown", &errors);

    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            AuditAction::LocalCoreServiceStop,
            audit_status_for_errors(&errors),
            "POST",
            "/api/system/shutdown",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            None,
            Some(data),
            joined_errors(&errors),
        ),
    )
    .await;

    Ok(Json(ManagementActionResp::shutting_down()))
}

/// Management: restart proxy service (delegates to management handlers)
pub async fn restart(State(state): State<Arc<AppState>>) -> Result<Json<ManagementActionResp>, ApiError> {
    use std::{net::SocketAddr, time::Duration};

    let started_at = std::time::Instant::now();

    let Some(proxy) = state.http_proxy.clone() else {
        crate::audit::interceptor::emit_event(
            state.audit_service.as_ref(),
            crate::audit::interceptor::build_rest_event(
                AuditAction::LocalCoreServiceRestart,
                AuditStatus::Failed,
                "POST",
                "/api/system/restart",
                Some(started_at.elapsed().as_millis() as u64),
                None,
                None,
                None,
                Some(PROXY_NOT_AVAILABLE_ERROR.to_string()),
            ),
        )
        .await;
        return Err(ApiError::InternalError(PROXY_NOT_AVAILABLE_ERROR.into()));
    };

    let mut errors = Vec::new();

    // Clear capabilities cache as part of restart to force fresh capability discovery
    if let Err(e) = state.redb_cache.clear_all().await {
        tracing::warn!(error = %e, "Failed to clear capabilities cache during restart");
        errors.push(format!("clear_cache: {e}"));
    }

    if let Err(err) = proxy.initiate_shutdown().await {
        tracing::warn!(error = %err, "Failed to initiate proxy shutdown before restart");
        errors.push(format!("initiate_shutdown: {err}"));
    }
    if let Err(err) = proxy.complete_shutdown().await {
        tracing::warn!(error = %err, "Failed to complete proxy shutdown before restart");
        errors.push(format!("complete_shutdown: {err}"));
    }

    tokio::time::sleep(Duration::from_millis(150)).await;

    let mcp_port = get_runtime_port_config().mcp_port;
    let bind_address: SocketAddr =
        bind_socket_addr(mcp_port).map_err(|e| ApiError::InternalError(format!("Invalid MCP bind address: {}", e)))?;

    let start_result = proxy.start_unified(bind_address).await;

    let mut data = build_management_action_data("restart", &errors);
    data.insert("mcp_port".to_string(), Value::from(mcp_port));

    match start_result {
        Ok(_handle) => {
            crate::audit::interceptor::emit_event(
                state.audit_service.as_ref(),
                crate::audit::interceptor::build_rest_event(
                    AuditAction::LocalCoreServiceRestart,
                    audit_status_for_errors(&errors),
                    "POST",
                    "/api/system/restart",
                    Some(started_at.elapsed().as_millis() as u64),
                    None,
                    None,
                    Some(data),
                    joined_errors(&errors),
                ),
            )
            .await;
            Ok(Json(ManagementActionResp::restarted(mcp_port, "uni")))
        }
        Err(err) => {
            errors.push(format!("start_unified: {err}"));
            let mut failed_data = data;
            failed_data.insert(
                "errors".to_string(),
                Value::Array(errors.iter().cloned().map(Value::String).collect()),
            );
            failed_data.insert("error_count".to_string(), Value::from(errors.len() as u64));
            crate::audit::interceptor::emit_event(
                state.audit_service.as_ref(),
                crate::audit::interceptor::build_rest_event(
                    AuditAction::LocalCoreServiceRestart,
                    AuditStatus::Failed,
                    "POST",
                    "/api/system/restart",
                    Some(started_at.elapsed().as_millis() as u64),
                    None,
                    None,
                    Some(failed_data),
                    Some(errors.join("; ")),
                ),
            )
            .await;
            Err(ApiError::InternalError(format!("Failed to restart proxy: {}", err)))
        }
    }
}

use std::sync::atomic::{AtomicU64, Ordering};

// Static variable to store the server start time
static SERVER_START_TIME: AtomicU64 = AtomicU64::new(0);

/// Initialize the server start time
/// This should be called once when the server starts
pub fn initialize_server_start_time() {
    // Only set if not already set
    if SERVER_START_TIME.load(Ordering::Relaxed) == 0 {
        // Get current time as seconds since UNIX epoch
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        SERVER_START_TIME.store(now, Ordering::Relaxed);
        tracing::info!("Server start time initialized: {}", now);
    }
}

/// Get system uptime in seconds
fn get_uptime_seconds() -> u64 {
    let start_time = SERVER_START_TIME.load(Ordering::Relaxed);

    // If start time is not initialized, return 0
    if start_time == 0 {
        return 0;
    }

    // Get current time as seconds since UNIX epoch
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Calculate uptime
    now.saturating_sub(start_time)
}
