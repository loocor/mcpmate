//! Domain model - Pure business logic without external dependencies
//!
//! Defines core domain concepts for unified capability queries, including query parameters, results, error types, etc.
//! Follows domain-driven design principles to ensure business logic purity and testability.
//!
//! NOTE: This module defines its own domain types as specified in the refactoring guide.
//! Integration with existing code is done through adapter pattern, not through type re-export.

use chrono::{DateTime, Utc};
use rmcp::model::Icon;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Capability type enumeration - Domain-specific definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityType {
    Tools,
    Resources,
    Prompts,
    ResourceTemplates,
}

impl CapabilityType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityType::Tools => "tools",
            CapabilityType::Resources => "resources",
            CapabilityType::Prompts => "prompts",
            CapabilityType::ResourceTemplates => "resource_templates",
        }
    }

    /// Convert to existing capability type
    pub fn to_existing(&self) -> crate::api::handlers::server::capability::CapabilityType {
        match self {
            CapabilityType::Tools => crate::api::handlers::server::capability::CapabilityType::Tools,
            CapabilityType::Resources => crate::api::handlers::server::capability::CapabilityType::Resources,
            CapabilityType::Prompts => crate::api::handlers::server::capability::CapabilityType::Prompts,
            CapabilityType::ResourceTemplates => {
                crate::api::handlers::server::capability::CapabilityType::ResourceTemplates
            }
        }
    }

    /// Convert from existing capability type
    pub fn from_existing(existing: crate::api::handlers::server::capability::CapabilityType) -> Self {
        match existing {
            crate::api::handlers::server::capability::CapabilityType::Tools => CapabilityType::Tools,
            crate::api::handlers::server::capability::CapabilityType::Resources => CapabilityType::Resources,
            crate::api::handlers::server::capability::CapabilityType::Prompts => CapabilityType::Prompts,
            crate::api::handlers::server::capability::CapabilityType::ResourceTemplates => {
                CapabilityType::ResourceTemplates
            }
        }
    }
}

impl fmt::Display for CapabilityType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Query context - Distinguish different usage scenarios
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryContext {
    /// API call scenario - Just get information, no后续使用 (no subsequent use)
    ApiCall,
    /// MCP protocol client scenario - Get information and will actually use it
    McpClient,
}

impl QueryContext {
    /// Whether persistent instance is needed
    pub fn needs_persistent_instance(&self) -> bool {
        match self {
            QueryContext::ApiCall => false,
            QueryContext::McpClient => true,
        }
    }
}

/// Data freshness requirements - mapped to existing FreshnessLevel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FreshnessRequirement {
    /// Prefer cache, query runtime when cache misses
    #[default]
    CachePreferred,
    /// Force refresh, bypass cache
    ForceRefresh,
    /// Use cache only, don't query runtime
    CacheOnly,
}

impl FreshnessRequirement {
    /// Convert to existing cache FreshnessLevel
    pub fn to_cache_level(&self) -> crate::core::cache::FreshnessLevel {
        match self {
            FreshnessRequirement::CachePreferred => crate::core::cache::FreshnessLevel::Cached,
            FreshnessRequirement::ForceRefresh => crate::core::cache::FreshnessLevel::RealTime,
            FreshnessRequirement::CacheOnly => crate::core::cache::FreshnessLevel::Cached,
        }
    }

    /// Convert from existing RefreshStrategy
    pub fn from_refresh_strategy(strategy: crate::api::handlers::server::common::RefreshStrategy) -> Self {
        match strategy {
            crate::api::handlers::server::common::RefreshStrategy::CacheFirst => FreshnessRequirement::CachePreferred,
            crate::api::handlers::server::common::RefreshStrategy::RefreshIfStale => {
                FreshnessRequirement::CachePreferred
            }
            crate::api::handlers::server::common::RefreshStrategy::Force => FreshnessRequirement::ForceRefresh,
        }
    }
}

/// Adapter for converting between domain and existing types
pub struct Adapter;

impl Adapter {
    /// Convert from existing InspectParams to domain types
    pub fn domain_freshness_from_params(
        params: &crate::api::handlers::server::common::InspectParams
    ) -> FreshnessRequirement {
        params
            .refresh
            .map(FreshnessRequirement::from_refresh_strategy)
            .unwrap_or(FreshnessRequirement::CachePreferred)
    }

    /// Convert domain capability type to existing
    pub fn existing_capability_type(
        domain_type: CapabilityType
    ) -> crate::api::handlers::server::capability::CapabilityType {
        domain_type.to_existing()
    }

    /// Convert existing capability type to domain
    pub fn domain_capability_type(
        existing_type: crate::api::handlers::server::capability::CapabilityType
    ) -> CapabilityType {
        CapabilityType::from_existing(existing_type)
    }

    /// Convert cached tool to domain tool capability
    pub fn convert_tool_to_domain(tool: &crate::core::cache::CachedToolInfo) -> ToolCapability {
        ToolCapability {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema().unwrap_or_default(),
            unique_name: tool.unique_name.clone().unwrap_or_default(),
            enabled: tool.enabled,
            icons: tool.icons.clone(),
        }
    }

    /// Convert cached resource to domain resource capability
    pub fn convert_resource_to_domain(resource: &crate::core::cache::CachedResourceInfo) -> ResourceCapability {
        ResourceCapability {
            uri: resource.uri.clone(),
            name: resource.name.clone(),
            description: resource.description.clone(),
            mime_type: resource.mime_type.clone(),
            unique_uri: resource.uri.clone(), // Use uri as unique_uri
            enabled: resource.enabled,
            icons: resource.icons.clone(),
        }
    }

    /// Convert cached prompt to domain prompt capability
    pub fn convert_prompt_to_domain(prompt: &crate::core::cache::CachedPromptInfo) -> PromptCapability {
        // Convert cache PromptArgument to domain PromptArgument
        let domain_arguments = Some(
            prompt
                .arguments
                .iter()
                .map(|arg| PromptArgument {
                    name: arg.name.clone(),
                    description: arg.description.clone(),
                    required: Some(arg.required),
                })
                .collect::<Vec<PromptArgument>>(),
        );

        PromptCapability {
            name: prompt.name.clone(),
            description: prompt.description.clone(),
            arguments: domain_arguments,
            unique_name: prompt.name.clone(), // Use name as unique_name
            enabled: prompt.enabled,
            icons: prompt.icons.clone(),
        }
    }

    /// Convert cached resource template to domain resource template capability
    pub fn convert_template_to_domain(
        template: &crate::core::cache::CachedResourceTemplateInfo
    ) -> ResourceTemplateCapability {
        ResourceTemplateCapability {
            uri_template: template.uri_template.clone(),
            name: template.name.clone(),
            description: template.description.clone(),
            mime_type: template.mime_type.clone(),
            unique_template: template.uri_template.clone(), // Use uri_template as unique_template
            enabled: template.enabled,
        }
    }
}

/// Capability query parameters
#[derive(Debug, Clone)]
pub struct CapabilityQuery {
    /// Server ID
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Capability type
    pub capability_type: CapabilityType,
    /// Data freshness requirements
    pub freshness: FreshnessRequirement,
    /// Query context
    pub context: QueryContext,
    /// Query timeout duration
    pub timeout: std::time::Duration,
}

impl CapabilityQuery {
    /// Create new query
    pub fn new(
        server_id: String,
        server_name: String,
        capability_type: CapabilityType,
        context: QueryContext,
    ) -> Self {
        Self {
            server_id,
            server_name,
            capability_type,
            freshness: FreshnessRequirement::default(),
            context,
            timeout: std::time::Duration::from_secs(30),
        }
    }

    /// Set freshness requirements
    pub fn with_freshness(
        mut self,
        freshness: FreshnessRequirement,
    ) -> Self {
        self.freshness = freshness;
        self
    }

    /// Set timeout duration
    pub fn with_timeout(
        mut self,
        timeout: std::time::Duration,
    ) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Data source enumeration - indicates result source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    /// L1 cache (memory)
    CacheL1,
    /// L2 cache (ReDB)
    CacheL2,
    /// Runtime instance
    Runtime,
    /// Temporary instance
    Temporary,
    /// Warm-up startup
    WarmUp,
    /// No available data
    None,
}

impl DataSource {
    /// Whether it's from cache source
    pub fn is_cache(&self) -> bool {
        matches!(self, DataSource::CacheL1 | DataSource::CacheL2)
    }

    /// Get description string
    pub fn description(&self) -> &'static str {
        match self {
            DataSource::CacheL1 => "memory_cache",
            DataSource::CacheL2 => "redb_cache",
            DataSource::Runtime => "runtime_instance",
            DataSource::Temporary => "temporary_instance",
            DataSource::WarmUp => "warmup_instance",
            DataSource::None => "none",
        }
    }
}

/// 响应元数据
#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    /// 是否缓存命中
    pub cache_hit: bool,
    /// 数据来源
    pub source: DataSource,
    /// 查询耗时
    pub duration_ms: u64,
    /// 结果数量
    pub item_count: usize,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl ResponseMetadata {
    /// 创建缓存命中的元数据
    pub fn cache_hit(
        source: DataSource,
        duration: std::time::Duration,
        count: usize,
    ) -> Self {
        Self {
            cache_hit: true,
            source,
            duration_ms: duration.as_millis() as u64,
            item_count: count,
            timestamp: Utc::now(),
        }
    }

    /// 创建运行时查询的元数据
    pub fn runtime(
        duration: std::time::Duration,
        count: usize,
    ) -> Self {
        Self {
            cache_hit: false,
            source: DataSource::Runtime,
            duration_ms: duration.as_millis() as u64,
            item_count: count,
            timestamp: Utc::now(),
        }
    }
}

/// Capability result
#[derive(Debug, Clone)]
pub struct CapabilityResult {
    /// Capability items list
    pub items: Vec<CapabilityItem>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl CapabilityResult {
    /// Create empty result
    pub fn empty(source: DataSource) -> Self {
        Self {
            items: Vec::new(),
            metadata: ResponseMetadata {
                cache_hit: false,
                source,
                duration_ms: 0,
                item_count: 0,
                timestamp: Utc::now(),
            },
        }
    }

    /// Get result count
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Capability item - Unified capability representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CapabilityItem {
    Tool(ToolCapability),
    Resource(ResourceCapability),
    Prompt(PromptCapability),
    ResourceTemplate(ResourceTemplateCapability),
}

/// Tool capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub unique_name: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<Icon>>,
}

/// Resource capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapability {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub unique_uri: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<Icon>>,
}

/// Prompt capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCapability {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
    pub unique_name: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<Icon>>,
}

/// Prompt argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

/// Resource template capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplateCapability {
    pub uri_template: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub unique_template: String,
    pub enabled: bool,
}

/// Capability error enumeration
#[derive(Error, Debug)]
pub enum CapabilityError {
    #[error("Server {server_id} is disabled")]
    ServerDisabled { server_id: String },

    #[error("Capability {capability_type} is disabled for server {server_id}")]
    CapabilityDisabled {
        capability_type: CapabilityType,
        server_id: String,
    },

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Warm-up error: {0}")]
    WarmUpError(String),

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("No available instances for server {0}")]
    NoAvailableInstances(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl CapabilityError {
    /// Whether it's a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CapabilityError::CacheError(_) | CapabilityError::RuntimeError(_) | CapabilityError::Timeout(_)
        )
    }

    /// Convert to HTTP status code (for API layer)
    pub fn to_status_code(&self) -> u16 {
        match self {
            CapabilityError::ServerDisabled { .. } => 403,
            CapabilityError::CapabilityDisabled { .. } => 403,
            CapabilityError::Timeout(_) => 408,
            CapabilityError::NoAvailableInstances(_) => 503,
            _ => 500,
        }
    }
}

impl From<anyhow::Error> for CapabilityError {
    fn from(err: anyhow::Error) -> Self {
        CapabilityError::InternalError(err.to_string())
    }
}

/// Connection isolation strategies as defined in refactoring guide
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum IsolationMode {
    /// Shareable: suitable for weak/stateless upstream services (HTTP/SSE, query-only tools)
    #[default]
    Shareable,
    /// Per-client: each downstream MCP client connection has independent upstream instance
    PerClient,
    /// Per-session: isolate by downstream session/conversation ID
    PerSession,
}

/// Connection affinity key for routing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AffinityKey {
    /// Default shared connection
    Default,
    /// Per-client isolation using connection ID
    PerClient(String),
    /// Per-session isolation using session ID
    PerSession(String),
}

/// Connection mode combining isolation strategy and routing key
#[derive(Debug, Clone)]
pub struct ConnectionMode {
    /// Isolation strategy
    pub isolation_mode: IsolationMode,
    /// Affinity key for routing
    pub affinity_key: AffinityKey,
}

impl ConnectionMode {
    /// Create shareable mode (default for HTTP/SSE)
    pub fn shareable() -> Self {
        Self {
            isolation_mode: IsolationMode::Shareable,
            affinity_key: AffinityKey::Default,
        }
    }

    /// Create per-client mode (default for stdio)
    pub fn per_client(client_id: String) -> Self {
        Self {
            isolation_mode: IsolationMode::PerClient,
            affinity_key: AffinityKey::PerClient(client_id),
        }
    }

    /// Create per-session mode
    pub fn per_session(session_id: String) -> Self {
        Self {
            isolation_mode: IsolationMode::PerSession,
            affinity_key: AffinityKey::PerSession(session_id),
        }
    }

    /// Get string representation of affinity key for indexing
    pub fn affinity_key_string(&self) -> String {
        match &self.affinity_key {
            AffinityKey::Default => "default".to_string(),
            AffinityKey::PerClient(id) => format!("client:{}", id),
            AffinityKey::PerSession(id) => format!("session:{}", id),
        }
    }
}

/// Cache key - using existing cache types
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub server_id: String,
    pub capability_type: CapabilityType,
    pub freshness_requirement: String, // Use string to match cache expectations
}

impl CacheKey {
    /// Create from query parameters
    pub fn from_query(
        server_id: &str,
        capability_type: CapabilityType,
        freshness: FreshnessRequirement,
    ) -> Self {
        Self {
            server_id: server_id.to_string(),
            capability_type,
            freshness_requirement: format!("{:?}", freshness).to_lowercase(),
        }
    }
}

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub items: Vec<CapabilityItem>,
    pub cached_at: DateTime<Utc>,
    pub ttl: std::time::Duration,
}

impl CacheEntry {
    /// Check if expired
    pub fn is_expired(&self) -> bool {
        Utc::now() - self.cached_at > chrono::Duration::from_std(self.ttl).unwrap_or(chrono::Duration::MAX)
    }

    /// Convert to capability result
    pub fn into_result(self) -> CapabilityResult {
        let count = self.items.len();
        CapabilityResult {
            items: self.items,
            metadata: ResponseMetadata {
                cache_hit: true,
                source: DataSource::CacheL2,
                duration_ms: 0,
                item_count: count,
                timestamp: self.cached_at,
            },
        }
    }
}
