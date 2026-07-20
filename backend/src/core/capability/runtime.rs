use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind as CatalogKind, CapabilityPayload, CatalogCommit, CatalogError, CatalogSnapshot,
    InventoryState, KindObservation, ProjectionKey, ProjectionNameDomain, ProjectionPayload, SnapshotState,
    SqliteCapabilityCatalog,
};
use rmcp::service::{Peer, RoleClient};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;

use crate::config::database::Database;
use crate::core::capability::connection_provider::CapabilityOwner;
#[cfg(test)]
use crate::core::capability::index::{CachedResourceInfo, CachedResourceTemplateInfo};
use crate::core::capability::internal::{
    CapabilityFetchFailure, collect_capability_from_instance_peer, is_method_not_supported,
};
use crate::core::capability::naming::NamingKind;
use crate::core::capability::{CapabilityType, ConnectionSelection, RuntimeIdentity};
use crate::core::pool::{CapSyncFlags, FailureKind, UpstreamConnectionPool};

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
    pub visibility_snapshot: Option<Arc<crate::core::profile::visibility::VisibilitySnapshot>>,
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

fn catalog_kind(capability: CapabilityType) -> CatalogKind {
    match capability {
        CapabilityType::Tools => CatalogKind::Tools,
        CapabilityType::Prompts => CatalogKind::Prompts,
        CapabilityType::Resources => CatalogKind::Resources,
        CapabilityType::ResourceTemplates => CatalogKind::ResourceTemplates,
    }
}

fn items_from_catalog(
    capability: CapabilityType,
    records: Vec<mcpmate_capability_store::CatalogRecord>,
) -> CapabilityItems {
    match capability {
        CapabilityType::Tools => CapabilityItems::Tools(
            records
                .into_iter()
                .filter_map(|record| match record.payload {
                    CapabilityPayload::Tool(tool) => Some(tool),
                    _ => None,
                })
                .collect(),
        ),
        CapabilityType::Prompts => CapabilityItems::Prompts(
            records
                .into_iter()
                .filter_map(|record| match record.payload {
                    CapabilityPayload::Prompt(prompt) => Some(prompt),
                    _ => None,
                })
                .collect(),
        ),
        CapabilityType::Resources => CapabilityItems::Resources(
            records
                .into_iter()
                .filter_map(|record| match record.payload {
                    CapabilityPayload::Resource(resource) => Some(resource),
                    _ => None,
                })
                .collect(),
        ),
        CapabilityType::ResourceTemplates => CapabilityItems::ResourceTemplates(
            records
                .into_iter()
                .filter_map(|record| match record.payload {
                    CapabilityPayload::ResourceTemplate(template) => Some(template),
                    _ => None,
                })
                .collect(),
        ),
    }
}

fn projection_payload(items: CapabilityItems) -> ProjectionPayload {
    match items {
        CapabilityItems::Tools(items) => ProjectionPayload::Tools(items),
        CapabilityItems::Prompts(items) => ProjectionPayload::Prompts(items),
        CapabilityItems::Resources(items) => ProjectionPayload::Resources(items),
        CapabilityItems::ResourceTemplates(items) => ProjectionPayload::ResourceTemplates(items),
    }
}

fn items_from_projection(payload: &ProjectionPayload) -> CapabilityItems {
    match payload {
        ProjectionPayload::Tools(items) => CapabilityItems::Tools(items.clone()),
        ProjectionPayload::Prompts(items) => CapabilityItems::Prompts(items.clone()),
        ProjectionPayload::Resources(items) => CapabilityItems::Resources(items.clone()),
        ProjectionPayload::ResourceTemplates(items) => CapabilityItems::ResourceTemplates(items.clone()),
    }
}

fn projection_key(
    ctx: &ListCtx,
    revision: i64,
) -> ProjectionKey {
    let selection_key = ctx
        .connection_selection
        .as_ref()
        .map(ConnectionSelection::cache_scope_key)
        .unwrap_or_else(|| format!("{}#shared", ctx.server_id));
    let surface_fingerprint = ctx
        .runtime_identity
        .as_ref()
        .map(|identity| identity.surface_fingerprint.clone())
        .unwrap_or_else(|| "shared_raw".to_string());
    let revision_set = format!("{}:{revision}", ctx.server_id);
    let catalog_revision_set_hash = format!("{:x}", Sha256::digest(revision_set));
    ProjectionKey::new(
        selection_key,
        surface_fingerprint,
        catalog_kind(ctx.capability),
        match ctx.name_domain {
            NameDomain::Upstream => ProjectionNameDomain::Upstream,
            NameDomain::External => ProjectionNameDomain::External,
        },
        catalog_revision_set_hash,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeFailureKind {
    Timeout,
    SessionGone,
    TransportClosed,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "generation-aware owners are part of the read-service contract but the current pool has no generation counter"
        )
    )]
    StaleGeneration,
    Authentication,
    Protocol,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "application failures remain part of the typed runtime contract and are exercised by read-service tests"
        )
    )]
    Application,
    Other,
}

impl RuntimeFailureKind {
    pub(crate) const fn retry_disposition(
        self
    ) -> crate::core::capability::connection_provider::DiscoveryRetryDisposition {
        use crate::core::capability::connection_provider::DiscoveryRetryDisposition;

        match self {
            Self::SessionGone | Self::TransportClosed | Self::StaleGeneration => DiscoveryRetryDisposition::FreshOnce,
            Self::Timeout | Self::Authentication | Self::Protocol | Self::Application | Self::Other => {
                DiscoveryRetryDisposition::DoNotRetry
            }
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("capability runtime failure ({kind:?}): {message}", message = .message.as_deref().unwrap_or("no detail"))]
pub(crate) struct RuntimeFailure {
    pub kind: RuntimeFailureKind,
    pub message: Option<String>,
    pub timeout_ms: Option<u128>,
}

/// Protocol observation returned by a capability discovery adapter.
#[derive(Debug)]
pub(crate) struct CapabilityDiscoveryObservation {
    pub(crate) items: CapabilityItems,
    pub(crate) flags: CapSyncFlags,
    pub(crate) kind_states: Vec<KindObservation>,
}

pub(crate) enum CatalogReadFailure {
    Catalog(CatalogError),
    Operation(anyhow::Error),
}

/// Evidence that changes whether a persisted capability snapshot may be exposed.
#[derive(Clone, Debug)]
pub enum CapabilityEvidence {
    RuntimeFailure {
        server_id: String,
        kind: CatalogKind,
        reason: String,
    },
    Invalidated {
        server_id: String,
        reason: String,
    },
}

/// Stable boundary for recording runtime evidence without coupling the store to the pool.
#[async_trait]
pub trait CapabilityEvidenceSink: Send + Sync {
    async fn record(
        &self,
        evidence: CapabilityEvidence,
    ) -> mcpmate_capability_store::Result<CatalogCommit>;
}

#[derive(Clone)]
pub struct SqliteCapabilityEvidenceSink {
    catalog: SqliteCapabilityCatalog,
}

impl SqliteCapabilityEvidenceSink {
    pub fn new(catalog: SqliteCapabilityCatalog) -> Self {
        Self { catalog }
    }
}

#[async_trait]
impl CapabilityEvidenceSink for SqliteCapabilityEvidenceSink {
    async fn record(
        &self,
        evidence: CapabilityEvidence,
    ) -> mcpmate_capability_store::Result<CatalogCommit> {
        match evidence {
            CapabilityEvidence::RuntimeFailure {
                server_id,
                kind,
                reason,
            } => self.catalog.record_failure(&server_id, Some(kind), &reason).await,
            CapabilityEvidence::Invalidated { server_id, reason } => {
                self.catalog.invalidate_server(&server_id, &reason).await
            }
        }
    }
}

pub fn message_indicates_session_gone(msg_lower: &str) -> bool {
    msg_lower.contains("gone") || contains_status_code(msg_lower, "404") || contains_status_code(msg_lower, "410")
}

/// Matches an HTTP status code as a standalone token rather than a bare substring, so arbitrary
/// application content that happens to contain the same digits (a resource URI, a business
/// value, an unrelated numeric id, ...) cannot be misread as a session-gone transport signal.
fn contains_status_code(
    msg_lower: &str,
    code: &str,
) -> bool {
    let is_boundary = |c: Option<char>| !c.is_some_and(|c| c.is_ascii_alphanumeric());
    msg_lower.match_indices(code).any(|(start, matched)| {
        let before = msg_lower[..start].chars().next_back();
        let after = msg_lower[start + matched.len()..].chars().next();
        is_boundary(before) && is_boundary(after)
    })
}

pub(crate) async fn handle_runtime_failure(
    pool: &Arc<Mutex<UpstreamConnectionPool>>,
    server_id: &str,
    instance_id: &str,
    failure: RuntimeFailure,
) {
    let message = failure.message.clone();
    let failure_kind = match failure.kind {
        RuntimeFailureKind::Timeout => FailureKind::RuntimeTimeout,
        RuntimeFailureKind::SessionGone | RuntimeFailureKind::TransportClosed | RuntimeFailureKind::StaleGeneration => {
            FailureKind::RuntimeGone
        }
        RuntimeFailureKind::Authentication
        | RuntimeFailureKind::Protocol
        | RuntimeFailureKind::Application
        | RuntimeFailureKind::Other => FailureKind::RuntimeOther,
    };
    let mut pool_guard = pool.lock().await;
    let _ = pool_guard.register_failure(server_id, failure_kind, message);
    // Only tear down the connection for session-gone errors to avoid penalizing transient timeouts
    if matches!(
        failure.kind,
        RuntimeFailureKind::SessionGone | RuntimeFailureKind::TransportClosed
    ) {
        let _ = pool_guard.disconnect_non_blocking(server_id, instance_id).await;
    }
}

fn runtime_failure_from_capability(failure: Option<CapabilityFetchFailure>) -> Option<RuntimeFailure> {
    failure.map(|f| match f {
        CapabilityFetchFailure::Timeout { timeout_ms } => RuntimeFailure {
            kind: RuntimeFailureKind::Timeout,
            message: None,
            timeout_ms: Some(timeout_ms),
        },
        CapabilityFetchFailure::TransportClosed => RuntimeFailure {
            kind: RuntimeFailureKind::TransportClosed,
            message: Some("transport closed".to_string()),
            timeout_ms: None,
        },
        CapabilityFetchFailure::Unsupported { message } => RuntimeFailure {
            kind: RuntimeFailureKind::Protocol,
            message: Some(message),
            timeout_ms: None,
        },
        CapabilityFetchFailure::Authentication { message } => RuntimeFailure {
            kind: RuntimeFailureKind::Authentication,
            message: Some(message),
            timeout_ms: None,
        },
        CapabilityFetchFailure::Other { message } => RuntimeFailure {
            kind: RuntimeFailureKind::Other,
            message: Some(message),
            timeout_ms: None,
        },
    })
}

/// Execute a tool call using the shared runtime pipeline.
pub async fn call_tool(
    ctx: &CallCtx,
    pool: &Arc<Mutex<UpstreamConnectionPool>>,
) -> Result<rmcp::model::CallToolResult> {
    call_tool_impl(ctx, pool).await
}

pub(crate) async fn try_catalog_read(
    ctx: &ListCtx,
    database: &Arc<Database>,
) -> std::result::Result<Option<ListResult>, CatalogReadFailure> {
    try_catalog_read_with_hook(ctx, database, || async {}).await
}

/// Records a persisted snapshot as untrusted, invalidates the memory cache, and publishes the
/// resulting catalog revision so other readers observe the same fact. Shared by every cache-read
/// integrity check (config drift, shadow index divergence, ...) so each check only needs to
/// decide *whether* the snapshot is untrusted, not how to react once it is.
async fn invalidate_untrusted_snapshot(
    database: &Arc<Database>,
    snapshot: &CatalogSnapshot,
    reason: String,
) -> std::result::Result<(), CatalogReadFailure> {
    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());
    let commit = SqliteCapabilityEvidenceSink::new(catalog)
        .record(CapabilityEvidence::Invalidated {
            server_id: snapshot.server_id.clone(),
            reason,
        })
        .await
        .map_err(CatalogReadFailure::Catalog)?;
    database.capability_cache.invalidate_server(&snapshot.server_id).await;
    crate::config::server::capabilities::publish_catalog_commit(
        &snapshot.server_id,
        &snapshot.server_name,
        commit.revision,
    );
    Ok(())
}

async fn try_catalog_read_with_hook<F, Fut>(
    ctx: &ListCtx,
    database: &Arc<Database>,
    after_snapshot_load: F,
) -> std::result::Result<Option<ListResult>, CatalogReadFailure>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let start = std::time::Instant::now();
    let projection_epoch = database.capability_cache.projection_epoch();
    let (snapshot, raw_memory_hit) = database
        .load_capability_snapshot_typed(&ctx.server_id)
        .await
        .map_err(CatalogReadFailure::Catalog)?;
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };
    after_snapshot_load().await;
    let current_fingerprint =
        crate::config::server::capabilities::current_config_fingerprint(&database.pool, &ctx.server_id)
            .await
            .map_err(CatalogReadFailure::Operation)?;
    if snapshot.config_fingerprint != current_fingerprint {
        invalidate_untrusted_snapshot(
            database,
            &snapshot,
            "server configuration fingerprint changed".to_string(),
        )
        .await?;
        return Ok(None);
    }
    if snapshot.state != SnapshotState::Ready {
        return Ok(None);
    }
    let kind = catalog_kind(ctx.capability);
    let Some(state) = snapshot.kind_states.iter().find(|state| state.kind == kind) else {
        return Ok(None);
    };
    if state.inventory != InventoryState::Complete {
        return Ok(None);
    }
    let shadow_index_trustworthy = crate::config::server::capabilities::shadow_index_matches_catalog_kind(
        &database.pool,
        &ctx.server_id,
        kind,
        &snapshot.records,
    )
    .await
    .map_err(CatalogReadFailure::Operation)?;
    if !shadow_index_trustworthy {
        invalidate_untrusted_snapshot(
            database,
            &snapshot,
            format!("shadow index for {kind:?} diverged from the catalog snapshot"),
        )
        .await?;
        return Ok(None);
    }

    let projected_from_sqlite = Arc::new(AtomicBool::new(false));
    let projector_flag = projected_from_sqlite.clone();
    let projection_database = database.clone();
    let projection_server_id = ctx.server_id.clone();
    let projection_name_domain = ctx.name_domain;
    let visibility_snapshot = ctx.visibility_snapshot.clone();
    let raw_items = items_from_catalog(ctx.capability, snapshot.records.clone());
    let projected = database
        .capability_cache
        .get_or_project_at_epoch(
            projection_key(ctx, snapshot.revision),
            projection_epoch,
            || async move {
                projector_flag.store(true, Ordering::Relaxed);
                let items = project_items_for_context(
                    &projection_database,
                    &projection_server_id,
                    raw_items,
                    projection_name_domain,
                    visibility_snapshot.as_deref(),
                )
                .await?;
                Ok::<_, anyhow::Error>(projection_payload(items))
            },
        )
        .await
        .map_err(CatalogReadFailure::Operation)?;
    let memory_hit = raw_memory_hit && !projected_from_sqlite.load(Ordering::Relaxed);
    Ok(Some(ListResult {
        items: items_from_projection(&projected),
        meta: Meta {
            cache_hit: true,
            source: if memory_hit { "memory_cache" } else { "sqlite_catalog" }.to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            had_peer: false,
        },
    }))
}

pub(crate) async fn discover_owner(
    ctx: &ListCtx,
    owner: &CapabilityOwner,
) -> std::result::Result<CapabilityDiscoveryObservation, RuntimeFailure> {
    if owner.server_id != ctx.server_id {
        return Err(RuntimeFailure {
            kind: RuntimeFailureKind::Other,
            message: Some(format!(
                "owner targets server '{}' instead of '{}'",
                owner.server_id, ctx.server_id
            )),
            timeout_ms: None,
        });
    }
    let (items, flags, failure, kind_states) = fetch_runtime_items(
        ctx.capability,
        owner.peer.clone(),
        ctx.timeout.unwrap_or_else(|| Duration::from_secs(10)),
        &owner.server_id,
        &owner.server_name,
        &owner.instance_id,
    )
    .await
    .map_err(|error| RuntimeFailure {
        kind: RuntimeFailureKind::Other,
        message: Some(error.to_string()),
        timeout_ms: None,
    })?;
    match failure {
        Some(failure) => Err(failure),
        None => Ok(CapabilityDiscoveryObservation {
            items,
            flags,
            kind_states,
        }),
    }
}

pub(crate) async fn commit_discovery_observation(
    owner: &CapabilityOwner,
    observation: &CapabilityDiscoveryObservation,
    database: &Arc<Database>,
) -> Result<i64> {
    let (tools, resources, prompts, templates) = protocol_vectors(&observation.items);
    let commit = crate::config::server::capabilities::commit_capability_protocol_observation(
        &database.pool,
        database.capability_cache.as_ref(),
        &owner.server_id,
        &owner.server_name,
        crate::config::server::capabilities::CapabilityProtocolObservation {
            initialize: owner.peer.peer_info().as_deref().cloned(),
            tools,
            resources,
            prompts,
            templates,
            kinds: observation.flags,
            kind_states: observation.kind_states.clone(),
        },
    )
    .await
    .context("Failed to commit live capability observation")?;
    tracing::debug!(
        server_id = %owner.server_id,
        instance_id = %owner.instance_id,
        owner_source = ?owner.source,
        revision = commit.revision,
        "Committed a live capability observation"
    );
    Ok(commit.revision)
}

pub(crate) async fn project_discovery_observation(
    ctx: &ListCtx,
    owner: &CapabilityOwner,
    items: CapabilityItems,
    committed_revision: i64,
    database: &Arc<Database>,
) -> Result<ListResult> {
    let projection_epoch = database.capability_cache.projection_epoch();
    let (snapshot, _) = database
        .load_capability_snapshot_typed(&owner.server_id)
        .await
        .context("Failed to warm the committed capability snapshot")?;
    let snapshot = snapshot.context("Committed capability snapshot is missing")?;
    let projected = if snapshot.revision == committed_revision {
        let projection_database = database.clone();
        let projection_server_id = owner.server_id.clone();
        let projection_name_domain = ctx.name_domain;
        let visibility_snapshot = ctx.visibility_snapshot.clone();
        database
            .capability_cache
            .get_or_project_at_epoch(
                projection_key(ctx, committed_revision),
                projection_epoch,
                || async move {
                    let items = project_items_for_context(
                        &projection_database,
                        &projection_server_id,
                        items,
                        projection_name_domain,
                        visibility_snapshot.as_deref(),
                    )
                    .await?;
                    Ok::<_, anyhow::Error>(projection_payload(items))
                },
            )
            .await?
    } else {
        tracing::debug!(
            server_id = %owner.server_id,
            committed_revision,
            current_revision = snapshot.revision,
            "Skipped live projection cache warm because a newer catalog revision won"
        );
        Arc::new(projection_payload(
            project_items_for_context(
                database,
                &owner.server_id,
                items,
                ctx.name_domain,
                ctx.visibility_snapshot.as_deref(),
            )
            .await?,
        ))
    };
    tracing::debug!(
        server_id = %owner.server_id,
        instance_id = %owner.instance_id,
        owner_source = ?owner.source,
        committed_revision,
        current_revision = snapshot.revision,
        "Projected and cached a live capability observation"
    );
    Ok(ListResult {
        items: items_from_projection(&projected),
        meta: Meta {
            cache_hit: false,
            source: "live".to_string(),
            duration_ms: 0,
            had_peer: true,
        },
    })
}

pub(crate) async fn record_discovery_failure(
    ctx: &ListCtx,
    _server_name: &str,
    instance_id: Option<&str>,
    connection_generation: Option<u64>,
    reason: &str,
    database: &Arc<Database>,
) -> mcpmate_capability_store::Result<()> {
    crate::config::server::capabilities::record_capability_failure(
        &database.pool,
        database.capability_cache.as_ref(),
        crate::config::server::capabilities::CapabilityFailureEvidence {
            server_id: ctx.server_id.clone(),
            kind: catalog_kind(ctx.capability),
            instance_id: instance_id.map(ToOwned::to_owned),
            connection_generation,
            reason: reason.to_string(),
        },
    )
    .await?;
    Ok(())
}

/// Feeds capability-negating evidence observed while executing `tools/call`, `resources/read`,
/// or `prompts/get` back into the capability catalog, so a stale Ready declaration is not kept
/// serving a capability whose upstream session or transport has actually gone away.
///
/// Only transport-level session-gone/closed evidence qualifies here. Ordinary business or
/// parameter errors returned by the tool/resource/prompt itself must never reach this path,
/// per the correction design's explicit rule against escalating routine failures to a
/// whole-catalog invalidation.
pub(crate) async fn record_capability_usage_evidence(
    database: &Arc<Database>,
    server_id: &str,
    kind: CatalogKind,
    instance_id: Option<&str>,
    error_message: &str,
) {
    if !message_indicates_session_gone(&error_message.to_ascii_lowercase()) {
        return;
    }
    let outcome = crate::config::server::capabilities::record_capability_failure(
        &database.pool,
        database.capability_cache.as_ref(),
        crate::config::server::capabilities::CapabilityFailureEvidence {
            server_id: server_id.to_string(),
            kind,
            instance_id: instance_id.map(ToOwned::to_owned),
            connection_generation: None,
            reason: error_message.to_string(),
        },
    )
    .await;
    if let Err(error) = outcome {
        tracing::warn!(
            server_id,
            ?kind,
            %error,
            "failed to record capability usage evidence for a session/transport failure"
        );
    }
}

fn protocol_vectors(
    items: &CapabilityItems
) -> (
    Vec<rmcp::model::Tool>,
    Vec<rmcp::model::Resource>,
    Vec<rmcp::model::Prompt>,
    Vec<rmcp::model::ResourceTemplate>,
) {
    match items {
        CapabilityItems::Tools(items) => (items.clone(), Vec::new(), Vec::new(), Vec::new()),
        CapabilityItems::Resources(items) => (Vec::new(), items.clone(), Vec::new(), Vec::new()),
        CapabilityItems::Prompts(items) => (Vec::new(), Vec::new(), items.clone(), Vec::new()),
        CapabilityItems::ResourceTemplates(items) => (Vec::new(), Vec::new(), Vec::new(), items.clone()),
    }
}

async fn fetch_runtime_items(
    capability: CapabilityType,
    peer: Peer<RoleClient>,
    timeout: Duration,
    server_id: &str,
    server_name: &str,
    instance_id: &str,
) -> Result<(
    CapabilityItems,
    CapSyncFlags,
    Option<RuntimeFailure>,
    Vec<mcpmate_capability_store::KindObservation>,
)> {
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
            if matches!(&out.failure, Some(CapabilityFetchFailure::Unsupported { .. })) {
                return Ok((
                    CapabilityItems::Tools(Vec::new()),
                    CapSyncFlags::TOOLS,
                    None,
                    vec![crate::config::server::capabilities::unsupported_complete_observation(
                        CatalogKind::Tools,
                    )],
                ));
            }
            let runtime_failure = runtime_failure_from_capability(out.failure);
            if runtime_failure.is_some() {
                return Ok((
                    CapabilityItems::Tools(Vec::new()),
                    CapSyncFlags::TOOLS,
                    runtime_failure,
                    Vec::new(),
                ));
            }
            Ok((
                CapabilityItems::Tools(out.items),
                CapSyncFlags::TOOLS,
                runtime_failure,
                Vec::new(),
            ))
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
            if matches!(&out.failure, Some(CapabilityFetchFailure::Unsupported { .. })) {
                return Ok((
                    CapabilityItems::Prompts(Vec::new()),
                    CapSyncFlags::PROMPTS,
                    None,
                    vec![crate::config::server::capabilities::unsupported_complete_observation(
                        CatalogKind::Prompts,
                    )],
                ));
            }
            let runtime_failure = runtime_failure_from_capability(out.failure);
            if runtime_failure.is_some() {
                return Ok((
                    CapabilityItems::Prompts(Vec::new()),
                    CapSyncFlags::PROMPTS,
                    runtime_failure,
                    Vec::new(),
                ));
            }
            Ok((
                CapabilityItems::Prompts(out.items),
                CapSyncFlags::PROMPTS,
                None,
                Vec::new(),
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
            if matches!(&out.failure, Some(CapabilityFetchFailure::Unsupported { .. })) {
                return Ok((
                    CapabilityItems::Resources(Vec::new()),
                    CapSyncFlags::RESOURCES,
                    None,
                    vec![crate::config::server::capabilities::unsupported_complete_observation(
                        CatalogKind::Resources,
                    )],
                ));
            }
            let runtime_failure = runtime_failure_from_capability(out.failure);
            if runtime_failure.is_some() {
                return Ok((
                    CapabilityItems::Resources(Vec::new()),
                    CapSyncFlags::RESOURCES,
                    runtime_failure,
                    Vec::new(),
                ));
            }
            Ok((
                CapabilityItems::Resources(out.items),
                CapSyncFlags::RESOURCES,
                None,
                Vec::new(),
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
            if matches!(&out.failure, Some(CapabilityFetchFailure::Unsupported { .. })) {
                return Ok((
                    CapabilityItems::ResourceTemplates(Vec::new()),
                    CapSyncFlags::RESOURCE_TEMPLATES,
                    None,
                    vec![crate::config::server::capabilities::unsupported_complete_observation(
                        CatalogKind::ResourceTemplates,
                    )],
                ));
            }
            let runtime_failure = runtime_failure_from_capability(out.failure);
            if runtime_failure.is_some() {
                return Ok((
                    CapabilityItems::ResourceTemplates(Vec::new()),
                    CapSyncFlags::RESOURCE_TEMPLATES,
                    runtime_failure,
                    Vec::new(),
                ));
            }
            Ok((
                CapabilityItems::ResourceTemplates(out.items),
                CapSyncFlags::RESOURCE_TEMPLATES,
                None,
                Vec::new(),
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
                        timeout_ms: None,
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
                        timeout_ms: Some(timeout.as_millis()),
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

#[cfg(test)]
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

#[cfg(test)]
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

async fn project_items_for_context(
    database: &Arc<Database>,
    server_id: &str,
    items: CapabilityItems,
    name_domain: NameDomain,
    visibility_snapshot: Option<&crate::core::profile::visibility::VisibilitySnapshot>,
) -> Result<CapabilityItems> {
    let items = project_external_items(database, server_id, items, name_domain).await?;
    let Some(snapshot) = visibility_snapshot else {
        return Ok(items);
    };
    let visibility = crate::core::profile::visibility::ProfileVisibilityService::new(None, None);
    Ok(match items {
        CapabilityItems::Tools(items) => CapabilityItems::Tools(visibility.filter_tools_with_snapshot(snapshot, items)),
        CapabilityItems::Prompts(items) => {
            CapabilityItems::Prompts(visibility.filter_prompts_with_snapshot(snapshot, items))
        }
        CapabilityItems::Resources(items) => {
            let (resources, _) = visibility.filter_resources_with_snapshot(snapshot, items, Vec::new());
            CapabilityItems::Resources(resources)
        }
        CapabilityItems::ResourceTemplates(items) => {
            let (_, templates) = visibility.filter_resources_with_snapshot(snapshot, Vec::new(), items);
            CapabilityItems::ResourceTemplates(templates)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::capability::index::{CachedResourceInfo, CachedResourceTemplateInfo};
    use crate::core::models::Config;
    use chrono::Utc;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::{path::PathBuf, str::FromStr};
    use tempfile::TempDir;

    async fn list(
        ctx: &ListCtx,
        pool: &Arc<Mutex<UpstreamConnectionPool>>,
        database: &Arc<Database>,
    ) -> Result<ListResult> {
        crate::core::capability::read_service::CapabilityReadService::from_runtime(database.clone(), pool.clone())
            .list(ctx)
            .await
            .map_err(anyhow::Error::from)
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
            capability_cache: Arc::new(mcpmate_capability_store::DerivedCapabilityCache::default()),
        })
    }

    fn runtime_initialize_result() -> rmcp::model::InitializeResult {
        serde_json::from_value(serde_json::json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {"tools": {"listChanged": true}},
            "serverInfo": {"name": "runtime-fixture", "version": "1.0.0"}
        }))
        .expect("fixture must match RMCP 2.2")
    }

    fn runtime_tool(name: &str) -> rmcp::model::Tool {
        serde_json::from_value(serde_json::json!({
            "name": name,
            "description": "Runtime cache fixture",
            "inputSchema": {"type": "object"},
            "outputSchema": {"type": "object"},
            "_meta": {"fixture": "runtime-cache"}
        }))
        .expect("fixture must match RMCP 2.2")
    }

    async fn insert_runtime_server(database: &Database) {
        sqlx::query(
            "INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'runtime_fixture', 'stdio')",
        )
        .execute(&database.pool)
        .await
        .expect("insert server");
    }

    async fn commit_runtime_catalog(
        database: &Database,
        tools: Vec<rmcp::model::Tool>,
    ) {
        crate::config::server::capabilities::commit_protocol_items_for_kinds(
            &database.pool,
            "server-a",
            "runtime_fixture",
            Some(runtime_initialize_result()),
            tools,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            CapSyncFlags::ALL,
        )
        .await
        .expect("commit runtime catalog fixture");
    }

    fn empty_runtime_pool(database: Arc<Database>) -> Arc<Mutex<UpstreamConnectionPool>> {
        Arc::new(Mutex::new(UpstreamConnectionPool::new(
            Arc::new(Config::default()),
            Some(database),
        )))
    }

    fn list_ctx(refresh: RefreshStrategy) -> ListCtx {
        ListCtx {
            capability: CapabilityType::Tools,
            server_id: "server-a".to_string(),
            refresh: Some(refresh),
            timeout: None,
            validation_session: None,
            runtime_identity: None,
            connection_selection: None,
            visibility_snapshot: None,
            name_domain: NameDomain::Upstream,
        }
    }

    #[tokio::test]
    async fn cache_first_serves_ready_sqlite_catalog_without_a_peer() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, vec![runtime_tool("cached-tool")]).await;
        let pool = empty_runtime_pool(database.clone());

        let result = list(&list_ctx(RefreshStrategy::CacheFirst), &pool, &database)
            .await
            .expect("ready SQLite catalog must not require a peer");

        assert!(result.meta.cache_hit);
        assert_eq!(result.meta.source, "sqlite_catalog");
        assert!(!result.meta.had_peer);
        let tools = result.items.into_tools().expect("tools result");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name.as_ref(), "cached-tool");

        let memory_result = list(&list_ctx(RefreshStrategy::CacheFirst), &pool, &database)
            .await
            .expect("second request must reuse the in-process cache");
        assert!(memory_result.meta.cache_hit);
        assert_eq!(memory_result.meta.source, "memory_cache");
        assert!(!memory_result.meta.had_peer);
        assert_eq!(database.capability_cache.metrics().await.raw_loads, 1);
        assert_eq!(database.capability_cache.metrics().await.projection_loads, 1);
    }

    #[tokio::test]
    async fn cache_first_invalidation_after_snapshot_load_does_not_restore_projection() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, vec![runtime_tool("stale-tool")]).await;

        let (snapshot_loaded_tx, snapshot_loaded_rx) = tokio::sync::oneshot::channel();
        let (continue_tx, continue_rx) = tokio::sync::oneshot::channel();
        let read_database = database.clone();
        let read_task = tokio::spawn(async move {
            try_catalog_read_with_hook(&list_ctx(RefreshStrategy::CacheFirst), &read_database, || async move {
                snapshot_loaded_tx.send(()).expect("signal snapshot load");
                continue_rx.await.expect("continue stale read");
            })
            .await
            .unwrap_or_else(|_| panic!("stale request remains request-local"))
            .expect("stale request has a catalog result")
        });

        snapshot_loaded_rx.await.expect("snapshot loaded before invalidation");
        database.capability_cache.invalidate_server("server-a").await;
        continue_tx.send(()).expect("release stale read");

        let stale_result = read_task.await.expect("join stale catalog read");
        assert_eq!(
            stale_result.items.into_tools().expect("stale tools result")[0]
                .name
                .as_ref(),
            "stale-tool"
        );
        assert_eq!(
            database.capability_cache.metrics().await.projection_entries,
            0,
            "an invalidated catalog read must not repopulate the projection cache"
        );

        let fresh_result = try_catalog_read(&list_ctx(RefreshStrategy::CacheFirst), &database)
            .await
            .unwrap_or_else(|_| panic!("fresh catalog read succeeds"))
            .expect("fresh catalog result exists");
        assert_eq!(fresh_result.meta.source, "sqlite_catalog");
        assert_eq!(database.capability_cache.metrics().await.projection_entries, 1);
    }

    #[tokio::test]
    async fn post_restart_cache_first_preserves_all_protocol_payloads_without_a_peer() {
        let temp_dir = TempDir::new().expect("create database directory");
        let database_path = temp_dir.path().join("capability-restart.db");
        let database_url = format!("sqlite://{}", database_path.display());
        let connect = || {
            SqliteConnectOptions::from_str(&database_url)
                .expect("parse database URL")
                .create_if_missing(true)
                .foreign_keys(true)
        };
        let first_pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(connect())
            .await
            .expect("open first database instance");
        crate::config::initialization::run_initialization(&first_pool)
            .await
            .expect("initialize first database instance");
        sqlx::query(
            "INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'runtime_fixture', 'stdio')",
        )
        .execute(&first_pool)
        .await
        .expect("insert server");

        let initialize: rmcp::model::InitializeResult = serde_json::from_value(serde_json::json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "tools": {"listChanged": true},
                "prompts": {"listChanged": true},
                "resources": {"subscribe": true, "listChanged": true}
            },
            "serverInfo": {"name": "restart-fixture", "title": "Restart Fixture", "version": "2.2.0"},
            "instructions": "Persist every standard field.",
            "_meta": {"fixture": "post-restart"}
        }))
        .expect("build initialize fixture");
        let tool: rmcp::model::Tool = serde_json::from_value(serde_json::json!({
            "name": "analyze",
            "title": "Analyze",
            "description": "Analyze input",
            "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}},
            "outputSchema": {"type": "object", "properties": {"result": {"type": "string"}}},
            "icons": [{"src": "https://icons.example/tool.svg", "mimeType": "image/svg+xml"}],
            "_meta": {"fixture": "tool"}
        }))
        .expect("build tool fixture");
        let prompt: rmcp::model::Prompt = serde_json::from_value(serde_json::json!({
            "name": "summarize",
            "title": "Summarize",
            "description": "Summarize input",
            "arguments": [{"name": "document", "title": "Document", "required": true}],
            "icons": [{"src": "https://icons.example/prompt.svg", "mimeType": "image/svg+xml"}],
            "_meta": {"fixture": "prompt"}
        }))
        .expect("build prompt fixture");
        let resource: rmcp::model::Resource = serde_json::from_value(serde_json::json!({
            "uri": "file:///fixture/report.md",
            "name": "report",
            "title": "Fixture Report",
            "description": "Persisted resource",
            "mimeType": "text/markdown",
            "size": 4096,
            "icons": [{"src": "https://icons.example/resource.svg", "mimeType": "image/svg+xml"}],
            "annotations": {"audience": ["user", "assistant"], "priority": 0.75},
            "_meta": {"fixture": "resource"}
        }))
        .expect("build resource fixture");
        let template: rmcp::model::ResourceTemplate = serde_json::from_value(serde_json::json!({
            "uriTemplate": "file:///fixture/{name}.md",
            "name": "fixture-template",
            "title": "Fixture Template",
            "description": "Persisted template",
            "mimeType": "text/markdown",
            "icons": [{"src": "https://icons.example/template.svg", "mimeType": "image/svg+xml"}],
            "annotations": {"audience": ["assistant"], "priority": 0.5},
            "_meta": {"fixture": "template"}
        }))
        .expect("build template fixture");
        crate::config::server::capabilities::commit_protocol_items_for_kinds(
            &first_pool,
            "server-a",
            "runtime_fixture",
            Some(initialize),
            vec![tool.clone()],
            vec![resource.clone()],
            vec![prompt.clone()],
            vec![template.clone()],
            CapSyncFlags::ALL,
        )
        .await
        .expect("commit live protocol observation");
        first_pool.close().await;

        let reopened_pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(connect())
            .await
            .expect("reopen database");
        crate::config::initialization::run_initialization(&reopened_pool)
            .await
            .expect("initialize reopened database");
        let database = Arc::new(Database {
            pool: reopened_pool,
            path: database_path,
            capability_cache: Arc::new(mcpmate_capability_store::DerivedCapabilityCache::default()),
        });
        let pool = empty_runtime_pool(database.clone());
        let fixtures = [
            (CapabilityType::Tools, ProjectionPayload::Tools(vec![tool])),
            (CapabilityType::Prompts, ProjectionPayload::Prompts(vec![prompt])),
            (CapabilityType::Resources, ProjectionPayload::Resources(vec![resource])),
            (
                CapabilityType::ResourceTemplates,
                ProjectionPayload::ResourceTemplates(vec![template]),
            ),
        ];

        for (capability, expected) in fixtures {
            let mut ctx = list_ctx(RefreshStrategy::CacheFirst);
            ctx.capability = capability;
            let result = list(&ctx, &pool, &database)
                .await
                .expect("post-restart SQLite snapshot must be readable without an upstream peer");
            assert_eq!(result.meta.source, "sqlite_catalog");
            assert!(result.meta.cache_hit);
            assert!(!result.meta.had_peer);
            assert_eq!(projection_payload(result.items), expected);
        }
        assert!(pool.lock().await.connections.is_empty());

        let memory_result = list(&list_ctx(RefreshStrategy::CacheFirst), &pool, &database)
            .await
            .expect("second post-restart read must use the node-local LRU");
        assert_eq!(memory_result.meta.source, "memory_cache");
        assert!(!memory_result.meta.had_peer);
    }

    #[tokio::test]
    async fn supported_complete_empty_catalog_is_a_cache_hit_without_discovery() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, Vec::new()).await;
        let pool = empty_runtime_pool(database.clone());

        let result = list(&list_ctx(RefreshStrategy::CacheFirst), &pool, &database)
            .await
            .expect("supported empty catalog must not require discovery");

        assert!(result.meta.cache_hit);
        assert_eq!(result.meta.source, "sqlite_catalog");
        assert!(!result.meta.had_peer);
        assert!(result.items.into_tools().expect("tools result").is_empty());
    }

    #[tokio::test]
    async fn invalidated_catalog_does_not_fall_back_to_last_known_good() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, vec![runtime_tool("stale-tool")]).await;
        SqliteCapabilityCatalog::new(database.pool.clone())
            .invalidate_server("server-a", "explicit validation evidence")
            .await
            .expect("invalidate catalog");
        let pool = empty_runtime_pool(database.clone());

        let error = list(&list_ctx(RefreshStrategy::CacheFirst), &pool, &database)
            .await
            .expect_err("invalidated catalog must require live confirmation");

        assert!(error.to_string().contains("No connected capability peer"));
        let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
            .load_snapshot("server-a")
            .await
            .expect("load catalog")
            .expect("catalog remains available for diagnosis");
        assert_eq!(snapshot.state, SnapshotState::Unavailable);
        assert_eq!(snapshot.records.len(), 1);
    }

    #[tokio::test]
    async fn shadow_index_divergence_invalidates_the_catalog_instead_of_serving_a_stale_read() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, vec![runtime_tool("cached-tool")]).await;

        // Manufacture a divergence between the catalog's tool records and the tools shadow
        // index, simulating a bug that wrote one without the other outside the observation
        // commit. The read path must treat this as untrusted rather than silently serving it.
        sqlx::query("DELETE FROM server_tools WHERE server_id = 'server-a'")
            .execute(&database.pool)
            .await
            .expect("manufacture a shadow index divergence");

        let result = try_catalog_read(&list_ctx(RefreshStrategy::CacheFirst), &database)
            .await
            .unwrap_or_else(|_| panic!("integrity mismatch must not surface as a catalog error"));
        assert!(
            result.is_none(),
            "a diverged shadow index must not be served as a cache hit"
        );

        let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
            .load_snapshot("server-a")
            .await
            .expect("load catalog after integrity mismatch")
            .expect("catalog remains available for diagnosis");
        assert_eq!(snapshot.state, SnapshotState::Invalidated);
        assert_eq!(snapshot.revision, 2);
        assert!(
            snapshot
                .last_error
                .as_deref()
                .is_some_and(|reason| reason.contains("shadow index")),
            "unexpected last_error: {:?}",
            snapshot.last_error
        );

        assert_eq!(
            database.capability_cache.metrics().await.projection_entries,
            0,
            "an untrusted catalog must not populate the projection cache"
        );
    }

    #[tokio::test]
    async fn capability_usage_session_gone_evidence_marks_the_catalog_unavailable() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, vec![runtime_tool("cached-tool")]).await;

        record_capability_usage_evidence(
            &database,
            "server-a",
            CatalogKind::Tools,
            Some("runtime-instance"),
            "session not found (status: 404)",
        )
        .await;

        let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
            .load_snapshot("server-a")
            .await
            .expect("load catalog after usage evidence")
            .expect("catalog remains available for diagnosis");
        assert_eq!(snapshot.state, SnapshotState::Unavailable);
        assert_eq!(snapshot.revision, 2);
        assert!(
            snapshot
                .last_error
                .as_deref()
                .is_some_and(|reason| reason.contains("session not found")),
            "unexpected last_error: {:?}",
            snapshot.last_error
        );
    }

    #[tokio::test]
    async fn capability_usage_ordinary_error_does_not_invalidate_the_catalog() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, vec![runtime_tool("cached-tool")]).await;

        // A tool's own business/parameter error must never be escalated to a whole-catalog
        // invalidation; only transport-level session/connection evidence qualifies.
        record_capability_usage_evidence(
            &database,
            "server-a",
            CatalogKind::Tools,
            Some("runtime-instance"),
            "invalid params: missing required field 'path'",
        )
        .await;

        let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
            .load_snapshot("server-a")
            .await
            .expect("load catalog after ordinary tool error")
            .expect("catalog remains available");
        assert_eq!(snapshot.state, SnapshotState::Ready);
        assert_eq!(snapshot.revision, 1);
    }

    #[test]
    fn session_gone_classifier_requires_a_standalone_status_code_not_an_arbitrary_substring() {
        assert!(message_indicates_session_gone("session not found (status: 404)"));
        assert!(message_indicates_session_gone("upstream returned 410 gone"));
        assert!(message_indicates_session_gone("the transport is gone"));

        // A URI, id, or other business value that merely contains the same digits as an HTTP
        // status code must not be misread as a session-gone transport signal; only Batch 3's
        // catalog-invalidating call site makes this distinction expensive to get wrong.
        assert!(!message_indicates_session_gone(
            "resource not found: file:///reports/q410/report.pdf"
        ));
        assert!(!message_indicates_session_gone(
            "invalid params: expected 44100 but got 22050"
        ));
        assert!(!message_indicates_session_gone(
            "invalid params: missing required field 'path'"
        ));
    }

    #[tokio::test]
    async fn config_fingerprint_change_invalidates_catalog_and_publishes_revision() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, vec![runtime_tool("stale-tool")]).await;
        sqlx::query("UPDATE server_config SET command = 'changed-command' WHERE id = 'server-a'")
            .execute(&database.pool)
            .await
            .expect("change server configuration");
        let pool = empty_runtime_pool(database.clone());
        let mut events = crate::core::events::EventBus::global().subscribe_async();

        let error = list(&list_ctx(RefreshStrategy::CacheFirst), &pool, &database)
            .await
            .expect_err("config mismatch must require live confirmation");

        assert!(error.to_string().contains("No connected capability peer"));
        let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
            .load_snapshot("server-a")
            .await
            .expect("load catalog")
            .expect("catalog remains available for diagnosis");
        assert_eq!(snapshot.state, SnapshotState::Unavailable);
        assert_eq!(snapshot.revision, 3);
        let committed_revision = tokio::time::timeout(Duration::from_secs(1), async move {
            loop {
                match events.recv().await {
                    Ok(crate::core::events::Event::CapabilityCatalogCommitted {
                        server_id, revision, ..
                    }) if server_id == "server-a" => break revision,
                    Ok(_) => continue,
                    Err(error) => panic!("event receiver failed: {error}"),
                }
            }
        })
        .await
        .expect("catalog revision event must be published");
        assert_eq!(committed_revision, 2);
    }

    #[tokio::test]
    async fn failure_commit_event_uses_canonical_database_server_name() {
        let database = test_database().await;
        insert_runtime_server(&database).await;
        commit_runtime_catalog(&database, vec![runtime_tool("stale-tool")]).await;
        let mut events = crate::core::events::EventBus::global().subscribe_async();

        record_discovery_failure(
            &list_ctx(RefreshStrategy::Force),
            "runtime_fixture",
            Some("runtime-instance"),
            None,
            "upstream unavailable",
            &database,
        )
        .await
        .expect("record failure evidence");

        let event_name = tokio::time::timeout(Duration::from_secs(1), async move {
            loop {
                match events.recv().await {
                    Ok(crate::core::events::Event::CapabilityCatalogCommitted {
                        server_id, server_name, ..
                    }) if server_id == "server-a" => break server_name,
                    Ok(_) => continue,
                    Err(error) => panic!("event receiver failed: {error}"),
                }
            }
        })
        .await
        .expect("failure commit event");

        assert_eq!(event_name, "runtime_fixture");
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
            visibility_snapshot: None,
            name_domain: NameDomain::Upstream,
        };

        let error = list(&ctx, &pool, &database)
            .await
            .expect_err("a missing validation peer must not look like an empty inventory");

        assert!(error.to_string().contains("validation-a"));
        assert!(error.to_string().contains("server-a"));
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
}
