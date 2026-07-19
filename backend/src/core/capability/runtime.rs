use anyhow::{Context, Result};
use futures::future::BoxFuture;
use rmcp::service::{Peer, RoleClient};
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::config::database::Database;
use crate::core::cache::{
    CacheQuery, CacheScope, CachedPromptInfo, CachedResourceInfo, CachedResourceTemplateInfo, CachedServerData,
    CachedToolInfo, FreshnessLevel, RedbCacheManager,
};
use crate::core::capability::internal::{
    CapabilityFetchFailure, capability_declared, collect_capability_from_instance_peer, is_method_not_supported,
};
use crate::core::capability::naming::NamingKind;
use crate::core::capability::{CapabilityType, ConnectionSelection, RuntimeIdentity};
use crate::core::pool::{CapSyncFlags, FailureKind, UpstreamConnectionPool};

/// Derive the appropriate cache scope from runtime identity and connection selection.
///
/// When both are present, returns `CacheScope::ClientFiltered` for per-client cache isolation.
/// Otherwise, falls back to `CacheScope::SharedRaw` for shared raw capability snapshots.
fn derive_cache_scope(
    runtime_identity: Option<&RuntimeIdentity>,
    connection_selection: Option<&ConnectionSelection>,
) -> CacheScope {
    match (runtime_identity, connection_selection) {
        (Some(identity), Some(selection)) => {
            CacheScope::client_filtered(selection.cache_scope_key(), identity.surface_fingerprint.clone())
        }
        _ => CacheScope::shared_raw(),
    }
}

/// Context for capability listing operations.
#[derive(Clone, Debug)]
pub struct ListCtx {
    pub capability: CapabilityType,
    pub server_id: String,
    pub refresh: Option<RefreshStrategy>,
    pub timeout: Option<Duration>,
    pub validation_session: Option<String>,
    pub runtime_identity: Option<RuntimeIdentity>,
    pub connection_selection: Option<ConnectionSelection>,
    pub name_domain: NameDomain,
}

/// Selects which capability identifier domain a listing surface exposes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameDomain {
    /// Exact identifiers reported by the upstream MCP server.
    Upstream,
    /// Stable MCPMate identifiers exposed outside the connection pool.
    External,
}

/// Context for capability call operations.
#[derive(Clone, Debug)]
pub struct CallCtx {
    pub call_id: String,
    pub server_id: String,
    pub tool_name: String,
    pub timeout: Option<Duration>,
    pub arguments: Option<rmcp::model::JsonObject>,
    pub runtime_identity: Option<RuntimeIdentity>,
    pub connection_selection: Option<ConnectionSelection>,
}

/// Refresh strategy for list operations.
#[derive(Clone, Copy, Debug)]
pub enum RefreshStrategy {
    CacheFirst,
    Force,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CapabilityPeerError {
    #[error("No connected capability peer for server '{server_id}'")]
    Runtime { server_id: String },
    #[error("Validation session '{session_id}' has no connected peer for server '{server_id}'")]
    Validation { server_id: String, session_id: String },
}

pub(crate) fn is_missing_peer_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<CapabilityPeerError>().is_some()
}

impl RefreshStrategy {
    pub fn to_cache_level(self) -> FreshnessLevel {
        match self {
            RefreshStrategy::CacheFirst => FreshnessLevel::Cached,
            RefreshStrategy::Force => FreshnessLevel::RealTime,
        }
    }
}

/// Metadata returned alongside capability results.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Meta {
    pub cache_hit: bool,
    pub source: String,
    pub duration_ms: u64,
    pub had_peer: bool,
}

/// Unified capability result container.
#[derive(Clone, Debug)]
pub struct ListResult {
    pub items: CapabilityItems,
    pub meta: Meta,
}

/// Polymorphic capability item collection.
#[derive(Clone, Debug)]
pub enum CapabilityItems {
    Tools(Vec<rmcp::model::Tool>),
    Prompts(Vec<rmcp::model::Prompt>),
    Resources(Vec<rmcp::model::Resource>),
    ResourceTemplates(Vec<rmcp::model::ResourceTemplate>),
}

impl CapabilityItems {
    pub fn empty(kind: CapabilityType) -> Self {
        match kind {
            CapabilityType::Tools => CapabilityItems::Tools(Vec::new()),
            CapabilityType::Prompts => CapabilityItems::Prompts(Vec::new()),
            CapabilityType::Resources => CapabilityItems::Resources(Vec::new()),
            CapabilityType::ResourceTemplates => CapabilityItems::ResourceTemplates(Vec::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            CapabilityItems::Tools(items) => items.is_empty(),
            CapabilityItems::Prompts(items) => items.is_empty(),
            CapabilityItems::Resources(items) => items.is_empty(),
            CapabilityItems::ResourceTemplates(items) => items.is_empty(),
        }
    }

    pub fn into_tools(self) -> Option<Vec<rmcp::model::Tool>> {
        match self {
            CapabilityItems::Tools(items) => Some(items),
            _ => None,
        }
    }

    pub fn into_prompts(self) -> Option<Vec<rmcp::model::Prompt>> {
        match self {
            CapabilityItems::Prompts(items) => Some(items),
            _ => None,
        }
    }

    pub fn into_resources(self) -> Option<Vec<rmcp::model::Resource>> {
        match self {
            CapabilityItems::Resources(items) => Some(items),
            _ => None,
        }
    }

    pub fn into_resource_templates(self) -> Option<Vec<rmcp::model::ResourceTemplate>> {
        match self {
            CapabilityItems::ResourceTemplates(items) => Some(items),
            _ => None,
        }
    }
}

fn cached_items_from_data(
    capability: CapabilityType,
    data: CachedServerData,
) -> CapabilityItems {
    let CachedServerData {
        tools,
        resources,
        prompts,
        resource_templates,
        ..
    } = data;

    match capability {
        CapabilityType::Tools => CapabilityItems::Tools(tools.into_iter().filter_map(convert_cached_tool).collect()),
        CapabilityType::Prompts => CapabilityItems::Prompts(prompts.into_iter().map(convert_cached_prompt).collect()),
        CapabilityType::Resources => {
            CapabilityItems::Resources(resources.into_iter().filter_map(convert_cached_resource).collect())
        }
        CapabilityType::ResourceTemplates => CapabilityItems::ResourceTemplates(
            resource_templates
                .into_iter()
                .filter_map(convert_cached_resource_template)
                .collect(),
        ),
    }
}

#[derive(Debug, Clone)]
pub enum OperationSource {
    Runtime,
    Temporary,
}

impl OperationSource {
    fn as_str(&self) -> &'static str {
        match self {
            OperationSource::Runtime => "runtime",
            OperationSource::Temporary => "temporary",
        }
    }
}

#[derive(Clone)]
struct InstanceHandle<P> {
    server_id: String,
    server_name: String,
    instance_id: String,
    peer: P,
    source: OperationSource,
}

#[derive(Debug, Clone)]
pub enum RuntimeFailureKind {
    Timeout,
    SessionGone,
    Other,
}

#[derive(Debug, Clone)]
pub struct RuntimeFailure {
    pub kind: RuntimeFailureKind,
    pub message: Option<String>,
}

pub fn message_indicates_session_gone(msg_lower: &str) -> bool {
    msg_lower.contains("status: 404")
        || msg_lower.contains("status: 410")
        || msg_lower.contains("404")
        || msg_lower.contains("410")
        || msg_lower.contains("gone")
}

pub async fn handle_runtime_failure(
    pool: &Arc<Mutex<UpstreamConnectionPool>>,
    server_id: &str,
    instance_id: &str,
    failure: RuntimeFailure,
) {
    let message = failure.message.clone();
    let failure_kind = match failure.kind {
        RuntimeFailureKind::Timeout => FailureKind::RuntimeTimeout,
        RuntimeFailureKind::SessionGone => FailureKind::RuntimeGone,
        RuntimeFailureKind::Other => FailureKind::RuntimeOther,
    };
    let mut pool_guard = pool.lock().await;
    let _ = pool_guard.register_failure(server_id, failure_kind, message);
    // Only tear down the connection for session-gone errors to avoid penalizing transient timeouts
    if matches!(failure.kind, RuntimeFailureKind::SessionGone) {
        let _ = pool_guard.disconnect_non_blocking(server_id, instance_id).await;
    }
}

fn runtime_failure_from_capability(failure: Option<CapabilityFetchFailure>) -> Option<RuntimeFailure> {
    failure.map(|f| match f {
        CapabilityFetchFailure::Timeout => RuntimeFailure {
            kind: RuntimeFailureKind::Timeout,
            message: None,
        },
        CapabilityFetchFailure::Gone { message } => RuntimeFailure {
            kind: RuntimeFailureKind::SessionGone,
            message: Some(message),
        },
        CapabilityFetchFailure::Other { message } => RuntimeFailure {
            kind: RuntimeFailureKind::Other,
            message: Some(message),
        },
    })
}

fn runtime_inventory_failure_error(
    capability: CapabilityType,
    failure: &RuntimeFailure,
) -> anyhow::Error {
    let operation = match capability {
        CapabilityType::Tools => "tools/list",
        CapabilityType::Prompts => "prompts/list",
        CapabilityType::Resources => "resources/list",
        CapabilityType::ResourceTemplates => "resources/templates/list",
    };
    match failure.kind {
        RuntimeFailureKind::Timeout => anyhow::anyhow!("Upstream {operation} timed out"),
        RuntimeFailureKind::SessionGone => anyhow::anyhow!(
            "Upstream {operation} session is no longer available: {}",
            failure.message.as_deref().unwrap_or("session closed")
        ),
        RuntimeFailureKind::Other => anyhow::anyhow!(
            "Upstream {operation} failed: {}",
            failure
                .message
                .as_deref()
                .unwrap_or("unknown protocol or transport error")
        ),
    }
}

/// Execute a capability list operation (REDB-first, runtime fallback, async sync).
pub async fn list(
    ctx: &ListCtx,
    redb: &Arc<RedbCacheManager>,
    pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: &Arc<Database>,
) -> Result<ListResult> {
    list_impl(ctx, redb, pool, database).await
}

/// Execute a tool call using the shared runtime pipeline.
pub async fn call_tool(
    ctx: &CallCtx,
    pool: &Arc<Mutex<UpstreamConnectionPool>>,
) -> Result<rmcp::model::CallToolResult> {
    call_tool_impl(ctx, pool).await
}

async fn list_impl(
    ctx: &ListCtx,
    redb: &Arc<RedbCacheManager>,
    pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: &Arc<Database>,
) -> Result<ListResult> {
    let start = std::time::Instant::now();
    let timeout = ctx.timeout.unwrap_or_else(|| Duration::from_secs(10));
    let derived_scope = derive_cache_scope(ctx.runtime_identity.as_ref(), ctx.connection_selection.as_ref());
    let is_client_filtered = derived_scope.is_client_filtered();

    if !matches!(ctx.refresh, Some(RefreshStrategy::Force)) {
        let cache_query = CacheQuery {
            server_id: ctx.server_id.clone(),
            freshness_level: FreshnessLevel::Cached,
            include_disabled: false,
            scope: derived_scope.clone(),
        };
        if let Ok(result) = redb.get_server_data(&cache_query).await {
            if result.cache_hit {
                if let Some(data) = result.data {
                    let items = project_cached_items(
                        database,
                        &ctx.server_id,
                        cached_items_from_data(ctx.capability, data),
                        &derived_scope,
                        ctx.name_domain,
                    )
                    .await?;
                    if !items.is_empty() {
                        return Ok(ListResult {
                            items,
                            meta: Meta {
                                cache_hit: true,
                                source: "cache".to_string(),
                                duration_ms: start.elapsed().as_millis() as u64,
                                had_peer: false,
                            },
                        });
                    }

                    let token = match ctx.capability {
                        CapabilityType::Tools => Some("tools"),
                        CapabilityType::Prompts => Some("prompts"),
                        CapabilityType::Resources => Some("resources"),
                        CapabilityType::ResourceTemplates => Some("resources"),
                    };
                    let mut declared_has = true;
                    if let Some(tok) = token {
                        declared_has = if let Ok(Some(row)) =
                            sqlx::query("SELECT capabilities FROM server_config WHERE id = ?")
                                .bind(&ctx.server_id)
                                .fetch_optional(&database.pool)
                                .await
                        {
                            let caps: Option<String> = row.try_get("capabilities").ok();
                            capability_declared(caps.as_deref(), tok)
                        } else {
                            true
                        };
                    }
                    if !declared_has {
                        return Ok(ListResult {
                            items: CapabilityItems::empty(ctx.capability),
                            meta: Meta {
                                cache_hit: true,
                                source: "cache".to_string(),
                                duration_ms: start.elapsed().as_millis() as u64,
                                had_peer: false,
                            },
                        });
                    }
                }
            }
        }

        // SharedRaw fallback for client-filtered cache miss
        if is_client_filtered {
            let shared_raw_query = CacheQuery {
                server_id: ctx.server_id.clone(),
                freshness_level: FreshnessLevel::Cached,
                include_disabled: false,
                scope: CacheScope::shared_raw(),
            };
            if let Ok(result) = redb.get_server_data(&shared_raw_query).await {
                if result.cache_hit {
                    if let Some(data) = result.data {
                        let items = project_external_items(
                            database,
                            &ctx.server_id,
                            cached_items_from_data(ctx.capability, data),
                            ctx.name_domain,
                        )
                        .await?;
                        if !items.is_empty() {
                            tracing::debug!(
                                server_id = %ctx.server_id,
                                capability = ?ctx.capability,
                                "Client-filtered cache miss, fell back to SharedRaw cache"
                            );
                            return Ok(ListResult {
                                items,
                                meta: Meta {
                                    cache_hit: true,
                                    source: "cache_shared_raw_fallback".to_string(),
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    had_peer: false,
                                },
                            });
                        }
                    }
                }
            }
        }
    }

    let (peer_opt, instance_id_opt, server_name, instance_source) = {
        let pool_guard = pool.lock().await;
        let name = match crate::core::capability::resolver::to_name(&ctx.server_id).await {
            Ok(Some(n)) => n,
            _ => ctx.server_id.clone(),
        };
        if let Some(session_id) = ctx.validation_session.as_ref() {
            let peer = pool_guard
                .validation_sessions
                .get(session_id)
                .and_then(|session_servers| session_servers.get(&ctx.server_id))
                .and_then(|connection| {
                    connection
                        .service
                        .as_ref()
                        .map(|service| (service.peer().clone(), connection.id.clone()))
                });
            match peer {
                Some((peer, instance_id)) => (Some(peer), Some(instance_id), name, OperationSource::Temporary),
                None => (None, None, name, OperationSource::Temporary),
            }
        } else {
            let snap = pool_guard.get_snapshot();
            let mut peer_opt = None;
            let mut instance_id_opt = None;
            if let Some(selection) = ctx.connection_selection.as_ref() {
                if let Ok(Some(selected_instance_id)) = pool_guard.select_ready_instance_id(selection) {
                    if let Some(instances) = snap.get(&ctx.server_id) {
                        if let Some((iid, _status, _res, _prm, peer)) =
                            instances.iter().find(|(candidate_id, st, _, _, p)| {
                                **candidate_id == selected_instance_id
                                    && matches!(st, crate::core::foundation::types::ConnectionStatus::Ready)
                                    && p.is_some()
                            })
                        {
                            peer_opt = peer.clone();
                            instance_id_opt = Some(iid.clone());
                        }
                    }
                }
            }
            if let Some(instances) = snap.get(&ctx.server_id) {
                if peer_opt.is_none() {
                    if let Some((iid, _status, _res, _prm, peer)) = instances.iter().find(|(_, st, _, _, p)| {
                        matches!(st, crate::core::foundation::types::ConnectionStatus::Ready) && p.is_some()
                    }) {
                        peer_opt = peer.clone();
                        instance_id_opt = Some(iid.clone());
                    }
                }
            }
            (peer_opt, instance_id_opt, name, OperationSource::Runtime)
        }
    };

    let peer = match peer_opt {
        Some(p) => p,
        None => {
            if let Some(session_id) = ctx.validation_session.as_ref() {
                return Err(CapabilityPeerError::Validation {
                    server_id: ctx.server_id.clone(),
                    session_id: session_id.clone(),
                }
                .into());
            }
            return Err(CapabilityPeerError::Runtime {
                server_id: ctx.server_id.clone(),
            }
            .into());
        }
    };
    let instance_id = instance_id_opt.unwrap_or_else(|| "default".to_string());
    let instance = InstanceHandle {
        server_id: ctx.server_id.clone(),
        server_name,
        instance_id,
        peer: peer.clone(),
        source: instance_source,
    };

    if matches!(instance.source, OperationSource::Runtime) {
        let instance_id = instance.instance_id.clone();
        let mut pool_guard = pool.lock().await;
        pool_guard.mark_instance_activity(&ctx.server_id, &instance_id);
        drop(pool_guard);
    }

    let (mut result, runtime_failure) = list_with_instance(ctx, instance, timeout, database.clone()).await?;
    result.meta.duration_ms = start.elapsed().as_millis() as u64;
    if let Some(failure) = runtime_failure {
        let error = runtime_inventory_failure_error(ctx.capability, &failure);
        handle_runtime_failure(pool, &ctx.server_id, &result.meta.source, failure).await;
        return Err(error);
    }
    Ok(result)
}

async fn list_with_instance(
    ctx: &ListCtx,
    instance: InstanceHandle<Peer<RoleClient>>,
    timeout: Duration,
    database: Arc<Database>,
) -> Result<(ListResult, Option<RuntimeFailure>)> {
    let server_id = instance.server_id.clone();
    let server_name = instance.server_name.clone();
    let instance_id = instance.instance_id.clone();
    let peer = instance.peer.clone();

    let (items, flags, runtime_failure) = fetch_runtime_items(
        ctx.capability,
        peer.clone(),
        timeout,
        &server_id,
        &server_name,
        &instance_id,
        database.clone(),
        ctx.name_domain,
    )
    .await?;

    if !items.is_empty() {
        let db = database.clone();
        let server_id = server_id.clone();
        let instance_id = instance_id.clone();
        let peer = peer.clone();
        tokio::spawn(async move {
            if let Err(error) =
                UpstreamConnectionPool::sync_capabilities(&db, &server_id, &instance_id, &peer, flags, None).await
            {
                tracing::error!(
                    server_id,
                    error = %error,
                    "Runtime capability write-back failed"
                );
            }
        });
    }

    let cache_scope = derive_cache_scope(ctx.runtime_identity.as_ref(), ctx.connection_selection.as_ref());
    if cache_scope.is_client_filtered() && !items.is_empty() {
        if let Ok(cache_manager) = RedbCacheManager::global() {
            let server_id = server_id.clone();
            let server_name = server_name.clone();
            let scope = cache_scope;
            let capability = ctx.capability;
            let cached_items = convert_items_to_cached(&items, capability);
            tokio::spawn(async move {
                let protocol_version = "latest".to_string();
                if let Err(e) = crate::config::server::capabilities::store_redb_snapshot_with_scope(
                    &cache_manager,
                    &server_id,
                    &server_name,
                    cached_items.tools,
                    cached_items.resources,
                    cached_items.prompts,
                    cached_items.resource_templates,
                    Some(&protocol_version),
                    scope,
                )
                .await
                {
                    tracing::warn!(
                        server_id = %server_id,
                        error = %e,
                        "Failed to store client-filtered cache entry"
                    );
                }
            });
        }
    }

    Ok((
        ListResult {
            items,
            meta: Meta {
                cache_hit: false,
                source: instance.source.as_str().to_string(),
                duration_ms: 0,
                had_peer: true,
            },
        },
        runtime_failure,
    ))
}

async fn fetch_runtime_items(
    capability: CapabilityType,
    peer: Peer<RoleClient>,
    timeout: Duration,
    server_id: &str,
    server_name: &str,
    instance_id: &str,
    database: Arc<Database>,
    name_domain: NameDomain,
) -> Result<(CapabilityItems, CapSyncFlags, Option<RuntimeFailure>)> {
    match capability {
        CapabilityType::Tools => {
            let fetch_page =
                move |p: Peer<RoleClient>,
                      cursor: Option<String>|
                      -> BoxFuture<'static, anyhow::Result<(Vec<rmcp::model::Tool>, Option<String>)>> {
                    Box::pin(async move {
                        let result = p
                            .list_tools(Some(rmcp::model::PaginatedRequestParams::default().with_cursor(cursor)))
                            .await?;
                        Ok((result.tools, result.next_cursor))
                    })
                };
            let out = collect_capability_from_instance_peer(
                peer,
                timeout,
                fetch_page,
                |tool, _s, _i, _n| tool,
                server_id,
                server_name,
                instance_id,
                is_method_not_supported,
            )
            .await;
            let runtime_failure = runtime_failure_from_capability(out.failure);
            if runtime_failure.is_some() {
                return Ok((CapabilityItems::Tools(Vec::new()), CapSyncFlags::TOOLS, runtime_failure));
            }
            let items = ensure_tool_unique_names(&database, server_id, server_name, out.items, name_domain).await?;
            Ok((CapabilityItems::Tools(items), CapSyncFlags::TOOLS, runtime_failure))
        }
        CapabilityType::Prompts => {
            let fetch_page = move |p: Peer<RoleClient>,
                                   cursor: Option<String>|
                  -> BoxFuture<
                'static,
                anyhow::Result<(Vec<rmcp::model::Prompt>, Option<String>)>,
            > {
                Box::pin(async move {
                    let result = p
                        .list_prompts(Some(rmcp::model::PaginatedRequestParams::default().with_cursor(cursor)))
                        .await?;
                    Ok((result.prompts, result.next_cursor))
                })
            };
            let out = collect_capability_from_instance_peer(
                peer,
                timeout,
                fetch_page,
                |prompt, _s, _i, _n| prompt,
                server_id,
                server_name,
                instance_id,
                is_method_not_supported,
            )
            .await;
            let runtime_failure = runtime_failure_from_capability(out.failure);
            if runtime_failure.is_some() {
                return Ok((
                    CapabilityItems::Prompts(Vec::new()),
                    CapSyncFlags::PROMPTS,
                    runtime_failure,
                ));
            }
            let items = ensure_prompt_unique_names(&database, server_id, server_name, out.items, name_domain).await?;
            Ok((CapabilityItems::Prompts(items), CapSyncFlags::PROMPTS, None))
        }
        CapabilityType::Resources => {
            let fetch_page = move |p: Peer<RoleClient>,
                                   cursor: Option<String>|
                  -> BoxFuture<
                'static,
                anyhow::Result<(Vec<rmcp::model::Resource>, Option<String>)>,
            > {
                Box::pin(async move {
                    let result = p
                        .list_resources(Some(rmcp::model::PaginatedRequestParams::default().with_cursor(cursor)))
                        .await?;
                    Ok((result.resources, result.next_cursor))
                })
            };
            let out = collect_capability_from_instance_peer(
                peer,
                timeout,
                fetch_page,
                |resource, _s, _i, _n| resource,
                server_id,
                server_name,
                instance_id,
                is_method_not_supported,
            )
            .await;
            let runtime_failure = runtime_failure_from_capability(out.failure);
            if runtime_failure.is_some() {
                return Ok((
                    CapabilityItems::Resources(Vec::new()),
                    CapSyncFlags::RESOURCES,
                    runtime_failure,
                ));
            }
            let items = ensure_resource_unique_names(&database, server_id, server_name, out.items, name_domain).await?;
            Ok((CapabilityItems::Resources(items), CapSyncFlags::RESOURCES, None))
        }
        CapabilityType::ResourceTemplates => {
            let fetch_page = move |p: Peer<RoleClient>,
                                   cursor: Option<String>|
                  -> BoxFuture<
                'static,
                anyhow::Result<(Vec<rmcp::model::ResourceTemplate>, Option<String>)>,
            > {
                Box::pin(async move {
                    let result = p
                        .list_resource_templates(Some(
                            rmcp::model::PaginatedRequestParams::default().with_cursor(cursor),
                        ))
                        .await?;
                    Ok((result.resource_templates, result.next_cursor))
                })
            };
            let out = collect_capability_from_instance_peer(
                peer,
                timeout,
                fetch_page,
                |template, _s, _i, _n| template,
                server_id,
                server_name,
                instance_id,
                is_method_not_supported,
            )
            .await;
            let runtime_failure = runtime_failure_from_capability(out.failure);
            if runtime_failure.is_some() {
                return Ok((
                    CapabilityItems::ResourceTemplates(Vec::new()),
                    CapSyncFlags::RESOURCE_TEMPLATES,
                    runtime_failure,
                ));
            }
            let items =
                ensure_resource_template_unique_names(&database, server_id, server_name, out.items, name_domain)
                    .await?;
            Ok((
                CapabilityItems::ResourceTemplates(items),
                CapSyncFlags::RESOURCE_TEMPLATES,
                None,
            ))
        }
    }
}

async fn call_tool_impl(
    ctx: &CallCtx,
    pool: &Arc<Mutex<UpstreamConnectionPool>>,
) -> Result<rmcp::model::CallToolResult> {
    let timeout = ctx.timeout.unwrap_or_else(|| Duration::from_secs(60));
    let t0 = std::time::Instant::now();

    let fetch_peer = || async {
        let pool_guard = pool.lock().await;
        let snap = pool_guard.get_snapshot();
        if let Some(selection) = ctx.connection_selection.as_ref() {
            if let Ok(Some(selected_instance_id)) = pool_guard.select_ready_instance_id(selection) {
                if let Some(instances) = snap.get(&ctx.server_id) {
                    if let Some((instance_id, status, _res, _prm, peer)) =
                        instances.iter().find(|(candidate_id, st, _, _, p)| {
                            **candidate_id == selected_instance_id
                                && matches!(st, crate::core::foundation::types::ConnectionStatus::Ready)
                                && p.is_some()
                        })
                    {
                        return (peer.clone(), Some(instance_id.clone()), Some(status.clone()));
                    }
                }
            }
        }
        if let Some(instances) = snap.get(&ctx.server_id) {
            if let Some((instance_id, status, _res, _prm, peer)) = instances.iter().find(|(_, st, _, _, p)| {
                matches!(st, crate::core::foundation::types::ConnectionStatus::Ready) && p.is_some()
            }) {
                return (peer.clone(), Some(instance_id.clone()), Some(status.clone()));
            }
        }
        (None, None, None)
    };

    let t_fetch_begin = std::time::Instant::now();
    let mut peer_info = fetch_peer().await;
    let t_fetch_ms = t_fetch_begin.elapsed().as_millis();
    if peer_info.0.is_none() {
        let t_connect_begin = std::time::Instant::now();
        let mut pool_guard = pool.lock().await;
        if let Some(selection) = ctx.connection_selection.as_ref() {
            pool_guard.ensure_connected_with_selection(selection).await?;
        } else {
            pool_guard.ensure_connected(&ctx.server_id).await?;
        }
        drop(pool_guard);
        peer_info = fetch_peer().await;
        tracing::debug!(
            server_id = %ctx.server_id,
            tool = %ctx.tool_name,
            fetch_ms = %t_fetch_ms,
            ensure_connected_ms = %t_connect_begin.elapsed().as_millis(),
            "[CALL] ensured connection before tool call"
        );
    } else {
        tracing::debug!(
            server_id = %ctx.server_id,
            tool = %ctx.tool_name,
            fetch_ms = %t_fetch_ms,
            "[CALL] found ready peer in snapshot"
        );
    }

    let (peer_opt, selected_instance, _selected_status) = peer_info;
    let peer: Peer<RoleClient> =
        peer_opt.ok_or_else(|| anyhow::anyhow!("No ready instance for server {}", ctx.server_id))?;

    if let Some(instance_id) = selected_instance.as_ref() {
        let mut pool_guard = pool.lock().await;
        pool_guard.mark_instance_activity(&ctx.server_id, instance_id);
    }

    let selected_instance_id = selected_instance.clone();
    let tool_name = ctx.tool_name.clone();
    let arguments = ctx.arguments.clone();
    let fut = async move {
        tracing::debug!(server_id = %ctx.server_id, tool = %ctx.tool_name, timeout_secs = %timeout.as_secs(), "[CALL] sending to upstream");
        {
            let mut params = rmcp::model::CallToolRequestParams::new(tool_name);
            if let Some(arguments) = arguments {
                params = params.with_arguments(arguments);
            }
            peer.call_tool(params)
        }
        .await
    };
    match tokio::time::timeout(timeout, fut).await {
        Ok(Ok(res)) => {
            if selected_instance_id.is_some() {
                let mut pool = pool.lock().await;
                pool.clear_failure_state(&ctx.server_id);
                if let Some(instance_id) = selected_instance_id.as_ref() {
                    pool.mark_instance_activity(&ctx.server_id, instance_id);
                }
            }
            tracing::info!(
                server_id = %ctx.server_id,
                tool = %ctx.tool_name,
                elapsed_ms = %t0.elapsed().as_millis(),
                "[CALL] upstream call succeeded"
            );
            Ok(res)
        }
        Ok(Err(e)) => {
            let error_msg = format!("{}", e);
            if let Some(instance_id) = selected_instance_id.as_deref() {
                let lower = error_msg.to_ascii_lowercase();
                let kind = if message_indicates_session_gone(&lower) {
                    RuntimeFailureKind::SessionGone
                } else {
                    RuntimeFailureKind::Other
                };
                handle_runtime_failure(
                    pool,
                    &ctx.server_id,
                    instance_id,
                    RuntimeFailure {
                        kind,
                        message: Some(error_msg.clone()),
                    },
                )
                .await;
            }
            tracing::error!(
                server_id = %ctx.server_id,
                tool = %ctx.tool_name,
                elapsed_ms = %t0.elapsed().as_millis(),
                error = %error_msg,
                "[CALL] upstream call failed"
            );
            Err(anyhow::anyhow!("Tool call failed: {}", error_msg))
        }
        Err(_) => {
            // TODO(mcpmate): revisit after RMCP fixes SSE stream recovery (see GitMCP long-tail investigation).
            if let Some(instance_id) = selected_instance_id.as_deref() {
                handle_runtime_failure(
                    pool,
                    &ctx.server_id,
                    instance_id,
                    RuntimeFailure {
                        kind: RuntimeFailureKind::Timeout,
                        message: Some(format!("Tool call timed out after {:.1}s", timeout.as_secs_f32())),
                    },
                )
                .await;
            }
            tracing::error!(
                server_id = %ctx.server_id,
                tool = %ctx.tool_name,
                elapsed_ms = %t0.elapsed().as_millis(),
                timeout_secs = %timeout.as_secs(),
                "[CALL] upstream call timeout"
            );
            Err(anyhow::anyhow!("Tool call timeout for server {}", ctx.server_id))
        }
    }
}

fn convert_cached_prompt(cached: CachedPromptInfo) -> rmcp::model::Prompt {
    let arguments = if cached.arguments.is_empty() {
        None
    } else {
        Some(
            cached
                .arguments
                .into_iter()
                .map(|arg| {
                    let mut prompt_argument = rmcp::model::PromptArgument::new(arg.name);
                    if let Some(description) = arg.description {
                        prompt_argument = prompt_argument.with_description(description);
                    }
                    prompt_argument.with_required(arg.required)
                })
                .collect(),
        )
    };

    let mut prompt = rmcp::model::Prompt::new(cached.name, cached.description, arguments);
    if let Some(icons) = cached.icons {
        prompt = prompt.with_icons(icons);
    }
    prompt
}

fn convert_cached_resource(cached: CachedResourceInfo) -> Option<rmcp::model::Resource> {
    let resolved_name = cached
        .name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| cached.uri.clone());
    let mut resource = rmcp::model::Resource::new(cached.uri.clone(), resolved_name);
    resource.description = cached.description;
    resource.mime_type = cached.mime_type;
    resource.icons = cached.icons;
    Some(resource)
}

fn convert_cached_resource_template(cached: CachedResourceTemplateInfo) -> Option<rmcp::model::ResourceTemplate> {
    let resolved_name = cached
        .name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| cached.uri_template.clone());
    let mut template = rmcp::model::ResourceTemplate::new(cached.uri_template, resolved_name);
    if let Some(description) = cached.description {
        template = template.with_description(description);
    }
    if let Some(mime_type) = cached.mime_type {
        template = template.with_mime_type(mime_type);
    }
    Some(template)
}

fn convert_cached_tool(cached: CachedToolInfo) -> Option<rmcp::model::Tool> {
    let schema_value: serde_json::Value = serde_json::from_str(&cached.input_schema_json).ok()?;
    let schema_object = schema_value.as_object()?.clone();
    let mut tool = if let Some(description) = cached.description.map(std::borrow::Cow::Owned) {
        rmcp::model::Tool::new(
            std::borrow::Cow::Owned(cached.name),
            description,
            Arc::new(schema_object),
        )
    } else {
        rmcp::model::Tool::new_with_raw(std::borrow::Cow::Owned(cached.name), None, Arc::new(schema_object))
    };
    if let Some(icons) = cached.icons {
        tool = tool.with_icons(icons);
    }
    Some(tool)
}

struct CachedItems {
    tools: Vec<CachedToolInfo>,
    resources: Vec<CachedResourceInfo>,
    prompts: Vec<CachedPromptInfo>,
    resource_templates: Vec<CachedResourceTemplateInfo>,
}

fn convert_items_to_cached(
    items: &CapabilityItems,
    _capability: CapabilityType,
) -> CachedItems {
    use chrono::Utc;

    let now = Utc::now();
    let mut result = CachedItems {
        tools: Vec::new(),
        resources: Vec::new(),
        prompts: Vec::new(),
        resource_templates: Vec::new(),
    };

    match items {
        CapabilityItems::Tools(tools) => {
            result.tools = tools
                .iter()
                .map(|tool| {
                    let schema = tool.schema_as_json_value();
                    let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
                    let output_schema_json = tool.output_schema.as_ref().map(|s| {
                        serde_json::to_string(&serde_json::Value::Object((**s).clone()))
                            .unwrap_or_else(|_| "{}".to_string())
                    });
                    CachedToolInfo {
                        name: tool.name.to_string(),
                        description: tool.description.clone().map(|d| d.into_owned()),
                        input_schema_json,
                        output_schema_json,
                        unique_name: Some(tool.name.to_string()),
                        icons: tool.icons.clone(),
                        enabled: true,
                        cached_at: now,
                    }
                })
                .collect();
        }
        CapabilityItems::Prompts(prompts) => {
            result.prompts = prompts
                .iter()
                .map(|prompt| CachedPromptInfo {
                    name: prompt.name.to_string(),
                    description: prompt.description.clone(),
                    arguments: prompt
                        .arguments
                        .clone()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|arg| crate::core::cache::PromptArgument {
                            name: arg.name,
                            description: arg.description,
                            required: arg.required.unwrap_or(false),
                        })
                        .collect(),
                    icons: prompt.icons.clone(),
                    enabled: true,
                    cached_at: now,
                })
                .collect();
        }
        CapabilityItems::Resources(resources) => {
            result.resources = resources
                .iter()
                .map(|resource| CachedResourceInfo {
                    uri: resource.uri.clone(),
                    name: Some(resource.name.clone()),
                    description: resource.description.clone(),
                    mime_type: resource.mime_type.clone(),
                    icons: resource.icons.clone(),
                    enabled: true,
                    cached_at: now,
                })
                .collect();
        }
        CapabilityItems::ResourceTemplates(templates) => {
            result.resource_templates = templates
                .iter()
                .map(|template| CachedResourceTemplateInfo {
                    uri_template: template.uri_template.clone(),
                    name: Some(template.name.clone()),
                    description: template.description.clone(),
                    mime_type: template.mime_type.clone(),
                    enabled: true,
                    cached_at: now,
                })
                .collect();
        }
    }

    result
}

async fn ensure_tool_unique_names(
    database: &Arc<Database>,
    server_id: &str,
    server_name: &str,
    mut tools: Vec<rmcp::model::Tool>,
    name_domain: NameDomain,
) -> anyhow::Result<Vec<rmcp::model::Tool>> {
    let upstream_names = tools.iter().map(|tool| tool.name.clone()).collect::<Vec<_>>();
    crate::config::server::tools::assign_unique_names_to_tools(&database.pool, server_id, server_name, &mut tools)
        .await
        .context("Failed to assign unique tool names")?;
    if name_domain == NameDomain::Upstream {
        for (tool, upstream_name) in tools.iter_mut().zip(upstream_names) {
            tool.name = upstream_name;
        }
    }
    Ok(tools)
}

async fn ensure_prompt_unique_names(
    database: &Arc<Database>,
    server_id: &str,
    server_name: &str,
    mut prompts: Vec<rmcp::model::Prompt>,
    name_domain: NameDomain,
) -> anyhow::Result<Vec<rmcp::model::Prompt>> {
    let upstream_names = prompts.iter().map(|prompt| prompt.name.clone()).collect::<Vec<_>>();
    let cached = convert_items_to_cached(&CapabilityItems::Prompts(prompts.clone()), CapabilityType::Prompts);
    crate::config::server::capabilities::upsert_shadow_prompts_batch(
        &database.pool,
        server_id,
        server_name,
        &cached.prompts,
    )
    .await
    .context("Failed to assign external prompt names")?;
    if name_domain == NameDomain::External {
        for (prompt, upstream_name) in prompts.iter_mut().zip(upstream_names) {
            prompt.name = crate::core::capability::naming::load_external_identifier(
                &database.pool,
                NamingKind::Prompt,
                server_id,
                &upstream_name,
            )
            .await?;
        }
    }
    Ok(prompts)
}

async fn ensure_resource_unique_names(
    database: &Arc<Database>,
    server_id: &str,
    server_name: &str,
    mut resources: Vec<rmcp::model::Resource>,
    name_domain: NameDomain,
) -> anyhow::Result<Vec<rmcp::model::Resource>> {
    let upstream_uris = resources
        .iter()
        .map(|resource| resource.uri.clone())
        .collect::<Vec<_>>();
    let cached = convert_items_to_cached(
        &CapabilityItems::Resources(resources.clone()),
        CapabilityType::Resources,
    );
    crate::config::server::capabilities::upsert_shadow_resources_batch(
        &database.pool,
        server_id,
        server_name,
        &cached.resources,
    )
    .await
    .context("Failed to assign external resource URIs")?;
    if name_domain == NameDomain::External {
        for (resource, upstream_uri) in resources.iter_mut().zip(upstream_uris) {
            resource.uri = crate::core::capability::naming::load_external_identifier(
                &database.pool,
                NamingKind::Resource,
                server_id,
                &upstream_uri,
            )
            .await?;
        }
    }
    Ok(resources)
}

async fn ensure_resource_template_unique_names(
    database: &Arc<Database>,
    server_id: &str,
    server_name: &str,
    templates: Vec<rmcp::model::ResourceTemplate>,
    name_domain: NameDomain,
) -> anyhow::Result<Vec<rmcp::model::ResourceTemplate>> {
    if name_domain == NameDomain::Upstream {
        return Ok(templates);
    }
    let upstream_templates = templates
        .iter()
        .map(|template| template.uri_template.clone())
        .collect::<Vec<_>>();
    let cached = convert_items_to_cached(
        &CapabilityItems::ResourceTemplates(templates.clone()),
        CapabilityType::ResourceTemplates,
    );
    crate::config::server::capabilities::upsert_shadow_resource_templates_batch(
        &database.pool,
        server_id,
        server_name,
        &cached.resource_templates,
    )
    .await
    .context("Failed to assign external resource template names")?;
    let mut projected = Vec::with_capacity(templates.len());
    for (mut template, upstream_template) in templates.into_iter().zip(upstream_templates) {
        if !crate::core::capability::resource_uri::resource_template_is_projectable(server_name, &upstream_template)? {
            continue;
        }
        template.uri_template = crate::core::capability::naming::load_external_identifier(
            &database.pool,
            NamingKind::ResourceTemplate,
            server_id,
            &upstream_template,
        )
        .await?;
        projected.push(template);
    }
    Ok(projected)
}

async fn project_external_items(
    database: &Arc<Database>,
    server_id: &str,
    mut items: CapabilityItems,
    name_domain: NameDomain,
) -> Result<CapabilityItems> {
    if name_domain == NameDomain::Upstream {
        return Ok(items);
    }

    match &mut items {
        CapabilityItems::Tools(tools) => {
            for tool in tools {
                tool.name = crate::core::capability::naming::load_external_identifier(
                    &database.pool,
                    NamingKind::Tool,
                    server_id,
                    &tool.name,
                )
                .await?
                .into();
            }
        }
        CapabilityItems::Prompts(prompts) => {
            for prompt in prompts {
                prompt.name = crate::core::capability::naming::load_external_identifier(
                    &database.pool,
                    NamingKind::Prompt,
                    server_id,
                    &prompt.name,
                )
                .await?;
            }
        }
        CapabilityItems::Resources(resources) => {
            for resource in resources {
                resource.uri = crate::core::capability::naming::load_external_identifier(
                    &database.pool,
                    NamingKind::Resource,
                    server_id,
                    &resource.uri,
                )
                .await?;
            }
        }
        CapabilityItems::ResourceTemplates(templates) => {
            let server_name = sqlx::query_scalar::<_, String>("SELECT name FROM server_config WHERE id = ?")
                .bind(server_id)
                .fetch_optional(&database.pool)
                .await
                .context("Failed to load server namespace for Resource Template projection")?
                .with_context(|| format!("Server '{server_id}' not found"))?;
            let mut projected = Vec::with_capacity(templates.len());
            for mut template in std::mem::take(templates) {
                let upstream_template = template.uri_template.to_string();
                if !crate::core::capability::resource_uri::resource_template_is_projectable(
                    &server_name,
                    &upstream_template,
                )? {
                    continue;
                }
                template.uri_template = crate::core::capability::naming::load_external_identifier(
                    &database.pool,
                    NamingKind::ResourceTemplate,
                    server_id,
                    &upstream_template,
                )
                .await?;
                projected.push(template);
            }
            *templates = projected;
        }
    }

    Ok(items)
}

async fn project_cached_items(
    database: &Arc<Database>,
    server_id: &str,
    items: CapabilityItems,
    scope: &CacheScope,
    name_domain: NameDomain,
) -> Result<CapabilityItems> {
    if scope.is_client_filtered() {
        if name_domain != NameDomain::External {
            return Err(anyhow::anyhow!(
                "Client-filtered capability caches cannot serve the upstream name domain"
            ));
        }
        return Ok(items);
    }

    project_external_items(database, server_id, items, name_domain).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::constants::protocol;
    use crate::core::cache::{
        CacheScope, CachedResourceInfo, CachedResourceTemplateInfo, CachedServerData, CachedToolInfo,
    };
    use crate::core::models::Config;
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_test_cached_data(
        server_id: &str,
        scope: CacheScope,
        tool_count: usize,
    ) -> CachedServerData {
        let now = Utc::now();
        CachedServerData {
            server_id: server_id.to_string(),
            server_name: "test-server".to_string(),
            server_version: Some("1.0.0".to_string()),
            protocol_version: protocol::V_2024_11_05.to_string(),
            tools: (0..tool_count)
                .map(|i| CachedToolInfo {
                    name: format!("tool_{}", i),
                    description: Some(format!("Test tool {}", i)),
                    input_schema_json: r#"{"type":"object"}"#.to_string(),
                    output_schema_json: None,
                    unique_name: Some(format!("test-server_tool_{i}")),
                    icons: None,
                    enabled: true,
                    cached_at: now,
                })
                .collect(),
            resources: Vec::new(),
            prompts: Vec::new(),
            resource_templates: Vec::new(),
            cached_at: now,
            fingerprint: "test-fingerprint".to_string(),
            scope,
        }
    }

    async fn test_database() -> Arc<Database> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        crate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");
        Arc::new(Database {
            pool,
            path: PathBuf::new(),
        })
    }

    #[test]
    fn cached_resource_models_preserve_baseline_persisted_fields() {
        let icon = rmcp::model::Icon::new("https://example.com/resource.png");
        let resource = convert_cached_resource(CachedResourceInfo {
            uri: "file:///guide.md".to_string(),
            name: Some("Guide".to_string()),
            description: Some("Project guide".to_string()),
            mime_type: Some("text/markdown".to_string()),
            icons: Some(vec![icon.clone()]),
            enabled: true,
            cached_at: Utc::now(),
        })
        .expect("convert cached resource");

        assert_eq!(resource.uri, "file:///guide.md");
        assert_eq!(resource.name, "Guide");
        assert_eq!(resource.description.as_deref(), Some("Project guide"));
        assert_eq!(resource.mime_type.as_deref(), Some("text/markdown"));
        assert_eq!(resource.icons, Some(vec![icon]));

        let template = convert_cached_resource_template(CachedResourceTemplateInfo {
            uri_template: "file:///{path}".to_string(),
            name: Some("Files".to_string()),
            description: Some("Project files".to_string()),
            mime_type: Some("text/plain".to_string()),
            enabled: true,
            cached_at: Utc::now(),
        })
        .expect("convert cached resource template");

        assert_eq!(template.uri_template, "file:///{path}");
        assert_eq!(template.name, "Files");
        assert_eq!(template.description.as_deref(), Some("Project files"));
        assert_eq!(template.mime_type.as_deref(), Some("text/plain"));
    }

    #[tokio::test]
    async fn force_list_without_a_peer_returns_an_explicit_error() {
        let database = test_database().await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'searxng', 'stdio')")
            .execute(&database.pool)
            .await
            .expect("insert server");
        let cache_dir = TempDir::new().expect("create cache directory");
        let redb = Arc::new(
            RedbCacheManager::new(
                cache_dir.path().join("runtime-no-peer.redb"),
                crate::core::cache::manager::CacheConfig::default(),
            )
            .expect("create cache manager"),
        );
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
            Arc::new(Config::default()),
            Some(database.clone()),
        )));
        let ctx = ListCtx {
            capability: CapabilityType::Tools,
            server_id: "server-a".to_string(),
            refresh: Some(RefreshStrategy::Force),
            timeout: None,
            validation_session: Some("validation-a".to_string()),
            runtime_identity: None,
            connection_selection: None,
            name_domain: NameDomain::Upstream,
        };

        let error = list(&ctx, &redb, &pool, &database)
            .await
            .expect_err("a missing validation peer must not look like an empty inventory");

        assert!(error.to_string().contains("validation-a"));
        assert!(error.to_string().contains("server-a"));
    }

    #[test]
    fn runtime_inventory_failure_is_reported_with_the_operation_name() {
        let error = runtime_inventory_failure_error(
            CapabilityType::ResourceTemplates,
            &RuntimeFailure {
                kind: RuntimeFailureKind::Timeout,
                message: None,
            },
        );

        assert!(error.to_string().contains("resources/templates/list"));
        assert!(error.to_string().contains("timed out"));
    }

    #[test]
    fn derive_cache_scope_returns_shared_raw_when_no_identity_or_selection() {
        let scope = derive_cache_scope(None, None);
        assert!(!scope.is_client_filtered());
        assert_eq!(scope.key_suffix(), "raw");
    }

    #[test]
    fn derive_cache_scope_returns_client_filtered_when_both_present() {
        use crate::core::capability::{AffinityKey, ConnectionSelection, RuntimeIdentity};

        let identity = RuntimeIdentity {
            client_id: "test-client".to_string(),
            profile_id: None,
            surface_fingerprint: "fp-123".to_string(),
        };
        let selection = ConnectionSelection {
            server_id: "srv-1".to_string(),
            affinity_key: AffinityKey::PerSession("sess-abc".to_string()),
        };

        let scope = derive_cache_scope(Some(&identity), Some(&selection));
        assert!(scope.is_client_filtered());
    }

    #[test]
    fn derive_cache_scope_returns_shared_raw_when_only_identity() {
        use crate::core::capability::RuntimeIdentity;

        let identity = RuntimeIdentity {
            client_id: "test-client".to_string(),
            profile_id: None,
            surface_fingerprint: "fp-123".to_string(),
        };

        let scope = derive_cache_scope(Some(&identity), None);
        assert!(!scope.is_client_filtered());
    }

    #[test]
    fn derive_cache_scope_returns_shared_raw_when_only_selection() {
        use crate::core::capability::{AffinityKey, ConnectionSelection};

        let selection = ConnectionSelection {
            server_id: "srv-1".to_string(),
            affinity_key: AffinityKey::PerSession("sess-abc".to_string()),
        };

        let scope = derive_cache_scope(None, Some(&selection));
        assert!(!scope.is_client_filtered());
    }

    #[test]
    fn cached_items_from_data_converts_tools_correctly() {
        let data = make_test_cached_data("srv-1", CacheScope::shared_raw(), 3);
        let items = cached_items_from_data(CapabilityType::Tools, data);

        match items {
            CapabilityItems::Tools(tools) => {
                assert_eq!(tools.len(), 3);
                assert_eq!(tools[0].name.as_ref(), "tool_0");
            }
            _ => panic!("Expected Tools variant"),
        }
    }

    #[tokio::test]
    async fn cached_tools_project_to_explicit_name_domains() {
        let database = test_database().await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'searxng', 'stdio')")
            .execute(&database.pool)
            .await
            .expect("insert server");
        crate::config::server::tools::upsert_server_tool(
            &database.pool,
            "server-a",
            "searxng",
            "get_searxng_status",
            None,
        )
        .await
        .expect("insert tool identity");

        let mut data = make_test_cached_data("server-a", CacheScope::shared_raw(), 0);
        data.tools = vec![CachedToolInfo {
            name: "get_searxng_status".to_string(),
            description: None,
            input_schema_json: r#"{"type":"object"}"#.to_string(),
            output_schema_json: None,
            unique_name: Some("searxng_get_status".to_string()),
            icons: None,
            enabled: true,
            cached_at: Utc::now(),
        }];

        let upstream = project_external_items(
            &database,
            "server-a",
            cached_items_from_data(CapabilityType::Tools, data.clone()),
            NameDomain::Upstream,
        )
        .await
        .expect("project upstream domain");
        let external = project_external_items(
            &database,
            "server-a",
            cached_items_from_data(CapabilityType::Tools, data),
            NameDomain::External,
        )
        .await
        .expect("project external domain");

        assert_eq!(upstream.into_tools().unwrap()[0].name.as_ref(), "get_searxng_status");
        assert_eq!(external.into_tools().unwrap()[0].name.as_ref(), "searxng_get_status");
    }

    #[tokio::test]
    async fn external_projection_rejects_missing_canonical_identifier() {
        let database = test_database().await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'searxng', 'stdio')")
            .execute(&database.pool)
            .await
            .expect("insert server");

        let data = make_test_cached_data("server-a", CacheScope::shared_raw(), 1);
        let error = project_external_items(
            &database,
            "server-a",
            cached_items_from_data(CapabilityType::Tools, data),
            NameDomain::External,
        )
        .await
        .expect_err("proxy projection must fail without a canonical identifier");

        assert!(
            error
                .to_string()
                .contains("Exact upstream Tool capability 'tool_0' is not registered for server 'server-a'")
        );
    }

    #[tokio::test]
    async fn client_filtered_cache_does_not_externalize_tools_twice() {
        let database = test_database().await;
        let scope = CacheScope::client_filtered("server-a#client".to_string(), "surface-a".to_string());
        let mut data = make_test_cached_data("server-a", scope.clone(), 0);
        data.tools = vec![CachedToolInfo {
            name: "searxng_get_status".to_string(),
            description: None,
            input_schema_json: r#"{"type":"object"}"#.to_string(),
            output_schema_json: None,
            unique_name: Some("searxng_get_status".to_string()),
            icons: None,
            enabled: true,
            cached_at: Utc::now(),
        }];

        let projected = project_cached_items(
            &database,
            "server-a",
            cached_items_from_data(CapabilityType::Tools, data),
            &scope,
            NameDomain::External,
        )
        .await
        .expect("reuse client-facing projection");

        assert_eq!(projected.into_tools().unwrap()[0].name.as_ref(), "searxng_get_status");
    }

    #[tokio::test]
    async fn raw_resource_template_cache_projects_uri_template_without_rewriting_name() {
        let database = test_database().await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-docs', 'docs', 'stdio')")
            .execute(&database.pool)
            .await
            .expect("insert server");
        crate::config::server::capabilities::upsert_shadow_resource_template(
            &database.pool,
            "server-docs",
            "docs",
            "file:///{path}",
            Some("Files"),
            None,
        )
        .await
        .expect("insert resource template identity");

        let mut data = make_test_cached_data("server-docs", CacheScope::shared_raw(), 0);
        data.resource_templates = vec![CachedResourceTemplateInfo {
            uri_template: "file:///{path}".to_string(),
            name: Some("Files".to_string()),
            description: None,
            mime_type: None,
            enabled: true,
            cached_at: Utc::now(),
        }];

        let projected = project_cached_items(
            &database,
            "server-docs",
            cached_items_from_data(CapabilityType::ResourceTemplates, data),
            &CacheScope::shared_raw(),
            NameDomain::External,
        )
        .await
        .expect("project resource template");
        let template = &projected.into_resource_templates().unwrap()[0];

        assert_eq!(template.name.as_str(), "Files");
        assert_eq!(
            template.uri_template.as_str(),
            "mcpmate://resources/template/docs/file/{path}"
        );
    }

    #[tokio::test]
    async fn raw_cache_external_projection_excludes_unprojectable_templates() {
        let database = test_database().await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-docs', 'docs', 'stdio')")
            .execute(&database.pool)
            .await
            .expect("insert server");
        crate::config::server::capabilities::upsert_shadow_resource_template(
            &database.pool,
            "server-docs",
            "docs",
            "file:///{path}",
            Some("Files"),
            None,
        )
        .await
        .expect("insert projected resource template identity");

        let mut data = make_test_cached_data("server-docs", CacheScope::shared_raw(), 0);
        data.resource_templates = vec![
            CachedResourceTemplateInfo {
                uri_template: "file:///{path}".to_string(),
                name: Some("Files".to_string()),
                description: None,
                mime_type: None,
                enabled: true,
                cached_at: Utc::now(),
            },
            CachedResourceTemplateInfo {
                uri_template: "file:///{+path}".to_string(),
                name: Some("Reserved Files".to_string()),
                description: None,
                mime_type: None,
                enabled: true,
                cached_at: Utc::now(),
            },
        ];

        let projected = project_cached_items(
            &database,
            "server-docs",
            cached_items_from_data(CapabilityType::ResourceTemplates, data),
            &CacheScope::shared_raw(),
            NameDomain::External,
        )
        .await
        .expect("project only canonical resource templates");
        let templates = projected.into_resource_templates().unwrap();

        assert_eq!(templates.len(), 1);
        assert_eq!(
            templates[0].uri_template.as_str(),
            "mcpmate://resources/template/docs/file/{path}"
        );
    }

    #[tokio::test]
    async fn upstream_resource_templates_do_not_require_external_projection() {
        let database = test_database().await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-docs', 'docs', 'stdio')")
            .execute(&database.pool)
            .await
            .expect("insert server");
        let upstream_template = "file:///{+path}";
        let templates = vec![rmcp::model::ResourceTemplate::new(upstream_template, "Files")];

        let result =
            ensure_resource_template_unique_names(&database, "server-docs", "docs", templates, NameDomain::Upstream)
                .await
                .expect("accept exact upstream resource template");

        assert_eq!(result[0].uri_template.as_str(), upstream_template);
    }

    #[tokio::test]
    async fn external_resource_templates_exclude_only_unprojectable_entries() {
        let database = test_database().await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-docs', 'docs', 'stdio')")
            .execute(&database.pool)
            .await
            .expect("insert server");
        let templates = vec![
            rmcp::model::ResourceTemplate::new("file:///{path}", "Files"),
            rmcp::model::ResourceTemplate::new("file:///{+path}", "Reserved Files"),
        ];

        let result =
            ensure_resource_template_unique_names(&database, "server-docs", "docs", templates, NameDomain::External)
                .await
                .expect("project supported templates without exposing raw fallbacks");

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].uri_template.as_str(),
            "mcpmate://resources/template/docs/file/{path}"
        );
    }

    #[tokio::test]
    async fn live_and_cached_resource_projection_share_registry_identity() {
        let database = test_database().await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-docs', 'docs', 'stdio')")
            .execute(&database.pool)
            .await
            .expect("insert server");
        crate::config::server::capabilities::upsert_shadow_resource(
            &database.pool,
            "server-docs",
            "docs",
            "demo://resource/static/document/architecture.md",
            Some("Architecture"),
            None,
            Some("text/markdown"),
        )
        .await
        .expect("insert resource identity");
        crate::config::server::capabilities::upsert_shadow_resource_template(
            &database.pool,
            "server-docs",
            "docs",
            "demo://resource/dynamic/text/{resourceId}",
            Some("Dynamic Text Resource"),
            None,
        )
        .await
        .expect("insert template identity");

        let upstream_resources = CapabilityItems::Resources(vec![rmcp::model::Resource::new(
            "demo://resource/static/document/architecture.md",
            "Architecture",
        )]);
        let upstream_templates = CapabilityItems::ResourceTemplates(vec![rmcp::model::ResourceTemplate::new(
            "demo://resource/dynamic/text/{resourceId}",
            "Dynamic Text Resource",
        )]);

        for items in [upstream_resources, upstream_templates] {
            let live = project_external_items(&database, "server-docs", items.clone(), NameDomain::External)
                .await
                .expect("project live capability items");
            let cached = project_cached_items(
                &database,
                "server-docs",
                items,
                &CacheScope::shared_raw(),
                NameDomain::External,
            )
            .await
            .expect("project cached capability items");
            assert_eq!(format!("{live:?}"), format!("{cached:?}"));
        }
    }

    #[test]
    fn cached_items_from_data_converts_prompts_correctly() {
        let mut data = make_test_cached_data("srv-1", CacheScope::shared_raw(), 0);
        data.prompts = vec![crate::core::cache::CachedPromptInfo {
            name: "test_prompt".to_string(),
            description: Some("Test prompt".to_string()),
            arguments: Vec::new(),
            icons: None,
            enabled: true,
            cached_at: Utc::now(),
        }];

        let items = cached_items_from_data(CapabilityType::Prompts, data);

        match items {
            CapabilityItems::Prompts(prompts) => {
                assert_eq!(prompts.len(), 1);
                assert_eq!(&prompts[0].name, "test_prompt");
            }
            _ => panic!("Expected Prompts variant"),
        }
    }

    #[test]
    fn cached_items_from_data_converts_resources_correctly() {
        let mut data = make_test_cached_data("srv-1", CacheScope::shared_raw(), 0);
        data.resources = vec![crate::core::cache::CachedResourceInfo {
            uri: "file:///test/resource".to_string(),
            name: Some("test_resource".to_string()),
            description: Some("Test resource".to_string()),
            mime_type: Some("text/plain".to_string()),
            icons: None,
            enabled: true,
            cached_at: Utc::now(),
        }];

        let items = cached_items_from_data(CapabilityType::Resources, data);

        match items {
            CapabilityItems::Resources(resources) => {
                assert_eq!(resources.len(), 1);
                assert_eq!(resources[0].uri, "file:///test/resource");
            }
            _ => panic!("Expected Resources variant"),
        }
    }

    #[test]
    fn capability_items_empty_returns_correct_variant() {
        assert!(matches!(
            CapabilityItems::empty(CapabilityType::Tools),
            CapabilityItems::Tools(v) if v.is_empty()
        ));
        assert!(matches!(
            CapabilityItems::empty(CapabilityType::Prompts),
            CapabilityItems::Prompts(v) if v.is_empty()
        ));
        assert!(matches!(
            CapabilityItems::empty(CapabilityType::Resources),
            CapabilityItems::Resources(v) if v.is_empty()
        ));
        assert!(matches!(
            CapabilityItems::empty(CapabilityType::ResourceTemplates),
            CapabilityItems::ResourceTemplates(v) if v.is_empty()
        ));
    }

    #[test]
    fn capability_items_is_empty_works_correctly() {
        assert!(CapabilityItems::empty(CapabilityType::Tools).is_empty());
        let schema: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        let tool = rmcp::model::Tool::new(
            std::borrow::Cow::Borrowed("test"),
            std::borrow::Cow::Borrowed("desc"),
            std::sync::Arc::new(schema),
        );
        assert!(!CapabilityItems::Tools(vec![tool]).is_empty());
    }

    #[test]
    fn cache_scope_is_client_filtered_returns_correct_value() {
        assert!(!CacheScope::shared_raw().is_client_filtered());
        assert!(CacheScope::client_filtered("key".to_string(), "fp".to_string()).is_client_filtered());
    }
}
