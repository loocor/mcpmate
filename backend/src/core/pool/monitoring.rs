//! Pool resource monitoring functionality
//!
//! Provides resource monitoring capabilities for UpstreamConnectionPool including:
//! - process resource usage tracking
//! - resource limit enforcement
//! - automatic restart/termination on limit violations

use anyhow::Result;

use super::UpstreamConnectionPool;
use crate::core::foundation::monitor::ResourceLimitAction;

/// Resource monitoring data for a single instance
#[derive(Debug, Clone)]
struct InstanceResourceData {
    server_name: String,
    instance_id: String,
    pid: u32,
    action: ResourceLimitAction,
    message: String,
}

impl UpstreamConnectionPool {
    /// Update process resource usage for all instances
    pub async fn update_process_resources(&mut self) -> Result<()> {
        // Early return if process monitor is not available
        let process_monitor = match &self.process_monitor {
            Some(monitor) => monitor.clone(),
            None => return Err(anyhow::anyhow!("Process monitor not available")),
        };

        // Collect instances that need resource limit actions
        let instances_to_update = self.update_resource_info(&process_monitor).await?;

        // Handle resource limit actions for collected instances
        self.handle_resource_limit_actions(instances_to_update).await
    }

    /// Update resource information for all connections and collect limit violations
    async fn update_resource_info(
        &mut self,
        process_monitor: &crate::core::foundation::monitor::ProcessMonitor,
    ) -> Result<Vec<InstanceResourceData>> {
        let mut limit_violations = Vec::new();

        for (server_name, instances) in &mut self.connections {
            let server_violations =
                Self::process_server_instances_static(server_name, instances, process_monitor).await?;
            limit_violations.extend(server_violations);
        }

        Ok(limit_violations)
    }

    /// Process resource monitoring for instances of a single server (static version)
    async fn process_server_instances_static(
        server_name: &str,
        instances: &mut std::collections::HashMap<String, crate::core::pool::UpstreamConnection>,
        process_monitor: &crate::core::foundation::monitor::ProcessMonitor,
    ) -> Result<Vec<InstanceResourceData>> {
        let mut violations = Vec::new();

        for (instance_id, conn) in instances {
            // Early continue if no process ID
            let pid = match conn.process_id {
                Some(pid) => pid,
                None => continue,
            };

            if let Some(resource_info) = process_monitor.get_process_info(pid).await {
                Self::update_connection_resource_info_static(conn, &resource_info, server_name, instance_id, pid);

                // Check for resource limit violations
                if let Some((action, message)) = process_monitor.check_resource_limits(pid).await {
                    Self::log_resource_limit_violation_static(server_name, instance_id, pid, &message, &action);
                    violations.push(InstanceResourceData {
                        server_name: server_name.to_string(),
                        instance_id: instance_id.clone(),
                        pid,
                        action,
                        message,
                    });
                }
            } else {
                Self::handle_missing_process_static(conn, server_name, instance_id, pid, process_monitor).await;
            }
        }

        Ok(violations)
    }

    /// Update connection with resource information (static version)
    fn update_connection_resource_info_static(
        conn: &mut crate::core::pool::UpstreamConnection,
        resource_info: &crate::core::foundation::monitor::ProcessResourceInfo,
        server_name: &str,
        instance_id: &str,
        pid: u32,
    ) {
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
    }

    /// Log resource limit violation (static version)
    fn log_resource_limit_violation_static(
        server_name: &str,
        instance_id: &str,
        pid: u32,
        message: &str,
        action: &ResourceLimitAction,
    ) {
        tracing::warn!(
            "Resource limit exceeded for '{}' instance '{}' (PID: {}): {} - Action: {:?}",
            server_name,
            instance_id,
            pid,
            message,
            action
        );
    }

    /// Handle missing process (clear resource info and process ID if not running, static version)
    async fn handle_missing_process_static(
        conn: &mut crate::core::pool::UpstreamConnection,
        server_name: &str,
        instance_id: &str,
        pid: u32,
        process_monitor: &crate::core::foundation::monitor::ProcessMonitor,
    ) {
        // Clear resource info
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

    /// Handle resource limit actions for all collected violations
    async fn handle_resource_limit_actions(
        &mut self,
        violations: Vec<InstanceResourceData>,
    ) -> Result<()> {
        for violation in violations.into_iter() {
            self.execute_resource_limit_action(violation).await?;
        }
        Ok(())
    }

    /// Execute a single resource limit action
    async fn execute_resource_limit_action(
        &mut self,
        violation: InstanceResourceData,
    ) -> Result<()> {
        match violation.action {
            ResourceLimitAction::Warn => {
                tracing::warn!("{}", violation.message);
            }
            ResourceLimitAction::Restart => {
                self.handle_restart_action(&violation).await?;
            }
            ResourceLimitAction::Terminate => {
                self.handle_terminate_action(&violation).await?;
            }
        }
        Ok(())
    }

    /// Handle restart action for resource limit violation
    async fn handle_restart_action(
        &mut self,
        violation: &InstanceResourceData,
    ) -> Result<()> {
        tracing::warn!(
            "{} - Attempting to restart process (PID: {})",
            violation.message,
            violation.pid
        );

        self.reconnect(&violation.server_name, &violation.instance_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to restart '{}' instance '{}' (PID: {}) after resource limit exceeded: {}",
                    violation.server_name,
                    violation.instance_id,
                    violation.pid,
                    e
                );
                e
            })
    }

    /// Handle terminate action for resource limit violation
    async fn handle_terminate_action(
        &mut self,
        violation: &InstanceResourceData,
    ) -> Result<()> {
        tracing::warn!(
            "{} - Attempting to terminate process (PID: {})",
            violation.message,
            violation.pid
        );

        self.disconnect(&violation.server_name, &violation.instance_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to terminate '{}' instance '{}' (PID: {}) after resource limit exceeded: {}",
                    violation.server_name,
                    violation.instance_id,
                    violation.pid,
                    e
                );
                e
            })
    }
}
