use anyhow::Result;
use futures::future::BoxFuture;
use rmcp::{
    model::ErrorCode,
    service::{Peer, RoleClient, ServiceError},
    transport::streamable_http_client::StreamableHttpError,
};
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CapabilityFetchFailure {
    Timeout { timeout_ms: u128 },
    TransportClosed,
    Unsupported { message: String },
    Authentication { message: String },
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
        CapabilityFetchFailure::Timeout { timeout_ms } => format!("request timed out after {timeout_ms} ms"),
        CapabilityFetchFailure::TransportClosed => "transport closed".to_string(),
        CapabilityFetchFailure::Unsupported { message }
        | CapabilityFetchFailure::Authentication { message }
        | CapabilityFetchFailure::Other { message } => message,
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

fn classify_service_error(error: &ServiceError) -> CapabilityFetchFailure {
    match error {
        ServiceError::McpError(error) if error.code == ErrorCode::METHOD_NOT_FOUND => {
            CapabilityFetchFailure::Unsupported {
                message: error.to_string(),
            }
        }
        ServiceError::TransportClosed => CapabilityFetchFailure::TransportClosed,
        ServiceError::Timeout { timeout } => CapabilityFetchFailure::Timeout {
            timeout_ms: timeout.as_millis(),
        },
        ServiceError::TransportSend(error)
            if error
                .error
                .downcast_ref::<StreamableHttpError<reqwest::Error>>()
                .is_some_and(|error| {
                    matches!(
                        error,
                        StreamableHttpError::AuthRequired(_) | StreamableHttpError::InsufficientScope(_)
                    )
                }) =>
        {
            CapabilityFetchFailure::Authentication {
                message: error.to_string(),
            }
        }
        ServiceError::McpError(_)
        | ServiceError::TransportSend(_)
        | ServiceError::UnexpectedResponse
        | ServiceError::Cancelled { .. } => CapabilityFetchFailure::Other {
            message: error.to_string(),
        },
        _ => CapabilityFetchFailure::Other {
            message: error.to_string(),
        },
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
    _is_unsupported: fn(&str) -> bool,
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
                failure = Some(CapabilityFetchFailure::Timeout {
                    timeout_ms: timeout.as_millis(),
                });
                break;
            }
            Ok(Err(e)) => {
                let classified = e
                    .downcast_ref::<ServiceError>()
                    .map(classify_service_error)
                    .unwrap_or_else(|| CapabilityFetchFailure::Other { message: e.to_string() });
                if matches!(classified, CapabilityFetchFailure::Unsupported { .. }) {
                    tracing::debug!(
                        "Capability not supported on '{}' ({}) instance {}: {}",
                        server_name,
                        server_id,
                        instance_id,
                        e
                    );
                } else {
                    tracing::warn!(
                        "Failed fetching capability page from '{}' ({}) instance {}: {}",
                        server_name,
                        server_id,
                        instance_id,
                        e
                    );
                }
                failure = Some(classified);
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
    use std::{any::TypeId, time::Duration};

    use rmcp::{
        ErrorData,
        model::ErrorCode,
        service::ServiceError,
        transport::{
            DynamicTransportError,
            streamable_http_client::{AuthRequiredError, InsufficientScopeError, StreamableHttpError},
        },
    };

    use super::{
        CapabilityFetchFailure, CapabilityFetchOutcome, classify_service_error, require_complete_capability_fetch,
    };

    #[test]
    fn incomplete_paginated_inventory_never_returns_partial_items() {
        let error = require_complete_capability_fetch(
            "prompts/list",
            "server-1",
            "docs",
            "instance-1",
            CapabilityFetchOutcome {
                items: vec!["first-page"],
                failure: Some(CapabilityFetchFailure::Timeout { timeout_ms: 1_000 }),
            },
        )
        .expect_err("partial inventory must fail closed");

        assert!(error.to_string().contains("prompts/list"));
        assert!(error.to_string().contains("server-1"));
    }

    #[test]
    fn rmcp_service_errors_are_classified_without_message_matching() {
        assert_eq!(
            classify_service_error(&ServiceError::TransportClosed),
            CapabilityFetchFailure::TransportClosed
        );
        assert_eq!(
            classify_service_error(&ServiceError::Timeout {
                timeout: Duration::from_millis(250),
            }),
            CapabilityFetchFailure::Timeout { timeout_ms: 250 }
        );
        assert!(matches!(
            classify_service_error(&ServiceError::McpError(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                "opaque message",
                None,
            ))),
            CapabilityFetchFailure::Unsupported { .. }
        ));
        assert!(matches!(
            classify_service_error(&ServiceError::McpError(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "gone 410",
                None,
            ))),
            CapabilityFetchFailure::Other { .. }
        ));
    }

    #[test]
    fn streamable_http_auth_failures_remain_typed_without_message_matching() {
        for error in [
            StreamableHttpError::<reqwest::Error>::AuthRequired(AuthRequiredError::new(
                "Bearer resource_metadata=\"https://example.com\"".to_string(),
            )),
            StreamableHttpError::<reqwest::Error>::InsufficientScope(InsufficientScopeError::new(
                "Bearer error=\"insufficient_scope\"".to_string(),
                Some("tools.read".to_string()),
            )),
        ] {
            let service_error = ServiceError::TransportSend(DynamicTransportError::from_parts(
                "streamable-http-client",
                TypeId::of::<()>(),
                Box::new(error),
            ));

            assert!(matches!(
                classify_service_error(&service_error),
                CapabilityFetchFailure::Authentication { .. }
            ));
            assert_eq!(
                crate::core::capability::runtime::RuntimeFailureKind::Authentication.retry_disposition(),
                crate::core::capability::connection_provider::DiscoveryRetryDisposition::DoNotRetry
            );
        }
    }
}
