// Cache capabilities handlers (Redb capabilities cache)
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::api::{handlers::ApiError, routes::AppState};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct DetailsQuery {
    #[serde(default = "default_view")]
    pub view: String, // "stats" | "keys"
    pub server_id: Option<String>,
    pub limit: Option<usize>,
    #[allow(dead_code)]
    pub sort_by: Option<String>, // size|age|key (reserved)
    #[allow(dead_code)]
    pub order: Option<String>, // asc|desc (reserved)
}

fn default_view() -> String {
    "stats".to_string()
}

#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub db_path: String,
    pub cache_size_bytes: u64,
    pub tables: TablesCount,
    pub last_cleanup: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TablesCount {
    pub servers: u64,
    pub tools: u64,
    pub resources: u64,
    pub prompts: u64,
    #[serde(rename = "resourceTemplates")]
    pub resource_templates: u64,
}

#[derive(Debug, Serialize)]
pub struct MetricsStats {
    #[serde(rename = "totalQueries")]
    pub total_queries: u64,
    #[serde(rename = "cacheHits")]
    pub cache_hits: u64,
    #[serde(rename = "cacheMisses")]
    pub cache_misses: u64,
    #[serde(rename = "hitRatio")]
    pub hit_ratio: f64,
    #[serde(rename = "readOperations")]
    pub read_operations: u64,
    #[serde(rename = "writeOperations")]
    pub write_operations: u64,
    #[serde(rename = "cacheInvalidations")]
    pub cache_invalidations: u64,
}

#[derive(Debug, Serialize)]
pub struct KeyItem {
    pub key: String,
    #[serde(rename = "serverId")]
    pub server_id: String,
    #[serde(rename = "approxValueSizeBytes")]
    pub approx_value_size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_at: Option<String>,
}

pub async fn details(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DetailsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match query.view.as_str() {
        "keys" => build_keys_view(state, &query).await,
        _ => build_stats_view(state).await,
    }
}

async fn build_keys_view(
    state: Arc<AppState>,
    query: &DetailsQuery,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let entries = state
        .redb_cache
        .list_server_entries(query.server_id.as_deref(), limit)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?;

    let keys: Vec<KeyItem> = entries
        .into_iter()
        .map(|e| KeyItem {
            key: e.key,
            server_id: e.server_id,
            approx_value_size_bytes: e.approx_value_size_bytes,
            cached_at: e.cached_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(serde_json::json!({
        "keys": keys,
        "total": keys.len()
    })))
}

async fn build_stats_view(state: Arc<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let stats = state.redb_cache.get_stats().await;
    let live = state.redb_cache.get_metrics().await;
    let db_path = state.redb_cache.database_path();
    let last_cleanup = state.redb_cache.get_last_cleanup_time();
    let storage = StorageStats {
        db_path: db_path.to_string_lossy().to_string(),
        cache_size_bytes: stats.cache_size_bytes,
        tables: TablesCount {
            servers: stats.total_servers,
            tools: stats.total_tools,
            resources: stats.total_resources,
            prompts: stats.total_prompts,
            resource_templates: stats.total_resource_templates,
        },
        last_cleanup,
    };

    let hit_ratio = live.hit_ratio();
    let hit_ratio = (hit_ratio * 10_000.0).round() / 10_000.0;

    let metrics = MetricsStats {
        total_queries: live.total_queries,
        cache_hits: live.cache_hits,
        cache_misses: live.cache_misses,
        hit_ratio,
        read_operations: live.read_operations,
        write_operations: live.write_operations,
        cache_invalidations: live.cache_invalidations,
    };

    Ok(Json(serde_json::json!({
        "storage": storage,
        "metrics": metrics,
        "generatedAt": stats.last_updated.to_rfc3339(),
    })))
}

pub async fn reset(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .redb_cache
        .clear_all()
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?;
    Ok(Json(serde_json::json!({"success": true})))
}
