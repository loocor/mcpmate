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
    /// Start health check task
    pub fn start_health_check(connection_pool: Arc<Mutex<Self>>) {
        // Start the main health check task
        let health_check_pool = connection_pool.clone();
        tokio::spawn(async move {
            loop {
                // Wait for health check interval (1 minute)
                sleep(std::time::Duration::from_secs(60)).await;

                // Check connection status for all instances
                {
                    let mut pool = health_check_pool.lock().await;
                    if let Err(e) = pool.check_connection_status().await {
                        tracing::error!("Error checking connection status: {}", e);
                    }
                }

                // Check all connections for periodic reconnects
                let mut reconnects = Vec::new();
                {
                    let pool_guard = health_check_pool.lock().await;
                    for (server_name, instances) in &pool_guard.connections {
                        for (instance_id, conn) in instances {
                            // Update last health check time
                            let now = std::time::Instant::now();

                            // Only monitor instances that should be monitored
                            if !matches!(conn.status, ConnectionStatus::Ready | ConnectionStatus::Error(_)) {
                                continue;
                            }

                            match &conn.status {
                                ConnectionStatus::Ready => {
                                    // Check if the service is still alive
                                    if let Some(_service) = &conn.service {
                                        // Periodic reconnect to ensure health
                                        if now > conn.last_connected
                                            && now.duration_since(conn.last_connected)
                                                > std::time::Duration::from_secs(3600)
                                        // Every 60 minutes
                                        {
                                            tracing::info!(
                                                "Health check triggering periodic reconnect for '{}' instance '{}' - Last connected: {:?} ago, threshold: 3600s",
                                                server_name,
                                                instance_id,
                                                now.duration_since(conn.last_connected)
                                            );
                                            reconnects.push((server_name.clone(), instance_id.clone()));
                                        } else {
                                            tracing::debug!(
                                                "Health check - '{}' instance '{}' still healthy, connected {:?} ago",
                                                server_name,
                                                instance_id,
                                                now.duration_since(conn.last_connected)
                                            );
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
                                ConnectionStatus::Disabled(_) => {
                                    // Skip disabled servers completely - no health checks, no reconnections
                                    tracing::debug!(
                                        "Health check: Skipping disabled server '{}' instance '{}'",
                                        server_name,
                                        instance_id
                                    );
                                    continue;
                                }
                                ConnectionStatus::Error(error_details) => {
                                    tracing::debug!(
                                        "Health check found error state for '{}' instance '{}': {} (type: {:?}, failures: {})",
                                        server_name,
                                        instance_id,
                                        error_details.message,
                                        error_details.error_type,
                                        error_details.failure_count
                                    );
                                    // Skip permanent errors to avoid unnecessary reconnection attempts
                                    if error_details.error_type == crate::core::foundation::types::ErrorType::Permanent
                                    {
                                        tracing::debug!(
                                            "Health check: Skipping permanent error for '{}' instance '{}'",
                                            server_name,
                                            instance_id
                                        );
                                        continue;
                                    }

                                    // Use progressive backoff based on failure count
                                    let min_delay = match error_details.failure_count {
                                        1 => 60,  // 1 minute for first failure
                                        2 => 120, // 2 minutes for second failure
                                        3 => 360, // 6 minutes for third failure
                                        _ => {
                                            // 4+ failures should be auto-disabled, but handle edge case
                                            tracing::warn!(
                                                "Health check: Server '{}' instance '{}' has {} failures but not disabled",
                                                server_name,
                                                instance_id,
                                                error_details.failure_count
                                            );
                                            600 // 10 minutes fallback
                                        }
                                    };

                                    if now > conn.last_connected
                                        && now.duration_since(conn.last_connected)
                                            > std::time::Duration::from_secs(min_delay)
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
                                _ => {}
                            }
                        }
                    }
                }

                // Reconnect instances that need it (non-blocking)
                if !reconnects.is_empty() {
                    tracing::info!(
                        "Health check: Scheduling {} reconnection(s) (non-blocking)",
                        reconnects.len()
                    );

                    // Use minimal lock time for scheduling reconnections
                    let mut pool_guard = health_check_pool.lock().await;
                    for (server_name, instance_id) in reconnects {
                        tracing::debug!(
                            "Health check: Scheduling reconnection to '{}' instance '{}'",
                            server_name,
                            instance_id
                        );
                        if let Err(e) = pool_guard.reconnect(&server_name, &instance_id).await {
                            tracing::warn!(
                                "Health check: Failed to schedule reconnection to '{}' instance '{}': {}",
                                server_name,
                                instance_id,
                                e
                            );
                        }
                    }
                    // Lock is released here, allowing other operations to proceed
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
                    // Check both Ready and Error states
                    if (matches!(conn.status, ConnectionStatus::Ready) && conn.service.is_some())
                        || matches!(conn.status, ConnectionStatus::Error(_))
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
}
