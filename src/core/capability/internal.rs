use anyhow::Result;
use futures::future::BoxFuture;
use rmcp::service::{Peer, RoleClient};
use std::time::Duration;

/// Determine concurrency limit based on OS CPU cores
pub fn concurrency_limit() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Common predicate to detect "not supported"/"method not found" errors
pub fn is_method_not_supported(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("method not found") || m.contains("not supported")
}

/// Collect capability items from a single instance peer with pagination, timeout and logging
///
/// - `peer`: upstream peer to call
/// - `timeout`: per-page fetch timeout
/// - `fetch_page`: closure to fetch a page -> (items, next_cursor)
/// - `map_item`: closure to map a raw item into target mapping/value
/// - `server_id`, `server_name`, `instance_id`: identity for logging/mapping
/// - `is_unsupported`: predicate to classify unsupported capability errors
pub async fn collect_capability_from_instance_peer<TItem, TMap, FFetch, FMap>(
    peer: Peer<RoleClient>,
    timeout: Duration,
    fetch_page: FFetch,
    mut map_item: FMap,
    server_id: &str,
    server_name: &str,
    instance_id: &str,
    is_unsupported: fn(&str) -> bool,
) -> Vec<TMap>
where
    FFetch: Fn(Peer<RoleClient>, Option<String>) -> BoxFuture<'static, Result<(Vec<TItem>, Option<String>)>>,
    FMap: FnMut(TItem, &str, &str, &str) -> TMap,
{
    let mut results: Vec<TMap> = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        match tokio::time::timeout(timeout, fetch_page(peer.clone(), cursor.clone())).await {
            Err(_) => {
                tracing::warn!(
                    "Timeout fetching capability page from '{}' ({}) instance {}",
                    server_name, server_id, instance_id
                );
                break;
            }
            Ok(Err(e)) => {
                let msg = format!("{}", e);
                if is_unsupported(&msg) {
                    tracing::debug!(
                        "Capability not supported on '{}' ({}) instance {}: {}",
                        server_name, server_id, instance_id, msg
                    );
                } else {
                    tracing::warn!(
                        "Failed fetching capability page from '{}' ({}) instance {}: {}",
                        server_name, server_id, instance_id, msg
                    );
                }
                break;
            }
            Ok(Ok((items, next))) => {
                for it in items {
                    results.push(map_item(it, server_name, server_id, instance_id));
                }
                cursor = next;
                if cursor.is_none() {
                    break;
                }
            }
        }
    }

    results
}
