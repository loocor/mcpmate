use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for runtime installation")]
pub struct RuntimeInstallReq {
    #[schemars(description = "Runtime type to install (uv or bun)")]
    pub runtime_type: String,
    #[schemars(description = "Specific version to install (optional)")]
    pub version: Option<String>,
    #[schemars(description = "Installation timeout in seconds")]
    pub timeout: Option<u64>,
    #[schemars(description = "Maximum retry attempts")]
    pub max_retries: Option<u32>,
    #[schemars(description = "Enable verbose output")]
    pub verbose: Option<bool>,
    #[schemars(description = "Enable interactive mode")]
    pub interactive: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for runtime installation")]
pub struct RuntimeInstallResp {
    #[schemars(description = "Whether installation was successful")]
    pub success: bool,
    #[schemars(description = "Installation result message")]
    pub message: String,
    #[schemars(description = "Runtime type that was installed")]
    pub runtime_type: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime status information")]
pub struct RuntimeStatus {
    #[schemars(description = "Runtime type (uv or bun)")]
    pub runtime_type: String,
    #[schemars(description = "Whether runtime is available")]
    pub available: bool,
    #[schemars(description = "Path to runtime executable")]
    pub path: Option<String>,
    #[schemars(description = "Runtime version string")]
    pub version: Option<String>,
    #[schemars(description = "Status message with details")]
    pub message: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for runtime status check")]
pub struct RuntimeStatusResp {
    #[schemars(description = "UV runtime status")]
    pub uv: RuntimeStatus,
    #[schemars(description = "Bun runtime status")]
    pub bun: RuntimeStatus,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Cache summary information")]
pub struct CacheSummaryInfo {
    #[schemars(description = "Total cache size in bytes")]
    pub total_size_bytes: u64,
    #[schemars(description = "ISO 8601 timestamp of last cleanup")]
    pub last_cleanup: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Cache item details")]
pub struct CacheItem {
    #[schemars(description = "Cache directory path")]
    pub path: String,
    #[schemars(description = "Cache size in bytes")]
    pub size_bytes: u64,
    #[schemars(description = "Number of cached packages")]
    pub package_count: u64,
    #[schemars(description = "ISO 8601 timestamp of last modification")]
    pub last_modified: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for runtime cache information")]
pub struct RuntimeCacheResp {
    #[schemars(description = "Cache summary across all runtimes")]
    pub summary: CacheSummaryInfo,
    #[schemars(description = "UV cache details")]
    pub uv: CacheItem,
    #[schemars(description = "Bun cache details")]
    pub bun: CacheItem,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for cache reset operation")]
pub struct RuntimeCacheResetReq {
    #[serde(default = "super::default_all")]
    #[schemars(description = "Cache type to reset: all, uv, or bun (default: all)")]
    pub cache_type: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for cache reset operation")]
pub struct RuntimeCacheResetResp {
    #[schemars(description = "Whether cache reset was successful")]
    pub success: bool,
}

// ==========================================
// SPECIFIC API RESPONSE TYPES
// ==========================================

use crate::api::models::clients::ApiError;

/// Response for runtime install operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime install API response")]
pub struct RuntimeInstallApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<RuntimeInstallResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for runtime status operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime status API response")]
pub struct RuntimeStatusApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<RuntimeStatusResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for runtime cache operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime cache API response")]
pub struct RuntimeCacheApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<RuntimeCacheResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for runtime cache reset operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime cache reset API response")]
pub struct RuntimeCacheResetApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<RuntimeCacheResetResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

// Implementation blocks for success and error methods
impl RuntimeInstallApiResp {
    pub fn success(data: RuntimeInstallResp) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(error: ApiError) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}

impl RuntimeStatusApiResp {
    pub fn success(data: RuntimeStatusResp) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(error: ApiError) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}

impl RuntimeCacheApiResp {
    pub fn success(data: RuntimeCacheResp) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(error: ApiError) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}

impl RuntimeCacheResetApiResp {
    pub fn success(data: RuntimeCacheResetResp) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(error: ApiError) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}
