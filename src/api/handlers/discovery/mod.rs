// Discovery API handlers module
// Provides handlers for MCP server capability discovery endpoints

pub mod aggregation;
pub mod capabilities;
pub mod prompts;
pub mod resources;
pub mod tools;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    config::database::Database,
    discovery::{DiscoveryParams, DiscoveryService, RefreshStrategy, ResponseFormat},
};

/// Common response wrapper for discovery endpoints
#[derive(Debug, Serialize)]
pub struct DiscoveryResponse<T> {
    /// Response data
    pub data: T,
    /// Response metadata
    pub meta: Option<ResponseMetadata>,
}

/// Response metadata for discovery endpoints
#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    /// Server ID
    pub server_id: String,
    /// Refresh strategy used
    pub refresh_strategy: RefreshStrategy,
    /// Response format used
    pub format: ResponseFormat,
    /// Cache hit indicator
    pub cache_hit: Option<bool>,
    /// Response timestamp
    pub timestamp: std::time::SystemTime,
}

/// Query parameters for discovery endpoints
#[derive(Debug, Deserialize)]
pub struct DiscoveryQuery {
    /// Refresh strategy
    pub refresh: Option<String>,
    /// Response format
    pub format: Option<String>,
    /// Include metadata in response
    pub include_meta: Option<bool>,
}

impl DiscoveryQuery {
    /// Convert to DiscoveryParams
    pub fn to_params(&self) -> Result<DiscoveryParams, ApiError> {
        let refresh = if let Some(ref refresh_str) = self.refresh {
            Some(match refresh_str.as_str() {
                "cache_first" => RefreshStrategy::CacheFirst,
                "refresh_if_stale" => RefreshStrategy::RefreshIfStale,
                "force" => RefreshStrategy::Force,
                _ => {
                    return Err(ApiError::BadRequest(format!(
                        "Invalid refresh strategy: {}",
                        refresh_str
                    )));
                }
            })
        } else {
            None
        };

        let format = if let Some(ref format_str) = self.format {
            Some(match format_str.as_str() {
                "json" => ResponseFormat::Json,
                "compact" => ResponseFormat::Compact,
                "detailed" => ResponseFormat::Detailed,
                _ => {
                    return Err(ApiError::BadRequest(format!(
                        "Invalid response format: {}",
                        format_str
                    )));
                }
            })
        } else {
            None
        };

        Ok(DiscoveryParams {
            refresh,
            format,
            include_meta: self.include_meta,
        })
    }
}

/// Get discovery service from app state
pub async fn get_discovery_service(state: &AppState) -> Result<Arc<DiscoveryService>, ApiError> {
    state
        .discovery_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Discovery service not available".to_string()))
        .cloned()
}

/// Get database from app state
pub async fn get_database(state: &AppState) -> Result<Arc<Database>, ApiError> {
    state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))
        .cloned()
}

/// Create discovery response with optional metadata
pub fn create_response<T>(
    data: T,
    server_id: &str,
    params: &DiscoveryParams,
    cache_hit: Option<bool>,
) -> DiscoveryResponse<T> {
    let meta = if params.include_meta.unwrap_or(false) {
        Some(ResponseMetadata {
            server_id: server_id.to_string(),
            refresh_strategy: params.refresh.unwrap_or_default(),
            format: params.format.unwrap_or_default(),
            cache_hit,
            timestamp: std::time::SystemTime::now(),
        })
    } else {
        None
    };

    DiscoveryResponse { data, meta }
}

/// Handle discovery errors and convert to API errors
pub fn handle_discovery_error(error: crate::discovery::DiscoveryError) -> ApiError {
    use crate::discovery::DiscoveryError;

    match error {
        DiscoveryError::ServerNotFound(server_id) => {
            ApiError::NotFound(format!("Server '{}' not found", server_id))
        }
        DiscoveryError::ConnectionFailed(msg) => {
            ApiError::InternalError(format!("Connection failed: {}", msg))
        }
        DiscoveryError::CacheError(msg) => ApiError::InternalError(format!("Cache error: {}", msg)),
        DiscoveryError::SerializationError(msg) => {
            ApiError::InternalError(format!("Serialization error: {}", msg))
        }
        DiscoveryError::PermissionDenied(msg) => {
            ApiError::Forbidden(format!("Permission denied: {}", msg))
        }
        DiscoveryError::InvalidConfig(msg) => {
            ApiError::BadRequest(format!("Invalid configuration: {}", msg))
        }
        DiscoveryError::Timeout(msg) => ApiError::InternalError(format!("Timeout: {}", msg)),
        DiscoveryError::IoError(e) => ApiError::InternalError(format!("IO error: {}", e)),
        DiscoveryError::JsonError(e) => ApiError::InternalError(format!("JSON error: {}", e)),
        DiscoveryError::DatabaseError(msg) => {
            ApiError::InternalError(format!("Database error: {}", msg))
        }
    }
}

/// Validate server ID parameter
pub fn validate_server_id(server_id: &str) -> Result<(), ApiError> {
    if server_id.is_empty() {
        return Err(ApiError::BadRequest(
            "Server ID cannot be empty".to_string(),
        ));
    }

    if server_id.len() > 255 {
        return Err(ApiError::BadRequest("Server ID too long".to_string()));
    }

    // Basic validation - alphanumeric, hyphens, underscores
    if !server_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ApiError::BadRequest(
            "Server ID contains invalid characters".to_string(),
        ));
    }

    Ok(())
}

/// Validate tool ID parameter
pub fn validate_tool_id(tool_id: &str) -> Result<(), ApiError> {
    if tool_id.is_empty() {
        return Err(ApiError::BadRequest("Tool ID cannot be empty".to_string()));
    }

    if tool_id.len() > 255 {
        return Err(ApiError::BadRequest("Tool ID too long".to_string()));
    }

    Ok(())
}
