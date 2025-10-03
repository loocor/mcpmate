pub(crate) const DEFAULT_QUERY_TIMEOUT_SECS: u64 = 120;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::api::handlers::server::common::{InspectParams, RefreshStrategy};
use crate::api::routes::AppState;
use crate::common::capability::CapabilityToken;
use crate::config::database::Database;
use crate::config::models::Server;
use crate::config::server;
use crate::core::cache::RedbCacheManager;
use crate::core::capability::domain::{
    CapabilityError, CapabilityItem, CapabilityResult, CapabilityType, DataSource,
    PromptArgument as DomainPromptArgument, PromptCapability, QueryContext, ResourceCapability,
    ResourceTemplateCapability, ResponseMetadata, ToolCapability,
};
use crate::core::capability::facade;
use crate::core::capability::runtime::{
    CapabilityItems, ListCtx, ListResult, Meta, RefreshStrategy as RuntimeRefreshStrategy,
};
use crate::core::capability::service::{CAPABILITY_VALIDATION_SESSION, CapabilityService};
use crate::core::pool::UpstreamConnectionPool;

/// Performance metrics collection trait used by the unified query service.
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

/// Shared state for unified capability queries
pub struct UnifiedQueryService {
    cache: Arc<RedbCacheManager>,
    pool: Arc<Mutex<UpstreamConnectionPool>>,
    database: Arc<Database>,
    app_state: Arc<AppState>,
    timeout_duration: Duration,
}

impl std::fmt::Debug for UnifiedQueryService {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("UnifiedQueryService")
            .field("cache_strong_refs", &Arc::strong_count(&self.cache))
            .field("pool_strong_refs", &Arc::strong_count(&self.pool))
            .field("database_strong_refs", &Arc::strong_count(&self.database))
            .field("app_state_refs", &Arc::strong_count(&self.app_state))
            .field("timeout_duration", &self.timeout_duration)
            .finish()
    }
}

pub struct UnifiedQueryServiceBuilder {
    cache: Option<Arc<RedbCacheManager>>,
    pool: Option<Arc<Mutex<UpstreamConnectionPool>>>,
    database: Option<Arc<Database>>,
    app_state: Option<Arc<AppState>>,
    timeout: Duration,
}

impl UnifiedQueryServiceBuilder {
    pub fn new() -> Self {
        Self {
            cache: None,
            pool: None,
            database: None,
            app_state: None,
            timeout: Duration::from_secs(DEFAULT_QUERY_TIMEOUT_SECS),
        }
    }

    pub fn with_cache(
        mut self,
        cache: Arc<RedbCacheManager>,
    ) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn with_pool(
        mut self,
        pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        self.pool = Some(pool);
        self
    }

    pub fn with_database(
        mut self,
        database: Arc<Database>,
    ) -> Self {
        self.database = Some(database);
        self
    }

    pub fn with_app_state(
        mut self,
        app_state: Arc<AppState>,
    ) -> Self {
        self.app_state = Some(app_state);
        self
    }

    pub fn with_timeout(
        mut self,
        timeout: Duration,
    ) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> Result<UnifiedQueryService, String> {
        Ok(UnifiedQueryService {
            cache: self.cache.ok_or("Cache manager is required")?,
            pool: self.pool.ok_or("Connection pool is required")?,
            database: self.database.ok_or("Database is required")?,
            app_state: self.app_state.ok_or("App state is required")?,
            timeout_duration: self.timeout,
        })
    }
}

impl Default for UnifiedQueryServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedQueryService {
    /// Unified entry for capability listing with cache-first strategy
    pub async fn query_capabilities(
        &self,
        server_id: &str,
        capability_type: CapabilityType,
        params: &InspectParams,
        context: QueryContext,
    ) -> Result<CapabilityResult, CapabilityError> {
        let server = self.load_server(server_id).await?;
        self.ensure_capability_enabled(&server, server_id, capability_type)?;

        let list_ctx = self.build_list_ctx(server_id, capability_type, params, context);
        let capability_service = CapabilityService::new(self.cache.clone(), self.pool.clone(), self.database.clone());

        let list_result = capability_service
            .list(&list_ctx)
            .await
            .map_err(|err| CapabilityError::RuntimeError(err.to_string()))?;

        Ok(list_to_capability_result(list_result, capability_type))
    }

    async fn load_server(
        &self,
        server_id: &str,
    ) -> Result<Server, CapabilityError> {
        server::get_server_by_id(&self.database.pool, server_id)
            .await
            .map_err(|err| CapabilityError::InternalError(err.to_string()))?
            .ok_or_else(|| CapabilityError::InternalError(format!("Server {} not found", server_id)))
    }

    fn ensure_capability_enabled(
        &self,
        server: &Server,
        server_id: &str,
        capability_type: CapabilityType,
    ) -> Result<(), CapabilityError> {
        if !server.enabled.as_bool() {
            return Err(CapabilityError::ServerDisabled {
                server_id: server_id.to_string(),
            });
        }

        let token = capability_capability_token(capability_type);
        if !facade::capability_declared(server.capabilities.as_deref(), token.as_str()) {
            return Err(CapabilityError::CapabilityDisabled {
                capability_type,
                server_id: server_id.to_string(),
            });
        }

        Ok(())
    }

    fn build_list_ctx(
        &self,
        server_id: &str,
        capability_type: CapabilityType,
        params: &InspectParams,
        context: QueryContext,
    ) -> ListCtx {
        let timeout = params.timeout.map(Duration::from_secs).unwrap_or(self.timeout_duration);

        ListCtx {
            capability: capability_type,
            server_id: server_id.to_string(),
            refresh: Some(map_refresh_strategy(params.refresh)),
            timeout: Some(timeout),
            validation_session: validation_session(context),
        }
    }
}

fn map_refresh_strategy(refresh: Option<RefreshStrategy>) -> RuntimeRefreshStrategy {
    match refresh.unwrap_or(RefreshStrategy::CacheFirst) {
        RefreshStrategy::Force => RuntimeRefreshStrategy::Force,
        _ => RuntimeRefreshStrategy::CacheFirst,
    }
}

fn validation_session(context: QueryContext) -> Option<String> {
    match context {
        QueryContext::ApiCall => Some(CAPABILITY_VALIDATION_SESSION.to_string()),
        QueryContext::McpClient => None,
    }
}

fn capability_capability_token(capability_type: CapabilityType) -> CapabilityToken {
    match capability_type {
        CapabilityType::Tools => CapabilityToken::Tools,
        CapabilityType::Prompts => CapabilityToken::Prompts,
        CapabilityType::Resources => CapabilityToken::Resources,
        CapabilityType::ResourceTemplates => CapabilityToken::Resources,
    }
}

/// Convert runtime list result into domain capability result
pub fn list_to_capability_result(
    list_result: ListResult,
    capability_type: CapabilityType,
) -> CapabilityResult {
    let items = match list_result.items {
        CapabilityItems::Tools(tools) => tools.into_iter().map(tool_to_capability).collect::<Vec<_>>(),
        CapabilityItems::Resources(resources) => resources.into_iter().map(resource_to_capability).collect::<Vec<_>>(),
        CapabilityItems::Prompts(prompts) => prompts.into_iter().map(prompt_to_capability).collect::<Vec<_>>(),
        CapabilityItems::ResourceTemplates(templates) => {
            templates.into_iter().map(template_to_capability).collect::<Vec<_>>()
        }
    };

    let metadata = build_metadata(&list_result.meta, items.len(), capability_type);

    CapabilityResult { items, metadata }
}

fn build_metadata(
    meta: &Meta,
    item_count: usize,
    capability_type: CapabilityType,
) -> ResponseMetadata {
    ResponseMetadata {
        cache_hit: meta.cache_hit,
        source: to_data_source(meta, capability_type),
        duration_ms: meta.duration_ms,
        item_count,
        timestamp: Utc::now(),
    }
}

fn to_data_source(
    meta: &Meta,
    capability_type: CapabilityType,
) -> DataSource {
    match meta.source.as_str() {
        "cache" => DataSource::CacheL2,
        "runtime" => match capability_type {
            CapabilityType::Tools | CapabilityType::Prompts | CapabilityType::Resources => DataSource::Runtime,
            CapabilityType::ResourceTemplates => {
                if meta.had_peer {
                    DataSource::Runtime
                } else {
                    DataSource::None
                }
            }
        },
        "temporary" => DataSource::Temporary,
        other => {
            tracing::debug!(source = other, "Unknown capability data source");
            if meta.cache_hit {
                DataSource::CacheL2
            } else if meta.had_peer {
                DataSource::Runtime
            } else {
                DataSource::None
            }
        }
    }
}

fn tool_to_capability(tool: rmcp::model::Tool) -> CapabilityItem {
    let name = tool.name.to_string();
    let schema = Value::Object((*tool.input_schema).clone());
    CapabilityItem::Tool(ToolCapability {
        name: name.clone(),
        description: tool.description.map(|d| d.into_owned()),
        input_schema: schema,
        unique_name: name,
        enabled: true,
        icons: tool.icons,
    })
}

fn resource_to_capability(resource: rmcp::model::Resource) -> CapabilityItem {
    let rmcp::model::Annotated { raw, .. } = resource;
    let unique_uri = raw.uri.clone();
    CapabilityItem::Resource(ResourceCapability {
        uri: raw.uri,
        name: Some(raw.name),
        description: raw.description,
        mime_type: raw.mime_type,
        unique_uri,
        enabled: true,
        icons: raw.icons,
    })
}

fn prompt_to_capability(prompt: rmcp::model::Prompt) -> CapabilityItem {
    let rmcp::model::Prompt {
        name,
        description,
        arguments,
        ..
    } = prompt;

    let unique_name = name.clone();
    CapabilityItem::Prompt(PromptCapability {
        name,
        description,
        arguments: arguments.map(|args| {
            args.into_iter()
                .map(|arg| DomainPromptArgument {
                    name: arg.name,
                    description: arg.description,
                    required: arg.required,
                })
                .collect()
        }),
        unique_name,
        enabled: true,
        icons: prompt.icons,
    })
}

fn template_to_capability(template: rmcp::model::ResourceTemplate) -> CapabilityItem {
    let rmcp::model::Annotated { raw, .. } = template;
    let unique_template = raw.uri_template.clone();
    CapabilityItem::ResourceTemplate(ResourceTemplateCapability {
        uri_template: raw.uri_template,
        name: Some(raw.name),
        description: raw.description,
        mime_type: raw.mime_type,
        unique_template,
        enabled: true,
    })
}
