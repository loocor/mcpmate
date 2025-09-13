//! Port interfaces - Define interaction boundaries with the external world
//! 
//! Following the ports and adapters pattern (Hexagonal Architecture), define core business logic
//! and external infrastructure interaction interfaces, implementing dependency inversion and decoupling.

use async_trait::async_trait;
use crate::core::capability::domain::{
    CapabilityError, CapabilityItem, CapabilityQuery, CapabilityResult, CapabilityType, FreshnessRequirement, QueryContext, DataSource,
};
use std::sync::Arc;

/// Cache port interface
/// 
/// Define interaction with cache systems, supporting multi-level caching (L1 memory, L2 ReDB)
#[async_trait]
pub trait CachePort: Send + Sync + 'static {
    /// Get entry by cache key
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError>;
    
    /// Store cache entry
    async fn set(&self, key: &CacheKey, value: &CacheEntry) -> Result<(), CacheError>;
    
    /// Invalidate cache by pattern
    async fn invalidate(&self, pattern: &str) -> Result<(), CacheError>;
    
    /// Check if cache is available
    async fn is_healthy(&self) -> bool;
    
    /// Get cache statistics
    async fn stats(&self) -> CacheStats;
}

/// Runtime port interface
/// 
/// Define interaction with connection pools and runtime instances
#[async_trait]
pub trait RuntimePort: Send + Sync + 'static {
    /// Get connected instances for specified server
    async fn get_connected(&self, server_name: &str) -> Result<Vec<InstanceRef>, RuntimeError>;
    
    /// Ensure server has available instances (create if necessary)
    async fn ensure_connected(&self, server_info: &ServerInfo) -> Result<InstanceRef, RuntimeError>;
    
    /// Get instance status
    async fn get_instance_status(&self, server_id: &str, instance_id: &str) -> Result<InstanceStatus, RuntimeError>;
    
    /// Check if runtime is healthy
    async fn is_healthy(&self) -> bool;
    
    /// Get runtime statistics
    async fn stats(&self) -> RuntimeStats;
}

/// Warm-up port interface
/// 
/// Define interaction with instance warm-up mechanisms
#[async_trait]
pub trait WarmUpPort: Send + Sync + 'static {
    /// Warm up instance for specified server
    async fn warmup_server(&self, server_info: &ServerInfo) -> Result<InstanceRef, WarmUpError>;
    
    /// Async warm-up and cache (non-blocking current request)
    async fn warmup_async(&self, server_info: &ServerInfo) -> Result<(), WarmUpError>;
    
    /// Check if warm-up is complete
    async fn is_warmup_complete(&self, server_id: &str) -> bool;
    
    /// Get warm-up statistics
    async fn stats(&self) -> WarmUpStats;
}

/// Enabled status port interface
/// 
/// Define interaction with enabled status checks
#[async_trait]
pub trait EnabledPort: Send + Sync + 'static {
    /// Check if server is enabled
    async fn is_server_enabled(&self, server_id: &str) -> Result<bool, EnabledError>;
    
    /// Check if specific capability type is enabled
    async fn is_capability_enabled(&self, server_id: &str, cap_type: CapabilityType) -> Result<bool, EnabledError>;
    
    /// Filter enabled capability items
    async fn filter_enabled(&self, items: Vec<CapabilityItem>, server_id: &str) -> Result<Vec<CapabilityItem>, EnabledError>;
    
    /// Get enabled status change event stream
    fn enabled_changes(&self) -> Box<dyn Stream<Item = EnabledChangeEvent> + Send + Unpin>;
}

/// Cache key - Used to uniquely identify cache entries
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub server_id: String,
    pub capability_type: CapabilityType,
    pub freshness_requirement: String, // "cache_preferred", "force_refresh", "cache_only"
}

impl CacheKey {
    /// Create cache key from capability query
    pub fn from_query(query: &CapabilityQuery) -> Self {
        Self {
            server_id: query.server_id.clone(),
            capability_type: query.capability_type,
            freshness_requirement: format!("{:?}", query.freshness).to_lowercase(),
        }
    }

    /// Create cache key pattern (for batch invalidation)
    pub fn pattern(server_id: &str, cap_type: CapabilityType) -> String {
        format!("{}:{}", server_id, cap_type.as_str())
    }
}

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub items: Vec<CapabilityItem>,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub ttl: std::time::Duration,
    pub etag: Option<String>, // For cache validation
}

/// Cache statistics
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub evictions: u64,
    pub hit_rate: f64,
    pub avg_latency_ms: f64,
}

/// Instance reference - Abstract representation of runtime instances
#[derive(Debug, Clone)]
pub struct InstanceRef {
    pub server_id: String,
    pub instance_id: String,
    pub server_name: String,
    pub status: InstanceStatus,
    pub connection_info: ConnectionInfo,
}

/// Instance status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    Connected,
    Disconnected,
    Initializing,
    Error,
}

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub transport_type: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub capabilities: Vec<String>,
}

/// Server information
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub server_id: String,
    pub server_name: String,
    pub server_type: String,
    pub transport_type: String,
}

/// Runtime statistics
#[derive(Debug, Default, Clone)]
pub struct RuntimeStats {
    pub total_instances: usize,
    pub connected_instances: usize,
    pub initializing_instances: usize,
    pub error_instances: usize,
    pub avg_connection_time_ms: f64,
    pub connection_success_rate: f64,
}

/// Warm-up statistics
#[derive(Debug, Default, Clone)]
pub struct WarmUpStats {
    pub warmup_requests: u64,
    pub successful_warmups: u64,
    pub failed_warmups: u64,
    pub avg_warmup_time_ms: f64,
    pub active_warmups: usize,
}

/// Enabled status change event
#[derive(Debug, Clone)]
pub struct EnabledChangeEvent {
    pub server_id: String,
    pub server_name: String,
    pub capability_type: Option<CapabilityType>, // None表示服务器级别变更
    pub enabled: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// Error type definitions

/// Cache error
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache unavailable: {0}")]
    Unavailable(String),
    
    #[error("Cache operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Runtime error
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Instance not found: {0}")]
    InstanceNotFound(String),
    
    #[error("Instance not connected: {0}")]
    InstanceNotConnected(String),
    
    #[error("Connection pool error: {0}")]
    PoolError(String),
    
    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),
    
    #[error("Runtime unavailable: {0}")]
    Unavailable(String),
}

/// Warm-up error
#[derive(Debug, Error)]
pub enum WarmUpError {
    #[error("Warmup failed for server {0}: {1}")]
    WarmupFailed(String, String),
    
    #[error("Warmup timeout for server {0}")]
    Timeout(String),
    
    #[error("Server configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Warmup service unavailable: {0}")]
    Unavailable(String),
}

/// Enabled error
#[derive(Debug, Error)]
pub enum EnabledError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Server not found: {0}")]
    ServerNotFound(String),
    
    #[error("Capability not found: {0}")]
    CapabilityNotFound(String),
    
    #[error("Enabled service unavailable: {0}")]
    Unavailable(String),
}

// Convenience trait implementations

impl From<CacheError> for CapabilityError {
    fn from(err: CacheError) -> Self {
        CapabilityError::CacheError(err.to_string())
    }
}

impl From<RuntimeError> for CapabilityError {
    fn from(err: RuntimeError) -> Self {
        CapabilityError::RuntimeError(err.to_string())
    }
}

impl From<WarmUpError> for CapabilityError {
    fn from(err: WarmUpError) -> Self {
        CapabilityError::WarmUpError(err.to_string())
    }
}

impl From<EnabledError> for CapabilityError {
    fn from(err: EnabledError) -> Self {
        match err {
            EnabledError::ServerNotFound(server_id) => {
                CapabilityError::ServerDisabled(server_id)
            }
            _ => CapabilityError::InternalError(err.to_string()),
        }
    }
}

/// Port health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Port health check trait
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Check port health status
    async fn health_check(&self) -> PortHealth;
    
    /// Get health check details
    async fn health_details(&self) -> serde_json::Value;
}

// Auto-implement health check for all ports
#[async_trait]
impl<T: CachePort + Send + Sync> HealthCheck for T {
    async fn health_check(&self) -> PortHealth {
        if self.is_healthy().await {
            PortHealth::Healthy
        } else {
            PortHealth::Unhealthy
        }
    }
    
    async fn health_details(&self) -> serde_json::Value {
        let stats = self.stats().await;
        serde_json::json!({
            "type": "cache",
            "hits": stats.hits,
            "misses": stats.misses,
            "hit_rate": stats.hit_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_from_query() {
        use crate::core::capability::domain::CapabilityQuery;
        use crate::core::capability::domain::QueryContext;
        
        let query = CapabilityQuery::new(
            "test-server".to_string(),
            "test-server-name".to_string(),
            CapabilityType::Tools,
            QueryContext::ApiCall,
        );
        
        let key = CacheKey::from_query(&query);
        assert_eq!(key.server_id, "test-server");
        assert_eq!(key.capability_type, CapabilityType::Tools);
        assert_eq!(key.freshness_requirement, "cache_preferred");
    }

    #[test]
    fn test_data_source_is_cache() {
        assert!(DataSource::CacheL1.is_cache());
        assert!(DataSource::CacheL2.is_cache());
        assert!(!DataSource::Runtime.is_cache());
        assert!(!DataSource::Temporary.is_cache());
        assert!(!DataSource::WarmUp.is_cache());
        assert!(!DataSource::None.is_cache());
    }

    #[test]
    fn test_query_context_needs_persistent() {
        assert!(!QueryContext::ApiCall.needs_persistent_instance());
        assert!(QueryContext::McpClient.needs_persistent_instance());
    }

    #[test]
    fn test_capability_type_as_str() {
        assert_eq!(CapabilityType::Tools.as_str(), "tools");
        assert_eq!(CapabilityType::Resources.as_str(), "resources");
        assert_eq!(CapabilityType::Prompts.as_str(), "prompts");
        assert_eq!(CapabilityType::ResourceTemplates.as_str(), "resource_templates");
    }

    #[test]
    fn test_error_conversions() {
        let cache_err = CacheError::Unavailable("test".to_string());
        let cap_err: CapabilityError = cache_err.into();
        assert!(matches!(cap_err, CapabilityError::CacheError(_)));
    }
}

