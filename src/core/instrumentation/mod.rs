//! Instrumentation and monitoring utilities
//!
//! This module provides tools for monitoring system performance,
//! including lock contention tracking and metrics collection.

pub mod mutex;
pub mod traits;

pub use mutex::{InstrumentedMutex, InstrumentedMutexGuard, LockMetrics};

/// Global metrics registry for collecting system-wide metrics
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global metrics registry
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
