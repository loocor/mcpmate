// Discovery system core data types
// Provides type definitions for the MCPMate discovery system

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Server capabilities information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Server identifier
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Capabilities metadata
    pub metadata: CapabilitiesMetadata,
    /// Available tools
    pub tools: Vec<ToolInfo>,
    /// Available resources
    pub resources: Vec<ResourceInfo>,
    /// Available prompts
    pub prompts: Vec<PromptInfo>,
    /// Resource templates
    pub resource_templates: Vec<ResourceTemplateInfo>,
}

/// Capabilities metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesMetadata {
    /// Last updated timestamp (ISO 8601 format)
    #[serde(with = "iso8601_timestamp")]
    pub last_updated: SystemTime,
    /// Cache version
    pub version: String,
    /// Time to live for cache in seconds
    #[serde(rename = "ttl_seconds", with = "duration_as_seconds")]
    pub ttl: Duration,
    /// Server protocol version
    pub protocol_version: Option<String>,
}

/// Serialize SystemTime as ISO 8601 string
mod iso8601_timestamp {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(
        time: &SystemTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(
            duration.as_secs() as i64,
            duration.subsec_nanos(),
        )
        .ok_or_else(|| serde::ser::Error::custom("Invalid timestamp"))?;
        datetime.to_rfc3339().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let datetime =
            chrono::DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(datetime.timestamp() as u64))
    }
}

/// Serialize Duration as seconds
mod duration_as_seconds {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(
        duration: &Duration,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema (JSON object)
    pub input_schema: serde_json::Value,
    /// Tool annotations
    pub annotations: Option<serde_json::Value>,
}

/// Resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: Option<String>,
    /// Resource description
    pub description: Option<String>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Resource annotations
    pub annotations: Option<serde_json::Value>,
}

/// Prompt information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInfo {
    /// Prompt name
    pub name: String,
    /// Prompt description
    pub description: Option<String>,
    /// Prompt arguments
    pub arguments: Vec<PromptArgument>,
    /// Prompt annotations
    pub annotations: Option<serde_json::Value>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    pub description: Option<String>,
    /// Whether argument is required
    pub required: bool,
}

/// Resource template information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplateInfo {
    /// Template URI pattern
    pub uri_template: String,
    /// Template name
    pub name: Option<String>,
    /// Template description
    pub description: Option<String>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Template annotations
    pub annotations: Option<serde_json::Value>,
}

/// Refresh strategy for capability queries
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum RefreshStrategy {
    /// Use cache if available, don't refresh
    CacheFirst,
    /// Refresh if cache is stale based on TTL
    #[default]
    RefreshIfStale,
    /// Force refresh regardless of cache state
    Force,
}

/// Response format for discovery APIs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum ResponseFormat {
    /// Standard JSON format
    #[default]
    Json,
    /// Compact JSON format (minimal fields)
    Compact,
    /// Detailed JSON format (all fields)
    Detailed,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Memory cache size (number of entries)
    pub memory_cache_size: usize,
    /// Default TTL for cached capabilities
    pub default_ttl: Duration,
    /// Maximum total file cache size in bytes
    pub max_file_cache_size: u64,
    /// Maximum age for cached files
    pub max_file_age: Duration,
    /// Cleanup interval for file cache
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            memory_cache_size: 10,
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_file_cache_size: 50 * 1024 * 1024, // 50MB
            max_file_age: Duration::from_secs(7 * 24 * 3600), // 7 days
            cleanup_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Discovery query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryParams {
    /// Refresh strategy
    pub refresh: Option<RefreshStrategy>,
    /// Response format
    pub format: Option<ResponseFormat>,
    /// Include metadata in response
    pub include_meta: Option<bool>,
}

impl Default for DiscoveryParams {
    fn default() -> Self {
        Self {
            refresh: Some(RefreshStrategy::default()),
            format: Some(ResponseFormat::default()),
            include_meta: Some(false),
        }
    }
}

/// Capability selection for config suit synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySelections {
    /// Server ID
    pub server_id: String,
    /// Selected tools (tool name -> enabled)
    pub tools: HashMap<String, bool>,
    /// Selected resources (resource URI -> enabled)
    pub resources: HashMap<String, bool>,
    /// Selected prompts (prompt name -> enabled)
    pub prompts: HashMap<String, bool>,
    /// Server enabled status
    pub server_enabled: bool,
}

/// Sync result for config suit operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Whether sync was successful
    pub success: bool,
    /// Number of tools updated
    pub tools_updated: usize,
    /// Number of resources updated
    pub resources_updated: usize,
    /// Number of prompts updated
    pub prompts_updated: usize,
    /// Error message if sync failed
    pub error: Option<String>,
}

/// Discovery system errors
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Server '{0}' not found")]
    ServerNotFound(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Cache operation failed: {0}")]
    CacheError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Timeout occurred: {0}")]
    Timeout(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Type alias for discovery results
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Discovery API response wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponse<T> {
    /// Response data
    pub data: T,
    /// Response metadata
    pub meta: Option<ResponseMetadata>,
}

/// Response metadata for discovery APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Refresh strategy used
    pub refresh_strategy: RefreshStrategy,
    /// Response format used
    pub format: ResponseFormat,
    /// Whether response came from cache
    pub cache_hit: Option<bool>,
    /// Last updated timestamp (ISO 8601 format)
    #[serde(with = "iso8601_timestamp")]
    pub last_updated: SystemTime,
    /// Cache version
    pub version: String,
    /// Time to live for cache in seconds
    #[serde(rename = "ttl_seconds", with = "duration_as_seconds")]
    pub ttl: Duration,
    /// Server protocol version
    pub protocol_version: Option<String>,
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryMetadata {
    /// Entry creation time
    pub created_at: SystemTime,
    /// Last access time
    pub last_accessed: SystemTime,
    /// Entry size in bytes
    pub size: u64,
    /// Entry version
    pub version: String,
}

/// File cache manifest entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCacheManifest {
    /// Server capabilities entries
    pub entries: HashMap<String, CacheEntryMetadata>,
    /// Total cache size
    pub total_size: u64,
    /// Last cleanup time
    pub last_cleanup: SystemTime,
}
