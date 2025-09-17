//! Instrumented Mutex wrapper for lock contention monitoring
//!
//! This module provides a wrapper around tokio::sync::Mutex that tracks
//! lock acquisition times and provides metrics for monitoring lock contention.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, MutexGuard};
use tracing;

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
    pub fn record_acquisition(&self, wait_time: Duration) {
        let wait_us = wait_time.as_micros() as u64;

        self.total_acquisitions.fetch_add(1, Ordering::Relaxed);
        self.total_wait_time_us.fetch_add(wait_us, Ordering::Relaxed);

        // Update max wait time
        let current_max = self.max_wait_time_us.load(Ordering::Relaxed);
        if wait_us > current_max {
            self.max_wait_time_us.compare_exchange_weak(
                current_max,
                wait_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ).ok(); // Ignore race conditions - approximation is fine
        }

        // Record slow acquisitions (>300ms)
        if wait_time > Duration::from_millis(300) {
            self.slow_acquisitions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record lock hold time
    pub fn record_hold_time(&self, hold_time: Duration) {
        let hold_us = hold_time.as_micros() as u64;

        self.total_hold_time_us.fetch_add(hold_us, Ordering::Relaxed);

        // Update max hold time
        let current_max = self.max_hold_time_us.load(Ordering::Relaxed);
        if hold_us > current_max {
            self.max_hold_time_us.compare_exchange_weak(
                current_max,
                hold_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ).ok(); // Ignore race conditions - approximation is fine
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
    guard: MutexGuard<'a, T>,
    metrics: Arc<LockMetrics>,
    lock_name: String,
    acquired_at: Instant,
}

impl<'a, T> InstrumentedMutexGuard<'a, T> {
    fn new(guard: MutexGuard<'a, T>, metrics: Arc<LockMetrics>, lock_name: String) -> Self {
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
            tracing::warn!(
                "Slow lock hold detected: {} held for {:?}",
                self.lock_name,
                hold_time
            );
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
    mutex: Mutex<T>,
    metrics: Arc<LockMetrics>,
    name: String,
}

impl<T> InstrumentedMutex<T> {
    /// Create a new instrumented mutex with a given name
    pub fn new(value: T, name: String) -> Self {
        Self {
            mutex: Mutex::new(value),
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstrumentedMutex")
            .field("name", &self.name)
            .field("metrics", &self.metrics)
            .finish()
    }
}