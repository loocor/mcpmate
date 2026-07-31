use crate::core::capability::read_service::CapabilityReadError;

/// Maps a typed capability read failure to the REST-facing `ApiError`, so every read
/// surface (tools/prompts/resources/templates/detail) reports the same status code and
/// keeps the underlying reason in the response body instead of collapsing to a bare 500.
pub(crate) fn map_capability_read_error(error: &CapabilityReadError) -> crate::api::handlers::ApiError {
    use crate::api::handlers::ApiError;

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
