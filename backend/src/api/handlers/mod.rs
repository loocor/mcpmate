// MCP Proxy API handlers module
// Contains handler functions for API endpoints

pub mod audit;
pub mod client;
pub mod common;
pub mod inspector;
pub mod llm;
pub mod onboarding;
pub mod profile;
pub mod runtime;
pub mod secrets;
pub mod secrets_password;
pub mod server;
pub mod system;
use std::fmt;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// API error type
#[derive(Debug)]
pub enum ApiError {
    /// Not found error
    NotFound(String),
    /// Bad request error
    BadRequest(String),
    /// Internal server error
    InternalError(String),
    /// Service unavailable error
    ServiceUnavailable(String),
    /// Conflict error
    Conflict(String),
    /// Forbidden error
    Forbidden(String),
    /// Timeout error
    Timeout(String),
    /// Upstream gateway timeout error
    GatewayTimeout(String),
    /// Upstream authentication error
    Unauthorized(String),
    /// Upstream/bad gateway error
    BadGateway(String),
}

impl fmt::Display for ApiError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ApiError::NotFound(msg) => write!(f, "Not found: {msg}"),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {msg}"),
            ApiError::InternalError(msg) => write!(f, "Internal error: {msg}"),
            ApiError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {msg}"),
            ApiError::Conflict(msg) => write!(f, "Conflict: {msg}"),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {msg}"),
            ApiError::Timeout(msg) => write!(f, "Timeout: {msg}"),
            ApiError::GatewayTimeout(msg) => write!(f, "Gateway timeout: {msg}"),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            ApiError::BadGateway(msg) => write!(f, "Bad gateway: {msg}"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::Timeout(msg) => (StatusCode::REQUEST_TIMEOUT, msg),
            ApiError::GatewayTimeout(msg) => (StatusCode::GATEWAY_TIMEOUT, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::BadGateway(msg) => (StatusCode::BAD_GATEWAY, msg),
        };

        let body = Json(json!({
            "error": {
                "message": message,
                "status": status.as_u16()
            }
        }));

        (status, body).into_response()
    }
}

/// Convert anyhow errors to API errors
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

/// Convert a bare status code (from shared helpers that predate typed errors) into an
/// ApiError, preserving a best-effort reason so the response body is never empty.
impl From<StatusCode> for ApiError {
    fn from(status: StatusCode) -> Self {
        let message = status.canonical_reason().unwrap_or("Request failed").to_string();
        match status {
            StatusCode::NOT_FOUND => ApiError::NotFound(message),
            StatusCode::BAD_REQUEST => ApiError::BadRequest(message),
            StatusCode::CONFLICT => ApiError::Conflict(message),
            StatusCode::FORBIDDEN => ApiError::Forbidden(message),
            StatusCode::UNAUTHORIZED => ApiError::Unauthorized(message),
            StatusCode::SERVICE_UNAVAILABLE => ApiError::ServiceUnavailable(message),
            StatusCode::REQUEST_TIMEOUT => ApiError::Timeout(message),
            StatusCode::GATEWAY_TIMEOUT => ApiError::GatewayTimeout(message),
            StatusCode::BAD_GATEWAY => ApiError::BadGateway(message),
            _ => ApiError::InternalError(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_unavailable_errors_return_503() {
        let response = ApiError::ServiceUnavailable("secure store unavailable".to_string()).into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn ordinary_timeout_errors_remain_request_timeout() {
        let response = ApiError::Timeout("operation timed out".to_string()).into_response();

        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    }

    #[test]
    fn unauthorized_errors_return_401() {
        let response = ApiError::Unauthorized("upstream auth failed".to_string()).into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn bad_gateway_errors_return_502() {
        let response = ApiError::BadGateway("upstream discovery failed".to_string()).into_response();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn status_code_conversion_preserves_a_reason_in_the_body() {
        let error: ApiError = StatusCode::NOT_FOUND.into();
        match error {
            ApiError::NotFound(msg) => assert_eq!(msg, "Not Found"),
            other => panic!("Expected NotFound, got {other:?}"),
        }
    }
}
