// MCP Proxy API handlers for system management
// Contains handler functions for system endpoints

use std::{collections::HashMap, sync::Arc};

use axum::{Json, extract::State};

use super::ApiError;
use crate::api::{
    models::system::{SystemMetricsResp, SystemStatusResp},
    routes::AppState,
};

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
