// Cache-related data models
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Import the unified response macro
use crate::macros::resp::api_resp;

// ==================== Request Models ====================

#[derive(Debug, Clone, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "View type for cache details")]
pub enum CacheViewType {
    #[default]
    #[schemars(description = "Show storage statistics and performance metrics")]
    Stats,
    #[schemars(description = "Show individual cache keys")]
    Keys,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Query parameters for cache details")]
pub struct CacheDetailsReq {
    #[serde(default)]
    #[schemars(description = "View type - stats or keys")]
    pub view: CacheViewType,
    #[schemars(description = "Optional server ID to filter by")]
    pub server_id: Option<String>,
    #[schemars(description = "Maximum number of items to return (max 1000)")]
    pub limit: Option<usize>,
    // TODO: Implement cache result sorting functionality for better data organization
    #[allow(dead_code)]
    #[schemars(description = "Sort field - size, age, or key (reserved)")]
    pub sort_by: Option<String>,
    // TODO: Implement cache result ordering functionality (ascending/descending)
    #[allow(dead_code)]
    #[schemars(description = "Sort order - asc or desc (reserved)")]
    pub order: Option<String>,
}

// ==================== Response Models ====================

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Cache details response")]
pub struct CacheDetailsData {
    #[schemars(description = "Cache keys information (when view=keys)")]
    pub keys: Option<Vec<CacheKeyItem>>,
    #[schemars(description = "Storage statistics (when view=stats)")]
    pub storage: Option<CacheStorageStats>,
    #[schemars(description = "Performance metrics (when view=stats)")]
    pub metrics: Option<CacheMetricsStats>,
    #[schemars(description = "Total count of items")]
    pub total: Option<usize>,
    #[schemars(description = "ISO 8601 timestamp when data was generated")]
    pub generated_at: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Cache reset operation response")]
pub struct CacheResetData {
    #[schemars(description = "Whether the reset operation was successful")]
    pub success: bool,
    #[schemars(description = "Optional message about the operation")]
    pub message: Option<String>,
}

// ==================== Data Models ====================

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "SQLite catalog and node-local memory cache statistics")]
pub struct CacheStorageStats {
    pub catalog: CacheCatalogStats,
    pub memory: CacheMemoryStats,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CacheCatalogStats {
    pub snapshots: u64,
    pub ready_snapshots: u64,
    pub invalidated_snapshots: u64,
    pub unavailable_snapshots: u64,
    pub records: u64,
    pub tools: u64,
    pub resources: u64,
    pub prompts: u64,
    pub resource_templates: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CacheMemoryStats {
    pub raw_snapshot_entries: usize,
    pub projection_entries: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Cache performance metrics")]
pub struct CacheMetricsStats {
    #[serde(rename = "totalQueries")]
    #[schemars(description = "Total number of cache queries")]
    pub total_queries: u64,
    #[serde(rename = "cacheHits")]
    #[schemars(description = "Number of cache hits")]
    pub cache_hits: u64,
    #[serde(rename = "cacheMisses")]
    #[schemars(description = "Number of cache misses")]
    pub cache_misses: u64,
    #[serde(rename = "hitRatio")]
    #[schemars(description = "Cache hit ratio (0.0 to 1.0)")]
    pub hit_ratio: f64,
    #[serde(rename = "rawSnapshotHits")]
    pub raw_snapshot_hits: u64,
    #[serde(rename = "rawSnapshotMisses")]
    pub raw_snapshot_misses: u64,
    #[serde(rename = "projectionHits")]
    pub projection_hits: u64,
    #[serde(rename = "projectionMisses")]
    pub projection_misses: u64,
    #[serde(rename = "singleFlightWaits")]
    pub single_flight_waits: u64,
    pub evictions: u64,
    #[serde(rename = "cacheInvalidations")]
    #[schemars(description = "Number of cache invalidations")]
    pub cache_invalidations: u64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Individual cache key item")]
pub struct CacheKeyItem {
    #[schemars(description = "Derived cache class")]
    pub cache: String,
    #[serde(rename = "keyHash")]
    #[schemars(description = "Redacted SHA-256 key prefix")]
    pub key_hash: String,
    #[serde(rename = "approxValueSizeBytes")]
    #[schemars(description = "Approximate size of cached value in bytes")]
    pub approx_value_size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "ISO 8601 timestamp when this item was cached")]
    pub cached_at: Option<String>,
}

// ==========================================
// SPECIFIC API RESPONSE TYPES
// ==========================================

// Generate response structures using macro
api_resp!(CacheDetailsResp, CacheDetailsData, "Cache details API response");
api_resp!(CacheResetResp, CacheResetData, "Cache reset API response");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_cache_stats_expose_sqlite_and_memory_details() {
        let response = CacheDetailsData {
            keys: None,
            storage: Some(CacheStorageStats {
                catalog: CacheCatalogStats {
                    snapshots: 2,
                    ready_snapshots: 1,
                    invalidated_snapshots: 1,
                    unavailable_snapshots: 0,
                    records: 10,
                    tools: 10,
                    resources: 0,
                    prompts: 0,
                    resource_templates: 0,
                },
                memory: CacheMemoryStats {
                    raw_snapshot_entries: 2,
                    projection_entries: 4,
                },
            }),
            metrics: None,
            total: None,
            generated_at: None,
        };

        let value = serde_json::to_value(response).expect("serialize capability cache statistics");
        assert_eq!(value["storage"]["catalog"]["readySnapshots"], 1);
        assert_eq!(value["storage"]["memory"]["projectionEntries"], 4);
        let encoded = value.to_string();
        for legacy_field in ["db_path", "cache_size_bytes", "tables", "last_cleanup"] {
            assert!(!encoded.contains(legacy_field));
        }
    }

    #[test]
    fn capability_cache_key_diagnostics_are_redacted() {
        let key = CacheKeyItem {
            cache: "raw_snapshot".to_string(),
            key_hash: "0123456789abcdef".to_string(),
            approx_value_size_bytes: 512,
            cached_at: Some("2026-07-20T00:00:00Z".to_string()),
        };

        let value = serde_json::to_value(key).expect("serialize redacted cache key");
        assert_eq!(value["keyHash"], "0123456789abcdef");
        assert!(value.get("key").is_none());
        assert!(value.get("serverId").is_none());
    }
}
