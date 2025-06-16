//! Core Process Monitor
//!
//! process resource monitoring module, used to monitor the CPU, memory, and other resource usage of processes

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use sysinfo::{Pid, System};
use tokio::{sync::Mutex, time::sleep};
use tracing;

/// process resource information
#[derive(Debug, Clone)]
pub struct ProcessResourceInfo {
    /// process ID
    pub pid: u32,
    /// CPU usage percentage (0-100)
    pub cpu_usage: f32,
    /// memory usage (bytes)
    pub memory_usage: u64,
    /// disk read since last update (bytes)
    pub disk_read: u64,
    /// disk write since last update (bytes)
    pub disk_write: u64,
    /// process start time
    pub start_time: u64,
    /// number of threads
    pub threads_count: usize,
}

/// resource limit configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// maximum CPU usage percentage (0-100)
    pub max_cpu_usage: f32,
    /// maximum memory usage (bytes)
    pub max_memory_usage: u64,
    /// action to take when exceeding limits
    pub action: ResourceLimitAction,
}

/// action to take when exceeding resource limits
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceLimitAction {
    /// record warning but do not take action
    Warn,
    /// restart process
    Restart,
    /// terminate process
    Terminate,
}

/// process resource monitor
#[derive(Debug)]
pub struct ProcessMonitor {
    /// system information
    system: Mutex<System>,
    /// process resource information
    resources: Mutex<HashMap<u32, ProcessResourceInfo>>,
    /// update interval
    update_interval: Duration,
    /// resource limits
    resource_limits: Option<ResourceLimits>,
}

impl ProcessMonitor {
    /// create a new process monitor
    pub fn new(update_interval: Duration) -> Self {
        let mut system = System::new();
        // initialize process information
        system.refresh_processes();

        Self {
            system: Mutex::new(system),
            resources: Mutex::new(HashMap::new()),
            update_interval,
            resource_limits: None,
        }
    }

    /// create a new process monitor with resource limits
    pub fn new_with_limits(
        update_interval: Duration,
        max_cpu_usage: f32,
        max_memory_usage: u64,
        action: ResourceLimitAction,
    ) -> Self {
        let mut system = System::new();
        // initialize process information
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

    /// start monitoring processes
    pub fn start_monitoring(monitor: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                // wait for update interval
                sleep(monitor.update_interval).await;

                // update process information
                if let Err(e) = monitor.update_process_info().await {
                    tracing::error!("Error updating process information: {}", e);
                }
            }
        });
    }

    /// update process information
    async fn update_process_info(&self) -> Result<()> {
        // lock system and refresh processes
        let mut system = self.system.lock().await;
        system.refresh_processes();

        // update resources
        let mut resources = self.resources.lock().await;

        // remove processes that no longer exist
        resources.retain(|&pid, _| system.process(Pid::from_u32(pid)).is_some());

        // update or add processes
        for (pid, process) in system.processes() {
            let pid_u32 = pid.as_u32();

            // create or update process resource information
            let resource_info = ProcessResourceInfo {
                pid: pid_u32,
                cpu_usage: process.cpu_usage(),
                memory_usage: process.memory(),
                disk_read: process.disk_usage().read_bytes,
                disk_write: process.disk_usage().written_bytes,
                start_time: process.start_time(),
                threads_count: 1, // default to 1 thread
            };

            // update or insert
            resources.insert(pid_u32, resource_info);
        }

        Ok(())
    }

    /// get resource information for a specific process
    pub async fn get_process_info(
        &self,
        pid: u32,
    ) -> Option<ProcessResourceInfo> {
        let resources = self.resources.lock().await;
        resources.get(&pid).cloned()
    }

    /// get all process resource information
    pub async fn get_all_process_info(&self) -> HashMap<u32, ProcessResourceInfo> {
        let resources = self.resources.lock().await;
        resources.clone()
    }

    /// check if a process is running
    pub async fn is_process_running(
        &self,
        pid: u32,
    ) -> bool {
        let system = self.system.lock().await;
        system.process(Pid::from_u32(pid)).is_some()
    }

    /// get CPU usage for a specific process
    pub async fn get_process_cpu_usage(
        &self,
        pid: u32,
    ) -> Option<f32> {
        let resources = self.resources.lock().await;
        resources.get(&pid).map(|info| info.cpu_usage)
    }

    /// get memory usage for a specific process
    pub async fn get_process_memory_usage(
        &self,
        pid: u32,
    ) -> Option<u64> {
        let resources = self.resources.lock().await;
        resources.get(&pid).map(|info| info.memory_usage)
    }

    /// check if a process exceeds resource limits
    pub async fn check_resource_limits(
        &self,
        pid: u32,
    ) -> Option<(ResourceLimitAction, String)> {
        // if no resource limits are set, skip
        let limits = match &self.resource_limits {
            Some(limits) => limits,
            None => return None,
        };

        let resources = self.resources.lock().await;
        let info = match resources.get(&pid) {
            Some(info) => info,
            None => return None,
        };

        // check CPU usage
        if info.cpu_usage > limits.max_cpu_usage {
            return Some((
                limits.action.clone(),
                format!(
                    "Process {} exceeds CPU limit: {:.2}% > {:.2}%",
                    pid, info.cpu_usage, limits.max_cpu_usage
                ),
            ));
        }

        // check memory usage
        if info.memory_usage > limits.max_memory_usage {
            return Some((
                limits.action.clone(),
                format!(
                    "Process {} exceeds memory limit: {} bytes > {} bytes",
                    pid, info.memory_usage, limits.max_memory_usage
                ),
            ));
        }

        None
    }

    /// get resource limit configuration
    pub fn get_resource_limits(&self) -> Option<&ResourceLimits> {
        self.resource_limits.as_ref()
    }

    /// set resource limits
    pub fn set_resource_limits(
        &mut self,
        limits: Option<ResourceLimits>,
    ) {
        self.resource_limits = limits;
    }

    /// get update interval
    pub fn get_update_interval(&self) -> Duration {
        self.update_interval
    }
}
