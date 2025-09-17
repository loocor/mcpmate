//! Unified query service - Orchestrate capability queries based on existing infrastructure

use std::sync::Arc;
use std::time::Instant;
use tokio::time::{Duration, timeout};

use crate::api::handlers::server::common::{
    InspectParams, InspectQuery, RefreshStrategy, ServerIdentification, get_server_info_for_inspect,
};

// Constants for unified query service
const DEFAULT_QUERY_TIMEOUT_SECS: u64 = 30;
// TODO: Implement cache update timeout functionality for background cache refresh operations
// const CACHE_UPDATE_TIMEOUT_SECS: u64 = 10; // Reserved for future cache update timeout feature
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
    /// Utility: Get enabled server IDs for pre-filtering in capability builders
    pub async fn get_enabled_server_ids(
        database: &Arc<crate::config::database::Database>
    ) -> Result<std::collections::HashSet<String>, anyhow::Error> {
        let query = r#"
            SELECT DISTINCT sc.id
            FROM server_config sc
            JOIN profile p ON p.is_active = true
            WHERE sc.enabled = 1
        "#;

        let server_ids = sqlx::query_as::<_, (String,)>(query)
            .fetch_all(&database.pool)
            .await?
            .into_iter()
            .map(|(id,)| id)
            .collect();

        Ok(server_ids)
    }
    /// Unified capability listing - moved from API layer to avoid duplication
    /// This function implements the complete REDB-first strategy
    ///
    /// Refactored to accept independent components instead of full AppState,
    /// enabling usage from both API and MCP protocol layers
    pub async fn list_capabilities_redb_first(
        capability_type: CapabilityType,
        server_info: &crate::api::handlers::server::common::ServerIdentification,
        params: &crate::api::handlers::server::common::InspectParams,
        _database: &Arc<crate::config::database::Database>,
        connection_pool: &Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
        redb_cache: &Arc<crate::core::cache::RedbCacheManager>,
        _query_context: QueryContext,
    ) -> Result<Vec<serde_json::Value>, anyhow::Error> {
        use chrono::Utc;

        // Step 1: Try REDB cache first
        let cache_query = crate::api::handlers::server::common::build_cache_query(&server_info.server_id, params);

        if let Ok(cache_result) = redb_cache.get_server_data(&cache_query).await {
            if cache_result.cache_hit {
                if let Some(data) = cache_result.data {
                    let cached_items = match capability_type {
                        CapabilityType::Tools => data
                            .tools
                            .into_iter()
                            .map(|t| crate::api::handlers::server::capability::tool_json_from_cached(&t))
                            .collect::<Vec<_>>(),
                        CapabilityType::Resources => data
                            .resources
                            .into_iter()
                            .map(crate::api::handlers::server::capability::resource_json_from_cached)
                            .collect::<Vec<_>>(),
                        CapabilityType::Prompts => data
                            .prompts
                            .into_iter()
                            .map(crate::api::handlers::server::capability::prompt_json_from_cached)
                            .collect::<Vec<_>>(),
                        CapabilityType::ResourceTemplates => data
                            .resource_templates
                            .into_iter()
                            .map(crate::api::handlers::server::capability::resource_template_json_from_cached)
                            .collect::<Vec<_>>(),
                    };

                    if !cached_items.is_empty() {
                        tracing::debug!(
                            "REDB cache hit for {} {}: {} items",
                            server_info.server_id,
                            capability_type.as_str(),
                            cached_items.len()
                        );
                        return Ok(cached_items);
                    }
                }
            }
        }

        // Step 2: Runtime fallback - read from connected instances without holding the pool lock
        if let Ok(pool_guard) = tokio::time::timeout(
            std::time::Duration::from_millis(crate::common::constants::timeouts::LOCK_MS),
            connection_pool.lock(),
        )
        .await
        {
            // Collect minimal data then drop the lock
            let (services, tools_snapshots) =
                if let Some(instances) = pool_guard.connections.get(&server_info.server_id) {
                    let services: Vec<_> = instances
                        .values()
                        .filter(|conn| conn.is_connected())
                        .filter_map(|conn| conn.service.clone())
                        .collect();

                    let tools_snapshots: Vec<Vec<rmcp::model::Tool>> = instances
                        .values()
                        .filter(|conn| conn.is_connected())
                        .map(|conn| conn.tools.clone())
                        .collect();

                    (services, tools_snapshots)
                } else {
                    (Vec::new(), Vec::new())
                };

            drop(pool_guard);

            let mut items = Vec::new();
            let now = Utc::now();

            match capability_type {
                CapabilityType::Tools => {
                    let mut cached_tools = Vec::new();
                    for tool_vec in tools_snapshots.into_iter() {
                        for tool in tool_vec.into_iter() {
                            let schema = tool.schema_as_json_value();
                            items.push(crate::api::handlers::server::capability::tool_json(
                                &tool.name,
                                tool.description.clone().map(|d| d.into_owned()),
                                schema.clone(),
                                None,
                                None,
                            ));

                            // Build cacheable tool info
                            let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
                            cached_tools.push(crate::core::cache::CachedToolInfo {
                                name: tool.name.to_string(),
                                description: tool.description.clone().map(|d| d.into_owned()),
                                input_schema_json,
                                unique_name: None,
                                enabled: true,
                                cached_at: now,
                            });
                        }
                    }

                    if !cached_tools.is_empty() {
                        // Persist into REDB cache
                        let server_data = crate::api::handlers::server::common::create_runtime_cache_data(
                            server_info,
                            cached_tools,
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                        );
                        let _ = redb_cache.store_server_data(&server_data).await;
                    }
                }
                CapabilityType::Resources => {
                    for service in services.iter() {
                        match service.list_all_resources().await {
                            Ok(resources) => {
                                let mut cached_resources = Vec::new();
                                for resource in resources {
                                    items.push(crate::api::handlers::server::capability::resource_json(
                                        &resource.uri,
                                        Some(resource.name.clone()),
                                        resource.description.clone(),
                                        resource.mime_type.clone(),
                                        None,
                                        None,
                                    ));

                                    cached_resources.push(crate::core::cache::CachedResourceInfo {
                                        uri: resource.uri.to_string(),
                                        name: Some(resource.name.clone()),
                                        description: resource.description.clone(),
                                        mime_type: resource.mime_type.clone(),
                                        enabled: true,
                                        cached_at: now,
                                    });
                                }

                                if !cached_resources.is_empty() {
                                    let server_data = crate::api::handlers::server::common::create_runtime_cache_data(
                                        server_info,
                                        Vec::new(),
                                        cached_resources,
                                        Vec::new(),
                                        Vec::new(),
                                    );
                                    let _ = redb_cache.store_server_data(&server_data).await;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to list resources from server {}: {}", server_info.server_id, e);
                            }
                        }
                    }
                }
                CapabilityType::Prompts => {
                    for service in services.iter() {
                        match service.list_all_prompts().await {
                            Ok(prompts) => {
                                let mut cached_prompts = Vec::new();
                                for prompt in prompts {
                                    // Convert rmcp::model::PromptArgument to cache::PromptArgument
                                    let cache_args: Vec<crate::core::cache::PromptArgument> = prompt
                                        .arguments
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|arg| crate::core::cache::PromptArgument {
                                            name: arg.name,
                                            description: arg.description,
                                            required: arg.required.unwrap_or(false),
                                        })
                                        .collect();

                                    items.push(crate::api::handlers::server::capability::prompt_json(
                                        &prompt.name,
                                        prompt.description.clone(),
                                        cache_args.clone(),
                                        None,
                                        None,
                                    ));

                                    cached_prompts.push(crate::core::cache::CachedPromptInfo {
                                        name: prompt.name.to_string(),
                                        description: prompt.description,
                                        arguments: cache_args,
                                        enabled: true,
                                        cached_at: now,
                                    });
                                }

                                if !cached_prompts.is_empty() {
                                    let server_data = crate::api::handlers::server::common::create_runtime_cache_data(
                                        server_info,
                                        Vec::new(),
                                        Vec::new(),
                                        cached_prompts,
                                        Vec::new(),
                                    );
                                    let _ = redb_cache.store_server_data(&server_data).await;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to list prompts from server {}: {}", server_info.server_id, e);
                            }
                        }
                    }
                }
                CapabilityType::ResourceTemplates => {
                    // No-op here for now
                }
            }

            if !items.is_empty() {
                return Ok(items);
            }
        }

        // Step 3: Instance creation is handled by the caller layer
        // This allows API layer to handle temporary instances and MCP layer to handle standard instances
        tracing::debug!(
            "No {} found for server {} in cache or runtime instances - instance creation deferred to caller",
            capability_type.as_str(),
            server_info.server_id
        );

        Ok(Vec::new())
    }

    /// Get enabled tools from database (proxy compatibility adapter)
    /// TODO: Refactor to use unified entry point once AppState dependency is resolved
    pub async fn list_enabled_tools(
        database: &Arc<crate::config::database::Database>,
        pool: &Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
    ) -> Result<Vec<rmcp::model::Tool>, anyhow::Error> {
        use crate::config::profile::tool::build_enabled_tools_query;
        use anyhow::Context;

        let query = format!("{} ORDER BY st.unique_name", build_enabled_tools_query(None));
        let enabled_tools = sqlx::query_as::<_, (String, String, String, String)>(&query)
            .fetch_all(&database.pool)
            .await
            .context("Failed to query enabled tools from database")?;

        let mut all_tools = Vec::new();
        let pool = pool.lock().await;

        for (unique_name, _server_name, tool_name, server_id) in enabled_tools {
            if let Some(instances) = pool.connections.get(&server_id) {
                for conn in instances.values() {
                    if conn.is_disabled() || !conn.is_connected() {
                        continue;
                    }
                    if let Some(tool) = conn.tools.iter().find(|t| t.name == *tool_name) {
                        let mut unique_tool = tool.clone();
                        unique_tool.name = unique_name.clone().into();
                        all_tools.push(unique_tool);
                        break;
                    }
                }
            }
        }

        Ok(all_tools)
    }

    /// Call tool through unified system (proxy compatibility adapter)
    pub async fn call_tool_unified(
        database: &Arc<crate::config::database::Database>,
        pool: &Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
        request: rmcp::model::CallToolRequestParam,
    ) -> Result<rmcp::model::CallToolResult, anyhow::Error> {
        use crate::config::profile::tool::build_enabled_tools_query;
        use anyhow::Context;

        // Resolve tool using database mapping
        let query = format!("{} AND st.unique_name = ? LIMIT 1", build_enabled_tools_query(None));
        let result = sqlx::query_as::<_, (String, String, String, String)>(&query)
            .bind(&request.name)
            .fetch_optional(&database.pool)
            .await
            .context("Failed to query tool mapping from database")?;

        let (_unique_name, _server_name, original_tool_name, server_id) =
            result.ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", request.name))?;

        // CRITICAL FIX: Get service reference without holding pool lock during network call
        let service = {
            let pool = pool.lock().await;
            let mut found_service = None;

            if let Some(instances) = pool.connections.get(&server_id) {
                for conn in instances.values() {
                    if conn.is_connected() && !conn.is_disabled() {
                        if let Some(service) = &conn.service {
                            found_service = Some(service.clone());
                            break;
                        }
                    }
                }
            }
            // Pool lock is automatically dropped here
            found_service
        };

        // Now make the network call WITHOUT holding the pool lock (with timeout protection)
        if let Some(service) = service {
            let mut upstream_request = request.clone();
            upstream_request.name = original_tool_name.into();
            match tokio::time::timeout(std::time::Duration::from_secs(30), service.call_tool(upstream_request)).await {
                Ok(Ok(res)) => Ok(res),
                Ok(Err(e)) => Err(anyhow::anyhow!("Tool call failed: {}", e)),
                Err(_) => Err(anyhow::anyhow!("Tool call timeout for server {}", server_id)),
            }
        } else {
            Err(anyhow::anyhow!("No connected instance for server {}", server_id))
        }
    }

    /// Get enabled resources from database (proxy compatibility adapter)
    ///
    /// ARCHITECTURAL ISSUE: This function cannot work properly without REDB cache access.
    /// The unified entry point requires REDB cache for REDB-first strategy.
    ///
    /// SOLUTION: ProxyServer needs to be updated to include REDB cache reference.
    /// Until then, this function will fail explicitly rather than hide the problem.
    pub async fn list_enabled_resources(
        database: &Arc<crate::config::database::Database>,
        pool: &Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
        redb_cache: Option<&Arc<crate::core::cache::RedbCacheManager>>,
    ) -> Result<Vec<rmcp::model::Resource>, anyhow::Error> {
        use anyhow::Context;

        // Fail fast if REDB cache is not available
        let redb_cache = redb_cache.ok_or_else(|| {
            anyhow::anyhow!(
                "CRITICAL ARCHITECTURE ISSUE: MCP protocol layer cannot access REDB cache. \
                ProxyServer needs REDB cache reference to use unified entry point. \
                This is not a runtime error - it's a missing dependency in the architecture."
            )
        })?;

        // Query enabled servers from database
        let enabled_servers = sqlx::query_as::<_, (String, String)>(
            "SELECT id, name FROM mcp_server WHERE enabled = 1 AND globally_enabled = 1",
        )
        .fetch_all(&database.pool)
        .await
        .context("Failed to query enabled servers from database")?;

        let mut all_resources = Vec::new();

        // Use unified entry point with REDB-first strategy for all enabled servers
        for (server_id, server_name) in enabled_servers {
            let server_info = crate::api::handlers::server::common::ServerIdentification {
                server_id: server_id.clone(),
                server_name: server_name.clone(),
            };
            let params = crate::api::handlers::server::common::InspectParams {
                refresh: Some(crate::api::handlers::server::common::RefreshStrategy::CacheFirst),
                format: None,
                include_meta: Some(false),
                timeout: Some(10),
            };

            match Self::list_capabilities_redb_first(
                crate::core::capability::domain::CapabilityType::Resources,
                &server_info,
                &params,
                database,
                pool,
                redb_cache,
                crate::core::capability::domain::QueryContext::McpClient,
            )
            .await
            {
                Ok(json_resources) => {
                    // Convert JSON responses back to rmcp::model::Resource
                    for json_resource in json_resources {
                        if let Ok(resource) = serde_json::from_value::<rmcp::model::Resource>(json_resource) {
                            all_resources.push(resource);
                        }
                    }
                    tracing::info!(
                        "Unified entry returned {} resources for server {}",
                        all_resources.len(),
                        server_id
                    );
                }
                Err(e) => {
                    tracing::error!("Unified entry failed for server {}: {}", server_id, e);
                    // Don't hide the problem with fallbacks - let it bubble up
                    return Err(e);
                }
            }
        }

        Ok(all_resources)
    }

    /// Get enabled prompts from database (proxy compatibility adapter)
    ///
    /// ARCHITECTURAL ISSUE: Same as list_enabled_resources - requires REDB cache access.
    pub async fn list_enabled_prompts(
        database: &Arc<crate::config::database::Database>,
        pool: &Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
        redb_cache: Option<&Arc<crate::core::cache::RedbCacheManager>>,
    ) -> Result<Vec<rmcp::model::Prompt>, anyhow::Error> {
        use anyhow::Context;

        // Fail fast if REDB cache is not available
        let redb_cache = redb_cache.ok_or_else(|| {
            anyhow::anyhow!(
                "CRITICAL ARCHITECTURE ISSUE: MCP protocol layer cannot access REDB cache. \
                ProxyServer needs REDB cache reference to use unified entry point."
            )
        })?;

        // Query enabled servers from database
        let enabled_servers = sqlx::query_as::<_, (String, String)>(
            "SELECT id, name FROM mcp_server WHERE enabled = 1 AND globally_enabled = 1",
        )
        .fetch_all(&database.pool)
        .await
        .context("Failed to query enabled servers from database")?;

        let mut all_prompts = Vec::new();

        // Use unified entry point with REDB-first strategy for all enabled servers
        for (server_id, server_name) in enabled_servers {
            let server_info = crate::api::handlers::server::common::ServerIdentification {
                server_id: server_id.clone(),
                server_name: server_name.clone(),
            };
            let params = crate::api::handlers::server::common::InspectParams {
                refresh: Some(crate::api::handlers::server::common::RefreshStrategy::CacheFirst),
                format: None,
                include_meta: Some(false),
                timeout: Some(10),
            };

            match Self::list_capabilities_redb_first(
                crate::core::capability::domain::CapabilityType::Prompts,
                &server_info,
                &params,
                database,
                pool,
                redb_cache,
                crate::core::capability::domain::QueryContext::McpClient,
            )
            .await
            {
                Ok(json_prompts) => {
                    // Convert JSON responses back to rmcp::model::Prompt
                    for json_prompt in json_prompts {
                        if let Ok(prompt) = serde_json::from_value::<rmcp::model::Prompt>(json_prompt) {
                            all_prompts.push(prompt);
                        }
                    }
                    tracing::info!(
                        "Unified entry returned {} prompts for server {}",
                        all_prompts.len(),
                        server_id
                    );
                }
                Err(e) => {
                    tracing::error!("Unified entry failed for server {}: {}", server_id, e);
                    // Don't hide the problem with fallbacks - let it bubble up
                    return Err(e);
                }
            }
        }

        Ok(all_prompts)
    }

    /// Merge a single capability segment into REDB cached server data and store
    async fn merge_and_store_redb_segment(
        &self,
        server_id: &str,
        server_name: &str,
        cap_type: CapabilityType,
        items: &[CapabilityItem],
    ) -> Result<(), String> {
        use crate::core::cache::{
            CacheQuery, CachedPromptInfo, CachedResourceInfo, CachedResourceTemplateInfo, CachedServerData,
            CachedToolInfo, FreshnessLevel,
        };

        // 1) Read existing cached data (if any)
        let query = CacheQuery {
            server_id: server_id.to_string(),
            freshness_level: FreshnessLevel::Cached,
            include_disabled: false,
        };
        let existing = self.cache.get_server_data(&query).await.map_err(|e| e.to_string())?;

        let mut data = existing.data.unwrap_or(CachedServerData {
            server_id: server_id.to_string(),
            server_name: server_name.to_string(),
            server_version: None,
            protocol_version: "latest".to_string(),
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            resource_templates: Vec::new(),
            cached_at: chrono::Utc::now(),
            fingerprint: format!("merge:{}:{}", server_id, chrono::Utc::now().timestamp()),
        });

        // 2) Replace the targeted segment only
        match cap_type {
            CapabilityType::Tools => {
                data.tools = items
                    .iter()
                    .filter_map(|it| match it {
                        CapabilityItem::Tool(t) => Some(CachedToolInfo {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            input_schema_json: serde_json::to_string(&t.input_schema)
                                .unwrap_or_else(|_| "{}".to_string()),
                            unique_name: Some(t.unique_name.clone()),
                            enabled: t.enabled,
                            cached_at: chrono::Utc::now(),
                        }),
                        _ => None,
                    })
                    .collect();
            }
            CapabilityType::Resources => {
                data.resources = items
                    .iter()
                    .filter_map(|it| match it {
                        CapabilityItem::Resource(r) => Some(CachedResourceInfo {
                            uri: r.uri.clone(),
                            name: r.name.clone(),
                            description: r.description.clone(),
                            mime_type: r.mime_type.clone(),
                            enabled: r.enabled,
                            cached_at: chrono::Utc::now(),
                        }),
                        _ => None,
                    })
                    .collect();
            }
            CapabilityType::Prompts => {
                data.prompts = items
                    .iter()
                    .filter_map(|it| match it {
                        CapabilityItem::Prompt(p) => Some(CachedPromptInfo {
                            name: p.name.clone(),
                            description: p.description.clone(),
                            arguments: p
                                .arguments
                                .clone()
                                .unwrap_or_default()
                                .into_iter()
                                .map(|a| crate::core::cache::PromptArgument {
                                    name: a.name,
                                    description: a.description,
                                    required: a.required.unwrap_or(false),
                                })
                                .collect(),
                            enabled: p.enabled,
                            cached_at: chrono::Utc::now(),
                        }),
                        _ => None,
                    })
                    .collect();
            }
            CapabilityType::ResourceTemplates => {
                data.resource_templates = items
                    .iter()
                    .filter_map(|it| match it {
                        CapabilityItem::ResourceTemplate(t) => Some(CachedResourceTemplateInfo {
                            uri_template: t.uri_template.clone(),
                            name: t.name.clone(),
                            description: t.description.clone(),
                            mime_type: t.mime_type.clone(),
                            enabled: t.enabled,
                            cached_at: chrono::Utc::now(),
                        }),
                        _ => None,
                    })
                    .collect();
            }
        }

        // 3) Store back
        self.cache.store_server_data(&data).await.map_err(|e| e.to_string())
    }
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
        let ensured_instance_id = self
            .connection_manager
            .ensure_affinitized_connection(&server_info.server_id, connection_mode)
            .await?;

        // Build from light snapshot to minimize cloning; then optionally clone tools for Tools path
        let (instance_id, service_peer_opt, tools_snapshot_opt) = {
            // Take light snapshot (no network I/O, brief lock)
            let snapshot = {
                let pool = self.pool.lock().await;
                pool.get_snapshot()
            };

            let mut chosen_instance: Option<String> = None;
            let mut chosen_peer: Option<rmcp::service::Peer<rmcp::service::RoleClient>> = None;

            if let Some(instances) = snapshot.get(&server_info.server_id) {
                // Prefer ensured instance id if it is connected and has a peer
                if let Some((_, _status, _res, _prm, peer)) = instances
                    .iter()
                    .find(|(iid, status, _, _, peer)| iid == &ensured_instance_id && matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) && peer.is_some())
                {
                    chosen_instance = Some(ensured_instance_id.clone());
                    chosen_peer = peer.clone();
                } else if let Some((iid, _status, _res, _prm, peer)) = instances
                    .iter()
                    .find(|(_, status, _, _, peer)| matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) && peer.is_some())
                {
                    chosen_instance = Some(iid.clone());
                    chosen_peer = peer.clone();
                }
            }

            // For Tools path, fetch tool snapshot from the chosen instance only (brief lock)
            let tools_snapshot_opt = if matches!(capability_type, CapabilityType::Tools) {
                if let Some(ref iid) = chosen_instance {
                    let pool = self.pool.lock().await;
                    if let Some(instances) = pool.connections.get(&server_info.server_id) {
                        instances.get(iid).map(|conn| conn.tools.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            (
                chosen_instance.unwrap_or_else(|| ensured_instance_id.clone()),
                chosen_peer,
                tools_snapshot_opt,
            )
        };

        // If we have a service peer, sync DB via pool unified method and refresh REDB segment
        if let Some(service_peer) = service_peer_opt {
            let flags = match capability_type {
                CapabilityType::Tools => crate::core::pool::CapSyncFlags::TOOLS,
                CapabilityType::Resources => crate::core::pool::CapSyncFlags::RESOURCES,
                CapabilityType::Prompts => crate::core::pool::CapSyncFlags::PROMPTS,
                CapabilityType::ResourceTemplates => crate::core::pool::CapSyncFlags::RESOURCE_TEMPLATES,
            };

            // Perform DB sync (tools pass snapshot when available)
            {
                let db = &self.database;
                let tools_slice = tools_snapshot_opt.as_deref();
                if let Err(e) = crate::core::pool::UpstreamConnectionPool::sync_capabilities(
                    db,
                    &server_info.server_id,
                    &instance_id,
                    &service_peer,
                    flags,
                    tools_slice,
                )
                .await
                {
                    tracing::warn!(
                        "DB sync for capabilities failed (server={}): {}",
                        server_info.server_id,
                        e
                    );
                }
            }

            // Build items from runtime and update REDB segment (partial merge)
            let items = match capability_type {
                CapabilityType::Tools => {
                    // Reuse existing extraction on instance tools when available
                    let pool = self.pool.lock().await;
                    if let Some(instances) = pool.connections.get(&server_info.server_id) {
                        let connected_instances: Vec<_> = instances.values().filter(|c| c.is_connected()).collect();
                        let (items, _cache) =
                            self.extract_capabilities_from_instances(&connected_instances, CapabilityType::Tools)?;
                        items
                    } else {
                        Vec::new()
                    }
                }
                CapabilityType::Resources => match tokio::time::timeout(self.timeout_duration, service_peer.list_all_resources()).await {
                    Err(_) => {
                        tracing::warn!("Fetch resources timeout for server {}", server_info.server_id);
                        Vec::new()
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("Fetch resources failed: {}", e);
                        Vec::new()
                    }
                    Ok(Ok(resources)) => resources
                        .into_iter()
                        .map(|r| {
                            CapabilityItem::Resource(super::domain::ResourceCapability {
                                uri: r.uri.clone(),
                                name: None,
                                description: None,
                                mime_type: None,
                                unique_uri: String::new(),
                                enabled: true,
                            })
                        })
                        .collect(),
                },
                CapabilityType::Prompts => {
                    // Paginated fetch
                    let mut items = Vec::new();
                    let mut cursor = None;
                    loop {
                        match tokio::time::timeout(
                            self.timeout_duration,
                            service_peer.list_prompts(Some(rmcp::model::PaginatedRequestParam { cursor })),
                        )
                        .await
                        {
                            Err(_) => {
                                tracing::warn!("Fetch prompts timeout for server {}", server_info.server_id);
                                break;
                            }
                            Ok(Ok(result)) => {
                                for p in result.prompts {
                                    let args = p
                                        .arguments
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|a| super::domain::PromptArgument {
                                            name: a.name,
                                            description: a.description,
                                            required: a.required,
                                        })
                                        .collect();
                                    items.push(CapabilityItem::Prompt(super::domain::PromptCapability {
                                        name: p.name,
                                        description: p.description,
                                        arguments: Some(args),
                                        unique_name: String::new(),
                                        enabled: true,
                                    }));
                                }
                                cursor = result.next_cursor;
                                if cursor.is_none() {
                                    break;
                                }
                            }
                            Ok(Err(e)) => {
                                tracing::warn!("Fetch prompts failed: {}", e);
                                break;
                            }
                        }
                    }
                    items
                }
                CapabilityType::ResourceTemplates => {
                    // Not implemented yet
                    Vec::new()
                }
            };

            // REDB partial merge store
            if let Err(e) = self
                .merge_and_store_redb_segment(
                    &server_info.server_id,
                    &server_info.server_name,
                    capability_type,
                    &items,
                )
                .await
            {
                tracing::warn!("REDB merge store failed: {}", e);
            }

            let metadata = ResponseMetadata {
                cache_hit: false,
                source: DataSource::Runtime,
                duration_ms: start_time.elapsed().as_millis() as u64,
                item_count: items.len(),
                timestamp: chrono::Utc::now(),
            };
            return Ok(CapabilityResult { items, metadata });
        }

        // Fallback: create temporary instance (API scenario)
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
        instances: &[&crate::core::pool::UpstreamConnection],
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
