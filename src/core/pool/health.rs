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
                    // Check Ready, Error, and persistent Busy states
                    if (matches!(conn.status, ConnectionStatus::Ready) && conn.service.is_some())
                        || matches!(conn.status, ConnectionStatus::Error(_))
                        || matches!(conn.status, ConnectionStatus::Busy)
                    {
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

                // Ready state
                ConnectionStatus::Ready => {
                    // Check if the service is still connected
                    if !conn.is_connected() {
                        tracing::warn!(
                            "Connection check: Service for '{}' instance '{}' is not connected",
                            server_name,
                            instance_id
                        );

                        // Schedule reconnection (non-blocking)
                        if let Err(e) = self.reconnect(&server_name, &instance_id).await {
                            tracing::error!(
                                "Connection check: Failed to schedule reconnection to '{}' instance '{}': {}",
                                server_name,
                                instance_id,
                                e
                            );
                        } else {
                            tracing::info!(
                                "Connection check: Scheduled reconnection for '{}' instance '{}'",
                                server_name,
                                instance_id
                            );
                        }
                    }
                }

                // Error state
                ConnectionStatus::Error(error_details) => {
                    // Check if we should retry based on error type and failure count
                    let should_retry = match error_details.error_type {
                        ErrorType::Temporary => {
                            // Check if we should auto-disable (4+ failures)
                            if error_details.failure_count >= 4 {
                                tracing::warn!(
                                    "Connection check: Server '{}' instance '{}' has {} failures, should be auto-disabled",
                                    server_name,
                                    instance_id,
                                    error_details.failure_count
                                );
                                false // Don't retry, should be disabled
                            } else {
                                // Use progressive backoff based on failure count
                                let backoff_seconds = match error_details.failure_count {
                                    1 => 60,  // 1 minute for first failure
                                    2 => 120, // 2 minutes for second failure
                                    3 => 360, // 6 minutes for third failure
                                    _ => 600, // Fallback (shouldn't reach here due to check above)
                                };

                                // Calculate time since last failure
                                let now = chrono::Local::now().timestamp() as u64;
                                let seconds_since_last_failure = now.saturating_sub(error_details.last_failure_time);

                                // Only retry if enough time has passed based on progressive backoff
                                if seconds_since_last_failure >= backoff_seconds {
                                    tracing::info!(
                                        "Connection check: Retrying temporary error for '{}' instance '{}' after {}s (failure #{}, progressive backoff)",
                                        server_name,
                                        instance_id,
                                        seconds_since_last_failure,
                                        error_details.failure_count
                                    );
                                    true
                                } else {
                                    tracing::debug!(
                                        "Connection check: Waiting {}s before retrying '{}' instance '{}' (failure #{}, progressive backoff)",
                                        backoff_seconds - seconds_since_last_failure,
                                        server_name,
                                        instance_id,
                                        error_details.failure_count
                                    );
                                    false
                                }
                            }
                        }
                        ErrorType::Permanent => {
                            // Don't retry permanent errors
                            false
                        }
                        ErrorType::Unknown => {
                            // For unknown errors, retry with a fixed backoff
                            let backoff_seconds = 60; // 1 minute

                            // Calculate time since last failure
                            let now = chrono::Local::now().timestamp() as u64;
                            let seconds_since_last_failure = now.saturating_sub(error_details.last_failure_time);

                            // Only retry if enough time has passed
                            seconds_since_last_failure >= backoff_seconds
                        }
                    };

                    // If we should retry, attempt to reconnect
                    if should_retry {
                        // Schedule reconnection (non-blocking)
                        if let Err(e) = self.reconnect(&server_name, &instance_id).await {
                            tracing::error!(
                                "Connection check: Failed to schedule reconnection to '{}' instance '{}': {}",
                                server_name,
                                instance_id,
                                e
                            );
                        } else {
                            tracing::info!(
                                "Connection check: Scheduled reconnection for '{}' instance '{}' after progressive backoff",
                                server_name,
                                instance_id
                            );
                        }
                    }
                }

                // Disabled state
                _ => {
                    // Other states don't need checking
                }
            }
        }

        Ok(())
    }

    /// Collect reconnection candidates without holding lock for long time
    fn collect_reconnection_candidates(
        pool_guard: &tokio::sync::MutexGuard<'_, UpstreamConnectionPool>
    ) -> Vec<(String, String)> {
        let mut reconnects = Vec::new();
        let now = std::time::Instant::now();

        for (server_name, instances) in &pool_guard.connections {
            for (instance_id, conn) in instances {
                // Monitor Ready, Error, Shutdown, and persistent Busy connections
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
                                reconnects.push((server_name.clone(), instance_id.clone()));
                            }
                        } else {
                            // If service is None but status is Ready, something is wrong
                            tracing::warn!(
                                "Health check: Server '{}' instance '{}' has Ready status but no service, will reconnect",
                                server_name,
                                instance_id
                            );
                            reconnects.push((server_name.clone(), instance_id.clone()));
                        }
                    }
                    ConnectionStatus::Shutdown => {
                        // Skip if server is not part of active configuration (e.g., recently removed)
                        if !pool_guard.config.mcp_servers.contains_key(server_name) {
                            continue;
                        }

                        // Respect active backoff window to avoid spamming reconnect attempts
                        if let Some(remaining) = pool_guard.remaining_backoff(server_name) {
                            tracing::debug!(
                                "Health check: '{}' currently backing off for {:.1}s, skipping reconnect",
                                server_name,
                                remaining.as_secs_f32()
                            );
                            continue;
                        }

                        tracing::debug!(
                            "Health check: Scheduling reconnect for '{}' instance '{}' in Shutdown state",
                            server_name,
                            instance_id
                        );
                        reconnects.push((server_name.clone(), instance_id.clone()));
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

                        // Use progressive backoff based on failure count
                        let min_delay = match error_details.failure_count {
                            1 => 60,  // 1 minute for first failure
                            2 => 120, // 2 minutes for second failure
                            3 => 360, // 6 minutes for third failure
                            _ => 600, // 10 minutes for 4+ failures
                        };

                        if now > conn.last_connected
                            && now.duration_since(conn.last_connected) > std::time::Duration::from_secs(min_delay)
                        {
                            tracing::debug!(
                                "Health check: Scheduling reconnect for '{}' instance '{}' after {}s delay (failure count: {})",
                                server_name,
                                instance_id,
                                min_delay,
                                error_details.failure_count
                            );
                            reconnects.push((server_name.clone(), instance_id.clone()));
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
        reconnects: Vec<(String, String)>,
    ) {
        // Use SyncHelper for concurrent reconnection processing
        let _sync_result = crate::common::sync::SyncHelper::execute_concurrent_sync(
            reconnects,
            "health_check_reconnections",
            2, // Limit concurrent reconnections to avoid overwhelming
            move |(server_name, instance_id)| {
                let pool_clone = connection_pool.clone();
                async move {
                    // Use short timeout for individual reconnection attempts
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(10), // 10 second timeout per reconnection
                        Self::reconnect_single_instance(pool_clone, server_name.clone(), instance_id.clone()),
                    )
                    .await
                    {
                        Ok(Ok(())) => {
                            tracing::info!(
                                "Health check: Successfully reconnected '{}' instance '{}'",
                                server_name,
                                instance_id
                            );
                            Ok(())
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(
                                "Health check: Failed to reconnect '{}' instance '{}': {}",
                                server_name,
                                instance_id,
                                e
                            );
                            Err(anyhow::anyhow!("Reconnection failed: {}", e))
                        }
                        Err(_) => {
                            tracing::warn!(
                                "Health check: Timeout reconnecting '{}' instance '{}'",
                                server_name,
                                instance_id
                            );
                            Err(anyhow::anyhow!("Reconnection timeout"))
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
        server_name: String,
        instance_id: String,
    ) -> Result<()> {
        // Use timeout to avoid indefinite blocking on pool lock
        let mut pool = tokio::time::timeout(
            std::time::Duration::from_secs(2), // 2 second timeout for lock acquisition
            connection_pool.lock(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Timeout acquiring pool lock for reconnection"))?;

        // Use the non-blocking reconnect method
        pool.trigger_connect(&server_name, &instance_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to trigger reconnect: {}", e))
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
