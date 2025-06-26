// Capabilities Cache Manager
// Implements dual-level caching strategy with memory LRU and file storage

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use super::client::McpDiscoveryClient;
use super::storage::{CacheStats, TempFileStorage};
use super::types::{
    CacheConfig, DiscoveryError, DiscoveryResult, RefreshStrategy, ServerCapabilities,
};
use crate::config::database::Database;
use crate::core::events::{
    bus::EventBus,
    types::{DiscoveryCacheUpdateType, Event},
};

/// Dual-level capabilities cache manager
pub struct CapabilitiesCache {
    /// L1: Memory LRU cache for hot data
    memory_cache: Arc<RwLock<LruCache<String, CachedCapabilities>>>,
    /// L2: File storage for warm data
    file_storage: TempFileStorage,
    /// Discovery client for fetching fresh data
    client: McpDiscoveryClient,
    /// Cache configuration
    config: CacheConfig,
    /// Event bus for notifications (optional)
    event_bus: Option<Arc<EventBus>>,
}

/// Cached capabilities with metadata
#[derive(Debug, Clone)]
struct CachedCapabilities {
    /// The actual capabilities data
    capabilities: ServerCapabilities,
    /// When this entry was cached
    cached_at: SystemTime,
    /// When this entry was last accessed
    last_accessed: SystemTime,
}

/// Cache result with hit information
#[derive(Debug, Clone)]
pub struct CacheResult {
    /// The capabilities data
    pub capabilities: ServerCapabilities,
    /// Whether this was a cache hit
    pub cache_hit: bool,
    /// Cache level that was hit (if any)
    pub cache_level: Option<CacheLevel>,
}

/// Cache level enumeration
#[derive(Debug, Clone, Copy)]
pub enum CacheLevel {
    /// L1 memory cache
    Memory,
    /// L2 file cache
    File,
}

impl CapabilitiesCache {
    /// Create new capabilities cache
    pub fn new(config: CacheConfig) -> DiscoveryResult<Self> {
        let memory_cache_size = NonZeroUsize::new(config.memory_cache_size).ok_or_else(|| {
            DiscoveryError::InvalidConfig("Memory cache size must be greater than 0".to_string())
        })?;

        let memory_cache = Arc::new(RwLock::new(LruCache::new(memory_cache_size)));

        let mut file_storage = TempFileStorage::new(config.clone())?;
        file_storage.start_cleanup_scheduler();

        let client = McpDiscoveryClient::new();

        Ok(Self {
            memory_cache,
            file_storage,
            client,
            config,
            event_bus: None,
        })
    }

    /// Create capabilities cache with event bus support
    pub fn with_event_bus(
        config: CacheConfig,
        event_bus: Arc<EventBus>,
    ) -> DiscoveryResult<Self> {
        let mut cache = Self::new(config)?;
        cache.event_bus = Some(event_bus);
        Ok(cache)
    }

    /// Get server capabilities with caching strategy
    pub async fn get_capabilities(
        &self,
        server_id: &str,
        refresh_strategy: RefreshStrategy,
        database: &Database,
    ) -> DiscoveryResult<CacheResult> {
        match refresh_strategy {
            RefreshStrategy::CacheFirst => {
                self.get_capabilities_cache_first(server_id, database).await
            }
            RefreshStrategy::RefreshIfStale => {
                self.get_capabilities_refresh_if_stale(server_id, database)
                    .await
            }
            RefreshStrategy::Force => {
                self.get_capabilities_force_refresh(server_id, database)
                    .await
            }
        }
    }

    /// Cache-first strategy: use cache if available, otherwise fetch
    async fn get_capabilities_cache_first(
        &self,
        server_id: &str,
        database: &Database,
    ) -> DiscoveryResult<CacheResult> {
        // Try L1 cache first
        if let Some(capabilities) = self.get_from_memory_cache(server_id).await {
            tracing::debug!("Cache hit (L1) for server '{}'", server_id);
            return Ok(CacheResult {
                capabilities,
                cache_hit: true,
                cache_level: Some(CacheLevel::Memory),
            });
        }

        // Try L2 cache
        if let Some(capabilities) = self.get_from_file_cache(server_id).await? {
            // Promote to L1 cache
            self.put_to_memory_cache(server_id, &capabilities).await;
            tracing::debug!("Cache hit (L2) for server '{}'", server_id);
            return Ok(CacheResult {
                capabilities,
                cache_hit: true,
                cache_level: Some(CacheLevel::File),
            });
        }

        // Cache miss - fetch fresh data
        tracing::debug!("Cache miss for server '{}', fetching fresh data", server_id);
        let capabilities = self.fetch_and_cache(server_id, database).await?;
        Ok(CacheResult {
            capabilities,
            cache_hit: false,
            cache_level: None,
        })
    }

    /// Refresh-if-stale strategy: check TTL and refresh if needed
    async fn get_capabilities_refresh_if_stale(
        &self,
        server_id: &str,
        database: &Database,
    ) -> DiscoveryResult<CacheResult> {
        // Check L1 cache with TTL
        if let Some(cached) = self.get_from_memory_cache_with_metadata(server_id).await {
            if !self.is_stale(&cached) {
                tracing::debug!("Cache hit (L1, fresh) for server '{}'", server_id);
                return Ok(CacheResult {
                    capabilities: cached.capabilities,
                    cache_hit: true,
                    cache_level: Some(CacheLevel::Memory),
                });
            }
        }

        // Check L2 cache with TTL
        if let Some(capabilities) = self.get_from_file_cache(server_id).await? {
            if !self.is_capabilities_stale(&capabilities) {
                // Promote to L1 cache
                self.put_to_memory_cache(server_id, &capabilities).await;
                tracing::debug!("Cache hit (L2, fresh) for server '{}'", server_id);
                return Ok(CacheResult {
                    capabilities,
                    cache_hit: true,
                    cache_level: Some(CacheLevel::File),
                });
            }
        }

        // Stale or missing - fetch fresh data
        tracing::debug!(
            "Cache stale for server '{}', fetching fresh data",
            server_id
        );
        let capabilities = self.fetch_and_cache(server_id, database).await?;
        Ok(CacheResult {
            capabilities,
            cache_hit: false,
            cache_level: None,
        })
    }

    /// Force refresh strategy: always fetch fresh data
    async fn get_capabilities_force_refresh(
        &self,
        server_id: &str,
        database: &Database,
    ) -> DiscoveryResult<CacheResult> {
        tracing::debug!("Force refresh for server '{}'", server_id);
        let capabilities = self.fetch_and_cache(server_id, database).await?;
        Ok(CacheResult {
            capabilities,
            cache_hit: false,
            cache_level: None,
        })
    }

    /// Get capabilities from memory cache
    async fn get_from_memory_cache(
        &self,
        server_id: &str,
    ) -> Option<ServerCapabilities> {
        let mut cache = self.memory_cache.write().await;
        if let Some(cached) = cache.get_mut(server_id) {
            cached.last_accessed = SystemTime::now();
            Some(cached.capabilities.clone())
        } else {
            None
        }
    }

    /// Get capabilities from memory cache with metadata
    async fn get_from_memory_cache_with_metadata(
        &self,
        server_id: &str,
    ) -> Option<CachedCapabilities> {
        let mut cache = self.memory_cache.write().await;
        if let Some(cached) = cache.get_mut(server_id) {
            cached.last_accessed = SystemTime::now();
            Some(cached.clone())
        } else {
            None
        }
    }

    /// Get capabilities from file cache
    async fn get_from_file_cache(
        &self,
        server_id: &str,
    ) -> DiscoveryResult<Option<ServerCapabilities>> {
        self.file_storage.load_capabilities(server_id).await
    }

    /// Put capabilities to memory cache
    async fn put_to_memory_cache(
        &self,
        server_id: &str,
        capabilities: &ServerCapabilities,
    ) {
        let cached = CachedCapabilities {
            capabilities: capabilities.clone(),
            cached_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
        };

        let mut cache = self.memory_cache.write().await;
        cache.put(server_id.to_string(), cached);
    }

    /// Fetch fresh capabilities and cache them
    async fn fetch_and_cache(
        &self,
        server_id: &str,
        database: &Database,
    ) -> DiscoveryResult<ServerCapabilities> {
        // Fetch fresh data from server
        let capabilities = self
            .client
            .get_server_capabilities(server_id, database)
            .await?;

        // Store in both cache levels
        self.put_to_memory_cache(server_id, &capabilities).await;
        self.file_storage
            .store_capabilities(server_id, &capabilities)
            .await?;

        tracing::debug!(
            "Fetched and cached fresh capabilities for server '{}'",
            server_id
        );
        Ok(capabilities)
    }

    /// Check if cached capabilities are stale
    fn is_stale(
        &self,
        cached: &CachedCapabilities,
    ) -> bool {
        let elapsed = SystemTime::now()
            .duration_since(cached.cached_at)
            .unwrap_or(Duration::from_secs(0));

        elapsed > self.config.default_ttl
    }

    /// Check if capabilities from file are stale
    fn is_capabilities_stale(
        &self,
        capabilities: &ServerCapabilities,
    ) -> bool {
        let elapsed = SystemTime::now()
            .duration_since(capabilities.metadata.last_updated)
            .unwrap_or(Duration::from_secs(0));

        elapsed > capabilities.metadata.ttl
    }

    /// Invalidate cache entry for a server
    pub async fn invalidate(
        &self,
        server_id: &str,
    ) -> DiscoveryResult<()> {
        // Remove from memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.pop(server_id);
        }

        // Note: We don't remove from file cache immediately to allow for
        // potential recovery if the server is temporarily unavailable
        tracing::debug!("Invalidated cache for server '{}'", server_id);
        Ok(())
    }

    /// Clear all cache entries
    pub async fn clear_all(&self) -> DiscoveryResult<()> {
        // Clear memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.clear();
        }

        // Clear file cache
        self.file_storage.clear_cache().await?;

        tracing::info!("Cleared all cache entries");
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> DiscoveryResult<CacheManagerStats> {
        let memory_stats = {
            let cache = self.memory_cache.read().await;
            MemoryCacheStats {
                entries: cache.len(),
                capacity: cache.cap().get(),
            }
        };

        let file_stats = self.file_storage.get_cache_stats().await?;

        Ok(CacheManagerStats {
            memory: memory_stats,
            file: file_stats,
        })
    }

    /// Preload capabilities for multiple servers
    pub async fn preload_servers(
        &self,
        server_ids: &[String],
        database: &Database,
    ) -> DiscoveryResult<PreloadResult> {
        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for server_id in server_ids {
            match self
                .get_capabilities(server_id, RefreshStrategy::CacheFirst, database)
                .await
            {
                Ok(_) => {
                    successful.push(server_id.clone());
                    tracing::debug!("Preloaded capabilities for server '{}'", server_id);
                }
                Err(e) => {
                    failed.push((server_id.clone(), e.to_string()));
                    tracing::warn!(
                        "Failed to preload capabilities for server '{}': {}",
                        server_id,
                        e
                    );
                }
            }
        }

        Ok(PreloadResult { successful, failed })
    }

    /// Refresh capabilities for multiple servers in background
    pub async fn background_refresh(
        &self,
        server_ids: &[String],
        database: &Database,
    ) -> DiscoveryResult<()> {
        let cache = self.memory_cache.clone();
        let client = self.client.clone();
        let file_storage = &self.file_storage;
        let event_bus = self.event_bus.clone();

        for server_id in server_ids {
            let server_id = server_id.clone();
            let cache = cache.clone();
            let client = client.clone();
            let database = database.clone();
            let file_storage = file_storage.clone();
            let event_bus = event_bus.clone();

            tokio::spawn(async move {
                match client.get_server_capabilities(&server_id, &database).await {
                    Ok(capabilities) => {
                        // Update memory cache
                        let cached = CachedCapabilities {
                            capabilities: capabilities.clone(),
                            cached_at: SystemTime::now(),
                            last_accessed: SystemTime::now(),
                        };

                        let mut memory_cache = cache.write().await;
                        memory_cache.put(server_id.clone(), cached);
                        drop(memory_cache); // Release lock early

                        // Update file cache to maintain consistency
                        if let Err(e) = file_storage
                            .store_capabilities(&server_id, &capabilities)
                            .await
                        {
                            tracing::warn!(
                                "Failed to update file cache during background refresh for server '{}': {}",
                                server_id,
                                e
                            );
                        }

                        // Emit background refresh event
                        if let Some(event_bus) = &event_bus {
                            let event = Event::DiscoveryCacheUpdated {
                                server_id: server_id.clone(),
                                server_name: capabilities.server_name.clone(),
                                update_type: DiscoveryCacheUpdateType::BackgroundRefresh,
                            };
                            event_bus.publish(event);
                        }

                        tracing::debug!("Background refresh completed for server '{}'", server_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Background refresh failed for server '{}': {}",
                            server_id,
                            e
                        );
                    }
                }
            });
        }

        Ok(())
    }

    /// Get memory cache hit ratio
    pub async fn get_hit_ratio(&self) -> f64 {
        // This is a simplified implementation
        // In a real scenario, you'd track hits and misses
        let cache = self.memory_cache.read().await;
        if cache.cap().get() == 0 {
            0.0
        } else {
            cache.len() as f64 / cache.cap().get() as f64
        }
    }
}

/// Cache manager statistics
#[derive(Debug, Clone)]
pub struct CacheManagerStats {
    /// Memory cache statistics
    pub memory: MemoryCacheStats,
    /// File cache statistics
    pub file: CacheStats,
}

/// Memory cache statistics
#[derive(Debug, Clone)]
pub struct MemoryCacheStats {
    /// Number of entries in memory cache
    pub entries: usize,
    /// Memory cache capacity
    pub capacity: usize,
}

/// Preload operation result
#[derive(Debug, Clone)]
pub struct PreloadResult {
    /// Successfully preloaded servers
    pub successful: Vec<String>,
    /// Failed preloads with error messages
    pub failed: Vec<(String, String)>,
}
