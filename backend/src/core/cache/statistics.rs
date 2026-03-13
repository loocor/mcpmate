//! Cache statistics and fingerprint operations

use anyhow::Result;
use chrono::Utc;
use redb::{Database, ReadableTable, ReadableTableMetadata};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use super::{fingerprint::MCPServerFingerprint, manager::CacheMetrics, schema::*, types::*};

/// Cache statistics calculator
pub struct CacheStatistics<'a> {
    db: &'a Database,
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl<'a> CacheStatistics<'a> {
    pub fn new(
        db: &'a Database,
        metrics: Arc<RwLock<CacheMetrics>>,
    ) -> Self {
        Self { db, metrics }
    }

    /// Get comprehensive cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let db_size = self.calculate_database_size().unwrap_or(0);

        CacheStats {
            total_servers: self.count_servers().unwrap_or(0),
            total_tools: self.count_tools().unwrap_or(0),
            total_resources: self.count_resources().unwrap_or(0),
            total_prompts: self.count_prompts().unwrap_or(0),
            total_resource_templates: self.count_resource_templates().unwrap_or(0),
            cache_size_bytes: db_size,
            hit_ratio: self.calculate_hit_ratio(),
            last_updated: Utc::now(),
        }
    }

    /// Count servers in cache
    fn count_servers(&self) -> Result<u64> {
        let read_txn = self.db.begin_read()?;
        let servers_table = read_txn.open_table(SERVERS_TABLE)?;
        Ok(servers_table.len()?)
    }

    /// Count tools in cache
    fn count_tools(&self) -> Result<u64> {
        let read_txn = self.db.begin_read()?;
        let tools_table = read_txn.open_table(TOOLS_TABLE)?;
        Ok(tools_table.len()?)
    }

    /// Count resources in cache
    fn count_resources(&self) -> Result<u64> {
        let read_txn = self.db.begin_read()?;
        let resources_table = read_txn.open_table(RESOURCES_TABLE)?;
        Ok(resources_table.len()?)
    }

    /// Count prompts in cache
    fn count_prompts(&self) -> Result<u64> {
        let read_txn = self.db.begin_read()?;
        let prompts_table = read_txn.open_table(PROMPTS_TABLE)?;
        Ok(prompts_table.len()?)
    }

    /// Count resource templates in cache
    fn count_resource_templates(&self) -> Result<u64> {
        let read_txn = self.db.begin_read()?;
        let templates_table = read_txn.open_table(RESOURCE_TEMPLATES_TABLE)?;
        Ok(templates_table.len()?)
    }

    /// Calculate actual database size in bytes
    fn calculate_database_size(&self) -> Result<u64> {
        let read_txn = self.db.begin_read()?;
        let mut total_size = 0u64;

        // Sum up all table sizes
        if let Ok(servers_table) = read_txn.open_table(SERVERS_TABLE) {
            for (_, value) in (servers_table.iter()?).flatten() {
                total_size += value.value().len() as u64;
            }
        }

        if let Ok(tools_table) = read_txn.open_table(TOOLS_TABLE) {
            for (_, value) in (tools_table.iter()?).flatten() {
                total_size += value.value().len() as u64;
            }
        }

        if let Ok(resources_table) = read_txn.open_table(RESOURCES_TABLE) {
            for (_, value) in (resources_table.iter()?).flatten() {
                total_size += value.value().len() as u64;
            }
        }

        if let Ok(prompts_table) = read_txn.open_table(PROMPTS_TABLE) {
            for (_, value) in (prompts_table.iter()?).flatten() {
                total_size += value.value().len() as u64;
            }
        }

        Ok(total_size)
    }

    /// TODO: Improve cache hit ratio calculation
    fn calculate_hit_ratio(&self) -> f64 {
        // For now, return 0.0 as we need async access to metrics
        // This could be improved by making get_stats async or using try_read
        0.0
    }

    /// Update metrics for a cache hit
    pub async fn record_cache_hit(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_queries += 1;
        metrics.cache_hits += 1;
        metrics.read_operations += 1;
    }

    /// Update metrics for a cache miss
    pub async fn record_cache_miss(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_queries += 1;
        metrics.cache_misses += 1;
        metrics.read_operations += 1;
    }

    /// Update metrics for a write operation
    pub async fn record_write_operation(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.write_operations += 1;
    }

    /// Update metrics for cache invalidation
    pub async fn record_cache_invalidation(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.cache_invalidations += 1;
    }

    /// Reset all metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = CacheMetrics::default();
    }
}

/// Fingerprint operations for cache manager
pub struct FingerprintOperations<'a> {
    db: &'a Database,
}

impl<'a> FingerprintOperations<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Store fingerprint for a server
    pub fn store_fingerprint(
        &self,
        server_id: &str,
        fingerprint: &MCPServerFingerprint,
    ) -> Result<(), CacheError> {
        let write_txn = self.db.begin_write()?;

        {
            let mut fingerprints_table = write_txn.open_table(FINGERPRINTS_TABLE)?;
            let serialized = bincode::serialize(fingerprint)?;
            fingerprints_table.insert(server_id, serialized.as_slice())?;
        }

        write_txn.commit()?;
        debug!("Stored fingerprint for server: {}", server_id);
        Ok(())
    }

    /// Get stored fingerprint for a server
    pub fn get_fingerprint(
        &self,
        server_id: &str,
    ) -> Result<Option<MCPServerFingerprint>, CacheError> {
        let read_txn = self.db.begin_read()?;
        let fingerprints_table = read_txn.open_table(FINGERPRINTS_TABLE)?;

        if let Some(data) = fingerprints_table.get(server_id)? {
            let fingerprint: MCPServerFingerprint = bincode::deserialize(data.value())?;
            Ok(Some(fingerprint))
        } else {
            Ok(None)
        }
    }

    /// Check if server data should be invalidated based on fingerprint changes
    pub fn should_invalidate_cache(
        &self,
        server_id: &str,
        current_fingerprint: &MCPServerFingerprint,
    ) -> Result<bool, CacheError> {
        match self.get_fingerprint(server_id)? {
            Some(stored_fingerprint) => {
                let should_invalidate = stored_fingerprint.combined_hash != current_fingerprint.combined_hash;
                if should_invalidate {
                    debug!(
                        "Cache invalidation needed for server {}: fingerprint changed",
                        server_id
                    );
                }
                Ok(should_invalidate)
            }
            None => {
                debug!(
                    "No stored fingerprint for server {}, cache invalidation not needed",
                    server_id
                );
                Ok(false)
            }
        }
    }

    /// Generate fingerprint for a server based on its path/config
    pub fn generate_server_fingerprint(
        &self,
        _cache_manager: crate::core::cache::manager::RedbCacheManager,
        _server_path: &std::path::Path,
    ) -> Result<MCPServerFingerprint, CacheError> {
        // Simplified sync version - full async fingerprint generation
        // would require async trait methods which complicate the database manager integration
        Err(CacheError::InvalidFormat(
            "Fingerprint generation not implemented in sync context".to_string(),
        ))
    }
}
