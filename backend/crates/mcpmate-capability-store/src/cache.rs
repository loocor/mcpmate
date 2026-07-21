use std::future::Future;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::{
    Arc, Weak,
    atomic::{AtomicU64, Ordering},
};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use lru::LruCache;
use rmcp::model::{Prompt, Resource, ResourceTemplate, Tool};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::{CapabilityKind, CatalogSnapshot};

pub const DEFAULT_RAW_SNAPSHOT_CAPACITY: usize = 1_024;
pub const DEFAULT_PROJECTION_CAPACITY: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionEpoch(u64);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct RawSnapshotKey {
    pub server_id: String,
    pub catalog_revision: i64,
}

impl RawSnapshotKey {
    pub fn new(
        server_id: impl Into<String>,
        catalog_revision: i64,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            catalog_revision,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionNameDomain {
    Upstream,
    External,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ProjectionKey {
    pub selection_key: String,
    pub surface_fingerprint: String,
    pub capability_kind: CapabilityKind,
    pub name_domain: ProjectionNameDomain,
    pub catalog_revision_set_hash: String,
}

impl ProjectionKey {
    pub fn new(
        selection_key: impl Into<String>,
        surface_fingerprint: impl Into<String>,
        capability_kind: CapabilityKind,
        name_domain: ProjectionNameDomain,
        catalog_revision_set_hash: impl Into<String>,
    ) -> Self {
        Self {
            selection_key: selection_key.into(),
            surface_fingerprint: surface_fingerprint.into(),
            capability_kind,
            name_domain,
            catalog_revision_set_hash: catalog_revision_set_hash.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "items", rename_all = "snake_case")]
pub enum ProjectionPayload {
    Tools(Vec<Tool>),
    Prompts(Vec<Prompt>),
    Resources(Vec<Resource>),
    ResourceTemplates(Vec<ResourceTemplate>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct DerivedCacheMetrics {
    pub raw_entries: usize,
    pub projection_entries: usize,
    pub raw_hits: u64,
    pub raw_misses: u64,
    pub raw_loads: u64,
    pub raw_evictions: u64,
    pub projection_hits: u64,
    pub projection_misses: u64,
    pub projection_loads: u64,
    pub projection_evictions: u64,
    pub single_flight_waits: u64,
    pub invalidations: u64,
    pub total_queries: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DerivedCacheKeyDiagnostic {
    pub cache: &'static str,
    pub key_hash: String,
    pub approx_value_size_bytes: u64,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug)]
struct CacheEntry<T> {
    value: Arc<T>,
    approx_value_size_bytes: u64,
    cached_at: DateTime<Utc>,
}

impl<T> Clone for CacheEntry<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            approx_value_size_bytes: self.approx_value_size_bytes,
            cached_at: self.cached_at,
        }
    }
}

#[derive(Debug, Default)]
struct CacheCounters {
    raw_hits: AtomicU64,
    raw_misses: AtomicU64,
    raw_loads: AtomicU64,
    raw_evictions: AtomicU64,
    projection_hits: AtomicU64,
    projection_misses: AtomicU64,
    projection_loads: AtomicU64,
    projection_evictions: AtomicU64,
    single_flight_waits: AtomicU64,
    invalidations: AtomicU64,
}

#[derive(Debug)]
pub struct DerivedCapabilityCache {
    raw_snapshots: Mutex<LruCache<RawSnapshotKey, CacheEntry<CatalogSnapshot>>>,
    projections: Mutex<LruCache<ProjectionKey, CacheEntry<ProjectionPayload>>>,
    current_raw_keys: DashMap<String, RawSnapshotKey>,
    current_raw_flights: DashMap<String, Weak<Mutex<()>>>,
    raw_flights: DashMap<RawSnapshotKey, Weak<Mutex<()>>>,
    projection_flights: DashMap<ProjectionKey, Weak<Mutex<()>>>,
    server_generations: DashMap<String, u64>,
    generation: AtomicU64,
    projection_generation: AtomicU64,
    raw_snapshot_capacity: usize,
    projection_capacity: usize,
    counters: CacheCounters,
}

impl Default for DerivedCapabilityCache {
    fn default() -> Self {
        Self::new(DEFAULT_RAW_SNAPSHOT_CAPACITY, DEFAULT_PROJECTION_CAPACITY)
    }
}

impl DerivedCapabilityCache {
    pub fn new(
        raw_snapshot_capacity: usize,
        projection_capacity: usize,
    ) -> Self {
        let raw_snapshot_capacity =
            NonZeroUsize::new(raw_snapshot_capacity).expect("raw snapshot capacity must be non-zero");
        let projection_capacity = NonZeroUsize::new(projection_capacity).expect("projection capacity must be non-zero");
        Self {
            raw_snapshots: Mutex::new(LruCache::new(raw_snapshot_capacity)),
            projections: Mutex::new(LruCache::new(projection_capacity)),
            current_raw_keys: DashMap::new(),
            current_raw_flights: DashMap::new(),
            raw_flights: DashMap::new(),
            projection_flights: DashMap::new(),
            server_generations: DashMap::new(),
            generation: AtomicU64::new(0),
            projection_generation: AtomicU64::new(0),
            raw_snapshot_capacity: raw_snapshot_capacity.get(),
            projection_capacity: projection_capacity.get(),
            counters: CacheCounters::default(),
        }
    }

    pub async fn get_or_load_snapshot<F, Fut, E>(
        &self,
        key: RawSnapshotKey,
        loader: F,
    ) -> std::result::Result<Option<Arc<CatalogSnapshot>>, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = std::result::Result<Option<CatalogSnapshot>, E>>,
    {
        let generation = self.generation.load(Ordering::Acquire);
        let server_generation = self.server_generation(&key.server_id);
        if let Some(value) = self.raw_snapshot_hit(&key).await {
            return Ok(Some(value));
        }
        self.counters.raw_misses.fetch_add(1, Ordering::Relaxed);

        let flight = flight_lock(&self.raw_flights, &key, self.raw_snapshot_capacity);
        let guard = acquire_flight_guard(&flight, &self.counters).await;

        if let Some(value) = self.raw_snapshot_peek(&key).await {
            drop(guard);
            return Ok(Some(value));
        }

        let loaded = loader().await?;
        self.counters.raw_loads.fetch_add(1, Ordering::Relaxed);
        let Some(snapshot) = loaded else {
            drop(guard);
            return Ok(None);
        };
        let entry = CacheEntry::new(snapshot);
        let value = entry.value.clone();
        let mut raw = self.raw_snapshots.lock().await;
        if self.is_raw_cache_stale(generation, &key.server_id, server_generation) {
            drop(raw);
            drop(guard);
            return Ok(Some(value));
        }
        if raw.push(key, entry).is_some() {
            self.counters.raw_evictions.fetch_add(1, Ordering::Relaxed);
        }
        drop(raw);
        drop(guard);
        Ok(Some(value))
    }

    pub async fn get_or_load_current_snapshot<F, Fut, E>(
        &self,
        server_id: &str,
        loader: F,
    ) -> std::result::Result<Option<Arc<CatalogSnapshot>>, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = std::result::Result<Option<CatalogSnapshot>, E>>,
    {
        if let Some(value) = self.current_snapshot_hit(server_id).await {
            return Ok(Some(value));
        }
        self.counters.raw_misses.fetch_add(1, Ordering::Relaxed);
        let generation = self.generation.load(Ordering::Acquire);
        let server_generation = self.server_generation(server_id);

        let flight = flight_lock(
            &self.current_raw_flights,
            &server_id.to_owned(),
            self.raw_snapshot_capacity,
        );
        let guard = acquire_flight_guard(&flight, &self.counters).await;
        if let Some(value) = self.current_snapshot_peek(server_id).await {
            drop(guard);
            return Ok(Some(value));
        }

        let loaded = loader().await?;
        self.counters.raw_loads.fetch_add(1, Ordering::Relaxed);
        let Some(snapshot) = loaded else {
            drop(guard);
            return Ok(None);
        };
        let key = RawSnapshotKey::new(&snapshot.server_id, snapshot.revision);
        let entry = CacheEntry::new(snapshot);
        let value = entry.value.clone();
        let mut raw = self.raw_snapshots.lock().await;
        if self.is_raw_cache_stale(generation, server_id, server_generation) {
            drop(raw);
            drop(guard);
            return Ok(Some(value));
        }
        if let Some((evicted_key, _)) = raw.push(key.clone(), entry) {
            if self
                .current_raw_keys
                .get(&evicted_key.server_id)
                .is_some_and(|current| *current == evicted_key)
            {
                self.current_raw_keys.remove(&evicted_key.server_id);
            }
            self.counters.raw_evictions.fetch_add(1, Ordering::Relaxed);
        }
        drop(raw);
        self.current_raw_keys.insert(server_id.to_owned(), key);
        drop(guard);
        Ok(Some(value))
    }

    pub async fn get_or_project<F, Fut, E>(
        &self,
        key: ProjectionKey,
        projector: F,
    ) -> std::result::Result<Arc<ProjectionPayload>, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = std::result::Result<ProjectionPayload, E>>,
    {
        self.get_or_project_at_epoch(key, self.projection_epoch(), projector)
            .await
    }

    pub fn projection_epoch(&self) -> ProjectionEpoch {
        ProjectionEpoch(self.projection_generation.load(Ordering::Acquire))
    }

    pub async fn get_or_project_at_epoch<F, Fut, E>(
        &self,
        key: ProjectionKey,
        expected_epoch: ProjectionEpoch,
        projector: F,
    ) -> std::result::Result<Arc<ProjectionPayload>, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = std::result::Result<ProjectionPayload, E>>,
    {
        if expected_epoch != self.projection_epoch() {
            return projector().await.map(Arc::new);
        }
        if let Some(value) = self.projection_hit(&key).await {
            return Ok(value);
        }
        self.counters.projection_misses.fetch_add(1, Ordering::Relaxed);
        let generation = self.generation.load(Ordering::Acquire);

        let flight = flight_lock(&self.projection_flights, &key, self.projection_capacity);
        let guard = acquire_flight_guard(&flight, &self.counters).await;

        if expected_epoch != self.projection_epoch() {
            drop(guard);
            return projector().await.map(Arc::new);
        }
        if let Some(value) = self.projection_peek(&key).await {
            drop(guard);
            return Ok(value);
        }

        let projected = projector().await?;
        self.counters.projection_loads.fetch_add(1, Ordering::Relaxed);
        let entry = CacheEntry::new(projected);
        let value = entry.value.clone();
        let mut projections = self.projections.lock().await;
        if generation != self.generation.load(Ordering::Acquire) || expected_epoch != self.projection_epoch() {
            drop(projections);
            drop(guard);
            return Ok(value);
        }
        if projections.push(key, entry).is_some() {
            self.counters.projection_evictions.fetch_add(1, Ordering::Relaxed);
        }
        drop(projections);
        drop(guard);
        Ok(value)
    }

    async fn raw_snapshot_hit(
        &self,
        key: &RawSnapshotKey,
    ) -> Option<Arc<CatalogSnapshot>> {
        let value = self.raw_snapshot_peek(key).await;
        if value.is_some() {
            self.counters.raw_hits.fetch_add(1, Ordering::Relaxed);
        }
        value
    }

    async fn raw_snapshot_peek(
        &self,
        key: &RawSnapshotKey,
    ) -> Option<Arc<CatalogSnapshot>> {
        self.raw_snapshots
            .lock()
            .await
            .get(key)
            .map(|entry| entry.value.clone())
    }

    async fn current_snapshot_hit(
        &self,
        server_id: &str,
    ) -> Option<Arc<CatalogSnapshot>> {
        self.current_snapshot_lookup(server_id, true).await
    }

    async fn current_snapshot_peek(
        &self,
        server_id: &str,
    ) -> Option<Arc<CatalogSnapshot>> {
        self.current_snapshot_lookup(server_id, false).await
    }

    async fn current_snapshot_lookup(
        &self,
        server_id: &str,
        count_hit: bool,
    ) -> Option<Arc<CatalogSnapshot>> {
        let key = self.current_raw_keys.get(server_id).map(|entry| entry.clone())?;
        let value = if count_hit {
            self.raw_snapshot_hit(&key).await
        } else {
            self.raw_snapshot_peek(&key).await
        };
        if value.is_none() {
            self.current_raw_keys.remove(server_id);
        }
        value
    }

    async fn projection_hit(
        &self,
        key: &ProjectionKey,
    ) -> Option<Arc<ProjectionPayload>> {
        let value = self.projection_peek(key).await;
        if value.is_some() {
            self.counters.projection_hits.fetch_add(1, Ordering::Relaxed);
        }
        value
    }

    async fn projection_peek(
        &self,
        key: &ProjectionKey,
    ) -> Option<Arc<ProjectionPayload>> {
        self.projections.lock().await.get(key).map(|entry| entry.value.clone())
    }

    pub async fn clear(&self) {
        let mut raw = self.raw_snapshots.lock().await;
        let mut projections = self.projections.lock().await;
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.projection_generation.fetch_add(1, Ordering::AcqRel);
        raw.clear();
        projections.clear();
        drop(projections);
        drop(raw);
        self.current_raw_keys.clear();
        self.current_raw_flights.clear();
        self.raw_flights.clear();
        self.projection_flights.clear();
        self.server_generations.clear();
        self.counters.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn invalidate_server(
        &self,
        server_id: &str,
    ) {
        self.projection_generation.fetch_add(1, Ordering::AcqRel);
        let mut raw = self.raw_snapshots.lock().await;
        self.server_generations
            .entry(server_id.to_owned())
            .and_modify(|generation| *generation = generation.saturating_add(1))
            .or_insert(1);
        let keys = raw
            .iter()
            .filter(|(key, _)| key.server_id == server_id)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in keys {
            raw.pop(&key);
            self.raw_flights.remove(&key);
        }
        drop(raw);
        self.current_raw_keys.remove(server_id);
        self.current_raw_flights.remove(server_id);
        self.projections.lock().await.clear();
        self.projection_flights.clear();
        self.counters.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    fn server_generation(
        &self,
        server_id: &str,
    ) -> u64 {
        self.server_generations.get(server_id).map_or(0, |entry| *entry)
    }

    fn is_raw_cache_stale(
        &self,
        observed_generation: u64,
        server_id: &str,
        observed_server_generation: u64,
    ) -> bool {
        observed_generation != self.generation.load(Ordering::Acquire)
            || observed_server_generation != self.server_generation(server_id)
    }

    pub async fn metrics(&self) -> DerivedCacheMetrics {
        let raw_entries = self.raw_snapshots.lock().await.len();
        let projection_entries = self.projections.lock().await.len();
        let raw_hits = self.counters.raw_hits.load(Ordering::Relaxed);
        let raw_misses = self.counters.raw_misses.load(Ordering::Relaxed);
        let projection_hits = self.counters.projection_hits.load(Ordering::Relaxed);
        let projection_misses = self.counters.projection_misses.load(Ordering::Relaxed);
        DerivedCacheMetrics {
            raw_entries,
            projection_entries,
            raw_hits,
            raw_misses,
            raw_loads: self.counters.raw_loads.load(Ordering::Relaxed),
            raw_evictions: self.counters.raw_evictions.load(Ordering::Relaxed),
            projection_hits,
            projection_misses,
            projection_loads: self.counters.projection_loads.load(Ordering::Relaxed),
            projection_evictions: self.counters.projection_evictions.load(Ordering::Relaxed),
            single_flight_waits: self.counters.single_flight_waits.load(Ordering::Relaxed),
            invalidations: self.counters.invalidations.load(Ordering::Relaxed),
            total_queries: raw_hits + raw_misses + projection_hits + projection_misses,
        }
    }

    pub async fn diagnostic_keys(
        &self,
        limit: usize,
    ) -> Vec<DerivedCacheKeyDiagnostic> {
        self.diagnostic_keys_for_server(limit, None).await
    }

    pub async fn diagnostic_keys_for_server(
        &self,
        limit: usize,
        server_id: Option<&str>,
    ) -> Vec<DerivedCacheKeyDiagnostic> {
        let mut diagnostics = Vec::with_capacity(limit);
        {
            let raw = self.raw_snapshots.lock().await;
            diagnostics.extend(
                raw.iter()
                    .filter(|(key, _)| server_id.is_none_or(|server_id| key.server_id == server_id))
                    .take(limit)
                    .map(|(key, entry)| DerivedCacheKeyDiagnostic::new("raw_snapshot", key, entry)),
            );
        }
        if server_id.is_none() && diagnostics.len() < limit {
            let remaining = limit - diagnostics.len();
            let projections = self.projections.lock().await;
            diagnostics.extend(
                projections
                    .iter()
                    .take(remaining)
                    .map(|(key, entry)| DerivedCacheKeyDiagnostic::new("client_projection", key, entry)),
            );
        }
        diagnostics
    }
}

fn flight_lock<K>(
    flights: &DashMap<K, Weak<Mutex<()>>>,
    key: &K,
    capacity: usize,
) -> Arc<Mutex<()>>
where
    K: Clone + Eq + Hash,
{
    if flights.len() > capacity.saturating_mul(2) {
        flights.retain(|_, flight| flight.strong_count() > 0);
    }
    match flights.entry(key.clone()) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            if let Some(flight) = entry.get().upgrade() {
                flight
            } else {
                let flight = Arc::new(Mutex::new(()));
                entry.insert(Arc::downgrade(&flight));
                flight
            }
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            let flight = Arc::new(Mutex::new(()));
            entry.insert(Arc::downgrade(&flight));
            flight
        }
    }
}

async fn acquire_flight_guard<'a>(
    flight: &'a Arc<Mutex<()>>,
    counters: &CacheCounters,
) -> tokio::sync::MutexGuard<'a, ()> {
    match flight.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            counters.single_flight_waits.fetch_add(1, Ordering::Relaxed);
            flight.lock().await
        }
    }
}

impl<T: Serialize> CacheEntry<T> {
    fn new(value: T) -> Self {
        let approx_value_size_bytes = serde_json::to_vec(&value)
            .map(|encoded| encoded.len() as u64)
            .unwrap_or_default();
        Self {
            value: Arc::new(value),
            approx_value_size_bytes,
            cached_at: Utc::now(),
        }
    }
}

impl DerivedCacheKeyDiagnostic {
    fn new<K: Serialize, V>(
        cache: &'static str,
        key: &K,
        entry: &CacheEntry<V>,
    ) -> Self {
        let encoded = serde_json::to_vec(key).unwrap_or_default();
        let digest = Sha256::digest(encoded);
        let key_hash = digest[..12].iter().map(|byte| format!("{byte:02x}")).collect();
        Self {
            cache,
            key_hash,
            approx_value_size_bytes: entry.approx_value_size_bytes,
            cached_at: entry.cached_at,
        }
    }
}
