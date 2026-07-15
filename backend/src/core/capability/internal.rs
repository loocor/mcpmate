use anyhow::Result;
use futures::future::BoxFuture;
use rmcp::service::{Peer, RoleClient};
use std::time::Duration;

/// Determine concurrency limit based on OS CPU cores
pub fn concurrency_limit() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

/// Common predicate to detect "not supported"/"method not found" errors
pub fn is_method_not_supported(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("method not found") || m.contains("not supported")
}

#[derive(Debug, Clone)]
pub enum CapabilityFetchFailure {
    Timeout,
    Gone { message: String },
    Other { message: String },
}

#[derive(Debug, Clone)]
pub struct CapabilityFetchOutcome<T> {
    pub items: Vec<T>,
    pub failure: Option<CapabilityFetchFailure>,
}

pub fn require_complete_capability_fetch<T>(
    operation: &str,
    server_id: &str,
    server_name: &str,
    instance_id: &str,
    outcome: CapabilityFetchOutcome<T>,
) -> Result<Vec<T>> {
    let Some(failure) = outcome.failure else {
        return Ok(outcome.items);
    };

    let detail = match failure {
        CapabilityFetchFailure::Timeout => "request timed out".to_string(),
        CapabilityFetchFailure::Gone { message } | CapabilityFetchFailure::Other { message } => message,
    };
    Err(anyhow::anyhow!(
        "Failed to complete '{}' for server '{}' ({}) instance '{}': {}",
        operation,
        server_name,
        server_id,
        instance_id,
        detail
    ))
}

fn looks_like_gone(msg_lower: &str) -> bool {
    msg_lower.contains("status: 404")
        || msg_lower.contains("status: 410")
        || msg_lower.contains("410")
        || msg_lower.contains("404")
        || msg_lower.contains("gone")
}

/// Parse capability declaration strings (e.g. "tools,prompts=false") to determine
/// whether a specific capability token is enabled. Defaults to `true` when the
/// declaration string is absent, matching legacy behaviour.
pub fn capability_declared(
    capabilities: Option<&str>,
    token: &str,
) -> bool {
    match capabilities {
        None => true,
        Some(caps) => {
            let mut saw_any = false;
            for part in caps.split(',') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                saw_any = true;
                let part_lower = part.to_ascii_lowercase();
                if let Some((key, value)) = part_lower.split_once('=') {
                    if key == token {
                        return value != "false";
                    }
                } else if part_lower == token {
                    return true;
                }
            }
            !saw_any
        }
    }
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
) -> CapabilityFetchOutcome<TMap>
where
    FFetch: Fn(Peer<RoleClient>, Option<String>) -> BoxFuture<'static, Result<(Vec<TItem>, Option<String>)>>,
    FMap: FnMut(TItem, &str, &str, &str) -> TMap,
{
    let mut results: Vec<TMap> = Vec::new();
    let mut cursor: Option<String> = None;
    let mut failure: Option<CapabilityFetchFailure> = None;

    loop {
        match tokio::time::timeout(timeout, fetch_page(peer.clone(), cursor.clone())).await {
            Err(_) => {
                tracing::warn!(
                    "Timeout fetching capability page from '{}' ({}) instance {}",
                    server_name,
                    server_id,
                    instance_id
                );
                failure = Some(CapabilityFetchFailure::Timeout);
                break;
            }
            Ok(Err(e)) => {
                let msg = format!("{}", e);
                if is_unsupported(&msg) {
                    tracing::debug!(
                        "Capability not supported on '{}' ({}) instance {}: {}",
                        server_name,
                        server_id,
                        instance_id,
                        msg
                    );
                    failure = Some(CapabilityFetchFailure::Other { message: msg });
                } else {
                    tracing::warn!(
                        "Failed fetching capability page from '{}' ({}) instance {}: {}",
                        server_name,
                        server_id,
                        instance_id,
                        msg
                    );
                    let msg_lower = msg.to_ascii_lowercase();
                    if looks_like_gone(&msg_lower) {
                        failure = Some(CapabilityFetchFailure::Gone { message: msg });
                    } else {
                        failure = Some(CapabilityFetchFailure::Other { message: msg });
                    }
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

    CapabilityFetchOutcome {
        items: results,
        failure,
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityFetchFailure, CapabilityFetchOutcome, require_complete_capability_fetch};

    #[test]
    fn incomplete_paginated_inventory_never_returns_partial_items() {
        let error = require_complete_capability_fetch(
            "prompts/list",
            "server-1",
            "docs",
            "instance-1",
            CapabilityFetchOutcome {
                items: vec!["first-page"],
                failure: Some(CapabilityFetchFailure::Timeout),
            },
        )
        .expect_err("partial inventory must fail closed");

        assert!(error.to_string().contains("prompts/list"));
        assert!(error.to_string().contains("server-1"));
    }
}
