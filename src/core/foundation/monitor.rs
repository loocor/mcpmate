//! Core Process Monitor
//!
//! process resource monitoring module, used to monitor the CPU, memory, and other resource usage of processes

use anyhow::Result;
use std::{collections::HashMap, sync::Arc, time::Duration};
use sysinfo::{Pid, System};
use tokio::sync::RwLock;
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

// === Lock instrumentation (moved from core::instrumentation) ===

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Trait for mutex-like types that can be locked asynchronously.
pub trait AsyncMutex<T> {
    type Guard<'a>: std::ops::Deref<Target = T> + std::ops::DerefMut<Target = T> + Send + 'a
    where
        Self: 'a;

    fn lock(&self) -> impl std::future::Future<Output = Self::Guard<'_>> + Send;
}

impl<T: Send> AsyncMutex<T> for tokio::sync::Mutex<T> {
    type Guard<'a>
        = tokio::sync::MutexGuard<'a, T>
    where
        Self: 'a;

    async fn lock(&self) -> Self::Guard<'_> {
        self.lock().await
    }
}

/// Metrics for lock contention monitoring
#[derive(Debug, Default)]
pub struct LockMetrics {
    /// Total number of lock acquisitions
    pub total_acquisitions: AtomicU64,
    /// Total time spent waiting for locks (in microseconds)
    pub total_wait_time_us: AtomicU64,
    /// Total time spent holding locks (in microseconds)
    pub total_hold_time_us: AtomicU64,
    /// Number of slow lock acquisitions (>300ms)
    pub slow_acquisitions: AtomicU64,
    /// Maximum wait time observed (in microseconds)
    pub max_wait_time_us: AtomicU64,
    /// Maximum hold time observed (in microseconds)
    pub max_hold_time_us: AtomicU64,
}

impl LockMetrics {
    /// Create new lock metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a lock acquisition
    pub fn record_acquisition(
        &self,
        wait_time: Duration,
    ) {
        let wait_us = wait_time.as_micros() as u64;

        self.total_acquisitions.fetch_add(1, Ordering::Relaxed);
        self.total_wait_time_us.fetch_add(wait_us, Ordering::Relaxed);

        // Update max wait time
        let current_max = self.max_wait_time_us.load(Ordering::Relaxed);
        if wait_us > current_max {
            self.max_wait_time_us
                .compare_exchange_weak(current_max, wait_us, Ordering::Relaxed, Ordering::Relaxed)
                .ok();
        }

        // Record slow acquisitions (>300ms)
        if wait_time > Duration::from_millis(300) {
            self.slow_acquisitions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record lock hold time
    pub fn record_hold_time(
        &self,
        hold_time: Duration,
    ) {
        let hold_us = hold_time.as_micros() as u64;

        self.total_hold_time_us.fetch_add(hold_us, Ordering::Relaxed);

        // Update max hold time
        let current_max = self.max_hold_time_us.load(Ordering::Relaxed);
        if hold_us > current_max {
            self.max_hold_time_us
                .compare_exchange_weak(current_max, hold_us, Ordering::Relaxed, Ordering::Relaxed)
                .ok();
        }
    }

    /// Get average wait time in microseconds
    pub fn average_wait_time_us(&self) -> u64 {
        let total_wait = self.total_wait_time_us.load(Ordering::Relaxed);
        let total_acquisitions = self.total_acquisitions.load(Ordering::Relaxed);
        if total_acquisitions > 0 {
            total_wait / total_acquisitions
        } else {
            0
        }
    }

    /// Get average hold time in microseconds
    pub fn average_hold_time_us(&self) -> u64 {
        let total_hold = self.total_hold_time_us.load(Ordering::Relaxed);
        let total_acquisitions = self.total_acquisitions.load(Ordering::Relaxed);
        if total_acquisitions > 0 {
            total_hold / total_acquisitions
        } else {
            0
        }
    }

    /// Get slow acquisition percentage
    pub fn slow_acquisition_percentage(&self) -> f64 {
        let total = self.total_acquisitions.load(Ordering::Relaxed);
        let slow = self.slow_acquisitions.load(Ordering::Relaxed);
        if total > 0 {
            (slow as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Instrumented mutex guard that tracks hold time
pub struct InstrumentedMutexGuard<'a, T> {
    guard: tokio::sync::MutexGuard<'a, T>,
    metrics: Arc<LockMetrics>,
    lock_name: String,
    acquired_at: Instant,
}

impl<'a, T> InstrumentedMutexGuard<'a, T> {
    fn new(
        guard: tokio::sync::MutexGuard<'a, T>,
        metrics: Arc<LockMetrics>,
        lock_name: String,
    ) -> Self {
        Self {
            guard,
            metrics,
            lock_name,
            acquired_at: Instant::now(),
        }
    }
}

impl<T> Drop for InstrumentedMutexGuard<'_, T> {
    fn drop(&mut self) {
        let hold_time = self.acquired_at.elapsed();
        self.metrics.record_hold_time(hold_time);

        // Log slow lock holds (>300ms)
        if hold_time > Duration::from_millis(300) {
            tracing::warn!("Slow lock hold detected: {} held for {:?}", self.lock_name, hold_time);
        }
    }
}

impl<T> std::ops::Deref for InstrumentedMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T> std::ops::DerefMut for InstrumentedMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

/// Instrumented mutex wrapper that tracks lock contention metrics
pub struct InstrumentedMutex<T> {
    mutex: tokio::sync::Mutex<T>,
    metrics: Arc<LockMetrics>,
    name: String,
}

impl<T> InstrumentedMutex<T> {
    /// Create a new instrumented mutex with a given name
    pub fn new(
        value: T,
        name: String,
    ) -> Self {
        Self {
            mutex: tokio::sync::Mutex::new(value),
            metrics: Arc::new(LockMetrics::new()),
            name,
        }
    }

    /// Lock the mutex and return an instrumented guard
    pub async fn lock(&self) -> InstrumentedMutexGuard<'_, T> {
        let start_time = Instant::now();
        let guard = self.mutex.lock().await;
        let wait_time = start_time.elapsed();

        self.metrics.record_acquisition(wait_time);

        // Log slow lock acquisitions (>300ms)
        if wait_time > Duration::from_millis(300) {
            tracing::warn!(
                "Slow lock acquisition detected: {} took {:?} to acquire",
                self.name,
                wait_time
            );
        }

        InstrumentedMutexGuard::new(guard, self.metrics.clone(), self.name.clone())
    }

    /// Get metrics for this mutex
    pub fn metrics(&self) -> Arc<LockMetrics> {
        self.metrics.clone()
    }

    /// Get the name of this mutex
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T> fmt::Debug for InstrumentedMutex<T> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("InstrumentedMutex")
            .field("name", &self.name)
            .field("metrics", &self.metrics)
            .finish()
    }
}

impl<T: Send> AsyncMutex<T> for InstrumentedMutex<T> {
    type Guard<'a>
        = InstrumentedMutexGuard<'a, T>
    where
        Self: 'a;

    async fn lock(&self) -> Self::Guard<'_> {
        self.lock().await
    }
}

#[derive(Debug, Default)]
pub struct MetricsRegistry {
    lock_metrics: RwLock<HashMap<String, Arc<LockMetrics>>>,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register lock metrics with a given name
    pub async fn register_lock_metrics(
        &self,
        name: String,
        metrics: Arc<LockMetrics>,
    ) {
        let mut lock_metrics = self.lock_metrics.write().await;
        lock_metrics.insert(name, metrics);
    }

    /// Get all lock metrics
    pub async fn get_all_lock_metrics(&self) -> HashMap<String, Arc<LockMetrics>> {
        let lock_metrics = self.lock_metrics.read().await;
        lock_metrics.clone()
    }

    /// Print summary of all lock metrics
    pub async fn print_summary(&self) {
        let metrics = self.get_all_lock_metrics().await;

        if metrics.is_empty() {
            tracing::debug!("Lock metrics summary requested but nothing registered yet");
            return;
        }

        tracing::info!("=== Lock Contention Summary ===");

        for (name, lock_metrics) in metrics {
            let total_acquisitions = lock_metrics
                .total_acquisitions
                .load(std::sync::atomic::Ordering::Relaxed);
            let avg_wait_us = lock_metrics.average_wait_time_us();
            let avg_hold_us = lock_metrics.average_hold_time_us();
            let slow_percentage = lock_metrics.slow_acquisition_percentage();
            let max_wait_us = lock_metrics.max_wait_time_us.load(std::sync::atomic::Ordering::Relaxed);
            let max_hold_us = lock_metrics.max_hold_time_us.load(std::sync::atomic::Ordering::Relaxed);

            tracing::info!(
                "{}: {} acquisitions, avg_wait={}μs, avg_hold={}μs, slow={}%, max_wait={}μs, max_hold={}μs",
                name,
                total_acquisitions,
                avg_wait_us,
                avg_hold_us,
                slow_percentage,
                max_wait_us,
                max_hold_us
            );
        }

        tracing::info!("=== End Lock Summary ===");
    }
}

/// Global metrics registry instance
static GLOBAL_METRICS: once_cell::sync::OnceCell<MetricsRegistry> = once_cell::sync::OnceCell::new();

/// Get the global metrics registry
pub fn global_metrics() -> &'static MetricsRegistry {
    GLOBAL_METRICS.get_or_init(MetricsRegistry::new)
}

/// Initialize metrics reporting (call this once at startup)
pub fn initialize_metrics_reporting() {
    // Spawn a background task to periodically print metrics
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // Every 5 minutes
        loop {
            interval.tick().await;
            global_metrics().print_summary().await;
        }
    });
}
