// Cache capabilities handlers (Redb capabilities cache)
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use crate::api::models::cache::{
    CacheDetailsData, CacheDetailsReq, CacheDetailsResp, CacheKeyItem, CacheMetricsStats, CacheResetData,
    CacheResetResp, CacheStorageStats, CacheTablesCount, CacheViewType,
};

use crate::api::routes::AppState;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 1000;

pub async fn details(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CacheDetailsReq>,
) -> Result<Json<CacheDetailsResp>, StatusCode> {
    let result = cache_details_core(&query, &state).await?;
    Ok(Json(result))
}

pub async fn reset(State(state): State<Arc<AppState>>) -> Result<Json<CacheResetResp>, StatusCode> {
    let result = cache_reset_core(&state).await?;
    Ok(Json(result))
}

// ==================== Core Business Functions ====================

async fn cache_details_core(
    query: &CacheDetailsReq,
    state: &Arc<AppState>,
) -> Result<CacheDetailsResp, StatusCode> {
    match query.view {
        CacheViewType::Keys => {
            let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

            let entries = state
                .redb_cache
                .list_server_entries(query.server_id.as_deref(), limit)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to list cache entries: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            let keys: Vec<CacheKeyItem> = entries
                .into_iter()
                .map(|e| CacheKeyItem {
                    key: e.key,
                    server_id: e.server_id,
                    approx_value_size_bytes: e.approx_value_size_bytes,
                    cached_at: e.cached_at.map(|t| t.to_rfc3339()),
                })
                .collect();

            let total = keys.len();
            let response = CacheDetailsData {
                keys: Some(keys),
                storage: None,
                metrics: None,
                total: Some(total),
                generated_at: None,
            };

            Ok(CacheDetailsResp::success(response))
        }
        CacheViewType::Stats => {
            let stats = state.redb_cache.get_stats().await;
            let live = state.redb_cache.get_metrics().await;
            let db_path = state.redb_cache.database_path();
            let last_cleanup = state.redb_cache.get_last_cleanup_time();

            let storage = CacheStorageStats {
                db_path: db_path.to_string_lossy().to_string(),
                cache_size_bytes: stats.cache_size_bytes,
                tables: CacheTablesCount {
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

            let metrics = CacheMetricsStats {
                total_queries: live.total_queries,
                cache_hits: live.cache_hits,
                cache_misses: live.cache_misses,
                hit_ratio,
                read_operations: live.read_operations,
                write_operations: live.write_operations,
                cache_invalidations: live.cache_invalidations,
            };

            let response = CacheDetailsData {
                keys: None,
                storage: Some(storage),
                metrics: Some(metrics),
                total: None,
                generated_at: Some(stats.last_updated.to_rfc3339()),
            };

            Ok(CacheDetailsResp::success(response))
        }
    }
}

async fn cache_reset_core(state: &Arc<AppState>) -> Result<CacheResetResp, StatusCode> {
    state.redb_cache.clear_all().await.map_err(|e| {
        tracing::error!("Failed to clear cache: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = CacheResetData {
        success: true,
        message: Some("Cache cleared successfully".to_string()),
    };

    Ok(CacheResetResp::success(response))
}
