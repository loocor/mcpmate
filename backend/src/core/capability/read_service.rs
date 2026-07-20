use std::{sync::Arc, time::Instant};

use async_trait::async_trait;
use mcpmate_capability_store::CatalogError;
use tokio::sync::Mutex;

use crate::config::database::Database;
use crate::core::capability::{
    CapabilityType,
    connection_provider::{
        CapabilityConnectionProvider, CapabilityOwner, CapabilityOwnerError, DiscoveryRetryDisposition, OwnerSource,
        PoolCapabilityConnectionProvider,
    },
    runtime::{
        self, CapabilityDiscoveryObservation, CatalogReadFailure, ListCtx, ListResult, RefreshStrategy, RuntimeFailure,
        RuntimeFailureKind,
    },
};
use crate::core::pool::UpstreamConnectionPool;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CapabilityReadError {
    #[error("capability catalog is untrusted for server '{server_id}': {source}")]
    CatalogUntrusted {
        server_id: String,
        #[source]
        source: CatalogError,
    },
    #[error("capability catalog read failed for server '{server_id}': {source}")]
    CatalogOperation {
        server_id: String,
        #[source]
        source: anyhow::Error,
    },
    #[error(
        "capability discovery failed for server '{server_name}' ({server_id}) during {operation}; catalog={catalog_error:?}; existing={existing:?}; fresh={fresh:?}"
    )]
    DiscoveryFailed {
        server_id: String,
        server_name: String,
        operation: &'static str,
        kind: CapabilityType,
        catalog_error: Option<CatalogError>,
        existing: Option<DiscoveryAttemptFailure>,
        fresh: Option<DiscoveryAttemptFailure>,
    },
    #[error(
        "capability owner cleanup failed for server '{server_name}' ({server_id}) during {operation}, instance '{instance_id}': {error}"
    )]
    CleanupFailed {
        server_id: String,
        server_name: String,
        operation: &'static str,
        instance_id: String,
        connection_generation: Option<u64>,
        owner_source: OwnerSource,
        #[source]
        error: CapabilityOwnerError,
    },
    #[error(
        "capability projection failed for server '{server_name}' ({server_id}) during {operation}, instance '{instance_id}': {source}"
    )]
    ProjectionFailed {
        server_id: String,
        server_name: String,
        operation: &'static str,
        kind: CapabilityType,
        instance_id: String,
        connection_generation: Option<u64>,
        owner_source: OwnerSource,
        #[source]
        source: CapabilityProjectionFailure,
    },
}

impl CapabilityReadError {
    fn discovery_attempt_ms(
        existing: &Option<DiscoveryAttemptFailure>,
        fresh: &Option<DiscoveryAttemptFailure>,
        extractor: impl Fn(&DiscoveryAttemptFailure) -> Option<u128>,
    ) -> Option<u128> {
        fresh
            .as_ref()
            .and_then(&extractor)
            .or_else(|| existing.as_ref().and_then(extractor))
    }

    pub(crate) fn connection_timeout_ms(&self) -> Option<u128> {
        if let Self::CleanupFailed {
            error: CapabilityOwnerError::Timeout { timeout_ms },
            ..
        } = self
        {
            return Some(*timeout_ms);
        }
        let Self::DiscoveryFailed { existing, fresh, .. } = self else {
            return None;
        };
        Self::discovery_attempt_ms(existing, fresh, DiscoveryAttemptFailure::connection_timeout_ms)
    }

    pub(crate) fn operation_timeout_ms(&self) -> Option<u128> {
        let Self::DiscoveryFailed { existing, fresh, .. } = self else {
            return None;
        };
        Self::discovery_attempt_ms(existing, fresh, DiscoveryAttemptFailure::operation_timeout_ms)
    }

    /// Surfaces an upstream authentication failure reason, when the discovery attempt or
    /// owner cleanup failed because the upstream server rejected our credentials.
    pub(crate) fn authentication_reason(&self) -> Option<&str> {
        if let Self::CleanupFailed {
            error: CapabilityOwnerError::Authentication { reason },
            ..
        } = self
        {
            return Some(reason.as_str());
        }
        let Self::DiscoveryFailed { existing, fresh, .. } = self else {
            return None;
        };
        fresh
            .as_ref()
            .and_then(DiscoveryAttemptFailure::authentication_reason)
            .or_else(|| {
                existing
                    .as_ref()
                    .and_then(DiscoveryAttemptFailure::authentication_reason)
            })
    }
}

enum OwnerReadError {
    Attempt {
        failure: DiscoveryAttemptFailure,
        disposition: DiscoveryRetryDisposition,
    },
    Cleanup(Box<CapabilityReadError>),
    Projection(Box<CapabilityReadError>),
}

#[derive(Debug)]
pub(crate) struct DiscoveryAttemptFailure {
    pub instance_id: Option<String>,
    pub connection_generation: Option<u64>,
    pub source: crate::core::capability::connection_provider::OwnerSource,
    pub error: CapabilityAttemptError,
}

impl DiscoveryAttemptFailure {
    fn owner(
        source: crate::core::capability::connection_provider::OwnerSource,
        error: CapabilityOwnerError,
    ) -> Self {
        Self {
            instance_id: None,
            connection_generation: None,
            source,
            error: CapabilityAttemptError::Owner(error),
        }
    }

    fn from_owner(
        owner: &CapabilityOwner,
        error: CapabilityAttemptError,
    ) -> Self {
        Self {
            instance_id: Some(owner.instance_id.clone()),
            connection_generation: owner.connection_generation,
            source: owner.source,
            error,
        }
    }

    fn runtime(
        owner: &CapabilityOwner,
        error: RuntimeFailure,
    ) -> Self {
        Self::from_owner(owner, CapabilityAttemptError::Runtime(error))
    }

    fn commit(
        owner: &CapabilityOwner,
        error: CapabilityCommitFailure,
    ) -> Self {
        Self::from_owner(owner, CapabilityAttemptError::Commit(error))
    }

    fn connection_timeout_ms(&self) -> Option<u128> {
        match self.error {
            CapabilityAttemptError::Owner(CapabilityOwnerError::Timeout { timeout_ms }) => Some(timeout_ms),
            _ => None,
        }
    }

    fn operation_timeout_ms(&self) -> Option<u128> {
        match &self.error {
            CapabilityAttemptError::Runtime(failure) => failure.timeout_ms,
            _ => None,
        }
    }

    fn authentication_reason(&self) -> Option<&str> {
        match &self.error {
            CapabilityAttemptError::Owner(CapabilityOwnerError::Authentication { reason }) => Some(reason.as_str()),
            CapabilityAttemptError::Runtime(RuntimeFailure {
                kind: RuntimeFailureKind::Authentication,
                message,
                ..
            }) => message.as_deref(),
            _ => None,
        }
    }

    fn summary(&self) -> String {
        format!(
            "source={:?}, instance={:?}, generation={:?}, error={}",
            self.source, self.instance_id, self.connection_generation, self.error
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CapabilityAttemptError {
    #[error(transparent)]
    Owner(#[from] CapabilityOwnerError),
    #[error(transparent)]
    Runtime(#[from] RuntimeFailure),
    #[error(transparent)]
    Commit(#[from] CapabilityCommitFailure),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CapabilityCommitFailure {
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error("capability catalog database commit failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("capability commit failed: {0}")]
    Operation(#[source] anyhow::Error),
}

impl CapabilityCommitFailure {
    fn from_anyhow(error: anyhow::Error) -> Self {
        match error.downcast::<CatalogError>() {
            Ok(error) => Self::Catalog(error),
            Err(error) => match error.downcast::<sqlx::Error>() {
                Ok(error) => Self::Database(error),
                Err(error) => Self::Operation(error),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("capability projection failed: {0}")]
pub(crate) struct CapabilityProjectionFailure(#[source] anyhow::Error);

#[async_trait]
pub(crate) trait CapabilityReadBackend: Send + Sync {
    async fn try_cache_first(
        &self,
        ctx: &ListCtx,
    ) -> Result<Option<ListResult>, CapabilityReadError>;
    async fn discover(
        &self,
        ctx: &ListCtx,
        owner: &CapabilityOwner,
    ) -> Result<CapabilityDiscoveryObservation, RuntimeFailure>;
    async fn canonical_server_name(
        &self,
        ctx: &ListCtx,
    ) -> Result<String, CapabilityReadError>;
    async fn commit_observation(
        &self,
        owner: &CapabilityOwner,
        observation: &CapabilityDiscoveryObservation,
    ) -> Result<i64, CapabilityCommitFailure>;
    async fn project_observation(
        &self,
        ctx: &ListCtx,
        owner: &CapabilityOwner,
        items: crate::core::capability::runtime::CapabilityItems,
        committed_revision: i64,
    ) -> Result<ListResult, CapabilityProjectionFailure>;
    async fn record_failure(
        &self,
        ctx: &ListCtx,
        server_name: &str,
        instance_id: Option<&str>,
        connection_generation: Option<u64>,
        reason: &str,
    ) -> Result<(), CatalogError>;
}

pub(crate) struct CapabilityReadService {
    backend: Arc<dyn CapabilityReadBackend>,
    connection_provider: Arc<dyn CapabilityConnectionProvider>,
}

struct RuntimeCapabilityReadBackend {
    database: Arc<Database>,
    pool: Option<Arc<Mutex<UpstreamConnectionPool>>>,
}

async fn apply_owner_runtime_failure(
    pool: Option<&Arc<Mutex<UpstreamConnectionPool>>>,
    owner: &CapabilityOwner,
    failure: &RuntimeFailure,
) {
    if owner.source != OwnerSource::Existing {
        return;
    }
    if let Some(pool) = pool {
        runtime::handle_runtime_failure(pool, &owner.server_id, &owner.instance_id, failure.clone()).await;
    }
}

#[async_trait]
impl CapabilityReadBackend for RuntimeCapabilityReadBackend {
    async fn try_cache_first(
        &self,
        ctx: &ListCtx,
    ) -> Result<Option<ListResult>, CapabilityReadError> {
        runtime::try_catalog_read(ctx, &self.database)
            .await
            .map_err(|error| match error {
                CatalogReadFailure::Catalog(source) => CapabilityReadError::CatalogUntrusted {
                    server_id: ctx.server_id.clone(),
                    source,
                },
                CatalogReadFailure::Operation(source) => CapabilityReadError::CatalogOperation {
                    server_id: ctx.server_id.clone(),
                    source,
                },
            })
    }

    async fn discover(
        &self,
        ctx: &ListCtx,
        owner: &CapabilityOwner,
    ) -> Result<CapabilityDiscoveryObservation, RuntimeFailure> {
        let result = runtime::discover_owner(ctx, owner).await;
        if let Err(failure) = &result {
            apply_owner_runtime_failure(self.pool.as_ref(), owner, failure).await;
        }
        result
    }

    async fn canonical_server_name(
        &self,
        ctx: &ListCtx,
    ) -> Result<String, CapabilityReadError> {
        let server = crate::config::server::get_server_by_id(&self.database.pool, &ctx.server_id)
            .await
            .map_err(|source| CapabilityReadError::CatalogOperation {
                server_id: ctx.server_id.clone(),
                source,
            })?
            .ok_or_else(|| CapabilityReadError::CatalogOperation {
                server_id: ctx.server_id.clone(),
                source: anyhow::anyhow!("server '{}' is missing from the canonical database", ctx.server_id),
            })?;
        crate::config::server::validate_server_namespace(&server.name).map_err(|source| {
            CapabilityReadError::CatalogOperation {
                server_id: ctx.server_id.clone(),
                source: source.into(),
            }
        })?;
        Ok(server.name)
    }

    async fn commit_observation(
        &self,
        owner: &CapabilityOwner,
        observation: &CapabilityDiscoveryObservation,
    ) -> Result<i64, CapabilityCommitFailure> {
        runtime::commit_discovery_observation(owner, observation, &self.database)
            .await
            .map_err(CapabilityCommitFailure::from_anyhow)
    }

    async fn project_observation(
        &self,
        ctx: &ListCtx,
        owner: &CapabilityOwner,
        items: crate::core::capability::runtime::CapabilityItems,
        committed_revision: i64,
    ) -> Result<ListResult, CapabilityProjectionFailure> {
        runtime::project_discovery_observation(ctx, owner, items, committed_revision, &self.database)
            .await
            .map_err(CapabilityProjectionFailure)
    }

    async fn record_failure(
        &self,
        ctx: &ListCtx,
        server_name: &str,
        instance_id: Option<&str>,
        connection_generation: Option<u64>,
        reason: &str,
    ) -> Result<(), CatalogError> {
        runtime::record_discovery_failure(
            ctx,
            server_name,
            instance_id,
            connection_generation,
            reason,
            &self.database,
        )
        .await
    }
}

impl CapabilityReadService {
    pub(crate) fn new(
        database: Arc<Database>,
        connection_provider: Arc<dyn CapabilityConnectionProvider>,
    ) -> Self {
        Self::with_backend(
            Arc::new(RuntimeCapabilityReadBackend { database, pool: None }),
            connection_provider,
        )
    }

    pub(crate) fn from_runtime(
        database: Arc<Database>,
        pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        let connection_provider = Arc::new(PoolCapabilityConnectionProvider::new(pool.clone(), database.clone()));
        let mut service = Self::new(database.clone(), connection_provider);
        service.backend = Arc::new(RuntimeCapabilityReadBackend {
            database,
            pool: Some(pool),
        });
        service
    }

    fn with_backend(
        backend: Arc<dyn CapabilityReadBackend>,
        connection_provider: Arc<dyn CapabilityConnectionProvider>,
    ) -> Self {
        Self {
            backend,
            connection_provider,
        }
    }

    pub(crate) async fn list(
        &self,
        ctx: &ListCtx,
    ) -> Result<ListResult, CapabilityReadError> {
        let started = Instant::now();
        let mut catalog_error = None;
        if !matches!(ctx.refresh, Some(RefreshStrategy::Force)) {
            match self.backend.try_cache_first(ctx).await {
                Ok(Some(mut result)) => {
                    result.meta.duration_ms = started.elapsed().as_millis() as u64;
                    return Ok(result);
                }
                Ok(None) => {}
                Err(CapabilityReadError::CatalogUntrusted { server_id, source }) => {
                    if is_replaceable_catalog_error(&source) {
                        catalog_error = Some(source);
                    } else {
                        return Err(CapabilityReadError::CatalogUntrusted { server_id, source });
                    }
                }
                Err(error) => return Err(error),
            }
        }
        let server_name = self.backend.canonical_server_name(ctx).await?;
        let mut result = self
            .discover_existing_then_fresh(ctx, &server_name, catalog_error)
            .await?;
        result.meta.duration_ms = started.elapsed().as_millis() as u64;
        Ok(result)
    }

    async fn discover_existing_then_fresh(
        &self,
        ctx: &ListCtx,
        server_name: &str,
        mut catalog_error: Option<CatalogError>,
    ) -> Result<ListResult, CapabilityReadError> {
        let (existing_error, disposition) = match self.connection_provider.existing_owner(ctx).await {
            Ok(Some(owner)) => match self
                .discover_with_owner(ctx, server_name, owner, &mut catalog_error)
                .await
            {
                Ok(result) => return Ok(result),
                Err(OwnerReadError::Attempt { failure, disposition }) => (Some(failure), disposition),
                Err(OwnerReadError::Cleanup(error) | OwnerReadError::Projection(error)) => {
                    return Err(*error);
                }
            },
            Ok(None) => {
                let error = CapabilityOwnerError::Missing {
                    reason: match ctx.validation_session.as_ref() {
                        Some(session_id) => format!(
                            "Validation session '{}' has no connected peer for server '{}'",
                            session_id, ctx.server_id
                        ),
                        None => format!("No connected capability peer for server '{}'", ctx.server_id),
                    },
                };
                (
                    Some(DiscoveryAttemptFailure::owner(OwnerSource::Existing, error)),
                    DiscoveryRetryDisposition::FreshOnce,
                )
            }
            Err(error) => {
                let disposition = error.retry_disposition();
                let reason = error.to_string();
                self.record_failure(ctx, server_name, None, None, &reason, &mut catalog_error)
                    .await;
                (
                    Some(DiscoveryAttemptFailure::owner(OwnerSource::Existing, error)),
                    disposition,
                )
            }
        };

        if disposition == DiscoveryRetryDisposition::DoNotRetry {
            return Err(discovery_error(ctx, server_name, catalog_error, existing_error, None));
        }

        let fresh_owner = match self.connection_provider.fresh_owner(ctx).await {
            Ok(owner) => owner,
            Err(error) => {
                let reason = error.to_string();
                self.record_failure(ctx, server_name, None, None, &reason, &mut catalog_error)
                    .await;
                return Err(discovery_error(
                    ctx,
                    server_name,
                    catalog_error,
                    existing_error,
                    Some(DiscoveryAttemptFailure::owner(OwnerSource::Fresh, error)),
                ));
            }
        };

        match self
            .discover_with_owner(ctx, server_name, fresh_owner, &mut catalog_error)
            .await
        {
            Ok(result) => Ok(result),
            Err(OwnerReadError::Attempt { failure, .. }) => Err(discovery_error(
                ctx,
                server_name,
                catalog_error,
                existing_error,
                Some(failure),
            )),
            Err(OwnerReadError::Cleanup(error) | OwnerReadError::Projection(error)) => Err(*error),
        }
    }

    async fn discover_with_owner(
        &self,
        ctx: &ListCtx,
        server_name: &str,
        owner: CapabilityOwner,
        catalog_error: &mut Option<CatalogError>,
    ) -> Result<ListResult, OwnerReadError> {
        let attempt = self.backend.discover(ctx, &owner).await;
        match attempt {
            Ok(observation) => {
                let committed_revision = match self.backend.commit_observation(&owner, &observation).await {
                    Ok(revision) => revision,
                    Err(failure) => {
                        let reason = owner_attempt_reason(&owner, &failure);
                        self.record_owner_failure(ctx, server_name, &owner, &reason, catalog_error)
                            .await;
                        let attempt = DiscoveryAttemptFailure::commit(&owner, failure);
                        self.release_after_failed_attempt(ctx, owner).await;
                        return Err(OwnerReadError::Attempt {
                            failure: attempt,
                            disposition: DiscoveryRetryDisposition::DoNotRetry,
                        });
                    }
                };

                let result = self
                    .backend
                    .project_observation(ctx, &owner, observation.items, committed_revision)
                    .await;
                let source = owner.source;
                let instance_id = owner.instance_id.clone();
                let connection_generation = owner.connection_generation;
                match result {
                    Ok(result) => match self.connection_provider.release_owner(owner).await {
                        Ok(()) => Ok(result),
                        Err(error) => Err(OwnerReadError::Cleanup(Box::new(CapabilityReadError::CleanupFailed {
                            server_id: ctx.server_id.clone(),
                            server_name: server_name.to_string(),
                            operation: capability_operation(ctx.capability),
                            instance_id,
                            connection_generation,
                            owner_source: source,
                            error,
                        }))),
                    },
                    Err(projection_failure) => {
                        if let Err(error) = self.connection_provider.release_owner(owner).await {
                            tracing::warn!(
                                server_id = %ctx.server_id,
                                capability = ?ctx.capability,
                                error = %error,
                                "Capability owner cleanup failed after projection failure"
                            );
                        }
                        Err(OwnerReadError::Projection(Box::new(
                            CapabilityReadError::ProjectionFailed {
                                server_id: ctx.server_id.clone(),
                                server_name: server_name.to_string(),
                                operation: capability_operation(ctx.capability),
                                kind: ctx.capability,
                                instance_id,
                                connection_generation,
                                owner_source: source,
                                source: projection_failure,
                            },
                        )))
                    }
                }
            }
            Err(failure) => {
                let disposition = failure.kind.retry_disposition();
                let reason = owner_attempt_reason(&owner, &failure);
                self.record_owner_failure(ctx, server_name, &owner, &reason, catalog_error)
                    .await;
                let attempt = DiscoveryAttemptFailure::runtime(&owner, failure);
                self.release_after_failed_attempt(ctx, owner).await;
                Err(OwnerReadError::Attempt {
                    failure: attempt,
                    disposition,
                })
            }
        }
    }

    async fn release_after_failed_attempt(
        &self,
        ctx: &ListCtx,
        owner: CapabilityOwner,
    ) {
        if let Err(error) = self.connection_provider.release_owner(owner).await {
            tracing::warn!(
                server_id = %ctx.server_id,
                capability = ?ctx.capability,
                error = %error,
                "Capability owner cleanup failed after discovery"
            );
        }
    }

    async fn record_owner_failure(
        &self,
        ctx: &ListCtx,
        server_name: &str,
        owner: &CapabilityOwner,
        reason: &str,
        catalog_error: &mut Option<CatalogError>,
    ) {
        self.record_failure(
            ctx,
            server_name,
            Some(&owner.instance_id),
            owner.connection_generation,
            reason,
            catalog_error,
        )
        .await;
    }

    async fn record_failure(
        &self,
        ctx: &ListCtx,
        server_name: &str,
        instance_id: Option<&str>,
        connection_generation: Option<u64>,
        reason: &str,
        catalog_error: &mut Option<CatalogError>,
    ) {
        if let Err(error) = self
            .backend
            .record_failure(ctx, server_name, instance_id, connection_generation, reason)
            .await
        {
            tracing::warn!(
                server_id = %ctx.server_id,
                capability = ?ctx.capability,
                error = %error,
                "Capability failure evidence could not be persisted"
            );
            if catalog_error.is_none() {
                *catalog_error = Some(error);
            }
        }
    }
}

fn is_replaceable_catalog_error(error: &CatalogError) -> bool {
    matches!(
        error,
        CatalogError::Json(_)
            | CatalogError::UnsupportedRecordVersion { .. }
            | CatalogError::InvalidValue { .. }
            | CatalogError::InvalidTimestamp { .. }
    )
}

fn owner_attempt_reason(
    owner: &CapabilityOwner,
    detail: &impl std::fmt::Display,
) -> String {
    format!(
        "owner '{}' generation {:?}: {}",
        owner.instance_id, owner.connection_generation, detail
    )
}

fn discovery_error(
    ctx: &ListCtx,
    server_name: &str,
    catalog_error: Option<CatalogError>,
    existing: Option<DiscoveryAttemptFailure>,
    fresh: Option<DiscoveryAttemptFailure>,
) -> CapabilityReadError {
    tracing::debug!(
        server_id = %ctx.server_id,
        capability = ?ctx.capability,
        existing = ?existing.as_ref().map(DiscoveryAttemptFailure::summary),
        fresh = ?fresh.as_ref().map(DiscoveryAttemptFailure::summary),
        "Capability discovery exhausted its permitted owners"
    );
    CapabilityReadError::DiscoveryFailed {
        server_id: ctx.server_id.clone(),
        server_name: server_name.to_string(),
        operation: capability_operation(ctx.capability),
        kind: ctx.capability,
        catalog_error,
        existing,
        fresh,
    }
}

const fn capability_operation(capability: CapabilityType) -> &'static str {
    match capability {
        CapabilityType::Tools => "tools/list",
        CapabilityType::Prompts => "prompts/list",
        CapabilityType::Resources => "resources/list",
        CapabilityType::ResourceTemplates => "resources/templates/list",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use async_trait::async_trait;
    use mcpmate_capability_store::{
        CapabilityCatalog, CapabilityKind as CatalogKind, CapabilityPayload, CatalogError, DeclarationState,
        InventoryState, KindObservation, SnapshotState, SqliteCapabilityCatalog,
    };
    use rmcp::{
        ServerHandler, ServiceExt,
        service::{Peer, RoleClient, RunningService},
    };
    use tokio::sync::Mutex;

    use super::{
        CapabilityAttemptError, CapabilityCommitFailure, CapabilityProjectionFailure, CapabilityReadBackend,
        CapabilityReadError, CapabilityReadService, DiscoveryAttemptFailure, RuntimeCapabilityReadBackend,
        apply_owner_runtime_failure,
    };
    use crate::config::database::Database;
    use crate::core::capability::{
        CapabilityType,
        connection_provider::{CapabilityConnectionProvider, CapabilityOwner, CapabilityOwnerError, OwnerSource},
        runtime::{
            CapabilityDiscoveryObservation, CapabilityItems, ListCtx, ListResult, Meta, NameDomain, RefreshStrategy,
            RuntimeFailure, RuntimeFailureKind,
        },
    };
    use crate::core::{
        events::{Event, EventBus},
        models::Config,
        pool::{CapSyncFlags, FailureKind, UpstreamConnection, UpstreamConnectionPool},
        transport::client::UpstreamClientHandler,
    };

    #[derive(Clone, Default)]
    struct TestServer;

    impl ServerHandler for TestServer {}

    struct TestPeerFixture {
        peer: Peer<RoleClient>,
        client: Option<RunningService<RoleClient, ()>>,
        server_task: tokio::task::JoinHandle<()>,
    }

    impl TestPeerFixture {
        async fn shutdown(mut self) {
            drop(self.peer);
            let mut client = self.client.take().expect("test client owner should exist");
            client.close().await.expect("test client should close");
            self.server_task.await.expect("test server task should join");
        }
    }

    #[derive(Clone, Debug)]
    struct EvidenceRecord {
        server_id: String,
        kind: CapabilityType,
        instance_id: Option<String>,
        connection_generation: Option<u64>,
        reason: String,
    }

    struct FakeBackend {
        cache_result: Mutex<Option<Result<Option<ListResult>, CapabilityReadError>>>,
        cache_calls: AtomicUsize,
        discoveries: Mutex<VecDeque<Result<CapabilityDiscoveryObservation, RuntimeFailure>>>,
        evidence: Mutex<Vec<EvidenceRecord>>,
        evidence_error: Mutex<Option<CatalogError>>,
        projection_error: Mutex<Option<CapabilityProjectionFailure>>,
        commits: AtomicUsize,
    }

    struct CommitFailureBackend {
        runtime: RuntimeCapabilityReadBackend,
        observation: Mutex<Option<CapabilityDiscoveryObservation>>,
        projection_error: Mutex<Option<CapabilityProjectionFailure>>,
    }

    #[async_trait]
    impl CapabilityReadBackend for CommitFailureBackend {
        async fn try_cache_first(
            &self,
            _ctx: &ListCtx,
        ) -> Result<Option<ListResult>, CapabilityReadError> {
            Ok(None)
        }

        async fn discover(
            &self,
            _ctx: &ListCtx,
            _owner: &CapabilityOwner,
        ) -> Result<CapabilityDiscoveryObservation, RuntimeFailure> {
            Ok(self
                .observation
                .lock()
                .await
                .take()
                .expect("commit fixture observation"))
        }

        async fn canonical_server_name(
            &self,
            ctx: &ListCtx,
        ) -> Result<String, CapabilityReadError> {
            self.runtime.canonical_server_name(ctx).await
        }

        async fn commit_observation(
            &self,
            owner: &CapabilityOwner,
            observation: &CapabilityDiscoveryObservation,
        ) -> Result<i64, CapabilityCommitFailure> {
            self.runtime.commit_observation(owner, observation).await
        }

        async fn project_observation(
            &self,
            ctx: &ListCtx,
            owner: &CapabilityOwner,
            items: crate::core::capability::runtime::CapabilityItems,
            committed_revision: i64,
        ) -> Result<ListResult, CapabilityProjectionFailure> {
            if let Some(error) = self.projection_error.lock().await.take() {
                return Err(error);
            }
            self.runtime
                .project_observation(ctx, owner, items, committed_revision)
                .await
        }

        async fn record_failure(
            &self,
            ctx: &ListCtx,
            server_name: &str,
            instance_id: Option<&str>,
            connection_generation: Option<u64>,
            reason: &str,
        ) -> Result<(), CatalogError> {
            self.runtime
                .record_failure(ctx, server_name, instance_id, connection_generation, reason)
                .await
        }
    }

    impl FakeBackend {
        fn new(cache_result: Result<Option<ListResult>, CapabilityReadError>) -> Self {
            Self {
                cache_result: Mutex::new(Some(cache_result)),
                cache_calls: AtomicUsize::new(0),
                discoveries: Mutex::new(VecDeque::new()),
                evidence: Mutex::new(Vec::new()),
                evidence_error: Mutex::new(None),
                projection_error: Mutex::new(None),
                commits: AtomicUsize::new(0),
            }
        }

        async fn push_discovery(
            &self,
            result: Result<CapabilityDiscoveryObservation, RuntimeFailure>,
        ) {
            self.discoveries.lock().await.push_back(result);
        }
    }

    #[async_trait]
    impl CapabilityReadBackend for FakeBackend {
        async fn try_cache_first(
            &self,
            _ctx: &ListCtx,
        ) -> Result<Option<ListResult>, CapabilityReadError> {
            self.cache_calls.fetch_add(1, Ordering::Relaxed);
            self.cache_result.lock().await.take().unwrap_or(Ok(None))
        }

        async fn discover(
            &self,
            _ctx: &ListCtx,
            _owner: &CapabilityOwner,
        ) -> Result<CapabilityDiscoveryObservation, RuntimeFailure> {
            self.discoveries
                .lock()
                .await
                .pop_front()
                .expect("a discovery result must be configured")
        }

        async fn canonical_server_name(
            &self,
            _ctx: &ListCtx,
        ) -> Result<String, CapabilityReadError> {
            Ok("docs".to_string())
        }

        async fn commit_observation(
            &self,
            _owner: &CapabilityOwner,
            _observation: &CapabilityDiscoveryObservation,
        ) -> Result<i64, CapabilityCommitFailure> {
            self.commits.fetch_add(1, Ordering::Relaxed);
            Ok(1)
        }

        async fn project_observation(
            &self,
            _ctx: &ListCtx,
            owner: &CapabilityOwner,
            items: crate::core::capability::runtime::CapabilityItems,
            _committed_revision: i64,
        ) -> Result<ListResult, CapabilityProjectionFailure> {
            if let Some(error) = self.projection_error.lock().await.take() {
                return Err(error);
            }
            Ok(ListResult {
                items,
                meta: Meta {
                    cache_hit: false,
                    source: match owner.source {
                        OwnerSource::Existing => "live_existing",
                        OwnerSource::Fresh => "live_fresh",
                        OwnerSource::Validation => "live_validation",
                    }
                    .to_string(),
                    duration_ms: 0,
                    had_peer: true,
                },
            })
        }

        async fn record_failure(
            &self,
            ctx: &ListCtx,
            _server_name: &str,
            instance_id: Option<&str>,
            connection_generation: Option<u64>,
            reason: &str,
        ) -> Result<(), CatalogError> {
            self.evidence.lock().await.push(EvidenceRecord {
                server_id: ctx.server_id.clone(),
                kind: ctx.capability,
                instance_id: instance_id.map(ToOwned::to_owned),
                connection_generation,
                reason: reason.to_string(),
            });
            match self.evidence_error.lock().await.take() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }

    struct FakeProvider {
        peer: rmcp::service::Peer<rmcp::service::RoleClient>,
        existing_result: Mutex<Option<Result<bool, CapabilityOwnerError>>>,
        fresh_result: Mutex<Option<Result<(), CapabilityOwnerError>>>,
        existing_calls: AtomicUsize,
        fresh_calls: AtomicUsize,
        acquisition_order: Mutex<Vec<&'static str>>,
        released: Mutex<Vec<OwnerSource>>,
        release_error: Mutex<Option<CapabilityOwnerError>>,
    }

    impl FakeProvider {
        fn new(peer: rmcp::service::Peer<rmcp::service::RoleClient>) -> Self {
            Self {
                peer,
                existing_result: Mutex::new(Some(Ok(false))),
                fresh_result: Mutex::new(Some(Ok(()))),
                existing_calls: AtomicUsize::new(0),
                fresh_calls: AtomicUsize::new(0),
                acquisition_order: Mutex::new(Vec::new()),
                released: Mutex::new(Vec::new()),
                release_error: Mutex::new(None),
            }
        }

        async fn set_existing(
            &self,
            result: Result<bool, CapabilityOwnerError>,
        ) {
            *self.existing_result.lock().await = Some(result);
        }

        fn owner(
            &self,
            source: OwnerSource,
            sequence: usize,
        ) -> CapabilityOwner {
            CapabilityOwner {
                server_id: "server-1".to_string(),
                server_name: "docs".to_string(),
                instance_id: format!("{source:?}-{sequence}"),
                connection_generation: None,
                peer: self.peer.clone(),
                source,
                cleanup: None,
            }
        }
    }

    #[async_trait]
    impl CapabilityConnectionProvider for FakeProvider {
        async fn existing_owner(
            &self,
            _ctx: &ListCtx,
        ) -> Result<Option<CapabilityOwner>, CapabilityOwnerError> {
            let sequence = self.existing_calls.fetch_add(1, Ordering::Relaxed) + 1;
            self.acquisition_order.lock().await.push("existing");
            match self.existing_result.lock().await.take().unwrap_or(Ok(false)) {
                Ok(true) => Ok(Some(self.owner(OwnerSource::Existing, sequence))),
                Ok(false) => Ok(None),
                Err(error) => Err(error),
            }
        }

        async fn fresh_owner(
            &self,
            _ctx: &ListCtx,
        ) -> Result<CapabilityOwner, CapabilityOwnerError> {
            let sequence = self.fresh_calls.fetch_add(1, Ordering::Relaxed) + 1;
            self.acquisition_order.lock().await.push("fresh");
            match self.fresh_result.lock().await.take().unwrap_or(Ok(())) {
                Ok(()) => Ok(self.owner(OwnerSource::Fresh, sequence)),
                Err(error) => Err(error),
            }
        }

        async fn release_owner(
            &self,
            owner: CapabilityOwner,
        ) -> Result<(), CapabilityOwnerError> {
            self.released.lock().await.push(owner.source);
            match self.release_error.lock().await.take() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }

    fn list_ctx(refresh: Option<RefreshStrategy>) -> ListCtx {
        ListCtx {
            capability: CapabilityType::Tools,
            server_id: "server-1".to_string(),
            refresh,
            timeout: Some(Duration::from_secs(1)),
            validation_session: None,
            runtime_identity: None,
            connection_selection: None,
            visibility_snapshot: None,
            name_domain: NameDomain::Upstream,
        }
    }

    fn result(source: &str) -> ListResult {
        ListResult {
            items: CapabilityItems::Tools(Vec::new()),
            meta: Meta {
                cache_hit: true,
                source: source.to_string(),
                duration_ms: 0,
                had_peer: false,
            },
        }
    }

    fn observation() -> CapabilityDiscoveryObservation {
        CapabilityDiscoveryObservation {
            items: CapabilityItems::Tools(Vec::new()),
            flags: CapSyncFlags::TOOLS,
            kind_states: vec![KindObservation::new(
                CatalogKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
        }
    }

    fn failure(
        kind: RuntimeFailureKind,
        message: &str,
    ) -> RuntimeFailure {
        RuntimeFailure {
            kind,
            message: Some(message.to_string()),
            timeout_ms: None,
        }
    }

    async fn test_peer() -> TestPeerFixture {
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_task = tokio::spawn(async move {
            let server = TestServer
                .serve(server_transport)
                .await
                .expect("test server should initialize");
            server.waiting().await.expect("test server should stop");
        });
        let client = ().serve(client_transport).await.expect("test client should initialize");
        let peer = client.peer().clone();
        TestPeerFixture {
            peer,
            client: Some(client),
            server_task,
        }
    }

    async fn runtime_database() -> Arc<Database> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
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
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-1', 'docs', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server fixture");
        Arc::new(Database {
            pool,
            path: PathBuf::new(),
            capability_cache: Arc::new(mcpmate_capability_store::DerivedCapabilityCache::default()),
        })
    }

    async fn commit_failure_database() -> Arc<Database> {
        let database = runtime_database().await;
        sqlx::query(
            "CREATE TRIGGER fail_capability_commit BEFORE INSERT ON server_tools BEGIN SELECT RAISE(ABORT, 'typed commit fixture'); END",
        )
        .execute(&database.pool)
        .await
        .expect("install commit failure trigger");
        database
    }

    async fn pooled_owner(
        source: OwnerSource
    ) -> (
        Arc<Mutex<UpstreamConnectionPool>>,
        CapabilityOwner,
        tokio::task::JoinHandle<()>,
    ) {
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_task = tokio::spawn(async move {
            let server = TestServer
                .serve(server_transport)
                .await
                .expect("test server should initialize");
            server.waiting().await.expect("test server should stop");
        });
        let service = UpstreamClientHandler::new("docs".to_string())
            .serve(client_transport)
            .await
            .expect("test client should initialize");
        let peer = service.peer().clone();
        let mut connection = UpstreamConnection::new("docs".to_string());
        connection.id = "owner-1".to_string();
        connection.update_connected(service, Vec::new(), Some(rmcp::model::ServerCapabilities::default()));
        let mut pool = UpstreamConnectionPool::new(Arc::new(Config::default()), None);
        pool.connections
            .entry("server-1".to_string())
            .or_default()
            .insert("owner-1".to_string(), connection);
        let pool = Arc::new(Mutex::new(pool));
        let owner = CapabilityOwner {
            server_id: "server-1".to_string(),
            server_name: "docs".to_string(),
            instance_id: "owner-1".to_string(),
            connection_generation: None,
            peer,
            source,
            cleanup: None,
        };
        (pool, owner, server_task)
    }

    #[tokio::test]
    async fn lru_or_sqlite_hit_never_acquires_an_owner() {
        for source in ["memory_cache", "sqlite_catalog"] {
            let backend = Arc::new(FakeBackend::new(Ok(Some(result(source)))));
            let fixture = test_peer().await;
            let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
            let service = CapabilityReadService::with_backend(backend, provider.clone());

            let listed = service
                .list(&list_ctx(None))
                .await
                .expect("cache hit should be returned");

            assert_eq!(listed.meta.source, source);
            assert_eq!(provider.existing_calls.load(Ordering::Relaxed), 0);
            assert_eq!(provider.fresh_calls.load(Ordering::Relaxed), 0);
            drop(service);
            drop(provider);
            fixture.shutdown().await;
        }
    }

    #[tokio::test]
    async fn missing_snapshot_uses_existing_owner_before_fresh_owner() {
        let backend = Arc::new(FakeBackend::new(Ok(None)));
        backend.push_discovery(Ok(observation())).await;
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        provider.set_existing(Ok(true)).await;
        let service = CapabilityReadService::with_backend(backend, provider.clone());

        let listed = service
            .list(&list_ctx(None))
            .await
            .expect("existing discovery should succeed");

        assert_eq!(listed.meta.source, "live_existing");
        assert_eq!(provider.existing_calls.load(Ordering::Relaxed), 1);
        assert_eq!(provider.fresh_calls.load(Ordering::Relaxed), 0);
        assert_eq!(*provider.acquisition_order.lock().await, ["existing"]);
        drop(service);
        drop(provider);
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn stale_existing_owner_retries_exactly_one_fresh_owner() {
        let backend = Arc::new(FakeBackend::new(Ok(None)));
        backend
            .push_discovery(Err(failure(RuntimeFailureKind::StaleGeneration, "stale generation")))
            .await;
        backend.push_discovery(Ok(observation())).await;
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        provider.set_existing(Ok(true)).await;
        let service = CapabilityReadService::with_backend(backend.clone(), provider.clone());

        let listed = service
            .list(&list_ctx(None))
            .await
            .expect("fresh discovery should recover");

        assert_eq!(listed.meta.source, "live_fresh");
        assert_eq!(provider.existing_calls.load(Ordering::Relaxed), 1);
        assert_eq!(provider.fresh_calls.load(Ordering::Relaxed), 1);
        assert_eq!(*provider.acquisition_order.lock().await, ["existing", "fresh"]);
        assert_eq!(
            *provider.released.lock().await,
            [OwnerSource::Existing, OwnerSource::Fresh]
        );
        assert_eq!(backend.evidence.lock().await.len(), 1);
        drop(service);
        drop(provider);
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn protocol_error_does_not_retry_with_a_fresh_owner() {
        let backend = Arc::new(FakeBackend::new(Ok(None)));
        backend
            .push_discovery(Err(failure(RuntimeFailureKind::Protocol, "invalid response")))
            .await;
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        provider.set_existing(Ok(true)).await;
        let service = CapabilityReadService::with_backend(backend.clone(), provider.clone());

        let error = service
            .list(&list_ctx(None))
            .await
            .expect_err("protocol failure must be returned");

        assert!(matches!(error, CapabilityReadError::DiscoveryFailed { .. }));
        assert_eq!(provider.fresh_calls.load(Ordering::Relaxed), 0);
        let evidence = backend.evidence.lock().await;
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].server_id, "server-1");
        assert_eq!(evidence[0].kind, CapabilityType::Tools);
        assert_eq!(evidence[0].instance_id.as_deref(), Some("Existing-1"));
        assert_eq!(evidence[0].connection_generation, None);
        assert!(evidence[0].reason.contains("invalid response"));
        drop(evidence);
        drop(service);
        drop(provider);
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn dual_stage_failure_preserves_canonical_name_and_protocol_operation() {
        let backend = Arc::new(FakeBackend::new(Ok(None)));
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        *provider.fresh_result.lock().await = Some(Err(CapabilityOwnerError::Other {
            reason: "fresh owner unavailable".to_string(),
        }));
        let service = CapabilityReadService::with_backend(backend, provider);

        let error = service
            .list(&list_ctx(None))
            .await
            .expect_err("both acquisition stages should fail");
        let display = error.to_string();

        match &error {
            CapabilityReadError::DiscoveryFailed {
                server_name, operation, ..
            } => {
                assert_eq!(server_name, "docs");
                assert_eq!(*operation, "tools/list");
            }
            other => panic!("unexpected error: {other:?}"),
        }

        assert!(display.contains("docs"), "missing canonical server name: {display}");
        assert!(display.contains("tools/list"), "missing protocol operation: {display}");
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn cleanup_failure_after_commit_does_not_record_inventory_failure() {
        let backend = Arc::new(FakeBackend::new(Ok(None)));
        backend.push_discovery(Ok(observation())).await;
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        *provider.release_error.lock().await = Some(CapabilityOwnerError::Other {
            reason: "shutdown join failed".to_string(),
        });
        let service = CapabilityReadService::with_backend(backend.clone(), provider);

        let error = service
            .list(&list_ctx(None))
            .await
            .expect_err("cleanup failure must remain visible");

        match error {
            CapabilityReadError::CleanupFailed {
                server_name,
                operation,
                owner_source,
                error: CapabilityOwnerError::Other { reason },
                ..
            } => {
                assert_eq!(server_name, "docs");
                assert_eq!(operation, "tools/list");
                assert_eq!(owner_source, OwnerSource::Fresh);
                assert_eq!(reason, "shutdown join failed");
            }
            other => panic!("unexpected cleanup error: {other:?}"),
        }
        assert_eq!(backend.commits.load(Ordering::Relaxed), 1);
        assert!(backend.evidence.lock().await.is_empty());
        fixture.shutdown().await;
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn sqlite_commit_failure_remains_typed_in_read_backend_error() {
        let database = commit_failure_database().await;
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        let tool: rmcp::model::Tool = serde_json::from_value(serde_json::json!({
            "name": "failing-tool",
            "description": "Commit failure fixture",
            "inputSchema": {"type": "object"}
        }))
        .expect("build tool fixture");
        let backend = Arc::new(CommitFailureBackend {
            runtime: RuntimeCapabilityReadBackend { database, pool: None },
            observation: Mutex::new(Some(CapabilityDiscoveryObservation {
                items: CapabilityItems::Tools(vec![tool]),
                flags: CapSyncFlags::ALL,
                kind_states: vec![KindObservation::new(
                    CatalogKind::Tools,
                    DeclarationState::Supported,
                    InventoryState::Complete,
                )],
            })),
            projection_error: Mutex::new(None),
        });
        let service = CapabilityReadService::with_backend(backend, provider);

        let error = service
            .list(&list_ctx(None))
            .await
            .expect_err("SQLite trigger should fail canonical commit through the read service");

        match error {
            CapabilityReadError::DiscoveryFailed {
                fresh:
                    Some(DiscoveryAttemptFailure {
                        error:
                            CapabilityAttemptError::Commit(
                                CapabilityCommitFailure::Catalog(_) | CapabilityCommitFailure::Database(_),
                            ),
                        ..
                    }),
                ..
            } => {}
            other => panic!("catalog error must remain typed in the final read error: {other:?}"),
        }
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn live_failure_never_returns_the_previous_ready_snapshot() {
        let backend = Arc::new(FakeBackend::new(Ok(Some(result("sqlite_catalog")))));
        backend
            .push_discovery(Err(failure(RuntimeFailureKind::Application, "application failure")))
            .await;
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        provider.set_existing(Ok(true)).await;
        *provider.release_error.lock().await = Some(CapabilityOwnerError::Other {
            reason: "shutdown join failed".to_string(),
        });
        let service = CapabilityReadService::with_backend(backend.clone(), provider);

        let error = service
            .list(&list_ctx(Some(RefreshStrategy::Force)))
            .await
            .expect_err("force discovery failure must not return the previous snapshot");

        assert!(matches!(error, CapabilityReadError::DiscoveryFailed { .. }));
        assert_eq!(backend.cache_calls.load(Ordering::Relaxed), 0);
        assert_eq!(backend.evidence.lock().await.len(), 1);
        assert_eq!(backend.commits.load(Ordering::Relaxed), 0);
        drop(service);
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn post_commit_projection_failure_does_not_record_inventory_failure() {
        let backend = Arc::new(FakeBackend::new(Ok(None)));
        backend.push_discovery(Ok(observation())).await;
        *backend.projection_error.lock().await = Some(CapabilityProjectionFailure(anyhow::anyhow!(
            "external-name projection failed after durable commit"
        )));
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        provider.set_existing(Ok(true)).await;
        let service = CapabilityReadService::with_backend(backend.clone(), provider.clone());

        let error = service
            .list(&list_ctx(None))
            .await
            .expect_err("post-commit projection failure must be surfaced");

        assert!(
            error
                .to_string()
                .contains("external-name projection failed after durable commit"),
            "projection cause was lost: {error:?}"
        );
        assert!(matches!(error, CapabilityReadError::ProjectionFailed { .. }));
        assert_eq!(backend.commits.load(Ordering::Relaxed), 1);
        assert!(
            backend.evidence.lock().await.is_empty(),
            "a local projection failure must not overwrite the committed inventory"
        );
        assert_eq!(provider.fresh_calls.load(Ordering::Relaxed), 0);
        assert_eq!(provider.released.lock().await.as_slice(), &[OwnerSource::Existing]);
        drop(service);
        drop(provider);
        fixture.shutdown().await;
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn durable_commit_survives_projection_failure_without_a_second_catalog_transition() {
        let database = runtime_database().await;
        let mut events = EventBus::global().subscribe_async();
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        provider.set_existing(Ok(true)).await;
        let tool: rmcp::model::Tool = serde_json::from_value(serde_json::json!({
            "name": "durable-tool",
            "description": "Projection failure fixture",
            "inputSchema": {"type": "object"}
        }))
        .expect("build tool fixture");
        let backend = Arc::new(CommitFailureBackend {
            runtime: RuntimeCapabilityReadBackend {
                database: database.clone(),
                pool: None,
            },
            observation: Mutex::new(Some(CapabilityDiscoveryObservation {
                items: CapabilityItems::Tools(vec![tool]),
                flags: CapSyncFlags::TOOLS,
                kind_states: vec![KindObservation::new(
                    CatalogKind::Tools,
                    DeclarationState::Supported,
                    InventoryState::Complete,
                )],
            })),
            projection_error: Mutex::new(Some(CapabilityProjectionFailure(anyhow::anyhow!(
                "forced projection failure after commit"
            )))),
        });
        let service = CapabilityReadService::with_backend(backend, provider.clone());

        let error = service
            .list(&list_ctx(None))
            .await
            .expect_err("projection failure must remain visible after the durable commit");

        assert!(matches!(error, CapabilityReadError::ProjectionFailed { .. }));
        let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
            .load_snapshot("server-1")
            .await
            .expect("load committed snapshot")
            .expect("durable snapshot exists");
        assert_eq!(snapshot.state, SnapshotState::Ready);
        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.last_error, None);
        assert_eq!(snapshot.records.len(), 1);
        match &snapshot.records[0].payload {
            CapabilityPayload::Tool(tool) => assert_eq!(tool.name, "durable-tool"),
            other => panic!("unexpected committed payload: {other:?}"),
        }
        assert_eq!(provider.fresh_calls.load(Ordering::Relaxed), 0);
        assert_eq!(provider.released.lock().await.as_slice(), &[OwnerSource::Existing]);

        let mut committed = 0;
        let mut changed = 0;
        tokio::time::timeout(Duration::from_secs(1), async {
            while committed == 0 || changed == 0 {
                match events.recv().await.expect("receive catalog transition") {
                    Event::CapabilityCatalogCommitted { server_id, .. } if server_id == "server-1" => committed += 1,
                    Event::CapabilityCatalogChanged { server_id, .. } if server_id == "server-1" => changed += 1,
                    _ => {}
                }
            }
        })
        .await
        .expect("durable commit must publish its transition");
        assert_eq!((committed, changed), (1, 1));

        let extra_transition = tokio::time::timeout(Duration::from_millis(100), async {
            loop {
                match events.recv().await.expect("receive catalog event") {
                    Event::CapabilityCatalogCommitted { server_id, .. }
                    | Event::CapabilityCatalogChanged { server_id, .. }
                        if server_id == "server-1" =>
                    {
                        break;
                    }
                    _ => {}
                }
            }
        })
        .await;
        assert!(
            extra_transition.is_err(),
            "projection failure published a second catalog transition"
        );
        drop(service);
        drop(provider);
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn owner_timeout_remains_typed_in_discovery_error() {
        let backend = Arc::new(FakeBackend::new(Ok(None)));
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        provider
            .set_existing(Err(CapabilityOwnerError::Timeout { timeout_ms: 125 }))
            .await;
        let service = CapabilityReadService::with_backend(backend, provider.clone());

        let error = service
            .list(&list_ctx(None))
            .await
            .expect_err("owner timeout must be returned without a fresh retry");

        assert_eq!(error.connection_timeout_ms(), Some(125));
        assert_eq!(error.operation_timeout_ms(), None);
        match error {
            CapabilityReadError::DiscoveryFailed {
                existing:
                    Some(DiscoveryAttemptFailure {
                        instance_id: None,
                        connection_generation: None,
                        source: OwnerSource::Existing,
                        error: CapabilityAttemptError::Owner(CapabilityOwnerError::Timeout { timeout_ms: 125 }),
                    }),
                fresh: None,
                ..
            } => {}
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(provider.fresh_calls.load(Ordering::Relaxed), 0);
        drop(service);
        drop(provider);
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn runtime_timeout_remains_typed_in_discovery_error() {
        let backend = Arc::new(FakeBackend::new(Ok(None)));
        backend
            .push_discovery(Err(RuntimeFailure {
                kind: RuntimeFailureKind::Timeout,
                message: Some("request timeout".to_string()),
                timeout_ms: Some(1_000),
            }))
            .await;
        let fixture = test_peer().await;
        let provider = Arc::new(FakeProvider::new(fixture.peer.clone()));
        provider.set_existing(Ok(true)).await;
        let service = CapabilityReadService::with_backend(backend, provider.clone());

        let error = service
            .list(&list_ctx(None))
            .await
            .expect_err("runtime timeout must be returned without a fresh retry");

        assert_eq!(error.connection_timeout_ms(), None);
        assert_eq!(error.operation_timeout_ms(), Some(1_000));
        match error {
            CapabilityReadError::DiscoveryFailed {
                existing:
                    Some(DiscoveryAttemptFailure {
                        instance_id: Some(instance_id),
                        connection_generation: None,
                        source: OwnerSource::Existing,
                        error:
                            CapabilityAttemptError::Runtime(RuntimeFailure {
                                kind: RuntimeFailureKind::Timeout,
                                timeout_ms: Some(1_000),
                                ..
                            }),
                    }),
                fresh: None,
                ..
            } => assert_eq!(instance_id, "Existing-1"),
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(provider.fresh_calls.load(Ordering::Relaxed), 0);
        drop(service);
        drop(provider);
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn only_existing_transport_failure_updates_pool_health_and_selection() {
        let selection = crate::core::capability::ConnectionSelection {
            server_id: "server-1".to_string(),
            affinity_key: crate::core::capability::AffinityKey::Default,
        };
        let failure = RuntimeFailure {
            kind: RuntimeFailureKind::TransportClosed,
            message: Some("transport closed".to_string()),
            timeout_ms: None,
        };

        let (existing_pool, existing_owner, existing_server) = pooled_owner(OwnerSource::Existing).await;
        apply_owner_runtime_failure(Some(&existing_pool), &existing_owner, &failure).await;
        {
            let guard = existing_pool.lock().await;
            assert_eq!(guard.select_ready_instance_id(&selection).expect("selection"), None);
            assert!(matches!(
                guard.failure_states.get("server-1").and_then(|state| state.last_kind),
                Some(FailureKind::RuntimeGone)
            ));
        }
        tokio::time::timeout(Duration::from_secs(1), existing_server)
            .await
            .expect("existing server should stop")
            .expect("existing server task should join");

        let (fresh_pool, fresh_owner, fresh_server) = pooled_owner(OwnerSource::Fresh).await;
        apply_owner_runtime_failure(Some(&fresh_pool), &fresh_owner, &failure).await;
        {
            let guard = fresh_pool.lock().await;
            assert_eq!(
                guard.select_ready_instance_id(&selection).expect("selection"),
                Some("owner-1".to_string())
            );
            assert!(!guard.failure_states.contains_key("server-1"));
        }
        fresh_pool
            .lock()
            .await
            .disconnect_non_blocking("server-1", "owner-1")
            .await
            .expect("fresh fixture should disconnect");
        tokio::time::timeout(Duration::from_secs(1), fresh_server)
            .await
            .expect("fresh server should stop")
            .expect("fresh server task should join");

        let (validation_pool, validation_owner, validation_server) = pooled_owner(OwnerSource::Validation).await;
        apply_owner_runtime_failure(Some(&validation_pool), &validation_owner, &failure).await;
        {
            let guard = validation_pool.lock().await;
            assert_eq!(
                guard.select_ready_instance_id(&selection).expect("selection"),
                Some("owner-1".to_string())
            );
            assert!(!guard.failure_states.contains_key("server-1"));
        }
        validation_pool
            .lock()
            .await
            .disconnect_non_blocking("server-1", "owner-1")
            .await
            .expect("validation fixture should disconnect");
        tokio::time::timeout(Duration::from_secs(1), validation_server)
            .await
            .expect("validation server should stop")
            .expect("validation server task should join");
    }

    mod rest_error_mapping {
        use super::{
            CapabilityAttemptError, CapabilityOwnerError, CapabilityProjectionFailure, CapabilityReadError,
            CapabilityType, DiscoveryAttemptFailure, OwnerSource, RuntimeFailure, RuntimeFailureKind,
        };
        use crate::api::handlers::ApiError;
        use crate::core::capability::service::map_capability_read_error;
        use mcpmate_capability_store::CatalogError;

        fn discovery_failed(existing_error: CapabilityAttemptError) -> CapabilityReadError {
            CapabilityReadError::DiscoveryFailed {
                server_id: "server-1".to_string(),
                server_name: "docs".to_string(),
                operation: "tools/list",
                kind: CapabilityType::Tools,
                catalog_error: None,
                existing: Some(DiscoveryAttemptFailure {
                    instance_id: Some("instance-1".to_string()),
                    connection_generation: None,
                    source: OwnerSource::Existing,
                    error: existing_error,
                }),
                fresh: None,
            }
        }

        #[test]
        fn connection_timeout_maps_to_gateway_timeout() {
            let error = discovery_failed(CapabilityAttemptError::Owner(CapabilityOwnerError::Timeout {
                timeout_ms: 750,
            }));

            assert!(matches!(map_capability_read_error(&error), ApiError::GatewayTimeout(_)));
        }

        #[test]
        fn operation_timeout_maps_to_request_timeout() {
            let error = discovery_failed(CapabilityAttemptError::Runtime(RuntimeFailure {
                kind: RuntimeFailureKind::Timeout,
                message: Some("request timeout".to_string()),
                timeout_ms: Some(500),
            }));

            assert!(matches!(map_capability_read_error(&error), ApiError::Timeout(_)));
        }

        #[test]
        fn authentication_failure_maps_to_unauthorized() {
            let error = discovery_failed(CapabilityAttemptError::Owner(CapabilityOwnerError::Authentication {
                reason: "401 from upstream".to_string(),
            }));

            assert!(matches!(map_capability_read_error(&error), ApiError::Unauthorized(_)));
        }

        #[test]
        fn runtime_authentication_failure_during_live_discovery_maps_to_unauthorized() {
            // Distinct from `authentication_failure_maps_to_unauthorized`: this failure comes
            // from the actual tools/list RPC call rejecting our credentials (a `Runtime`
            // failure), not from owner/session creation (an `Owner` failure). Both must map to
            // 401 so REST callers see a consistent, typed reason instead of a generic 502.
            let error = discovery_failed(CapabilityAttemptError::Runtime(RuntimeFailure {
                kind: RuntimeFailureKind::Authentication,
                message: Some("401 from upstream during tools/list".to_string()),
                timeout_ms: None,
            }));

            assert!(matches!(map_capability_read_error(&error), ApiError::Unauthorized(_)));
        }

        #[test]
        fn cleanup_authentication_failure_maps_to_unauthorized() {
            let error = CapabilityReadError::CleanupFailed {
                server_id: "server-1".to_string(),
                server_name: "docs".to_string(),
                operation: "tools/list",
                instance_id: "instance-1".to_string(),
                connection_generation: None,
                owner_source: OwnerSource::Existing,
                error: CapabilityOwnerError::Authentication {
                    reason: "403 from upstream".to_string(),
                },
            };

            assert!(matches!(map_capability_read_error(&error), ApiError::Unauthorized(_)));
        }

        #[test]
        fn catalog_failures_map_to_service_unavailable() {
            let untrusted = CapabilityReadError::CatalogUntrusted {
                server_id: "server-1".to_string(),
                source: CatalogError::InvalidValue {
                    field: "state",
                    value: "corrupted".to_string(),
                },
            };
            let operation = CapabilityReadError::CatalogOperation {
                server_id: "server-1".to_string(),
                source: anyhow::anyhow!("database unreachable"),
            };

            assert!(matches!(
                map_capability_read_error(&untrusted),
                ApiError::ServiceUnavailable(_)
            ));
            assert!(matches!(
                map_capability_read_error(&operation),
                ApiError::ServiceUnavailable(_)
            ));
        }

        #[test]
        fn exhausted_discovery_without_timeout_or_auth_maps_to_bad_gateway() {
            let error = discovery_failed(CapabilityAttemptError::Owner(CapabilityOwnerError::Missing {
                reason: "no owner available".to_string(),
            }));

            assert!(matches!(map_capability_read_error(&error), ApiError::BadGateway(_)));
        }

        #[test]
        fn projection_failure_maps_to_internal_error() {
            let error = CapabilityReadError::ProjectionFailed {
                server_id: "server-1".to_string(),
                server_name: "docs".to_string(),
                operation: "tools/list",
                kind: CapabilityType::Tools,
                instance_id: "instance-1".to_string(),
                connection_generation: None,
                owner_source: OwnerSource::Existing,
                source: CapabilityProjectionFailure(anyhow::anyhow!("projection decode failed")),
            };

            assert!(matches!(map_capability_read_error(&error), ApiError::InternalError(_)));
        }
    }
}
