// Health check functionality for UpstreamConnectionPool

use std::sync::Arc;

use anyhow::Result;
use tokio::{sync::Mutex, time::sleep};
use tracing;

use super::UpstreamConnectionPool;
use crate::core::types::ConnectionStatus;

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
                            if !matches!(
                                conn.status,
                                ConnectionStatus::Ready | ConnectionStatus::Error(_)
                            ) {
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
                                                "Health check: Periodic reconnect for '{}' instance '{}'",
                                                server_name,
                                                instance_id
                                            );
                                            reconnects
                                                .push((server_name.clone(), instance_id.clone()));
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
                                ConnectionStatus::Error(_) => {
                                    // Reconnect error instances after a delay
                                    if now > conn.last_connected
                                        && now.duration_since(conn.last_connected)
                                            > std::time::Duration::from_secs(60)
                                    {
                                        reconnects.push((server_name.clone(), instance_id.clone()));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Reconnect instances that need it
                for (server_name, instance_id) in reconnects {
                    tracing::info!(
                        "Health check: Attempting to reconnect to '{}' instance '{}'",
                        server_name,
                        instance_id
                    );
                    let mut pool_guard = health_check_pool.lock().await;
                    if let Err(e) = pool_guard.reconnect(&server_name, &instance_id).await {
                        tracing::warn!(
                            "Health check: Failed to reconnect to '{}' instance '{}': {}",
                            server_name,
                            instance_id,
                            e
                        );
                    }
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
                ConnectionStatus::Ready => {
                    // Check if the service is still connected
                    if !conn.is_connected() {
                        tracing::warn!(
                            "Connection check: Service for '{}' instance '{}' is not connected",
                            server_name,
                            instance_id
                        );

                        // Try to reconnect
                        if let Err(e) = self.reconnect(&server_name, &instance_id).await {
                            tracing::error!(
                                "Connection check: Failed to reconnect to '{}' instance '{}': {}",
                                server_name,
                                instance_id,
                                e
                            );
                        }
                    }
                }
                ConnectionStatus::Error(error_details) => {
                    // Check if we should retry based on error type and failure count
                    let should_retry = match error_details.error_type {
                        crate::core::types::ErrorType::Temporary => {
                            // Use exponential backoff for temporary errors
                            let backoff_seconds = std::cmp::min(
                                300,                                                     /* Maximum 5 minutes */
                                2u64.pow(std::cmp::min(8, error_details.failure_count)), /* Exponential backoff, max 2^8=256 seconds */
                            );

                            // Calculate time since last failure
                            let now = chrono::Local::now().timestamp() as u64;
                            let seconds_since_last_failure =
                                now.saturating_sub(error_details.last_failure_time);

                            // Only retry if enough time has passed based on backoff
                            if seconds_since_last_failure >= backoff_seconds {
                                tracing::info!(
                                    "Connection check: Retrying temporary error for '{}' instance '{}' after {}s (failure count: {})",
                                    server_name,
                                    instance_id,
                                    seconds_since_last_failure,
                                    error_details.failure_count
                                );
                                true
                            } else {
                                tracing::debug!(
                                    "Connection check: Waiting {}s before retrying '{}' instance '{}' (failure count: {})",
                                    backoff_seconds - seconds_since_last_failure,
                                    server_name,
                                    instance_id,
                                    error_details.failure_count
                                );
                                false
                            }
                        }
                        crate::core::types::ErrorType::Permanent => {
                            // Don't retry permanent errors
                            false
                        }
                        crate::core::types::ErrorType::Unknown => {
                            // For unknown errors, retry with a fixed backoff
                            let backoff_seconds = 60; // 1 minute

                            // Calculate time since last failure
                            let now = chrono::Local::now().timestamp() as u64;
                            let seconds_since_last_failure =
                                now.saturating_sub(error_details.last_failure_time);

                            // Only retry if enough time has passed
                            seconds_since_last_failure >= backoff_seconds
                        }
                    };

                    // If we should retry, attempt to reconnect
                    if should_retry {
                        // Check if we've exceeded the maximum retry count for temporary errors
                        let max_retries_exceeded = matches!(
                            error_details.error_type,
                            crate::core::types::ErrorType::Temporary
                        ) && error_details.failure_count > 10;

                        if max_retries_exceeded {
                            // Store the failure count for later use
                            let failure_count = error_details.failure_count;

                            // We need to break out of the match and for loop to avoid borrowing issues
                            // No need to explicitly drop a reference

                            // Convert to permanent error after too many retries
                            {
                                let conn = self.get_instance_mut(&server_name, &instance_id)?;
                                conn.update_permanent_error(format!(
                                    "Too many failed reconnection attempts ({failure_count}). Manual intervention required."
                                ));
                            }

                            tracing::error!(
                                "Connection check: Too many failed reconnection attempts for '{}' instance '{}' ({}). Marking as permanent error.",
                                server_name,
                                instance_id,
                                failure_count
                            );
                            continue;
                        }

                        // Try to reconnect
                        if let Err(e) = self.reconnect(&server_name, &instance_id).await {
                            tracing::error!(
                                "Connection check: Failed to reconnect to '{}' instance '{}': {}",
                                server_name,
                                instance_id,
                                e
                            );
                        }
                    }
                }
                _ => {
                    // Other states don't need checking
                }
            }
        }

        Ok(())
    }
}
