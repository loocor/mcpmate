// Common utilities for server API handlers
// Provides shared functions for server identification, validation, and response formatting

use sqlx::{Pool, Sqlite};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    config::{database::Database, server},
    discovery::{DiscoveryParams, DiscoveryService},
};

/// Server identification result
#[derive(Debug, Clone)]
pub struct ServerIdentification {
    /// Server ID (guaranteed to exist)
    pub server_id: String,
    /// Server name (guaranteed to exist)
    pub server_name: String,
}

/// Resolve server identifier (name or ID) to complete server information
///
/// This function provides intelligent resolution of server identifiers:
/// - Accepts both server_name and server_id as input
/// - Returns complete server identification information
/// - Handles edge cases and provides clear error messages
pub async fn resolve_server_identifier(
    pool: &Pool<Sqlite>,
    identifier: &str,
) -> Result<ServerIdentification, ApiError> {
    // Validate input
    if identifier.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Server identifier cannot be empty".to_string(),
        ));
    }

    // Try to find server by ID first (more efficient for ID-based lookups)
    if let Some(server) = server::get_server_by_id(pool, identifier)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {e}")))?
    {
        let server_id = server
            .id
            .ok_or_else(|| ApiError::InternalError("Server found but missing ID".to_string()))?;

        return Ok(ServerIdentification {
            server_id,
            server_name: server.name,
        });
    }

    // Try to find server by name
    if let Some(server) = server::get_server(pool, identifier)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {e}")))?
    {
        let server_id = server
            .id
            .ok_or_else(|| ApiError::InternalError("Server found but missing ID".to_string()))?;

        return Ok(ServerIdentification {
            server_id,
            server_name: server.name,
        });
    }

    // Server not found
    Err(ApiError::NotFound(format!(
        "Server '{}' not found. Please check the server name or ID.",
        identifier
    )))
}

/// Validate server access permissions
///
/// This function can be extended to include authorization checks,
/// rate limiting, or other access control mechanisms.
pub async fn validate_server_access(
    _pool: &Pool<Sqlite>,
    _server_id: &str,
) -> Result<(), ApiError> {
    // For now, all servers are accessible
    // TODO: Add authorization logic here if needed
    Ok(())
}

/// Get database from application state
///
/// Helper function to extract database connection from AppState
/// with proper error handling.
pub fn get_database_from_state(state: &Arc<AppState>) -> Result<Arc<Database>, ApiError> {
    state
        .http_proxy
        .as_ref()
        .and_then(|p| p.database.clone())
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))
}

/// Get discovery service from application state
///
/// Helper function to extract discovery service from AppState
/// with proper error handling.
pub async fn get_discovery_service(
    state: &Arc<AppState>
) -> Result<&Arc<DiscoveryService>, ApiError> {
    state
        .discovery_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Discovery service not available".to_string()))
}

/// Create standardized discovery response with metadata
///
/// This function provides consistent response formatting across
/// all discovery endpoints.
pub fn create_discovery_response<T>(
    data: T,
    params: &DiscoveryParams,
    cache_hit: Option<bool>,
    capabilities_metadata: Option<&crate::discovery::types::CapabilitiesMetadata>,
) -> crate::discovery::DiscoveryResponse<T> {
    use crate::discovery::{DiscoveryResponse, ResponseMetadata};

    let metadata = Some(ResponseMetadata {
        refresh_strategy: params.refresh.unwrap_or_default(),
        format: params.format.unwrap_or_default(),
        cache_hit,
        last_updated: capabilities_metadata
            .map(|m| m.last_updated)
            .unwrap_or_else(std::time::SystemTime::now),
        version: capabilities_metadata
            .map(|m| m.version.clone())
            .unwrap_or_else(|| "1.0".to_string()),
        ttl: capabilities_metadata
            .map(|m| m.ttl)
            .unwrap_or_else(|| std::time::Duration::from_secs(300)),
        protocol_version: capabilities_metadata.and_then(|m| m.protocol_version.clone()),
    });

    DiscoveryResponse {
        data,
        meta: metadata,
    }
}

/// Validate server ID format
///
/// Ensures server ID follows expected format patterns
pub fn validate_server_id(server_id: &str) -> Result<(), ApiError> {
    if server_id.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Server ID cannot be empty".to_string(),
        ));
    }

    // Allow both generated IDs (serv_*) and custom names
    // This is flexible to support different ID formats
    if server_id.len() > 255 {
        return Err(ApiError::BadRequest(
            "Server ID too long (max 255 characters)".to_string(),
        ));
    }

    Ok(())
}

/// Handle discovery service errors with appropriate HTTP status codes
///
/// Converts discovery service errors to appropriate API errors
pub fn handle_discovery_error(error: crate::discovery::DiscoveryError) -> ApiError {
    use crate::discovery::DiscoveryError;

    match error {
        DiscoveryError::ServerNotFound(msg) => ApiError::NotFound(msg),
        DiscoveryError::ConnectionFailed(msg) => ApiError::Timeout(msg),
        DiscoveryError::InvalidConfig(msg) => ApiError::BadRequest(msg),
        DiscoveryError::CacheError(msg) => ApiError::InternalError(format!("Cache error: {msg}")),
        DiscoveryError::Timeout(msg) => ApiError::Timeout(msg),
        DiscoveryError::SerializationError(msg) => {
            ApiError::InternalError(format!("Serialization error: {msg}"))
        }
        DiscoveryError::PermissionDenied(msg) => ApiError::Forbidden(msg),
        DiscoveryError::IoError(err) => ApiError::InternalError(format!("IO error: {err}")),
        DiscoveryError::JsonError(err) => ApiError::InternalError(format!("JSON error: {err}")),
        DiscoveryError::DatabaseError(msg) => {
            ApiError::InternalError(format!("Database error: {msg}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_server_id() {
        // Valid IDs
        assert!(validate_server_id("serv_123").is_ok());
        assert!(validate_server_id("my-server").is_ok());
        assert!(validate_server_id("context7").is_ok());

        // Invalid IDs
        assert!(validate_server_id("").is_err());
        assert!(validate_server_id("   ").is_err());
        assert!(validate_server_id(&"x".repeat(256)).is_err());
    }

    #[test]
    fn test_create_discovery_response() {
        let data = vec!["test"];
        let params = DiscoveryParams::default();
        let response = create_discovery_response(data, &params, Some(true), None);

        assert_eq!(response.data, vec!["test"]);
        assert!(response.meta.is_some());
        let meta = response.meta.unwrap();
        assert_eq!(meta.refresh_strategy, params.refresh.unwrap_or_default());
        assert_eq!(meta.format, params.format.unwrap_or_default());
        assert_eq!(meta.cache_hit, Some(true));
    }
}
