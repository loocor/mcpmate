//! Performance benchmarking utilities for cache operations

use std::{
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use super::{
    manager::RedbCacheManager,
    types::*,
};

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub num_servers: usize,
    pub tools_per_server: usize,
    pub resources_per_server: usize,
    pub prompts_per_server: usize,
    pub concurrent_operations: usize,
    pub warmup_iterations: usize,
    pub benchmark_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            num_servers: 10,
            tools_per_server: 5,
            resources_per_server: 3,
            prompts_per_server: 2,
            concurrent_operations: 4,
            warmup_iterations: 3,
            benchmark_iterations: 10,
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub config: BenchmarkConfigSummary,
    pub write_performance: OperationBenchmark,
    pub read_performance: OperationBenchmark,
    pub concurrent_performance: ConcurrentBenchmark,
    pub cache_hit_ratio: f64,
    pub storage_efficiency: StorageEfficiency,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfigSummary {
    pub num_servers: usize,
    pub total_items: usize,
    pub concurrent_operations: usize,
    pub iterations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationBenchmark {
    pub mean_duration_ms: f64,
    pub median_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub operations_per_second: f64,
    pub total_operations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrentBenchmark {
    pub concurrent_reads: OperationBenchmark,
    pub concurrent_writes: OperationBenchmark,
    pub mixed_operations: OperationBenchmark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEfficiency {
    pub total_cache_size_bytes: u64,
    pub average_item_size_bytes: f64,
    pub compression_ratio: f64,
    pub storage_overhead_percent: f64,
}

/// Cache performance benchmarker
pub struct CacheBenchmarker {
    cache_manager: RedbCacheManager,
    config: BenchmarkConfig,
}

impl CacheBenchmarker {
    /// Create a new benchmarker
    pub fn new(cache_manager: RedbCacheManager, config: BenchmarkConfig) -> Self {
        Self {
            cache_manager,
            config,
        }
    }
    
    /// Run comprehensive benchmark suite
    pub async fn run_benchmark_suite(&self) -> Result<BenchmarkResults, CacheError> {
        println!("Starting cache performance benchmark...");
        
        // Clear cache to start fresh
        self.cache_manager.clear_all().await?;
        
        // Generate test data
        let test_data = self.generate_test_data();
        
        // Warmup
        println!("Running warmup iterations...");
        for _ in 0..self.config.warmup_iterations {
            self.run_write_benchmark(&test_data).await?;
            self.run_read_benchmark(&test_data).await?;
        }
        
        // Clear cache after warmup
        self.cache_manager.clear_all().await?;
        
        // Run actual benchmarks
        println!("Running write performance benchmark...");
        let write_performance = self.benchmark_write_operations(&test_data).await?;
        
        println!("Running read performance benchmark...");
        let read_performance = self.benchmark_read_operations(&test_data).await?;
        
        println!("Running concurrent performance benchmark...");
        let concurrent_performance = self.benchmark_concurrent_operations(&test_data).await?;
        
        // Calculate cache hit ratio
        let stats = self.cache_manager.get_stats().await;
        let cache_hit_ratio = stats.hit_ratio;
        
        // Calculate storage efficiency
        let storage_efficiency = self.calculate_storage_efficiency(&test_data).await?;
        
        let results = BenchmarkResults {
            config: BenchmarkConfigSummary {
                num_servers: self.config.num_servers,
                total_items: test_data.len(),
                concurrent_operations: self.config.concurrent_operations,
                iterations: self.config.benchmark_iterations,
            },
            write_performance,
            read_performance,
            concurrent_performance,
            cache_hit_ratio,
            storage_efficiency,
            timestamp: Utc::now(),
        };
        
        println!("Benchmark completed successfully!");
        Ok(results)
    }
    
    /// Generate test data for benchmarking
    fn generate_test_data(&self) -> Vec<CachedServerData> {
        let mut test_data = Vec::new();
        
        for i in 0..self.config.num_servers {
            let server_id = format!("test_server_{}", i);
            let server_name = format!("Test Server {}", i);
            
            let mut tools = Vec::new();
            for j in 0..self.config.tools_per_server {
                tools.push(CachedToolInfo {
                    name: format!("tool_{}_{}", i, j),
                    description: Some(format!("Test tool {} for server {}", j, i)),
                    input_schema_json: r#"{"type": "object", "properties": {"input": {"type": "string"}}}"#.to_string(),
                    unique_name: Some(format!("server_{}_tool_{}", i, j)),
                    enabled: true,
                    cached_at: Utc::now(),
                });
            }
            
            let mut resources = Vec::new();
            for j in 0..self.config.resources_per_server {
                resources.push(CachedResourceInfo {
                    uri: format!("file://test_resource_{}_{}.txt", i, j),
                    name: Some(format!("Test Resource {} {}", i, j)),
                    description: Some(format!("Test resource {} for server {}", j, i)),
                    mime_type: Some("text/plain".to_string()),
                    enabled: true,
                    cached_at: Utc::now(),
                });
            }
            
            let mut prompts = Vec::new();
            for j in 0..self.config.prompts_per_server {
                prompts.push(CachedPromptInfo {
                    name: format!("prompt_{}_{}", i, j),
                    description: Some(format!("Test prompt {} for server {}", j, i)),
                    arguments: vec![
                        PromptArgument {
                            name: "input".to_string(),
                            description: Some("Input parameter".to_string()),
                            required: true,
                        }
                    ],
                    enabled: true,
                    cached_at: Utc::now(),
                });
            }
            
            test_data.push(CachedServerData {
                server_id: server_id.clone(),
                server_name,
                server_version: Some("1.0.0".to_string()),
                protocol_version: "2024-11-05".to_string(),
                tools,
                resources,
                prompts,
                resource_templates: Vec::new(),
                cached_at: Utc::now(),
                fingerprint: format!("fingerprint_{}", i),
            });
        }
        
        test_data
    }
    
    /// Benchmark write operations
    async fn benchmark_write_operations(&self, test_data: &[CachedServerData]) -> Result<OperationBenchmark, CacheError> {
        let mut durations = Vec::new();
        
        for _ in 0..self.config.benchmark_iterations {
            let start = Instant::now();
            
            for server_data in test_data {
                self.cache_manager.store_server_data(server_data).await?;
            }
            
            let duration = start.elapsed();
            durations.push(duration);
            
            // Clear cache for next iteration
            self.cache_manager.clear_all().await?;
        }
        
        Ok(self.calculate_operation_benchmark(durations, test_data.len()))
    }
    
    /// Benchmark read operations
    async fn benchmark_read_operations(&self, test_data: &[CachedServerData]) -> Result<OperationBenchmark, CacheError> {
        // First, populate cache with test data
        for server_data in test_data {
            self.cache_manager.store_server_data(server_data).await?;
        }
        
        let mut durations = Vec::new();
        
        for _ in 0..self.config.benchmark_iterations {
            let start = Instant::now();
            
            for server_data in test_data {
                let query = CacheQuery {
                    server_id: server_data.server_id.clone(),
                    freshness_level: FreshnessLevel::Cached,
                    include_disabled: false,
                };
                
                self.cache_manager.get_server_data(&query).await?;
            }
            
            let duration = start.elapsed();
            durations.push(duration);
        }
        
        Ok(self.calculate_operation_benchmark(durations, test_data.len()))
    }
    
    /// Benchmark concurrent operations
    async fn benchmark_concurrent_operations(&self, test_data: &[CachedServerData]) -> Result<ConcurrentBenchmark, CacheError> {
        // Concurrent reads
        let concurrent_reads = self.benchmark_concurrent_reads(test_data).await?;
        
        // Concurrent writes
        let concurrent_writes = self.benchmark_concurrent_writes(test_data).await?;
        
        // Mixed operations
        let mixed_operations = self.benchmark_mixed_operations(test_data).await?;
        
        Ok(ConcurrentBenchmark {
            concurrent_reads,
            concurrent_writes,
            mixed_operations,
        })
    }
    
    /// Benchmark concurrent read operations
    async fn benchmark_concurrent_reads(&self, test_data: &[CachedServerData]) -> Result<OperationBenchmark, CacheError> {
        // Populate cache first
        for server_data in test_data {
            self.cache_manager.store_server_data(server_data).await?;
        }
        
        let mut durations = Vec::new();
        
        for _ in 0..self.config.benchmark_iterations {
            let start = Instant::now();
            
            let mut handles = Vec::new();
            
            for chunk in test_data.chunks(self.config.concurrent_operations) {
                for server_data in chunk {
                    let cache_manager = self.cache_manager.clone();
                    let server_id = server_data.server_id.clone();
                    
                    let handle = tokio::spawn(async move {
                        let query = CacheQuery {
                            server_id,
                            freshness_level: FreshnessLevel::Cached,
                            include_disabled: false,
                        };
                        
                        cache_manager.get_server_data(&query).await
                    });
                    
                    handles.push(handle);
                }
                
                // Wait for this batch to complete
                for handle in handles.drain(..) {
                    handle.await.unwrap()?;
                }
            }
            
            let duration = start.elapsed();
            durations.push(duration);
        }
        
        Ok(self.calculate_operation_benchmark(durations, test_data.len()))
    }
    
    /// Benchmark concurrent write operations
    async fn benchmark_concurrent_writes(&self, test_data: &[CachedServerData]) -> Result<OperationBenchmark, CacheError> {
        let mut durations = Vec::new();
        
        for _ in 0..self.config.benchmark_iterations {
            self.cache_manager.clear_all().await?;
            
            let start = Instant::now();
            
            let mut handles = Vec::new();
            
            for chunk in test_data.chunks(self.config.concurrent_operations) {
                for server_data in chunk {
                    let cache_manager = self.cache_manager.clone();
                    let server_data = server_data.clone();
                    
                    let handle = tokio::spawn(async move {
                        cache_manager.store_server_data(&server_data).await
                    });
                    
                    handles.push(handle);
                }
                
                // Wait for this batch to complete
                for handle in handles.drain(..) {
                    handle.await.unwrap()?;
                }
            }
            
            let duration = start.elapsed();
            durations.push(duration);
        }
        
        Ok(self.calculate_operation_benchmark(durations, test_data.len()))
    }
    
    /// Benchmark mixed read/write operations
    async fn benchmark_mixed_operations(&self, test_data: &[CachedServerData]) -> Result<OperationBenchmark, CacheError> {
        let mut durations = Vec::new();
        
        for _ in 0..self.config.benchmark_iterations {
            self.cache_manager.clear_all().await?;
            
            let start = Instant::now();
            
            let mut handles = Vec::new();
            
            for (i, server_data) in test_data.iter().enumerate() {
                let cache_manager = self.cache_manager.clone();
                let server_data = server_data.clone();
                
                let handle = if i % 2 == 0 {
                    // Write operation
                    tokio::spawn(async move {
                        cache_manager.store_server_data(&server_data).await.map(|_| ())
                    })
                } else {
                    // Read operation (after a small delay to ensure some data exists)
                    tokio::spawn(async move {
                        sleep(Duration::from_millis(1)).await;
                        let query = CacheQuery {
                            server_id: server_data.server_id,
                            freshness_level: FreshnessLevel::Cached,
                            include_disabled: false,
                        };
                        cache_manager.get_server_data(&query).await.map(|_| ())
                    })
                };
                
                handles.push(handle);
                
                // Limit concurrency
                if handles.len() >= self.config.concurrent_operations {
                    for handle in handles.drain(..) {
                        handle.await.unwrap()?;
                    }
                }
            }
            
            // Wait for remaining handles
            for handle in handles {
                handle.await.unwrap()?;
            }
            
            let duration = start.elapsed();
            durations.push(duration);
        }
        
        Ok(self.calculate_operation_benchmark(durations, test_data.len()))
    }
    
    /// Calculate storage efficiency metrics
    async fn calculate_storage_efficiency(&self, test_data: &[CachedServerData]) -> Result<StorageEfficiency, CacheError> {
        // Store all test data
        for server_data in test_data {
            self.cache_manager.store_server_data(server_data).await?;
        }
        
        let stats = self.cache_manager.get_stats().await;
        
        // Calculate average item size
        let total_items = stats.total_servers + stats.total_tools + stats.total_resources + stats.total_prompts;
        let average_item_size = if total_items > 0 {
            stats.cache_size_bytes as f64 / total_items as f64
        } else {
            0.0
        };
        
        // Estimate compression ratio (simplified calculation)
        let estimated_json_size = self.estimate_json_size(test_data);
        let compression_ratio = if stats.cache_size_bytes > 0 {
            estimated_json_size as f64 / stats.cache_size_bytes as f64
        } else {
            1.0
        };
        
        // Calculate storage overhead (database metadata, indexes, etc.)
        let raw_data_size = self.estimate_raw_data_size(test_data);
        let storage_overhead_percent = if raw_data_size > 0 {
            ((stats.cache_size_bytes as f64 - raw_data_size as f64) / raw_data_size as f64) * 100.0
        } else {
            0.0
        };
        
        Ok(StorageEfficiency {
            total_cache_size_bytes: stats.cache_size_bytes,
            average_item_size_bytes: average_item_size,
            compression_ratio,
            storage_overhead_percent,
        })
    }
    
    /// Calculate operation benchmark statistics
    fn calculate_operation_benchmark(&self, durations: Vec<Duration>, operations_per_iteration: usize) -> OperationBenchmark {
        let mut duration_ms: Vec<f64> = durations.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
        duration_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let mean = duration_ms.iter().sum::<f64>() / duration_ms.len() as f64;
        let median = duration_ms[duration_ms.len() / 2];
        let p95_idx = (duration_ms.len() as f64 * 0.95) as usize;
        let p99_idx = (duration_ms.len() as f64 * 0.99) as usize;
        let p95 = duration_ms[p95_idx.min(duration_ms.len() - 1)];
        let p99 = duration_ms[p99_idx.min(duration_ms.len() - 1)];
        let min = duration_ms[0];
        let max = duration_ms[duration_ms.len() - 1];
        
        let total_operations = operations_per_iteration * self.config.benchmark_iterations;
        let total_time_seconds = durations.iter().map(|d| d.as_secs_f64()).sum::<f64>();
        let operations_per_second = total_operations as f64 / total_time_seconds;
        
        OperationBenchmark {
            mean_duration_ms: mean,
            median_duration_ms: median,
            p95_duration_ms: p95,
            p99_duration_ms: p99,
            min_duration_ms: min,
            max_duration_ms: max,
            operations_per_second,
            total_operations,
        }
    }
    
    /// Estimate JSON size for compression ratio calculation
    fn estimate_json_size(&self, test_data: &[CachedServerData]) -> u64 {
        test_data.iter()
            .map(|data| serde_json::to_string(data).unwrap_or_default().len() as u64)
            .sum()
    }
    
    /// Estimate raw data size (without serialization overhead)
    fn estimate_raw_data_size(&self, test_data: &[CachedServerData]) -> u64 {
        test_data.iter()
            .map(|data| {
                let tools_size: usize = data.tools.iter().map(|t| t.name.len() + t.description.as_ref().map_or(0, |d| d.len())).sum();
                let resources_size: usize = data.resources.iter().map(|r| r.uri.len() + r.name.as_ref().map_or(0, |n| n.len())).sum();
                let prompts_size: usize = data.prompts.iter().map(|p| p.name.len() + p.description.as_ref().map_or(0, |d| d.len())).sum();
                
                (data.server_id.len() + data.server_name.len() + tools_size + resources_size + prompts_size) as u64
            })
            .sum()
    }
    
    /// Run a single write benchmark iteration
    async fn run_write_benchmark(&self, test_data: &[CachedServerData]) -> Result<(), CacheError> {
        for server_data in test_data {
            self.cache_manager.store_server_data(server_data).await?;
        }
        Ok(())
    }
    
    /// Run a single read benchmark iteration
    async fn run_read_benchmark(&self, test_data: &[CachedServerData]) -> Result<(), CacheError> {
        for server_data in test_data {
            let query = CacheQuery {
                server_id: server_data.server_id.clone(),
                freshness_level: FreshnessLevel::Cached,
                include_disabled: false,
            };
            
            self.cache_manager.get_server_data(&query).await?;
        }
        Ok(())
    }
}

/// Print benchmark results in a formatted way
pub fn print_benchmark_results(results: &BenchmarkResults) {
    println!("\n=== Cache Performance Benchmark Results ===");
    println!("Timestamp: {}", results.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Configuration:");
    println!("  - Servers: {}", results.config.num_servers);
    println!("  - Total Items: {}", results.config.total_items);
    println!("  - Concurrent Operations: {}", results.config.concurrent_operations);
    println!("  - Iterations: {}", results.config.iterations);
    
    println!("\n--- Write Performance ---");
    print_operation_benchmark(&results.write_performance);
    
    println!("\n--- Read Performance ---");
    print_operation_benchmark(&results.read_performance);
    
    println!("\n--- Concurrent Performance ---");
    println!("Concurrent Reads:");
    print_operation_benchmark(&results.concurrent_performance.concurrent_reads);
    println!("Concurrent Writes:");
    print_operation_benchmark(&results.concurrent_performance.concurrent_writes);
    println!("Mixed Operations:");
    print_operation_benchmark(&results.concurrent_performance.mixed_operations);
    
    println!("\n--- Cache Efficiency ---");
    println!("Cache Hit Ratio: {:.2}%", results.cache_hit_ratio * 100.0);
    
    println!("\n--- Storage Efficiency ---");
    println!("Total Cache Size: {:.2} MB", results.storage_efficiency.total_cache_size_bytes as f64 / 1024.0 / 1024.0);
    println!("Average Item Size: {:.2} bytes", results.storage_efficiency.average_item_size_bytes);
    println!("Compression Ratio: {:.2}x", results.storage_efficiency.compression_ratio);
    println!("Storage Overhead: {:.2}%", results.storage_efficiency.storage_overhead_percent);
}

fn print_operation_benchmark(benchmark: &OperationBenchmark) {
    println!("  Mean: {:.2}ms", benchmark.mean_duration_ms);
    println!("  Median: {:.2}ms", benchmark.median_duration_ms);
    println!("  P95: {:.2}ms", benchmark.p95_duration_ms);
    println!("  P99: {:.2}ms", benchmark.p99_duration_ms);
    println!("  Min: {:.2}ms", benchmark.min_duration_ms);
    println!("  Max: {:.2}ms", benchmark.max_duration_ms);
    println!("  Operations/sec: {:.2}", benchmark.operations_per_second);
    println!("  Total Operations: {}", benchmark.total_operations);
}