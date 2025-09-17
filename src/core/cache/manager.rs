//! Enterprise-grade cache manager with optimized database access

use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use lru::LruCache;
use redb::{Database, ReadableTable};
use std::num::NonZeroUsize;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::{
    fingerprint::MCPServerFingerprint,
    operations::CacheOperations,
    statistics::{CacheStatistics, FingerprintOperations},
    types::*,
};

// Global singleton instance
static GLOBAL_CACHE_INSTANCE: OnceLock<Arc<RedbCacheManager>> = OnceLock::new();

/// High-performance cache manager using Redb
#[derive(Debug)]
pub struct RedbCacheManager {
    /// Shared database instance
    db: Arc<Database>,
    /// Database path for cloning
    db_path: PathBuf,
    /// L1 memory cache for hot data
    memory_cache: Arc<RwLock<LruCache<String, CachedServerData>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Runtime cache metrics (L1/L2 hits, misses, ops)
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl Clone for RedbCacheManager {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(), // Arc clone is cheap
            db_path: self.db_path.clone(),
            memory_cache: self.memory_cache.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_cache_size_mb: u64,
    pub cleanup_interval_minutes: u64,
    pub max_instance_ttl_hours: u64,
    pub enable_compression: bool,
    pub memory_cache_size: usize,
    pub enable_high_performance: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_cache_size_mb: 500,
            cleanup_interval_minutes: 30,
            max_instance_ttl_hours: 24,
            enable_compression: true,
            memory_cache_size: 1000,
            enable_high_performance: false,
        }
    }
}

/// Cache performance metrics
#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    pub total_queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub write_operations: u64,
    pub read_operations: u64,
    pub cache_invalidations: u64,
    pub last_cleanup: Option<DateTime<Utc>>,
    pub database_size_bytes: u64,
}

impl CacheMetrics {
    pub fn hit_ratio(&self) -> f64 {
        if self.total_queries == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_queries as f64
        }
    }
}

impl RedbCacheManager {
    /// Get or create global singleton instance
    pub fn global() -> Result<Arc<RedbCacheManager>> {
        Ok(GLOBAL_CACHE_INSTANCE
            .get_or_init(|| {
                let cache_path = crate::common::paths::global_paths().cache_dir().join("capability.redb");
                match Self::new(cache_path, CacheConfig::default()) {
                    Ok(manager) => Arc::new(manager),
                    Err(e) => {
                        tracing::error!("Failed to initialize global cache manager: {}", e);
                        panic!("Failed to initialize global cache manager: {}", e);
                    }
                }
            })
            .clone())
    }

    /// Generate cache key from server_id and instance_type (consistent with operations.rs)
    fn generate_cache_key(
        &self,
        server_id: &str,
        instance_type: &InstanceType,
    ) -> String {
        let instance_key = match instance_type {
            InstanceType::Production => "production".to_string(),
        };
        format!("{}#{}", server_id, instance_key)
    }

    /// Create a new cache manager
    pub fn new<P: AsRef<Path>>(
        db_path: P,
        config: CacheConfig,
    ) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create database
        let db = Arc::new(Database::create(&db_path)?);

        // Initialize database schema
        Self::initialize_schema(&db)?;

        // Create memory cache
        let memory_cache = Arc::new(RwLock::new(LruCache::new(
            NonZeroUsize::new(config.memory_cache_size).unwrap(),
        )));

        let manager = Self {
            db,
            db_path: db_path.clone(),
            memory_cache,
            config,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        };

        info!("Cache manager initialized at: {:?}", db_path);
        Ok(manager)
    }

    /// Initialize database schema
    fn initialize_schema(db: &Database) -> Result<()> {
        use super::schema::*;

        let write_txn = db.begin_write()?;
        {
            // Create all tables
            write_txn.open_table(SERVERS_TABLE)?;
            write_txn.open_table(TOOLS_TABLE)?;
            write_txn.open_table(RESOURCES_TABLE)?;
            write_txn.open_table(PROMPTS_TABLE)?;
            write_txn.open_table(RESOURCE_TEMPLATES_TABLE)?;
            write_txn.open_table(FINGERPRINTS_TABLE)?;
            write_txn.open_table(INSTANCE_METADATA_TABLE)?;
            write_txn.open_table(CACHE_STATS_TABLE)?;

            // Create multimap indexes
            write_txn.open_multimap_table(SERVER_TOOLS_INDEX)?;
            write_txn.open_multimap_table(SERVER_RESOURCES_INDEX)?;
            write_txn.open_multimap_table(SERVER_PROMPTS_INDEX)?;
            write_txn.open_multimap_table(SERVER_RESOURCE_TEMPLATES_INDEX)?;
        }
        write_txn.commit()?;

        debug!("Database schema initialized successfully");
        Ok(())
    }

    /// Store server data in cache with multi-layer caching
    pub async fn store_server_data(
        &self,
        server_data: &CachedServerData,
    ) -> Result<(), CacheError> {
        // Store in L2 (disk) cache
        let operations = CacheOperations::new(&self.db);
        operations.store_server_data(server_data)?;

        // Update L1 (memory) cache using composite key
        let cache_key = self.generate_cache_key(&server_data.server_id, &server_data.instance_type());
        let mut memory_cache = self.memory_cache.write().await;
        memory_cache.put(cache_key, server_data.clone());

        // Record write op
        {
            let mut m = self.metrics.write().await;
            m.write_operations += 1;
        }

        Ok(())
    }

    /// Retrieve server data from cache with multi-layer lookup
    pub async fn get_server_data(
        &self,
        query: &CacheQuery,
    ) -> Result<CacheResult<Option<CachedServerData>>, CacheError> {
        // Generate composite key for consistent lookup
        let cache_key = self.generate_cache_key(&query.server_id, &query.instance_type());
        tracing::debug!(
            "[CACHE][GET_DATA] key={} freshness={:?}",
            cache_key,
            query.freshness_level
        );

        // L1: Check memory cache first
        {
            let mut memory_cache = self.memory_cache.write().await;
            if let Some(cached_data) = memory_cache.get(&cache_key) {
                if self.is_data_fresh(cached_data, &query.freshness_level) {
                    tracing::debug!("[CACHE][L1 HIT] key={}", cache_key);
                    let mut m = self.metrics.write().await;
                    m.total_queries += 1;
                    m.cache_hits += 1;
                    m.read_operations += 1;
                    return Ok(CacheResult {
                        data: Some(cached_data.clone()),
                        cache_hit: true,
                        cached_at: Some(cached_data.cached_at),
                    });
                }
            }
            tracing::debug!("[CACHE][L1 MISS] key={}", cache_key);
        }

        // L2: Check disk cache
        let operations = CacheOperations::new(&self.db);
        let data = operations.get_server_data(query)?;

        let (cache_hit, cached_at) = if let Some(ref server_data) = data {
            tracing::debug!("[CACHE][L2 HIT] key={}", cache_key);
            // Update L1 cache with fresh data using composite key
            let mut memory_cache = self.memory_cache.write().await;
            memory_cache.put(cache_key.clone(), server_data.clone());

            let is_fresh = self.is_data_fresh(server_data, &query.freshness_level);
            // For RealTime requests, always return cache_hit=false to force fresh data
            if query.freshness_level == FreshnessLevel::RealTime {
                tracing::debug!("[CACHE][L2 FORCE REFRESH] key={} - RealTime requested", cache_key);
                let mut m = self.metrics.write().await;
                m.total_queries += 1;
                m.cache_misses += 1; // treat as miss for real-time
                m.read_operations += 1;
                (false, None)
            } else if is_fresh || query.freshness_level == FreshnessLevel::Cached {
                let mut m = self.metrics.write().await;
                m.total_queries += 1;
                m.cache_hits += 1;
                m.read_operations += 1;
                (true, Some(server_data.cached_at))
            } else {
                let mut m = self.metrics.write().await;
                m.total_queries += 1;
                m.cache_misses += 1;
                m.read_operations += 1;
                (false, None)
            }
        } else {
            tracing::debug!("[CACHE][L2 MISS] key={}", cache_key);
            let mut m = self.metrics.write().await;
            m.total_queries += 1;
            m.cache_misses += 1;
            m.read_operations += 1;
            (false, None)
        };

        Ok(CacheResult {
            data: if cache_hit { data } else { None },
            cache_hit,
            cached_at,
        })
    }

    /// Get server tools
    pub async fn get_server_tools(
        &self,
        server_id: &str,
        include_disabled: bool,
    ) -> Result<Vec<CachedToolInfo>, CacheError> {
        let operations = CacheOperations::new(&self.db);
        operations.get_server_tools(server_id, include_disabled)
    }

    /// Get server resources
    pub async fn get_server_resources(
        &self,
        server_id: &str,
        include_disabled: bool,
    ) -> Result<Vec<CachedResourceInfo>, CacheError> {
        let operations = CacheOperations::new(&self.db);
        operations.get_server_resources(server_id, include_disabled)
    }

    /// Get server prompts
    pub async fn get_server_prompts(
        &self,
        server_id: &str,
        include_disabled: bool,
    ) -> Result<Vec<CachedPromptInfo>, CacheError> {
        let operations = CacheOperations::new(&self.db);
        operations.get_server_prompts(server_id, include_disabled)
    }

    /// Get server resource templates
    pub async fn get_server_resource_templates(
        &self,
        server_id: &str,
        include_disabled: bool,
    ) -> Result<Vec<CachedResourceTemplateInfo>, CacheError> {
        let operations = CacheOperations::new(&self.db);
        operations.get_server_resource_templates(server_id, include_disabled)
    }

    /// Remove server data from cache
    pub async fn remove_server_data(
        &self,
        server_id: &str,
    ) -> Result<(), CacheError> {
        // Remove from L1 cache using composite key
        let composite_key = self.generate_cache_key(server_id, &InstanceType::Production);
        let mut memory_cache = self.memory_cache.write().await;
        memory_cache.pop(&composite_key);

        // Remove from L2 cache
        let operations = CacheOperations::new(&self.db);
        operations.remove_server_data(server_id)?;

        // Record invalidation
        {
            let mut m = self.metrics.write().await;
            m.cache_invalidations += 1;
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let statistics = CacheStatistics::new(&self.db, self.metrics.clone());
        let mut stats = statistics.get_stats();
        // Use live metrics for hit ratio
        let m = self.metrics.read().await;
        stats.hit_ratio = m.hit_ratio();
        stats
    }

    /// Get a snapshot of current metrics counters
    pub async fn get_metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }

    /// Clear all cache data
    pub async fn clear_all(&self) -> Result<(), CacheError> {
        // Clear L1 cache
        let mut memory_cache = self.memory_cache.write().await;
        memory_cache.clear();

        // Clear L2 cache
        let operations = CacheOperations::new(&self.db);
        operations.clear_all()?;

        // Record cleanup timestamp next to DB file (db_path's directory)
        let ts = Utc::now().to_rfc3339();
        if let Some(dir) = self.db_path.parent() {
            let stamp = dir.join(".last_cleanup");
            if let Err(e) = std::fs::write(&stamp, &ts) {
                tracing::warn!("Failed to write capability cache cleanup timestamp: {}", e);
            } else {
                tracing::info!("Recorded capability cache cleanup timestamp at {:?}", stamp);
            }
        }

        // Update metrics
        {
            let mut m = self.metrics.write().await;
            m.cache_invalidations += 1;
            m.last_cleanup = Some(Utc::now());
        }

        info!("Cache cleared successfully");
        Ok(())
    }

    /// Store fingerprint for a server
    pub async fn store_fingerprint(
        &self,
        server_id: &str,
        fingerprint: &MCPServerFingerprint,
    ) -> Result<(), CacheError> {
        let fingerprint_ops = FingerprintOperations::new(&self.db);
        fingerprint_ops.store_fingerprint(server_id, fingerprint)
    }

    /// Get stored fingerprint for a server
    pub async fn get_fingerprint(
        &self,
        server_id: &str,
    ) -> Result<Option<MCPServerFingerprint>, CacheError> {
        let fingerprint_ops = FingerprintOperations::new(&self.db);
        fingerprint_ops.get_fingerprint(server_id)
    }

    /// Check if server data should be invalidated based on fingerprint changes
    pub async fn should_invalidate_cache(
        &self,
        server_id: &str,
        current_fingerprint: &MCPServerFingerprint,
    ) -> Result<bool, CacheError> {
        let fingerprint_ops = FingerprintOperations::new(&self.db);
        fingerprint_ops.should_invalidate_cache(server_id, current_fingerprint)
    }

    /// Invalidate cache for a server and update fingerprint
    pub async fn invalidate_and_update(
        &self,
        server_id: &str,
        new_fingerprint: &MCPServerFingerprint,
    ) -> Result<(), CacheError> {
        self.remove_server_data(server_id).await?;
        self.store_fingerprint(server_id, new_fingerprint).await?;
        info!("Cache invalidated and fingerprint updated for server: {}", server_id);
        Ok(())
    }

    /// Smart cache update: only invalidate if fingerprint indicates changes
    pub async fn smart_cache_update(
        &self,
        server_id: &str,
        server_data: &CachedServerData,
        fingerprint: &MCPServerFingerprint,
    ) -> Result<bool, CacheError> {
        let should_invalidate = self.should_invalidate_cache(server_id, fingerprint).await?;

        if should_invalidate {
            self.invalidate_and_update(server_id, fingerprint).await?;
            self.store_server_data(server_data).await?;

            // Cache invalidation recorded
            Ok(true)
        } else {
            debug!("Fingerprint unchanged for server {}, skipping cache update", server_id);
            Ok(false)
        }
    }

    /// Generate fingerprint for a server based on its path/config
    pub async fn generate_server_fingerprint(
        &self,
        server_path: &std::path::Path,
    ) -> Result<MCPServerFingerprint, CacheError> {
        let fingerprint_ops = FingerprintOperations::new(&self.db);
        fingerprint_ops.generate_server_fingerprint(self.clone(), server_path)
    }

    /// Check if data is fresh based on freshness level
    fn is_data_fresh(
        &self,
        data: &CachedServerData,
        freshness_level: &FreshnessLevel,
    ) -> bool {
        match freshness_level {
            FreshnessLevel::Cached => true,
            FreshnessLevel::RecentlyFresh => {
                let age = Utc::now().signed_duration_since(data.cached_at);
                age.num_minutes() < 5
            }
            FreshnessLevel::RealTime => false,
        }
    }

    /// Expose database file path for observability
    pub fn database_path(&self) -> PathBuf {
        self.db_path.clone()
    }

    /// List entries in servers table with size and cached timestamp (for details view)
    pub async fn list_server_entries(
        &self,
        server_id_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ServerCacheEntry>, CacheError> {
        use super::schema::SERVERS_TABLE;
        let read_txn = self.db.begin_read()?;
        let servers_table = read_txn.open_table(SERVERS_TABLE)?;

        let mut results: Vec<ServerCacheEntry> = Vec::new();
        let mut count: usize = 0;
        for item in servers_table.iter()? {
            if count >= limit {
                break;
            }
            let (key, value) = item?;
            let key_str = key.value().to_string();
            if let Some(filter) = server_id_filter {
                if !key_str.starts_with(&format!("{}#", filter)) && key_str != filter {
                    continue;
                }
            }

            let value_bytes = value.value();
            // Try to deserialize to get cached_at and server_id for accurate data
            let (cached_at, server_id) = match bincode::deserialize::<CachedServerData>(value_bytes) {
                Ok(data) => (Some(data.cached_at), Some(data.server_id)),
                Err(_) => (None, None),
            };

            let parsed_server_id = parse_server_id_from_key(&key_str, server_id);

            results.push(ServerCacheEntry {
                key: key_str,
                server_id: parsed_server_id,
                approx_value_size_bytes: value_bytes.len() as u64,
                cached_at,
            });

            count += 1;
        }

        Ok(results)
    }

    /// Get last cleanup time (RFC3339) if recorded by clear_all
    pub fn get_last_cleanup_time(&self) -> Option<String> {
        if let Some(dir) = self.db_path.parent() {
            let stamp = dir.join(".last_cleanup");
            if stamp.exists() {
                return std::fs::read_to_string(stamp).ok().map(|s| s.trim().to_string());
            }
        }
        None
    }
}

/// Lightweight summary for a servers table entry (observability only)
#[derive(Debug, Clone)]
pub struct ServerCacheEntry {
    pub key: String,
    pub server_id: String,
    pub approx_value_size_bytes: u64,
    pub cached_at: Option<chrono::DateTime<Utc>>,
}

fn parse_server_id_from_key(
    key: &str,
    fallback: Option<String>,
) -> String {
    match key.split_once('#') {
        Some((sid, _)) => sid.to_string(),
        None => fallback.unwrap_or_else(|| key.to_string()),
    }
}
