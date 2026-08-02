//! Pool health check functionality
//!
//! Provides health monitoring and automatic recovery for UpstreamConnectionPool including:
//! - periodic connection health checks
//! - automatic reconnection on failures
//! - exponential backoff for retry logic
//! - process resource monitoring

use std::sync::Arc;

use anyhow::Result;
use tokio::{sync::Mutex, time::sleep};
use tracing;

use super::UpstreamConnectionPool;
use crate::core::foundation::types::{
    ConnectionStatus, // status of the connection
    ErrorType,        // type of the error
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReconnectIntent {
    server_id: String,
    instance_id: String,
    instance_created_at: std::time::Instant,
}

impl UpstreamConnectionPool {
    /// Start health check task with adaptive scheduling
    pub fn start_health_check(connection_pool: Arc<Mutex<Self>>) {
        // Start the main health check task
        let health_check_pool = connection_pool.clone();
        tokio::spawn(async move {
            let mut consecutive_failures = 0u32;
            let mut last_reconnection_count = 0usize;
            let mut quiet_cycles = 0u32;

            loop {
                // Adaptive interval based on system health
                let interval = Self::calculate_health_check_interval(consecutive_failures, last_reconnection_count > 0);
                sleep(interval).await;

                // Step 1: Collect reconnection candidates with minimal lock time
                let reconnects = {
                    // Use timeout to avoid indefinite blocking
                    let pool_guard = match tokio::time::timeout(
                        std::time::Duration::from_millis(500), // 500ms timeout
                        health_check_pool.lock(),
                    )
                    .await
                    {
                        Ok(guard) => guard,
                        Err(_) => {
                            consecutive_failures += 1;
                            Self::log_with_backoff(
                                &mut quiet_cycles,
                                "Health check: Timeout acquiring pool lock, skipping this cycle",
                                tracing::Level::WARN,
                            );
                            continue;
                        }
                    };

                    Self::collect_reconnection_candidates(&pool_guard)
                };

                // Step 2: Check connection status (separate from reconnection logic)
                {
                    let mut pool =
                        match tokio::time::timeout(std::time::Duration::from_millis(500), health_check_pool.lock())
                            .await
                        {
                            Ok(guard) => guard,
                            Err(_) => {
                                consecutive_failures += 1;
                                Self::log_with_backoff(
                                    &mut quiet_cycles,
                                    "Health check: Timeout acquiring pool lock for status check, skipping",
                                    tracing::Level::WARN,
                                );
                                continue;
                            }
                        };

                    if let Err(e) = pool.check_connection_status().await {
                        consecutive_failures += 1;
                        Self::log_with_backoff(
                            &mut quiet_cycles,
                            &format!("Error checking connection status: {}", e),
                            tracing::Level::ERROR,
                        );
                    } else {
                        consecutive_failures = 0; // Reset on success
                    }
                }

                // Step 3: Process reconnections asynchronously outside the lock
                if !reconnects.is_empty() {
                    last_reconnection_count = reconnects.len();

                    // Use adaptive logging for reconnection info
                    if quiet_cycles < 5 || reconnects.len() > 3 {
                        tracing::info!(
                            "Health check: Processing {} reconnection(s) asynchronously",
                            reconnects.len()
                        );
                    } else {
                        tracing::debug!(
                            "Health check: Processing {} reconnection(s) asynchronously",
                            reconnects.len()
                        );
                    }

                    // Process reconnections in parallel without holding the main lock
                    Self::process_reconnections_async(health_check_pool.clone(), reconnects).await;
                } else {
                    last_reconnection_count = 0;
                    quiet_cycles += 1;
                }
            }
        });

        // Start a separate process monitoring task with shorter interval
        let process_monitor_pool = connection_pool.clone();
        tokio::spawn(async move {
            // Wait a short time before starting to allow connections to initialize
            sleep(std::time::Duration::from_secs(5)).await;

            loop {
                // Wait for process monitoring interval (10 seconds)
                sleep(std::time::Duration::from_secs(10)).await;

                // Update process resource usage
                {
                    let mut pool = process_monitor_pool.lock().await;
                    if let Err(e) = pool.update_process_resources().await {
                        tracing::error!("Error updating process resources: {}", e);
                    }
                }
            }
        });
    }

    /// Check connection status for all instances
    pub async fn check_connection_status(&mut self) -> Result<()> {
        // Get all instances that need checking
        let instances_to_check = {
            let mut result = Vec::new();

            for (server_name, instances) in &self.connections {
                for (instance_id, conn) in instances {
                    if matches!(conn.status, ConnectionStatus::Busy) {
                        result.push((server_name.clone(), instance_id.clone()));
                    }
                }
            }

            result
        };

        // Check each instance
        for (server_name, instance_id) in instances_to_check {
            // Get the connection
            let conn = match self.get_instance(&server_name, &instance_id) {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            match &conn.status {
                // Busy state - check for persistent busy connections
                ConnectionStatus::Busy => {
                    let now = std::time::Instant::now();
                    let busy_timeout = std::time::Duration::from_secs(120); // 2 minutes

                    if now.duration_since(conn.last_health_check) > busy_timeout {
                        tracing::warn!(
                            "Connection check: Resetting persistent Busy connection to Ready: '{}' instance '{}'",
                            server_name,
                            instance_id
                        );

                        // Reset the connection status to Ready
                        if let Ok(mut_conn) = self.get_instance_mut(&server_name, &instance_id) {
                            mut_conn.status = ConnectionStatus::Ready;
                            mut_conn.last_health_check = now;
                        }
                    }
                }

                _ => {
                    // Candidate collection is the sole reconnect scheduler for this cycle.
                }
            }
        }

        Ok(())
    }

    /// Collect reconnection candidates without holding lock for long time
    fn collect_reconnection_candidates(
        pool_guard: &tokio::sync::MutexGuard<'_, UpstreamConnectionPool>
    ) -> Vec<ReconnectIntent> {
        let mut reconnects = Vec::new();
        let now = std::time::Instant::now();

        for (server_name, instances) in &pool_guard.connections {
            for (instance_id, conn) in instances {
                // Monitor connected, failed, and persistent Busy connections. Idle and Shutdown
                // instances are demand-driven and must not be started by health maintenance.
                match &conn.status {
                    ConnectionStatus::Ready => {
                        // Check if the service is still alive
                        if let Some(_service) = &conn.service {
                            // Periodic reconnect to ensure health (every 60 minutes)
                            if now > conn.last_connected
                                && now.duration_since(conn.last_connected) > std::time::Duration::from_secs(3600)
                            {
                                tracing::info!(
                                    "Health check triggering periodic reconnect for '{}' instance '{}' - Last connected: {:?} ago",
                                    server_name,
                                    instance_id,
                                    now.duration_since(conn.last_connected)
                                );
                                reconnects.push(ReconnectIntent {
                                    server_id: server_name.clone(),
                                    instance_id: instance_id.clone(),
                                    instance_created_at: conn.created_at,
                                });
                            }
                        } else {
                            // If service is None but status is Ready, something is wrong
                            tracing::warn!(
                                "Health check: Server '{}' instance '{}' has Ready status but no service, will reconnect",
                                server_name,
                                instance_id
                            );
                            reconnects.push(ReconnectIntent {
                                server_id: server_name.clone(),
                                instance_id: instance_id.clone(),
                                instance_created_at: conn.created_at,
                            });
                        }
                    }
                    ConnectionStatus::Disabled(_) => {
                        // Skip disabled servers completely
                        continue;
                    }
                    ConnectionStatus::Error(error_details) => {
                        // Skip permanent errors
                        if error_details.error_type == ErrorType::Permanent {
                            continue;
                        }

                        if pool_guard.automatic_recovery_exhausted(server_name) {
                            tracing::debug!(
                                server_id = %server_name,
                                "Health reconnect skipped because automatic recovery is exhausted"
                            );
                            continue;
                        }

                        // Respect pool-level backoff window
                        if let Some(remaining) = pool_guard.remaining_backoff(server_name) {
                            tracing::debug!(
                                "Health check: '{}' backing off for {:.1}s (Error state), skip reconnect",
                                server_name,
                                remaining.as_secs_f32()
                            );
                        } else {
                            tracing::debug!(
                                "Health check: Scheduling reconnect for '{}' instance '{}' (Error state, no backoff)",
                                server_name,
                                instance_id
                            );
                            reconnects.push(ReconnectIntent {
                                server_id: server_name.clone(),
                                instance_id: instance_id.clone(),
                                instance_created_at: conn.created_at,
                            });
                        }
                    }
                    ConnectionStatus::Busy => {
                        // Check for persistent Busy connections (stuck for more than 2 minutes)
                        let busy_timeout = std::time::Duration::from_secs(120); // 2 minutes

                        if now.duration_since(conn.last_health_check) > busy_timeout {
                            tracing::warn!(
                                "Health check: Found persistent Busy connection: '{}' instance '{}', will reset to Ready",
                                server_name,
                                instance_id
                            );
                            // Don't add to reconnects, we'll reset the status in check_connection_status
                        }
                    }
                    _ => {}
                }
            }
        }

        reconnects
    }

    /// Process reconnections asynchronously without blocking the main pool
    async fn process_reconnections_async(
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
        reconnects: Vec<ReconnectIntent>,
    ) {
        // Use SyncHelper for concurrent reconnection processing
        let _sync_result = crate::common::sync::SyncHelper::execute_concurrent_sync(
            reconnects,
            "health_check_reconnections",
            2, // Limit concurrent reconnections to avoid overwhelming
            move |intent| {
                let pool_clone = connection_pool.clone();
                async move {
                    match Self::reconnect_single_instance(pool_clone, intent.clone()).await {
                        Ok(()) => {
                            tracing::info!(
                                "Health check: Successfully reconnected '{}' instance '{}'",
                                intent.server_id,
                                intent.instance_id
                            );
                            Ok(())
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Health check: Failed to reconnect '{}' instance '{}': {}",
                                intent.server_id,
                                intent.instance_id,
                                e
                            );
                            Err(anyhow::anyhow!("Reconnection failed: {}", e))
                        }
                    }
                }
            },
        )
        .await;
    }

    /// Reconnect a single instance with proper error handling
    async fn reconnect_single_instance(
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
        intent: ReconnectIntent,
    ) -> Result<()> {
        let (mut worker, reconnect_epoch, config_fingerprint) = {
            let mut pool = tokio::time::timeout(std::time::Duration::from_secs(2), connection_pool.lock())
                .await
                .map_err(|_| anyhow::anyhow!("Timeout acquiring pool lock for reconnection"))?;

            let now = std::time::Instant::now();
            let should_reconnect = pool
                .get_instance(&intent.server_id, &intent.instance_id)
                .ok()
                .is_some_and(|connection| {
                    connection.created_at == intent.instance_created_at
                        && match &connection.status {
                            ConnectionStatus::Ready => {
                                connection.service.is_none()
                                    || now
                                        .checked_duration_since(connection.last_connected)
                                        .is_some_and(|elapsed| elapsed > std::time::Duration::from_secs(3600))
                            }
                            ConnectionStatus::Error(error) => error.error_type != ErrorType::Permanent,
                            _ => false,
                        }
                });
            if !should_reconnect {
                tracing::debug!(
                    server_id = %intent.server_id,
                    instance_id = %intent.instance_id,
                    "Health reconnect skipped because the instance is no longer eligible"
                );
                return Ok(());
            }
            if let Some(remaining) = pool.remaining_backoff(&intent.server_id) {
                tracing::debug!(
                    server_id = %intent.server_id,
                    instance_id = %intent.instance_id,
                    backoff_secs = remaining.as_secs_f32(),
                    "Health reconnect deferred by active backoff"
                );
                return Ok(());
            }
            if !pool.claim_automatic_recovery_attempt(&intent.server_id) {
                tracing::debug!(
                    server_id = %intent.server_id,
                    instance_id = %intent.instance_id,
                    "Health reconnect skipped because automatic recovery is exhausted"
                );
                return Ok(());
            }

            let config_fingerprint = pool
                .config
                .mcp_servers
                .get(&intent.server_id)
                .and_then(|config| config.source_fingerprint.clone());
            let reconnect_epoch = pool.begin_health_reconnect(&intent.server_id, &intent.instance_id);
            pool.get_instance_mut(&intent.server_id, &intent.instance_id)?
                .update_initializing();
            let mut worker = pool.runtime_worker();
            worker.database = None;
            (worker, reconnect_epoch, config_fingerprint)
        };

        // Transport startup and initialization run on a detached worker. The shared pool
        // lock is reacquired only to publish the result for the exact instance epoch.
        let result = worker
            .connect_transport_for_health(&intent.server_id, &intent.instance_id)
            .await;
        let mut worker_connection = worker
            .connections
            .get_mut(&intent.server_id)
            .and_then(|instances| instances.remove(&intent.instance_id));
        let worker_token = worker
            .cancellation_tokens
            .get_mut(&intent.server_id)
            .and_then(|tokens| tokens.remove(&intent.instance_id));

        let mut pool = connection_pool.lock().await;
        let current_config_fingerprint = pool
            .config
            .mcp_servers
            .get(&intent.server_id)
            .and_then(|config| config.source_fingerprint.clone());
        let still_owned = pool
            .get_instance(&intent.server_id, &intent.instance_id)
            .ok()
            .is_some_and(|connection| {
                connection.created_at == intent.instance_created_at
                    && matches!(connection.status, ConnectionStatus::Initializing)
            })
            && pool.health_reconnect_epoch(&intent.server_id, &intent.instance_id) == Some(reconnect_epoch)
            && current_config_fingerprint == config_fingerprint;
        if !still_owned {
            let owns_epoch =
                pool.health_reconnect_epoch(&intent.server_id, &intent.instance_id) == Some(reconnect_epoch);
            if owns_epoch {
                pool.invalidate_health_reconnect(&intent.server_id, &intent.instance_id);
                if let Ok(connection) = pool.get_instance_mut(&intent.server_id, &intent.instance_id)
                    && connection.created_at == intent.instance_created_at
                    && matches!(connection.status, ConnectionStatus::Initializing)
                {
                    connection.update_failed("Health reconnect invalidated before publication".to_string());
                }
            }
            tracing::debug!(
                server_id = %intent.server_id,
                instance_id = %intent.instance_id,
                "Discarding health reconnect result because instance ownership changed"
            );
            drop(pool);
            Self::discard_health_reconnect(worker_connection, worker_token).await;
            return Ok(());
        }

        if result.is_err()
            && let Some(connection) = worker_connection.as_mut()
            && matches!(connection.status, ConnectionStatus::Initializing)
        {
            connection.update_failed(format!(
                "Connection failed: {}",
                result.as_ref().expect_err("failed reconnect has an error")
            ));
        }
        let has_worker_connection = worker_connection.is_some();
        let replaced_connection = if let Some(connection) = worker_connection {
            pool.connections
                .entry(intent.server_id.clone())
                .or_default()
                .insert(intent.instance_id.clone(), connection)
        } else {
            None
        };
        if !has_worker_connection && let Err(error) = &result {
            pool.get_instance_mut(&intent.server_id, &intent.instance_id)?
                .update_failed(format!("Connection failed: {error}"));
        }
        let replaced_token = if let Some(token) = worker_token {
            pool.cancellation_tokens
                .entry(intent.server_id.clone())
                .or_default()
                .insert(intent.instance_id.clone(), token)
        } else {
            None
        };
        pool.invalidate_health_reconnect(&intent.server_id, &intent.instance_id);

        if result.is_ok() {
            pool.clear_failure_state(&intent.server_id);
        } else {
            let error = result.as_ref().expect_err("failed reconnect has an error");
            if crate::core::capability::connection_provider::PoolCapabilityConnectionProvider::authentication_failure_code(
                error,
            )
            .is_some()
            {
                pool.clear_failure_state(&intent.server_id);
            } else {
                pool.register_claimed_recovery_failure(
                    &intent.server_id,
                    crate::core::pool::FailureKind::Connect,
                    Some(error.to_string()),
                );
            }
        }
        let event_database = pool.database.clone();
        let event_error = result.as_ref().err().map(ToString::to_string);
        drop(pool);

        Self::discard_health_reconnect(replaced_connection, replaced_token).await;
        Self::publish_startup_result(event_database.clone(), &intent.server_id, result.is_ok(), event_error).await;
        if result.is_ok() {
            Self::spawn_coordinated_capability_sync(event_database, connection_pool.clone(), intent.server_id.clone());
        }

        result.map_err(|error| anyhow::anyhow!("Failed to trigger reconnect: {error}"))
    }

    async fn discard_health_reconnect(
        connection: Option<crate::core::pool::UpstreamConnection>,
        cancellation: Option<tokio_util::sync::CancellationToken>,
    ) {
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
        let Some(mut connection) = connection else {
            return;
        };
        let Some(service) = connection.service.take() else {
            return;
        };
        service.cancellation_token().cancel();
        match Arc::try_unwrap(service) {
            Ok(service) => {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(5), service.cancel()).await;
            }
            Err(_) => {
                tracing::warn!(
                    instance_id = %connection.id,
                    "Discarded health reconnect service still has another owner"
                );
            }
        }
    }

    /// Calculate adaptive health check interval based on system health
    fn calculate_health_check_interval(
        consecutive_failures: u32,
        has_recent_reconnections: bool,
    ) -> std::time::Duration {
        let base_interval = std::time::Duration::from_secs(60); // 1 minute base

        // Reduce interval when there are issues
        if consecutive_failures > 0 {
            let backoff_factor = std::cmp::min(consecutive_failures, 4); // Cap at 4x
            let reduced_interval = base_interval / (backoff_factor + 1);
            return std::cmp::max(reduced_interval, std::time::Duration::from_secs(15)); // Minimum 15 seconds
        }

        // Slightly reduce interval if there were recent reconnections
        if has_recent_reconnections {
            return std::time::Duration::from_secs(45); // 45 seconds
        }

        // Normal interval when all is well
        base_interval
    }

    /// Log with exponential backoff to reduce noise
    fn log_with_backoff(
        quiet_cycles: &mut u32,
        message: &str,
        level: tracing::Level,
    ) {
        let should_log = match *quiet_cycles {
            0..=2 => true,                      // Log first 3 occurrences
            3..=10 => *quiet_cycles % 3 == 0,   // Every 3rd occurrence
            11..=50 => *quiet_cycles % 10 == 0, // Every 10th occurrence
            _ => *quiet_cycles % 50 == 0,       // Every 50th occurrence
        };

        if should_log {
            match level {
                tracing::Level::ERROR => tracing::error!("{} (suppressed {} times)", message, *quiet_cycles),
                tracing::Level::WARN => tracing::warn!("{} (suppressed {} times)", message, *quiet_cycles),
                tracing::Level::INFO => tracing::info!("{} (suppressed {} times)", message, *quiet_cycles),
                tracing::Level::DEBUG => tracing::debug!("{} (suppressed {} times)", message, *quiet_cycles),
                tracing::Level::TRACE => tracing::trace!("{} (suppressed {} times)", message, *quiet_cycles),
            }
        }

        *quiet_cycles += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::common::{constants::protocol, server::ServerType};
    use crate::core::models::{Config, MCPServerConfig};
    use crate::core::pool::UpstreamConnection;
    use tempfile::TempDir;

    fn pool_with_connection(
        status: ConnectionStatus,
        configured: bool,
    ) -> Arc<Mutex<UpstreamConnectionPool>> {
        let server_id = "server-a";
        let mut config = Config::default();
        if configured {
            config.mcp_servers.insert(
                server_id.to_string(),
                MCPServerConfig {
                    source_fingerprint: None,
                    kind: ServerType::Stdio,
                    command: Some("unused-test-command".to_string()),
                    args: None,
                    url: None,
                    env: None,
                    headers: None,
                },
            );
        }
        let mut connection = UpstreamConnection::new(server_id.to_string());
        connection.status = status;
        connection.id = "instance-a".to_string();
        let mut pool = UpstreamConnectionPool::new(Arc::new(config), None);
        pool.connections.insert(
            server_id.to_string(),
            HashMap::from([(connection.id.clone(), connection)]),
        );
        Arc::new(Mutex::new(pool))
    }

    fn slow_stdio_fixture(temp_dir: &TempDir) -> std::path::PathBuf {
        let path = temp_dir.path().join("slow_health_fixture.py");
        let script = r#"
import json
import sys
import time

for line in sys.stdin:
    request = json.loads(line)
    request_id = request.get("id")
    if request_id is None:
        continue
    if request.get("method") == "initialize":
        time.sleep(0.50)
        result = {
            "protocolVersion": "__PROTOCOL_VERSION__",
            "capabilities": {},
            "serverInfo": {"name": "slow-health", "version": "1.0.0"}
        }
    elif request.get("method") == "tools/list":
        result = {"tools": []}
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()
"#
        .replace("__PROTOCOL_VERSION__", protocol::CURRENT_VERSION);
        std::fs::write(&path, script).expect("write slow health fixture");
        path
    }

    async fn slow_reconnect_fixture(temp_dir: &TempDir) -> (Arc<Mutex<UpstreamConnectionPool>>, ReconnectIntent) {
        let fixture = slow_stdio_fixture(temp_dir);
        let python = which::which("python3").expect("python3 is required for the stdio fixture");
        let status = ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
            message: "temporary failure".to_string(),
            error_type: ErrorType::Temporary,
            failure_count: 1,
            first_failure_time: 1,
            last_failure_time: 1,
        });
        let pool = pool_with_connection(status, true);
        {
            let mut guard = pool.lock().await;
            Arc::make_mut(&mut guard.config).mcp_servers.insert(
                "server-a".to_string(),
                MCPServerConfig {
                    source_fingerprint: Some("test-config".to_string()),
                    kind: ServerType::Stdio,
                    command: Some(python.to_string_lossy().into_owned()),
                    args: Some(vec![fixture.to_string_lossy().into_owned()]),
                    url: None,
                    env: None,
                    headers: None,
                },
            );
        }
        let intent = UpstreamConnectionPool::collect_reconnection_candidates(&pool.lock().await)
            .into_iter()
            .next()
            .expect("temporary failure is eligible");
        (pool, intent)
    }

    #[tokio::test]
    async fn health_check_keeps_shutdown_instances_demand_driven() {
        let pool = pool_with_connection(ConnectionStatus::Shutdown, true);
        let guard = pool.lock().await;

        let candidates = UpstreamConnectionPool::collect_reconnection_candidates(&guard);

        assert!(candidates.is_empty());
    }

    #[tokio::test]
    async fn health_check_keeps_permanent_failures_manual() {
        let pool = pool_with_connection(
            ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
                message: "OAuth authorization is required".to_string(),
                error_type: ErrorType::Permanent,
                failure_count: 1,
                first_failure_time: 1,
                last_failure_time: 1,
            }),
            true,
        );
        let guard = pool.lock().await;

        let candidates = UpstreamConnectionPool::collect_reconnection_candidates(&guard);

        assert!(candidates.is_empty());
    }

    #[tokio::test]
    async fn candidate_collection_stops_after_three_recorded_failures() {
        let pool = pool_with_connection(
            ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
                message: "temporary failure".to_string(),
                error_type: ErrorType::Temporary,
                failure_count: 1,
                first_failure_time: 1,
                last_failure_time: 1,
            }),
            true,
        );
        let mut guard = pool.lock().await;
        for _ in 0..2 {
            guard.register_failure(
                "server-a",
                crate::core::pool::FailureKind::Connect,
                Some("fixture failure".to_string()),
            );
        }
        guard
            .failure_states
            .get_mut("server-a")
            .expect("failure state")
            .next_retry_at = None;

        assert_eq!(
            UpstreamConnectionPool::collect_reconnection_candidates(&guard).len(),
            1,
            "the third total attempt must remain eligible"
        );

        guard.register_failure(
            "server-a",
            crate::core::pool::FailureKind::Connect,
            Some("fixture failure".to_string()),
        );
        guard
            .failure_states
            .get_mut("server-a")
            .expect("failure state")
            .next_retry_at = None;

        assert!(
            UpstreamConnectionPool::collect_reconnection_candidates(&guard).is_empty(),
            "health must not schedule a fourth total attempt"
        );
    }

    #[tokio::test]
    async fn status_check_does_not_start_a_second_reconnect_owner() {
        let pool = pool_with_connection(
            ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
                message: "temporary failure".to_string(),
                error_type: ErrorType::Temporary,
                failure_count: 1,
                first_failure_time: 1,
                last_failure_time: 1,
            }),
            false,
        );

        pool.lock()
            .await
            .check_connection_status()
            .await
            .expect("status maintenance succeeds");

        assert!(matches!(
            pool.lock().await.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Error(_)
        ));
    }

    #[tokio::test]
    async fn stale_reconnect_candidate_is_skipped_after_instance_becomes_idle() {
        let pool = pool_with_connection(ConnectionStatus::Idle, false);
        let instance_created_at = pool
            .lock()
            .await
            .get_instance("server-a", "instance-a")
            .unwrap()
            .created_at;

        UpstreamConnectionPool::reconnect_single_instance(
            pool.clone(),
            ReconnectIntent {
                server_id: "server-a".to_string(),
                instance_id: "instance-a".to_string(),
                instance_created_at,
            },
        )
        .await
        .expect("an idle instance is no longer a reconnect candidate");

        assert!(matches!(
            pool.lock().await.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Idle
        ));
    }

    #[tokio::test]
    async fn stale_reconnect_intent_does_not_touch_a_replacement_instance() {
        let status = ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
            message: "temporary failure".to_string(),
            error_type: ErrorType::Temporary,
            failure_count: 1,
            first_failure_time: 1,
            last_failure_time: 1,
        });
        let pool = pool_with_connection(status.clone(), true);
        let intent = UpstreamConnectionPool::collect_reconnection_candidates(&pool.lock().await)
            .into_iter()
            .next()
            .expect("first generation should be eligible");
        {
            let mut guard = pool.lock().await;
            let replacement = guard.get_instance_mut("server-a", "instance-a").unwrap();
            replacement.created_at = intent.instance_created_at + std::time::Duration::from_secs(1);
            replacement.status = status;
        }

        UpstreamConnectionPool::reconnect_single_instance(pool.clone(), intent)
            .await
            .expect("stale intent is a skipped success");

        assert!(matches!(
            pool.lock().await.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Error(_)
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_reconnect_releases_pool_lock_during_transport_startup() {
        let temp_dir = TempDir::new().expect("temp dir");
        let (pool, intent) = slow_reconnect_fixture(&temp_dir).await;
        let reconnect = tokio::spawn(UpstreamConnectionPool::reconnect_single_instance(pool.clone(), intent));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let guard = tokio::time::timeout(std::time::Duration::from_millis(100), pool.lock())
            .await
            .expect("pool lock remains available while initialize is pending");
        assert!(matches!(
            guard.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Initializing
        ));
        drop(guard);

        let _ = reconnect.await.expect("reconnect task joins");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_reconnect_discards_result_after_config_fingerprint_changes() {
        let temp_dir = TempDir::new().expect("temp dir");
        let (pool, intent) = slow_reconnect_fixture(&temp_dir).await;
        let reconnect = tokio::spawn(UpstreamConnectionPool::reconnect_single_instance(pool.clone(), intent));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Arc::make_mut(&mut pool.lock().await.config)
            .mcp_servers
            .get_mut("server-a")
            .expect("server config")
            .source_fingerprint = Some("test-config-v2".to_string());

        reconnect
            .await
            .expect("reconnect task joins")
            .expect("stale result is discarded");
        assert!(matches!(
            pool.lock().await.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Error(_)
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn newer_connection_owner_prevents_health_result_publication() {
        let temp_dir = TempDir::new().expect("temp dir");
        let (pool, intent) = slow_reconnect_fixture(&temp_dir).await;
        let reconnect = tokio::spawn(UpstreamConnectionPool::reconnect_single_instance(pool.clone(), intent));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        {
            let mut guard = pool.lock().await;
            guard.invalidate_health_reconnect("server-a", "instance-a");
            guard.get_instance_mut("server-a", "instance-a").unwrap().status = ConnectionStatus::Idle;
        }

        reconnect
            .await
            .expect("reconnect task joins")
            .expect("stale result is discarded");
        assert!(matches!(
            pool.lock().await.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Idle
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ready_health_reconnect_cancels_replaced_owner_and_keeps_new_owner_ready() {
        let temp_dir = TempDir::new().expect("temp dir");
        let (pool, _) = slow_reconnect_fixture(&temp_dir).await;
        let old_service = {
            let mut guard = pool.lock().await;
            guard
                .connect("server-a", "instance-a")
                .await
                .expect("initial owner connects");
            let connection = guard.get_instance_mut("server-a", "instance-a").unwrap();
            connection.last_connected = std::time::Instant::now() - std::time::Duration::from_secs(3700);
            connection.service.as_ref().expect("initial service").clone()
        };
        let intent = UpstreamConnectionPool::collect_reconnection_candidates(&pool.lock().await)
            .into_iter()
            .next()
            .expect("stale ready owner is eligible");

        UpstreamConnectionPool::reconnect_single_instance(pool.clone(), intent)
            .await
            .expect("health reconnect succeeds");

        let old_result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            old_service
                .peer()
                .list_tools(Some(rmcp::model::PaginatedRequestParams::default())),
        )
        .await
        .expect("cancelled old owner resolves promptly");
        assert!(old_result.is_err(), "replaced owner must be cancelled");
        let guard = pool.lock().await;
        let connection = guard.get_instance("server-a", "instance-a").unwrap();
        assert!(matches!(connection.status, ConnectionStatus::Ready));
        let replacement = connection.service.as_ref().expect("replacement service").clone();
        drop(guard);
        replacement
            .peer()
            .list_tools(Some(rmcp::model::PaginatedRequestParams::default()))
            .await
            .expect("replacement owner remains usable");
    }

    #[tokio::test]
    async fn health_reconnect_preflight_failure_never_leaves_initializing_state() {
        let status = ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
            message: "temporary failure".to_string(),
            error_type: ErrorType::Temporary,
            failure_count: 1,
            first_failure_time: 1,
            last_failure_time: 1,
        });
        let pool = pool_with_connection(status, true);
        let intent = UpstreamConnectionPool::collect_reconnection_candidates(&pool.lock().await)
            .into_iter()
            .next()
            .expect("temporary failure is eligible");

        UpstreamConnectionPool::reconnect_single_instance(pool.clone(), intent)
            .await
            .expect_err("missing command fails before transport startup");
        assert!(matches!(
            pool.lock().await.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Error(_)
        ));
    }

    #[tokio::test]
    async fn reconnect_backoff_is_a_deferred_success_not_a_health_failure() {
        let pool = pool_with_connection(
            ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
                message: "temporary failure".to_string(),
                error_type: ErrorType::Temporary,
                failure_count: 1,
                first_failure_time: 1,
                last_failure_time: 1,
            }),
            true,
        );
        pool.lock().await.register_failure(
            "server-a",
            crate::core::pool::FailureKind::Connect,
            Some("fixture failure".to_string()),
        );
        let instance_created_at = pool
            .lock()
            .await
            .get_instance("server-a", "instance-a")
            .unwrap()
            .created_at;

        UpstreamConnectionPool::reconnect_single_instance(
            pool.clone(),
            ReconnectIntent {
                server_id: "server-a".to_string(),
                instance_id: "instance-a".to_string(),
                instance_created_at,
            },
        )
        .await
        .expect("active backoff defers reconnect without failing the health cycle");

        assert!(matches!(
            pool.lock().await.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Error(_)
        ));
    }

    #[tokio::test]
    async fn stale_health_intent_is_skipped_after_automatic_recovery_is_exhausted() {
        let pool = pool_with_connection(
            ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
                message: "temporary failure".to_string(),
                error_type: ErrorType::Temporary,
                failure_count: 1,
                first_failure_time: 1,
                last_failure_time: 1,
            }),
            true,
        );
        let intent = UpstreamConnectionPool::collect_reconnection_candidates(&pool.lock().await)
            .into_iter()
            .next()
            .expect("temporary failure is initially eligible");
        {
            let mut guard = pool.lock().await;
            for _ in 0..3 {
                guard.register_failure(
                    "server-a",
                    crate::core::pool::FailureKind::Connect,
                    Some("fixture failure".to_string()),
                );
            }
            guard
                .failure_states
                .get_mut("server-a")
                .expect("failure state")
                .next_retry_at = None;
        }

        UpstreamConnectionPool::reconnect_single_instance(pool.clone(), intent)
            .await
            .expect("exhausted automatic recovery must skip a stale reconnect intent");
        assert!(matches!(
            pool.lock().await.get_instance("server-a", "instance-a").unwrap().status,
            ConnectionStatus::Error(_)
        ));
    }

    #[tokio::test]
    async fn next_health_tick_does_not_bypass_backoff_after_a_failure() {
        let pool = pool_with_connection(
            ConnectionStatus::Error(crate::core::foundation::types::ErrorDetails {
                message: "temporary failure".to_string(),
                error_type: ErrorType::Temporary,
                failure_count: 1,
                first_failure_time: 1,
                last_failure_time: 1,
            }),
            true,
        );

        let first_tick = UpstreamConnectionPool::collect_reconnection_candidates(&pool.lock().await);
        assert_eq!(first_tick.len(), 1);
        assert_eq!(first_tick[0].server_id, "server-a");
        assert_eq!(first_tick[0].instance_id, "instance-a");
        pool.lock().await.register_failure(
            "server-a",
            crate::core::pool::FailureKind::Connect,
            Some("fixture failure".to_string()),
        );

        assert!(UpstreamConnectionPool::collect_reconnection_candidates(&pool.lock().await).is_empty());
    }
}
