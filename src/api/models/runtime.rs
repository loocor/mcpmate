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
pub struct RuntimeInstallData {
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
pub struct RuntimeStatusData {
    #[schemars(description = "UV runtime status")]
    pub uv: RuntimeStatus,
    #[schemars(description = "Bun runtime status")]
    pub bun: RuntimeStatus,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime cache summary information")]
pub struct RuntimeCacheSummary {
    #[schemars(description = "Total cache size in bytes")]
    pub total_size_bytes: u64,
    #[schemars(description = "ISO 8601 timestamp of last cleanup")]
    pub last_cleanup: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime cache item details")]
pub struct RuntimeCacheItem {
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
pub struct RuntimeCacheData {
    #[schemars(description = "Cache summary across all runtimes")]
    pub summary: RuntimeCacheSummary,
    #[schemars(description = "UV cache details")]
    pub uv: RuntimeCacheItem,
    #[schemars(description = "Bun cache details")]
    pub bun: RuntimeCacheItem,
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
pub struct RuntimeCacheResetData {
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
pub struct RuntimeInstallResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<RuntimeInstallData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for runtime status operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime status API response")]
pub struct RuntimeStatusResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<RuntimeStatusData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for runtime cache operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime cache API response")]
pub struct RuntimeCacheResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<RuntimeCacheData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for runtime cache reset operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Runtime cache reset API response")]
pub struct RuntimeCacheResetResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<RuntimeCacheResetData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

// Implementation blocks for success and error methods
impl RuntimeInstallResp {
    pub fn success(data: RuntimeInstallData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl RuntimeStatusResp {
    pub fn success(data: RuntimeStatusData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl RuntimeCacheResp {
    pub fn success(data: RuntimeCacheData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl RuntimeCacheResetResp {
    pub fn success(data: RuntimeCacheResetData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}
