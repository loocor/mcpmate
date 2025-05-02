// MCPMan system metrics module
// Contains functionality for collecting system metrics

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::time;

/// System metrics collector
pub struct SystemMetricsCollector {
    /// System information
    system: Mutex<System>,
    /// Last update time
    last_update: Mutex<Instant>,
    /// Update interval
    update_interval: Duration,
    /// Smoothing factor for CPU usage (0.0-1.0)
    cpu_smoothing_factor: f32,
    /// Smoothing factor for memory usage (0.0-1.0)
    memory_smoothing_factor: f32,
    /// Previous CPU usage for smoothing
    previous_cpu_usage: Mutex<f32>,
    /// Previous memory usage for smoothing
    previous_memory_usage: Mutex<f32>,
}

impl SystemMetricsCollector {
    /// Create a new system metrics collector with default settings
    pub fn new(update_interval: Duration) -> Self {
        Self::new_with_smoothing(update_interval, 0.8, 0.9)
    }
    
    /// Create a new system metrics collector with custom smoothing factors
    pub fn new_with_smoothing(
        update_interval: Duration,
        cpu_smoothing_factor: f32,
        memory_smoothing_factor: f32,
    ) -> Self {
        let system = System::new_all();
        
        Self {
            system: Mutex::new(system),
            last_update: Mutex::new(Instant::now()),
            update_interval,
            cpu_smoothing_factor,
            memory_smoothing_factor,
            previous_cpu_usage: Mutex::new(0.0),
            previous_memory_usage: Mutex::new(0.0),
        }
    }
    
    /// Refresh system metrics if needed
    fn refresh_if_needed(&self, refresh_cpu: bool, refresh_memory: bool) {
        let mut system = self.system.lock().unwrap();
        let mut last_update = self.last_update.lock().unwrap();
        
        if last_update.elapsed() >= self.update_interval {
            if refresh_cpu {
                system.refresh_cpu();
            }
            if refresh_memory {
                system.refresh_memory();
            }
            *last_update = Instant::now();
        }
    }
    
    /// Apply smoothing to a value
    fn apply_smoothing(&self, current_value: f32, previous_value: &mut f32, smoothing_factor: f32) -> f32 {
        let result = *previous_value * (1.0 - smoothing_factor) + current_value * smoothing_factor;
        *previous_value = result;
        result
    }
    
    /// Get CPU usage percentage (0-100)
    pub fn get_cpu_usage(&self) -> f32 {
        // Refresh CPU metrics if needed
        self.refresh_if_needed(true, false);
        
        // Calculate global CPU usage
        let global_cpu_usage = {
            let system = self.system.lock().unwrap();
            system.global_cpu_info().cpu_usage()
        };
        
        // Apply smoothing
        let mut previous = self.previous_cpu_usage.lock().unwrap();
        self.apply_smoothing(global_cpu_usage, &mut previous, self.cpu_smoothing_factor)
    }
    
    /// Get memory usage in MB
    pub fn get_memory_usage_mb(&self) -> f32 {
        // Refresh memory metrics if needed
        self.refresh_if_needed(false, true);
        
        // Get memory usage in MB
        let memory_mb = {
            let system = self.system.lock().unwrap();
            (system.used_memory() as f32) / 1024.0 / 1024.0
        };
        
        // Apply smoothing
        let mut previous = self.previous_memory_usage.lock().unwrap();
        self.apply_smoothing(memory_mb, &mut previous, self.memory_smoothing_factor)
    }
    
    /// Get total memory in MB
    pub fn get_total_memory_mb(&self) -> f32 {
        let system = self.system.lock().unwrap();
        (system.total_memory() as f32) / 1024.0 / 1024.0
    }
    
    /// Get memory usage percentage (0-100)
    pub fn get_memory_percentage(&self) -> f32 {
        // Refresh memory metrics if needed
        self.refresh_if_needed(false, true);
        
        let system = self.system.lock().unwrap();
        
        let total = system.total_memory() as f32;
        if total == 0.0 {
            return 0.0;
        }
        
        let used = system.used_memory() as f32;
        (used / total) * 100.0
    }
    
    /// Start a background task to periodically refresh system metrics
    pub fn start_background_refresh(collector: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(collector.update_interval);
            
            loop {
                interval.tick().await;
                
                // Refresh all metrics
                let mut system = collector.system.lock().unwrap();
                let mut last_update = collector.last_update.lock().unwrap();
                
                system.refresh_all();
                *last_update = Instant::now();
                
                // Log current metrics at debug level
                tracing::debug!(
                    "System metrics refreshed: CPU: {:.1}%, Memory: {:.1}MB/{:.1}MB ({:.1}%)",
                    collector.get_cpu_usage(),
                    collector.get_memory_usage_mb(),
                    collector.get_total_memory_mb(),
                    collector.get_memory_percentage()
                );
            }
        });
    }
}
