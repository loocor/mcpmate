use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use crate::{
    config::database::Database,
    core::{
        capability::{
            read_service::CapabilityReadService,
            runtime::{ListCtx, ListResult},
        },
        pool::UpstreamConnectionPool,
    },
};

/// Legacy sentinel used by ordinary API callers before request-unique discovery owners.
pub const CAPABILITY_VALIDATION_SESSION: &str = "capability-service";

#[derive(Debug, thiserror::Error)]
#[error("Capability server connect exceeded {timeout_ms} ms")]
pub(crate) struct CapabilityConnectionTimeout {
    timeout_ms: u128,
    #[source]
    source: crate::core::capability::read_service::CapabilityReadError,
}

pub(crate) fn connection_timeout_ms(error: &anyhow::Error) -> Option<u128> {
    error
        .downcast_ref::<CapabilityConnectionTimeout>()
        .map(|timeout| timeout.timeout_ms)
        .or_else(|| {
            error
                .downcast_ref::<crate::core::capability::read_service::CapabilityReadError>()
                .and_then(crate::core::capability::read_service::CapabilityReadError::connection_timeout_ms)
        })
}

pub(crate) fn operation_timeout_ms(error: &anyhow::Error) -> Option<u128> {
    error
        .downcast_ref::<crate::core::capability::read_service::CapabilityReadError>()
        .and_then(crate::core::capability::read_service::CapabilityReadError::operation_timeout_ms)
}

/// Maps a typed capability read failure to the REST-facing `ApiError`, so every read
/// surface (tools/prompts/resources/templates/detail) reports the same status code and
/// keeps the underlying reason in the response body instead of collapsing to a bare 500.
pub(crate) fn map_capability_read_error(
    error: &crate::core::capability::read_service::CapabilityReadError
) -> crate::api::handlers::ApiError {
    use crate::api::handlers::ApiError;
    use crate::core::capability::read_service::CapabilityReadError;

    if let Some(timeout_ms) = error.connection_timeout_ms() {
        return ApiError::GatewayTimeout(format!("capability discovery exceeded {timeout_ms} ms: {error}"));
    }
    if let Some(timeout_ms) = error.operation_timeout_ms() {
        return ApiError::Timeout(format!("capability operation exceeded {timeout_ms} ms: {error}"));
    }
    if let Some(reason) = error.authentication_reason() {
        return ApiError::Unauthorized(format!("capability owner authentication failed: {reason}"));
    }
    match error {
        CapabilityReadError::CatalogUntrusted { .. } | CapabilityReadError::CatalogOperation { .. } => {
            ApiError::ServiceUnavailable(error.to_string())
        }
        CapabilityReadError::CleanupFailed { .. } => ApiError::ServiceUnavailable(error.to_string()),
        CapabilityReadError::DiscoveryFailed { .. } => ApiError::BadGateway(error.to_string()),
        CapabilityReadError::ProjectionFailed { .. } => ApiError::InternalError(error.to_string()),
    }
}

/// Compatibility facade for callers that Task 3 will migrate to CapabilityReadService.
pub struct CapabilityService {
    inner: CapabilityReadService,
}

impl CapabilityService {
    pub fn new(
        pool: Arc<Mutex<UpstreamConnectionPool>>,
        database: Arc<Database>,
    ) -> Self {
        Self {
            inner: CapabilityReadService::from_runtime(database, pool),
        }
    }

    pub async fn list(
        &self,
        ctx: &ListCtx,
    ) -> Result<ListResult> {
        let normalized;
        let ctx = if ctx.validation_session.as_deref() == Some(CAPABILITY_VALIDATION_SESSION) {
            normalized = ListCtx {
                validation_session: None,
                ..ctx.clone()
            };
            &normalized
        } else {
            ctx
        };
        match self.inner.list(ctx).await {
            Ok(result) => Ok(result),
            Err(error) => match error.connection_timeout_ms() {
                Some(timeout_ms) => Err(CapabilityConnectionTimeout {
                    timeout_ms,
                    source: error,
                }
                .into()),
                None => Err(error.into()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CAPABILITY_VALIDATION_SESSION, connection_timeout_ms, operation_timeout_ms};
    use crate::core::capability::{
        CapabilityType,
        connection_provider::{CapabilityOwnerError, OwnerSource},
        read_service::{CapabilityAttemptError, CapabilityReadError, DiscoveryAttemptFailure},
        runtime::{RuntimeFailure, RuntimeFailureKind},
    };

    #[test]
    fn legacy_api_session_remains_distinct_from_inspector_sessions() {
        assert_ne!(CAPABILITY_VALIDATION_SESSION, "inspector-session");
    }

    #[test]
    fn typed_read_timeout_reaches_the_inspector_boundary() {
        let error = anyhow::Error::from(CapabilityReadError::DiscoveryFailed {
            server_id: "server-1".to_string(),
            server_name: "docs".to_string(),
            operation: "tools/list",
            kind: CapabilityType::Tools,
            catalog_error: None,
            existing: Some(DiscoveryAttemptFailure {
                instance_id: None,
                connection_generation: None,
                source: OwnerSource::Existing,
                error: CapabilityAttemptError::Owner(CapabilityOwnerError::Timeout { timeout_ms: 750 }),
            }),
            fresh: None,
        });

        assert_eq!(connection_timeout_ms(&error), Some(750));
        assert_eq!(operation_timeout_ms(&error), None);
    }

    #[test]
    fn runtime_timeout_stays_an_operation_timeout() {
        let error = anyhow::Error::from(CapabilityReadError::DiscoveryFailed {
            server_id: "server-1".to_string(),
            server_name: "docs".to_string(),
            operation: "tools/list",
            kind: CapabilityType::Tools,
            catalog_error: None,
            existing: Some(DiscoveryAttemptFailure {
                instance_id: Some("instance-1".to_string()),
                connection_generation: None,
                source: OwnerSource::Existing,
                error: CapabilityAttemptError::Runtime(RuntimeFailure {
                    kind: RuntimeFailureKind::Timeout,
                    message: Some("request timeout".to_string()),
                    timeout_ms: Some(500),
                }),
            }),
            fresh: None,
        });

        assert_eq!(connection_timeout_ms(&error), None);
        assert_eq!(operation_timeout_ms(&error), Some(500));
    }
}
