//! Unified query service - Orchestrate capability queries based on existing infrastructure

use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{Duration, timeout};

use crate::api::handlers::server::common::{
    InspectParams, InspectQuery, RefreshStrategy, ServerIdentification, get_server_info_for_inspect,
};

// Constants for unified query service
const DEFAULT_QUERY_TIMEOUT_SECS: u64 = 30;
// TODO: Implement cache update timeout functionality for background cache refresh operations
#[allow(dead_code)]
const CACHE_UPDATE_TIMEOUT_SECS: u64 = 10;
use crate::api::routes::AppState;
use crate::common::capability::CapabilityToken;
use crate::config::database::Database;
use crate::config::server::ServerEnabledService;
use crate::core::cache::{CacheQuery, FreshnessLevel, RedbCacheManager};
use crate::core::pool::UpstreamConnectionPool;

use super::UnifiedConnectionManager;
use super::domain::{
    Adapter, CapabilityError, CapabilityItem, CapabilityResult, CapabilityType, ConnectionMode, DataSource,
    QueryContext, ResponseMetadata,
};

/// Helper methods for converting between cache types and domain types
// TODO: Implement cache-to-domain conversion methods for unified data model integration
#[allow(dead_code)]
impl UnifiedQueryService {
    /// Convert cached tool to domain tool capability
    fn convert_tool_to_domain(tool: &crate::core::cache::CachedToolInfo) -> super::domain::ToolCapability {
        super::domain::ToolCapability {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema().unwrap_or_default(),
            unique_name: tool.unique_name.clone().unwrap_or_default(),
            enabled: tool.enabled,
        }
    }

    /// Convert cached resource to domain resource capability
    fn convert_resource_to_domain(
        resource: &crate::core::cache::CachedResourceInfo
    ) -> super::domain::ResourceCapability {
        super::domain::ResourceCapability {
            uri: resource.uri.clone(),
            name: resource.name.clone(),
            description: resource.description.clone(),
            mime_type: resource.mime_type.clone(),
            unique_uri: resource.uri.clone(), // Use uri as unique_uri since it doesn't exist in cached type
            enabled: resource.enabled,
        }
    }

    /// Convert cached prompt to domain prompt capability
    fn convert_prompt_to_domain(prompt: &crate::core::cache::CachedPromptInfo) -> super::domain::PromptCapability {
        super::domain::PromptCapability {
            name: prompt.name.clone(),
            description: prompt.description.clone(),
            arguments: Some(
                prompt
                    .arguments
                    .iter()
                    .map(|arg| super::domain::PromptArgument {
                        name: arg.name.clone(),
                        description: arg.description.clone(),
                        required: Some(arg.required),
                    })
                    .collect(),
            ),
            unique_name: prompt.name.clone(), // Use name as unique_name since it doesn't exist in cached type
            enabled: prompt.enabled,
        }
    }

    /// Convert cached resource template to domain resource template capability
    fn convert_template_to_domain(
        template: &crate::core::cache::CachedResourceTemplateInfo
    ) -> super::domain::ResourceTemplateCapability {
        super::domain::ResourceTemplateCapability {
            uri_template: template.uri_template.clone(),
            name: template.name.clone(),
            description: template.description.clone(),
            mime_type: template.mime_type.clone(),
            unique_template: template.uri_template.clone(), // Use uri_template as unique_template
            enabled: template.enabled,
        }
    }
}

/// Unified query service - Orchestrate capability queries based on existing infrastructure
pub struct UnifiedQueryService {
    /// ReDB cache manager
    cache: Arc<RedbCacheManager>,
    /// Connection pool
    pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
    /// Database
    database: Arc<Database>,
    /// App state for existing API compatibility
    app_state: Arc<AppState>,
    /// Unified connection manager for MCP protocol unification
    connection_manager: Arc<UnifiedConnectionManager>,
    /// Query timeout configuration
    timeout_duration: Duration,
}

impl std::fmt::Debug for UnifiedQueryService {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("UnifiedQueryService")
            .field("cache", &"Arc<RedbCacheManager>")
            .field("pool", &self.pool)
            .field("database", &self.database)
            .field("app_state", &"Arc<AppState>")
            .field("connection_manager", &"Arc<UnifiedConnectionManager>")
            .field("timeout_duration", &self.timeout_duration)
            .finish()
    }
}

impl UnifiedQueryService {
    /// Create new unified service
    pub fn new(
        cache: Arc<RedbCacheManager>,
        pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
        database: Arc<Database>,
        app_state: Arc<AppState>,
        connection_manager: Arc<UnifiedConnectionManager>,
    ) -> Self {
        Self {
            cache,
            pool,
            database,
            app_state,
            connection_manager,
            timeout_duration: Duration::from_secs(DEFAULT_QUERY_TIMEOUT_SECS),
        }
    }

    /// Query capabilities - unified entry point
    pub async fn query_capabilities(
        &self,
        server_id: &str,
        capability_type: CapabilityType,
        query_params: &InspectParams,
        _context: QueryContext,
    ) -> Result<CapabilityResult, CapabilityError> {
        let start_time = Instant::now();

        // 1. Get server info and validate
        let (_db, server_info, params) = self.get_server_info(server_id, query_params).await?;

        // 2. Check enabled status
        self.check_enabled(&server_info.server_id, capability_type).await?;

        // 3. Route based on refresh strategy - simplified
        match query_params.refresh {
            Some(RefreshStrategy::Force) => {
                self.query_runtime(&server_info, capability_type, &params, start_time)
                    .await
            }
            _ => {
                self.query_with_cache_fallback(&server_info, capability_type, &params, start_time)
                    .await
            }
        }
    }

    /// Get server info
    async fn get_server_info(
        &self,
        server_id: &str,
        query_params: &InspectParams,
    ) -> Result<(Arc<Database>, ServerIdentification, InspectParams), CapabilityError> {
        // Reuse existing unified validation logic
        let inspect_query = InspectQuery {
            refresh: query_params.refresh,
            format: query_params.format.clone(),
            include_meta: query_params.include_meta,
            timeout: query_params.timeout,
        };

        get_server_info_for_inspect(&self.app_state, server_id, &inspect_query)
            .await
            .map_err(|e| CapabilityError::InternalError(format!("Failed to get server info: {}", e)))
    }

    /// Check enabled status
    async fn check_enabled(
        &self,
        server_id: &str,
        capability_type: CapabilityType,
    ) -> Result<(), CapabilityError> {
        // Check server enabled status - reuse existing logic
        let enabled_service = ServerEnabledService::new(self.database.pool.clone());

        if !enabled_service
            .is_server_enabled(server_id)
            .await
            .map_err(|e| CapabilityError::InternalError(format!("Failed to check server status: {}", e)))?
        {
            return Err(CapabilityError::ServerDisabled {
                server_id: server_id.to_string(),
            });
        }

        // Check specific capability enabled status
        let capability_token = match capability_type {
            CapabilityType::Tools => CapabilityToken::Tools,
            CapabilityType::Resources => CapabilityToken::Resources,
            CapabilityType::Prompts => CapabilityToken::Prompts,
            CapabilityType::ResourceTemplates => CapabilityToken::ResourceTemplates,
        };

        // Need to implement capability-level enabled check
        // Assume all capabilities are enabled for now, can be extended later
        let _ = capability_token;

        Ok(())
    }

    /// Query with cache fallback
    async fn query_with_cache_fallback(
        &self,
        server_info: &ServerIdentification,
        capability_type: CapabilityType,
        params: &InspectParams,
        start_time: Instant,
    ) -> Result<CapabilityResult, CapabilityError> {
        // 1. ReDB cache query
        let cache_result = self.query_redb_cache(server_info, capability_type).await?;
        if let Some(result) = cache_result {
            return Ok(result);
        }

        // 2. Runtime instance query
        let runtime_result = self
            .query_runtime_with_cache_update(server_info, capability_type, params, start_time)
            .await?;

        Ok(runtime_result)
    }

    /// ReDB cache query
    async fn query_redb_cache(
        &self,
        server_info: &ServerIdentification,
        capability_type: CapabilityType,
    ) -> Result<Option<CapabilityResult>, CapabilityError> {
        // Build cache query - reuse existing logic
        let cache_query = CacheQuery {
            server_id: server_info.server_id.clone(),
            freshness_level: FreshnessLevel::Cached, // Prefer cache
            include_disabled: false,
        };

        // Directly call existing cache query, avoid adapter wrapper, add timeout protection
        match timeout(self.timeout_duration, self.cache.get_server_data(&cache_query)).await {
            Ok(Ok(cache_result)) => {
                if cache_result.cache_hit {
                    if let Some(data) = cache_result.data {
                        // Direct conversion, avoid extra abstraction layer
                        let items = self.convert_cached_data_to_items(&data, capability_type)?;

                        let metadata = ResponseMetadata {
                            cache_hit: true,
                            source: DataSource::CacheL2,
                            duration_ms: 0, // Will be calculated at upper level
                            item_count: items.len(),
                            timestamp: chrono::Utc::now(),
                        };

                        return Ok(Some(CapabilityResult { items, metadata }));
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Cache query failed for server {}: {}", server_info.server_id, e);
            }
            Err(_) => {
                tracing::warn!("Cache query timeout for server {}", server_info.server_id);
            }
        }

        Ok(None)
    }

    /// Runtime query (with cache update)
    async fn query_runtime_with_cache_update(
        &self,
        server_info: &ServerIdentification,
        capability_type: CapabilityType,
        _params: &InspectParams,
        start_time: Instant,
    ) -> Result<CapabilityResult, CapabilityError> {
        // Determine connection mode based on server type and query context
        let connection_mode = self.determine_connection_mode(server_info).await?;

        // Use unified connection manager to ensure affinitized connection
        let _instance_id = self
            .connection_manager
            .ensure_affinitized_connection(&server_info.server_id, connection_mode)
            .await?;

        // Get connection pool lock to find connected instances
        let pool = self.pool.lock().await;

        // Find connection instances - reuse existing logic
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            let connected_instances: Vec<_> = instances.values().filter(|conn| conn.is_connected()).collect();

            if !connected_instances.is_empty() {
                // Extract capability info from connection instances
                let (items, should_cache) =
                    self.extract_capabilities_from_instances(&connected_instances, capability_type)?;

                let metadata = ResponseMetadata {
                    cache_hit: false,
                    source: DataSource::Runtime,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    item_count: items.len(),
                    timestamp: chrono::Utc::now(),
                };

                let result = CapabilityResult { items, metadata };

                // Update cache asynchronously (non-blocking current response)
                if should_cache {
                    let cache = self.cache.clone();
                    let server_info_clone = server_info.clone();
                    let result_clone = result.clone();

                    tokio::spawn(async move {
                        if let Err(e) = cache
                            .store_server_data(&create_cache_data(result_clone, &server_info_clone))
                            .await
                        {
                            tracing::warn!("Failed to update cache: {}", e);
                        }
                    });
                }

                return Ok(result);
            }
        }

        // No connection instances, create temporary instance (API scenario)
        self.create_temporary_instance(server_info, capability_type).await
    }

    /// Runtime query (force refresh scenario)
    async fn query_runtime(
        &self,
        server_info: &ServerIdentification,
        capability_type: CapabilityType,
        _params: &InspectParams,
        _start_time: Instant,
    ) -> Result<CapabilityResult, CapabilityError> {
        // Force refresh scenario: directly create temporary instance
        self.create_temporary_instance(server_info, capability_type).await
    }

    /// Create temporary instance (API scenario)
    async fn create_temporary_instance(
        &self,
        _server_info: &ServerIdentification,
        _capability_type: CapabilityType,
    ) -> Result<CapabilityResult, CapabilityError> {
        // Reuse existing temporary instance creation logic
        // Need to call create_temporary_instance_for_capability in src/api/handlers/server/capability.rs

        // Temporary implementation: return empty result, integrate specific logic later
        let metadata = ResponseMetadata {
            cache_hit: false,
            source: DataSource::Temporary,
            duration_ms: 0,
            item_count: 0,
            timestamp: chrono::Utc::now(),
        };

        Ok(CapabilityResult {
            items: Vec::new(),
            metadata,
        })
    }

    /// Determine connection mode based on server information
    async fn determine_connection_mode(
        &self,
        server_info: &ServerIdentification,
    ) -> Result<ConnectionMode, CapabilityError> {
        // Get server configuration to determine server type
        let db = &self.database;
        let server_config = crate::config::server::get_server(&db.pool, &server_info.server_name)
            .await
            .map_err(|e| CapabilityError::InternalError(format!("Failed to get server config: {}", e)))?;

        let server_config = server_config
            .ok_or_else(|| CapabilityError::InternalError(format!("Server '{}' not found", server_info.server_name)))?;

        // Use default isolation mode based on server type as per refactoring guide
        let connection_mode = UnifiedConnectionManager::get_connection_mode(
            &server_config.server_type,
            None, // No client ID for API context
            None, // No session ID for API context
        );

        tracing::debug!(
            "Determined connection mode {:?} for server '{}' (type: {:?})",
            connection_mode.isolation_mode,
            server_info.server_name,
            server_config.server_type
        );

        Ok(connection_mode)
    }

    /// Convert capability items from cached data
    fn convert_cached_data_to_items(
        &self,
        data: &crate::core::cache::CachedServerData,
        capability_type: CapabilityType,
    ) -> Result<Vec<CapabilityItem>, CapabilityError> {
        let mut items = Vec::new();

        match capability_type {
            CapabilityType::Tools => {
                for tool in &data.tools {
                    items.push(CapabilityItem::Tool(Adapter::convert_tool_to_domain(tool)));
                }
            }
            CapabilityType::Resources => {
                for resource in &data.resources {
                    items.push(CapabilityItem::Resource(Adapter::convert_resource_to_domain(resource)));
                }
            }
            CapabilityType::Prompts => {
                for prompt in &data.prompts {
                    items.push(CapabilityItem::Prompt(Adapter::convert_prompt_to_domain(prompt)));
                }
            }
            CapabilityType::ResourceTemplates => {
                for template in &data.resource_templates {
                    items.push(CapabilityItem::ResourceTemplate(Adapter::convert_template_to_domain(
                        template,
                    )));
                }
            }
        }

        Ok(items)
    }

    /// Extract capability information from instances
    fn extract_capabilities_from_instances(
        &self,
        instances: &[&crate::core::connection::upstream::UpstreamConnection],
        capability_type: CapabilityType,
    ) -> Result<(Vec<CapabilityItem>, bool), CapabilityError> {
        let mut items = Vec::new();
        let mut should_cache = true;

        for instance in instances {
            match capability_type {
                CapabilityType::Tools => {
                    for tool in &instance.tools {
                        items.push(CapabilityItem::Tool(super::domain::ToolCapability {
                            name: tool.name.to_string(),
                            description: tool.description.as_ref().map(|d| d.to_string()),
                            input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
                            unique_name: format!("{}:{}", instance.server_name, tool.name),
                            enabled: true, // Runtime instances are available by default
                        }));
                    }
                }
                CapabilityType::Resources => {
                    // UpstreamConnection doesn't store resources directly, only capability support info
                    // We need to fetch resources through the service if available
                    if instance.supports_resources() {
                        // For now, return empty and mark as should not cache since we can't extract directly
                        should_cache = false;
                        tracing::debug!(
                            "Server '{}' supports resources but needs service call to fetch them",
                            instance.server_name
                        );
                    } else {
                        tracing::debug!("Server '{}' does not support resources", instance.server_name);
                    }
                }
                CapabilityType::Prompts => {
                    // UpstreamConnection doesn't store prompts directly, only capability support info
                    // We need to fetch prompts through the service if available
                    if instance.supports_prompts() {
                        // For now, return empty and mark as should not cache since we can't extract directly
                        should_cache = false;
                        tracing::debug!(
                            "Server '{}' supports prompts but needs service call to fetch them",
                            instance.server_name
                        );
                    } else {
                        tracing::debug!("Server '{}' does not support prompts", instance.server_name);
                    }
                }
                CapabilityType::ResourceTemplates => {
                    // UpstreamConnection doesn't have direct resource_templates field
                    // Resource templates are part of ServerCapabilities but need service call to fetch
                    should_cache = false;
                    tracing::debug!(
                        "Server '{}' resource templates need service call to fetch",
                        instance.server_name
                    );
                }
            }
        }

        Ok((items, should_cache))
    }
}

/// Helper function to create cache data
fn create_cache_data(
    result: CapabilityResult,
    server_info: &ServerIdentification,
) -> crate::core::cache::CachedServerData {
    let now = Utc::now();

    // Convert capability items back to cache format
    let mut tools = Vec::new();
    let mut resources = Vec::new();
    let mut prompts = Vec::new();
    let mut resource_templates = Vec::new();

    for item in result.items {
        match item {
            CapabilityItem::Tool(tool) => {
                tools.push(crate::core::cache::CachedToolInfo {
                    name: tool.name,
                    description: tool.description,
                    input_schema_json: serde_json::to_string(&tool.input_schema).unwrap_or_else(|_| "{}".to_string()),
                    unique_name: Some(tool.unique_name),
                    enabled: tool.enabled,
                    cached_at: now,
                });
            }
            CapabilityItem::Resource(resource) => {
                resources.push(crate::core::cache::CachedResourceInfo {
                    uri: resource.uri,
                    name: resource.name,
                    description: resource.description,
                    mime_type: resource.mime_type,
                    enabled: resource.enabled,
                    cached_at: now,
                });
            }
            CapabilityItem::Prompt(prompt) => {
                prompts.push(crate::core::cache::CachedPromptInfo {
                    name: prompt.name,
                    description: prompt.description,
                    arguments: prompt
                        .arguments
                        .unwrap_or_default()
                        .iter()
                        .map(|arg| crate::core::cache::PromptArgument {
                            name: arg.name.clone(),
                            description: arg.description.clone(),
                            required: arg.required.unwrap_or(false),
                        })
                        .collect(),
                    enabled: prompt.enabled,
                    cached_at: now,
                });
            }
            CapabilityItem::ResourceTemplate(template) => {
                resource_templates.push(crate::core::cache::CachedResourceTemplateInfo {
                    uri_template: template.uri_template,
                    name: template.name,
                    description: template.description,
                    mime_type: template.mime_type,
                    enabled: template.enabled,
                    cached_at: now,
                });
            }
        }
    }

    crate::core::cache::CachedServerData {
        server_id: server_info.server_id.clone(),
        server_name: server_info.server_name.clone(),
        server_version: None,                // Not available in ServerIdentification
        protocol_version: "1.0".to_string(), // Default protocol version
        tools,
        resources,
        prompts,
        resource_templates,
        cached_at: now,
        fingerprint: String::new(), // Should actually get existing fingerprint
    }
}

/// Utility function to convert parameters
// TODO: Implement parameter conversion utility for API request transformation
#[allow(dead_code)]
fn convert_params(request: &crate::api::models::server::ServerCapabilityReq) -> InspectQuery {
    InspectQuery {
        refresh: request.refresh.as_ref().map(|r| match r {
            crate::api::models::server::ServerRefreshStrategy::Auto => {
                crate::api::handlers::server::common::RefreshStrategy::CacheFirst
            }
            crate::api::models::server::ServerRefreshStrategy::Force => {
                crate::api::handlers::server::common::RefreshStrategy::Force
            }
            crate::api::models::server::ServerRefreshStrategy::Cache => {
                crate::api::handlers::server::common::RefreshStrategy::CacheFirst
            }
        }),
        format: None,
        include_meta: None,
        timeout: None,
    }
}

/// Unified query service builder
pub struct UnifiedQueryServiceBuilder {
    cache: Option<Arc<RedbCacheManager>>,
    pool: Option<Arc<tokio::sync::Mutex<UpstreamConnectionPool>>>,
    database: Option<Arc<Database>>,
    app_state: Option<Arc<AppState>>,
    connection_manager: Option<Arc<UnifiedConnectionManager>>,
    timeout: Duration,
}

impl UnifiedQueryServiceBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            cache: None,
            pool: None,
            database: None,
            app_state: None,
            connection_manager: None,
            timeout: Duration::from_secs(DEFAULT_QUERY_TIMEOUT_SECS),
        }
    }

    /// Set cache manager
    pub fn with_cache(
        mut self,
        cache: Arc<RedbCacheManager>,
    ) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set connection pool
    pub fn with_pool(
        mut self,
        pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Set database
    pub fn with_database(
        mut self,
        database: Arc<Database>,
    ) -> Self {
        self.database = Some(database);
        self
    }

    /// Set app state
    pub fn with_app_state(
        mut self,
        app_state: Arc<AppState>,
    ) -> Self {
        self.app_state = Some(app_state);
        self
    }

    /// Set timeout duration
    pub fn with_timeout(
        mut self,
        timeout: Duration,
    ) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set connection manager
    pub fn with_connection_manager(
        mut self,
        connection_manager: Arc<UnifiedConnectionManager>,
    ) -> Self {
        self.connection_manager = Some(connection_manager);
        self
    }

    /// Build service
    pub fn build(self) -> Result<UnifiedQueryService, String> {
        Ok(UnifiedQueryService {
            cache: self.cache.ok_or("Cache manager is required")?,
            pool: self.pool.ok_or("Connection pool is required")?,
            database: self.database.ok_or("Database is required")?,
            app_state: self.app_state.ok_or("App state is required")?,
            connection_manager: self.connection_manager.ok_or("Connection manager is required")?,
            timeout_duration: self.timeout,
        })
    }
}

impl Default for UnifiedQueryServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics collection trait
pub trait MetricsCollector {
    fn record_capability_query_duration(
        &self,
        capability_type: CapabilityType,
        duration: std::time::Duration,
    );
    fn record_query_source(
        &self,
        source: DataSource,
    );
    fn record_query_result(
        &self,
        capability_type: CapabilityType,
        success: bool,
        item_count: usize,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_type_mapping() {
        assert_eq!(CapabilityType::Tools.as_str(), "tools");
        assert_eq!(CapabilityType::Resources.as_str(), "resources");
        assert_eq!(CapabilityType::Prompts.as_str(), "prompts");
        assert_eq!(CapabilityType::ResourceTemplates.as_str(), "resource_templates");
    }

    #[test]
    fn test_query_context_behavior() {
        assert!(!QueryContext::ApiCall.needs_persistent_instance());
        assert!(QueryContext::McpClient.needs_persistent_instance());
    }

    #[test]
    fn test_service_builder() {
        let builder = UnifiedQueryServiceBuilder::new().with_timeout(Duration::from_secs(60));
        let result = builder.build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_capability_token_mapping() {
        assert_eq!(
            CapabilityType::Tools.as_str(),
            crate::common::capability::CapabilityToken::Tools.to_string()
        );
    }

    #[test]
    fn test_unified_query_service_builder_validation() {
        let builder = UnifiedQueryServiceBuilder::new();
        let result = builder.build();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cache manager is required");
    }

    #[test]
    fn test_convert_params_function() {
        let request = crate::api::models::server::ServerCapabilityReq {
            refresh: Some(crate::api::models::server::ServerRefreshStrategy::Force),
            ..Default::default()
        };

        let inspect_query = convert_params(&request);
        assert_eq!(
            inspect_query.refresh,
            Some(crate::api::handlers::server::common::RefreshStrategy::Force)
        );
    }
}
