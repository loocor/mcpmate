// Tool mapping cache implementation for the HTTP proxy server

use std::collections::HashMap;
use std::time::Duration;

use crate::core::tool;
use crate::http::proxy::core::HttpProxyServer;

/// Cache expiration time (2 minutes)
const CACHE_EXPIRATION: Duration = Duration::from_secs(120);

/// Get the cached tool name mapping, or build a new one if the cache is expired or empty
///
/// This optimized version uses a more efficient caching strategy:
/// 1. Uses a longer cache expiration time (2 minutes instead of 30 seconds)
/// 2. Prioritizes connection state hash changes over time-based expiration
/// 3. Provides detailed logging about cache update reasons
/// 4. Reduces lock contention by acquiring locks only when necessary
pub async fn get_tool_name_mapping(
    server: &HttpProxyServer,
) -> HashMap<String, crate::core::tool::ToolNameMapping> {
    // First, check if cache exists without calculating hash (fast path)
    let cache_exists = {
        let cache = server.tool_name_mapping_cache.lock().await;
        cache.is_some()
    };

    // If cache doesn't exist, we definitely need to update
    if !cache_exists {
        return rebuild_tool_mapping_cache(server, "Cache is empty (first use)").await;
    }

    // Calculate current connection state hash
    let current_hash = {
        let pool = server.connection_pool.lock().await;
        pool.calculate_connection_state_hash()
    };

    // Check if connection state has changed
    let hash_changed = {
        let last_hash = server.last_connection_state_hash.lock().await;
        *last_hash != current_hash
    };

    // If hash changed, update cache immediately
    if hash_changed {
        return rebuild_tool_mapping_cache(server, "Connection state changed").await;
    }

    // Check if cache has expired (only if hash hasn't changed)
    let cache_expired = {
        let last_update = server.last_tool_mapping_update.lock().await;
        last_update.elapsed() > CACHE_EXPIRATION
    };

    // If cache has expired, update it
    if cache_expired {
        return rebuild_tool_mapping_cache(server, "Cache expired").await;
    }

    // Use the cached mapping (fast path)
    let cache = server.tool_name_mapping_cache.lock().await;
    cache.as_ref().unwrap().clone()
}

/// Helper method to rebuild the tool mapping cache
pub async fn rebuild_tool_mapping_cache(
    server: &HttpProxyServer,
    reason: &str,
) -> HashMap<String, crate::core::tool::ToolNameMapping> {
    // Calculate current hash
    let current_hash = {
        let pool = server.connection_pool.lock().await;
        pool.calculate_connection_state_hash()
    };

    // Build a new tool name mapping
    let start_time = std::time::Instant::now();
    let new_mapping = tool::build_tool_name_mapping(&server.connection_pool).await;
    let build_time = start_time.elapsed();

    // Update the cache
    {
        let mut cache = server.tool_name_mapping_cache.lock().await;
        *cache = Some(new_mapping.clone());

        // Update the last update time
        let mut last_update = server.last_tool_mapping_update.lock().await;
        *last_update = std::time::Instant::now();

        // Update the last connection state hash
        let mut last_hash = server.last_connection_state_hash.lock().await;
        *last_hash = current_hash;
    }

    tracing::info!(
        "Updated tool name mapping cache with {} entries (reason: {}, build time: {:?})",
        new_mapping.len(),
        reason,
        build_time
    );

    new_mapping
}
