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
#[schemars(description = "Storage statistics for cache database")]
pub struct CacheStorageStats {
    #[schemars(description = "Path to the cache database file")]
    pub db_path: String,
    #[schemars(description = "Total cache size in bytes")]
    pub cache_size_bytes: u64,
    #[schemars(description = "Count of items in each table")]
    pub tables: CacheTablesCount,
    #[schemars(description = "ISO 8601 timestamp of last cleanup")]
    pub last_cleanup: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Count of items in each cache table")]
pub struct CacheTablesCount {
    #[schemars(description = "Number of cached servers")]
    pub servers: u64,
    #[schemars(description = "Number of cached tools")]
    pub tools: u64,
    #[schemars(description = "Number of cached resources")]
    pub resources: u64,
    #[schemars(description = "Number of cached prompts")]
    pub prompts: u64,
    #[serde(rename = "resourceTemplates")]
    #[schemars(description = "Number of cached resource templates")]
    pub resource_templates: u64,
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
    #[serde(rename = "readOperations")]
    #[schemars(description = "Number of read operations")]
    pub read_operations: u64,
    #[serde(rename = "writeOperations")]
    #[schemars(description = "Number of write operations")]
    pub write_operations: u64,
    #[serde(rename = "cacheInvalidations")]
    #[schemars(description = "Number of cache invalidations")]
    pub cache_invalidations: u64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(description = "Individual cache key item")]
pub struct CacheKeyItem {
    #[schemars(description = "Cache key identifier")]
    pub key: String,
    #[serde(rename = "serverId")]
    #[schemars(description = "ID of the server this key belongs to")]
    pub server_id: String,
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
