// Resource monitoring functionality for UpstreamConnectionPool

use anyhow::Result;
use tracing;

use super::UpstreamConnectionPool;

impl UpstreamConnectionPool {
    /// Update process resource usage for all instances
    pub async fn update_process_resources(&mut self) -> Result<()> {
        // Skip if process monitor is not available
        let process_monitor = match &self.process_monitor {
            Some(monitor) => monitor.clone(),
            None => return Ok(()),
        };

        // Collect instances to update
        let mut instances_to_update = Vec::new();

        // First pass: collect resource info and update CPU/memory usage
        for (server_name, instances) in &mut self.connections {
            for (instance_id, conn) in instances {
                // Skip if process ID is not available
                let pid = match conn.process_id {
                    Some(pid) => pid,
                    None => continue,
                };

                // Get process resource info
                if let Some(resource_info) = process_monitor.get_process_info(pid).await {
                    // Update connection with resource info
                    conn.cpu_usage = Some(resource_info.cpu_usage);
                    conn.memory_usage = Some(resource_info.memory_usage);

                    tracing::debug!(
                        "Updated resource usage for '{}' instance '{}' (PID: {}): CPU: {:.1}%, Memory: {:.1} MB",
                        server_name,
                        instance_id,
                        pid,
                        resource_info.cpu_usage,
                        resource_info.memory_usage as f64 / 1024.0 / 1024.0
                    );

                    // Check resource limits
                    if let Some((action, message)) =
                        process_monitor.check_resource_limits(pid).await
                    {
                        // Store instance for action
                        instances_to_update.push((
                            server_name.clone(),
                            instance_id.clone(),
                            pid,
                            action,
                            message,
                        ));
                    }
                } else {
                    // Process not found, clear resource info
                    conn.cpu_usage = None;
                    conn.memory_usage = None;

                    // Clear process ID if process is not running
                    if !process_monitor.is_process_running(pid).await {
                        tracing::warn!(
                            "Process for '{}' instance '{}' (PID: {}) is not running, clearing process ID",
                            server_name,
                            instance_id,
                            pid
                        );
                        conn.process_id = None;
                    }
                }
            }
        }

        // Second pass: handle resource limit actions
        for (server_name, instance_id, _pid, action, message) in instances_to_update {
            match action {
                crate::core::monitor::ResourceLimitAction::Warn => {
                    // Just log a warning
                    tracing::warn!("{}", message);
                }
                crate::core::monitor::ResourceLimitAction::Restart => {
                    // Log the action
                    tracing::warn!("{} - Attempting to restart process", message);

                    // Try to reconnect
                    if let Err(e) = self.reconnect(&server_name, &instance_id).await {
                        tracing::error!(
                            "Failed to restart '{}' instance '{}' after resource limit exceeded: {}",
                            server_name,
                            instance_id,
                            e
                        );
                    }
                }
                crate::core::monitor::ResourceLimitAction::Terminate => {
                    // Log the action
                    tracing::warn!("{} - Attempting to terminate process", message);

                    // Try to disconnect
                    if let Err(e) = self.disconnect(&server_name, &instance_id).await {
                        tracing::error!(
                            "Failed to terminate '{}' instance '{}' after resource limit exceeded: {}",
                            server_name,
                            instance_id,
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
