//! Type definitions for the cache system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Cached server data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedServerData {
    pub server_id: String,
    pub server_name: String,
    pub server_version: Option<String>,
    pub protocol_version: String,
    pub tools: Vec<CachedToolInfo>,
    pub resources: Vec<CachedResourceInfo>,
    pub prompts: Vec<CachedPromptInfo>,
    pub resource_templates: Vec<CachedResourceTemplateInfo>,
    pub cached_at: DateTime<Utc>,
    pub fingerprint: String,
    pub instance_type: InstanceType,
}

/// Instance type classification for connection pool integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstanceType {
    Production,
    Exploration { session_id: String, ttl_minutes: u32 },
    Validation { session_id: String, ttl_minutes: u32 },
}

/// Cached tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema_json: String, // Store as JSON string for bincode compatibility
    pub unique_name: Option<String>,
    pub enabled: bool,
    pub cached_at: DateTime<Utc>,
}

impl CachedToolInfo {
    /// Get the input schema as a serde_json::Value
    pub fn input_schema(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.input_schema_json)
    }
    
    /// Set the input schema from a serde_json::Value
    pub fn set_input_schema(&mut self, schema: &serde_json::Value) -> Result<(), serde_json::Error> {
        self.input_schema_json = serde_json::to_string(schema)?;
        Ok(())
    }
}

/// Cached resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResourceInfo {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub enabled: bool,
    pub cached_at: DateTime<Utc>,
}

/// Cached prompt information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<PromptArgument>,
    pub enabled: bool,
    pub cached_at: DateTime<Utc>,
}

/// Cached resource template information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResourceTemplateInfo {
    pub uri_template: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub enabled: bool,
    pub cached_at: DateTime<Utc>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

/// Cache operation result
#[derive(Debug, Clone)]
pub struct CacheResult<T> {
    pub data: T,
    pub cache_hit: bool,
    pub cached_at: Option<DateTime<Utc>>,
    pub instance_type: InstanceType,
}

/// Cache query parameters
#[derive(Debug, Clone)]
pub struct CacheQuery {
    pub server_id: String,
    pub instance_type: InstanceType,
    pub freshness_level: FreshnessLevel,
    pub include_disabled: bool,
}

/// Freshness level for cache queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FreshnessLevel {
    /// Use cache if available, no freshness check
    Cached,
    /// Use cache if < 5 minutes old, otherwise refresh
    RecentlyFresh,
    /// Always fetch fresh data, update cache
    RealTime,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_servers: u64,
    pub total_tools: u64,
    pub total_resources: u64,
    pub total_prompts: u64,
    pub total_resource_templates: u64,
    pub cache_size_bytes: u64,
    pub hit_ratio: f64,
    pub last_updated: DateTime<Utc>,
}

/// Cache error types
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Database error: {0}")]
    Database(#[from] Box<redb::Error>),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    #[error("Server not found: {0}")]
    ServerNotFound(String),
    
    #[error("Tool not found: {server_id}/{tool_name}")]
    ToolNotFound { server_id: String, tool_name: String },
    
    #[error("Resource not found: {server_id}/{resource_uri}")]
    ResourceNotFound { server_id: String, resource_uri: String },
    
    #[error("Prompt not found: {server_id}/{prompt_name}")]
    PromptNotFound { server_id: String, prompt_name: String },
    
    #[error("Cache corruption detected: {0}")]
    Corruption(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),
    
    #[error("Cache operation timeout")]
    Timeout,
    
    #[error("Concurrent access conflict")]
    ConcurrentAccess,
    
    #[error("Cache size limit exceeded")]
    SizeLimitExceeded,
    
    #[error("Migration error: {0}")]
    Migration(String),
}

// Additional From implementations for specific redb error types
impl From<redb::TransactionError> for CacheError {
    fn from(err: redb::TransactionError) -> Self {
        CacheError::InvalidFormat(format!("Transaction error: {}", err))
    }
}

impl From<redb::TableError> for CacheError {
    fn from(err: redb::TableError) -> Self {
        CacheError::InvalidFormat(format!("Table error: {}", err))
    }
}

impl From<redb::StorageError> for CacheError {
    fn from(err: redb::StorageError) -> Self {
        CacheError::InvalidFormat(format!("Storage error: {}", err))
    }
}

impl From<redb::CommitError> for CacheError {
    fn from(err: redb::CommitError) -> Self {
        CacheError::InvalidFormat(format!("Commit error: {}", err))
    }
}