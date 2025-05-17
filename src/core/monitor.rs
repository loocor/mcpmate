// MCP Proxy process monitor module
// Contains functions for monitoring process resources

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use sysinfo::{Pid, System};
use tokio::{sync::Mutex, time::sleep};
use tracing;

/// Process resource information
#[derive(Debug, Clone)]
pub struct ProcessResourceInfo {
    /// Process ID
    pub pid: u32,
    /// CPU usage percentage (0-100)
    pub cpu_usage: f32,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Disk read since last update in bytes
    pub disk_read: u64,
    /// Disk write since last update in bytes
    pub disk_write: u64,
    /// Time when the process was started
    pub start_time: u64,
    /// Number of threads
    pub threads_count: usize,
}

/// Resource limits configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_usage: f32,
    /// Maximum memory usage in bytes
    pub max_memory_usage: u64,
    /// Action to take when limits are exceeded
    pub action: ResourceLimitAction,
}

/// Action to take when resource limits are exceeded
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceLimitAction {
    /// Log a warning but take no action
    Warn,
    /// Restart the process
    Restart,
    /// Terminate the process
    Terminate,
}

/// Process resource monitor
#[derive(Debug)]
pub struct ProcessMonitor {
    /// System information
    system: Mutex<System>,
    /// Process resource information
    resources: Mutex<HashMap<u32, ProcessResourceInfo>>,
    /// Update interval
    update_interval: Duration,
    /// Resource limits
    resource_limits: Option<ResourceLimits>,
}

impl ProcessMonitor {
    /// Create a new process monitor
    pub fn new(update_interval: Duration) -> Self {
        let mut system = System::new();
        // Initialize with processes information
        system.refresh_processes();

        Self {
            system: Mutex::new(system),
            resources: Mutex::new(HashMap::new()),
            update_interval,
            resource_limits: None,
        }
    }

    /// Create a new process monitor with resource limits
    pub fn new_with_limits(
        update_interval: Duration,
        max_cpu_usage: f32,
        max_memory_usage: u64,
        action: ResourceLimitAction,
    ) -> Self {
        let mut system = System::new();
        // Initialize with processes information
        system.refresh_processes();

        Self {
            system: Mutex::new(system),
            resources: Mutex::new(HashMap::new()),
            update_interval,
            resource_limits: Some(ResourceLimits {
                max_cpu_usage,
                max_memory_usage,
                action,
            }),
        }
    }

    /// Start monitoring processes
    pub fn start_monitoring(monitor: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                // Wait for update interval
                sleep(monitor.update_interval).await;

                // Update process information
                if let Err(e) = monitor.update_process_info().await {
                    tracing::error!("Error updating process information: {}", e);
                }
            }
        });
    }

    /// Update process information
    async fn update_process_info(&self) -> Result<()> {
        // Lock system and refresh processes
        let mut system = self.system.lock().await;
        system.refresh_processes();

        // Update resources
        let mut resources = self.resources.lock().await;

        // Remove processes that no longer exist
        resources.retain(|&pid, _| system.process(Pid::from_u32(pid)).is_some());

        // Update or add processes
        for (pid, process) in system.processes() {
            let pid_u32 = pid.as_u32();

            // Create or update process resource info
            let resource_info = ProcessResourceInfo {
                pid: pid_u32,
                cpu_usage: process.cpu_usage(),
                memory_usage: process.memory(),
                disk_read: process.disk_usage().read_bytes,
                disk_write: process.disk_usage().written_bytes,
                start_time: process.start_time(),
                threads_count: 1, // Default to 1 thread
            };

            // Update or insert
            resources.insert(pid_u32, resource_info);
        }

        Ok(())
    }

    /// Get resource information for a specific process
    pub async fn get_process_info(
        &self,
        pid: u32,
    ) -> Option<ProcessResourceInfo> {
        let resources = self.resources.lock().await;
        resources.get(&pid).cloned()
    }

    /// Get all process resource information
    pub async fn get_all_process_info(&self) -> HashMap<u32, ProcessResourceInfo> {
        let resources = self.resources.lock().await;
        resources.clone()
    }

    /// Check if a process is running
    pub async fn is_process_running(
        &self,
        pid: u32,
    ) -> bool {
        let system = self.system.lock().await;
        system.process(Pid::from_u32(pid)).is_some()
    }

    /// Get CPU usage for a specific process
    pub async fn get_process_cpu_usage(
        &self,
        pid: u32,
    ) -> Option<f32> {
        let resources = self.resources.lock().await;
        resources.get(&pid).map(|info| info.cpu_usage)
    }

    /// Get memory usage for a specific process
    pub async fn get_process_memory_usage(
        &self,
        pid: u32,
    ) -> Option<u64> {
        let resources = self.resources.lock().await;
        resources.get(&pid).map(|info| info.memory_usage)
    }

    /// Check if a process exceeds resource limits
    pub async fn check_resource_limits(
        &self,
        pid: u32,
    ) -> Option<(ResourceLimitAction, String)> {
        // Skip if no resource limits are set
        let limits = match &self.resource_limits {
            Some(limits) => limits,
            None => return None,
        };

        // Get process resource info
        let resources = self.resources.lock().await;
        let info = match resources.get(&pid) {
            Some(info) => info,
            None => return None,
        };

        // Check CPU usage
        if info.cpu_usage > limits.max_cpu_usage {
            let message = format!(
                "Process {} exceeded CPU limit: {:.1}% > {:.1}%",
                pid, info.cpu_usage, limits.max_cpu_usage
            );
            return Some((limits.action.clone(), message));
        }

        // Check memory usage
        if info.memory_usage > limits.max_memory_usage {
            let message = format!(
                "Process {} exceeded memory limit: {:.1} MB > {:.1} MB",
                pid,
                info.memory_usage as f64 / 1024.0 / 1024.0,
                limits.max_memory_usage as f64 / 1024.0 / 1024.0
            );
            return Some((limits.action.clone(), message));
        }

        // No limits exceeded
        None
    }
}
