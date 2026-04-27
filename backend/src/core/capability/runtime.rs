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
use crate::core::capability::naming::{NamingKind, generate_unique_name};
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
        server_name,
        tools,
        resources,
        prompts,
        resource_templates,
        ..
    } = data;

    match capability {
        CapabilityType::Tools => CapabilityItems::Tools(
            tools
                .into_iter()
                .filter_map(|tool| convert_cached_tool(&server_name, tool))
                .collect(),
        ),
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
                    let items = cached_items_from_data(ctx.capability, data);
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
                        let items = cached_items_from_data(ctx.capability, data);
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
        let snap = pool_guard.get_snapshot();
        let mut peer_opt = None;
        let mut instance_id_opt = None;
        let mut instance_source = OperationSource::Runtime;
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
        if peer_opt.is_none() {
            if let Some(session_id) = ctx.validation_session.as_ref() {
                if let Some(session_servers) = pool_guard.validation_sessions.get(session_id) {
                    if let Some(conn) = session_servers.get(&name) {
                        if let Some(service) = conn.service.as_ref() {
                            peer_opt = Some(service.peer().clone());
                            instance_id_opt = Some(conn.id.clone());
                            instance_source = OperationSource::Temporary;
                        }
                    }
                }
            }
        }
        (peer_opt, instance_id_opt, name, instance_source)
    };

    let peer = match peer_opt {
        Some(p) => p,
        None => {
            return Ok(ListResult {
                items: CapabilityItems::Resources(Vec::new()),
                meta: Meta {
                    cache_hit: false,
                    source: OperationSource::Runtime.as_str().to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    had_peer: false,
                },
            });
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
        handle_runtime_failure(pool, &ctx.server_id, &result.meta.source, failure).await;
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
    )
    .await?;

    if !items.is_empty() {
        let db = database.clone();
        let server_id = server_id.clone();
        let instance_id = instance_id.clone();
        let peer = peer.clone();
        tokio::spawn(async move {
            let _ = UpstreamConnectionPool::sync_capabilities(&db, &server_id, &instance_id, &peer, flags, None).await;
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
            let items = ensure_tool_unique_names(&database, server_id, server_name, out.items).await?;
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
            Ok((
                CapabilityItems::Prompts(out.items),
                CapSyncFlags::PROMPTS,
                runtime_failure_from_capability(out.failure),
            ))
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
            Ok((
                CapabilityItems::Resources(out.items),
                CapSyncFlags::RESOURCES,
                runtime_failure_from_capability(out.failure),
            ))
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
            Ok((
                CapabilityItems::ResourceTemplates(out.items),
                CapSyncFlags::RESOURCE_TEMPLATES,
                runtime_failure_from_capability(out.failure),
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
    let mut raw = rmcp::model::RawResource::new(cached.uri.clone(), resolved_name);
    raw.description = cached.description;
    raw.mime_type = cached.mime_type;
    raw.icons = cached.icons;
    Some(rmcp::model::Resource { raw, annotations: None })
}

fn convert_cached_resource_template(cached: CachedResourceTemplateInfo) -> Option<rmcp::model::ResourceTemplate> {
    let resolved_name = cached
        .name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| cached.uri_template.clone());
    let mut raw = rmcp::model::RawResourceTemplate::new(cached.uri_template, resolved_name);
    if let Some(description) = cached.description {
        raw = raw.with_description(description);
    }
    if let Some(mime_type) = cached.mime_type {
        raw = raw.with_mime_type(mime_type);
    }
    Some(rmcp::model::ResourceTemplate { raw, annotations: None })
}

fn convert_cached_tool(
    server_name: &str,
    cached: CachedToolInfo,
) -> Option<rmcp::model::Tool> {
    let schema_value: serde_json::Value = serde_json::from_str(&cached.input_schema_json).ok()?;
    let schema_object = schema_value.as_object()?.clone();
    let unique_name = cached
        .unique_name
        .clone()
        .unwrap_or_else(|| generate_unique_name(NamingKind::Tool, server_name, &cached.name));
    let mut tool = if let Some(description) = cached.description.map(std::borrow::Cow::Owned) {
        rmcp::model::Tool::new(
            std::borrow::Cow::Owned(unique_name),
            description,
            Arc::new(schema_object),
        )
    } else {
        rmcp::model::Tool::new_with_raw(std::borrow::Cow::Owned(unique_name), None, Arc::new(schema_object))
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
                        unique_name: None,
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
) -> anyhow::Result<Vec<rmcp::model::Tool>> {
    crate::config::server::tools::assign_unique_names_to_tools(&database.pool, server_id, server_name, &mut tools)
        .await
        .context("Failed to assign unique tool names")?;
    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cache::{CacheScope, CachedServerData, CachedToolInfo};
    use chrono::Utc;

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
            protocol_version: "2024-11-05".to_string(),
            tools: (0..tool_count)
                .map(|i| CachedToolInfo {
                    name: format!("tool_{}", i),
                    description: Some(format!("Test tool {}", i)),
                    input_schema_json: r#"{"type":"object"}"#.to_string(),
                    output_schema_json: None,
                    unique_name: None,
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
                // Tools get unique names via generate_unique_name when unique_name is None
                assert!(tools[0].name.as_ref().contains("tool_0"));
            }
            _ => panic!("Expected Tools variant"),
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
