// MCP Proxy API handlers module
// Contains handler functions for API endpoints

#[cfg(feature = "ai")]
pub mod ai;
pub mod audit;
pub mod client;
pub mod common;
pub mod inspector;
pub mod profile;
pub mod registry;
pub mod runtime;
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
    /// Conflict error
    Conflict(String),
    /// Forbidden error
    Forbidden(String),
    /// Timeout error
    Timeout(String),
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
            ApiError::Conflict(msg) => write!(f, "Conflict: {msg}"),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {msg}"),
            ApiError::Timeout(msg) => write!(f, "Timeout: {msg}"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::Timeout(msg) => (StatusCode::REQUEST_TIMEOUT, msg),
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
