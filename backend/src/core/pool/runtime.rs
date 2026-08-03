//! Unified ownership for upstream access that is not yet addressable by a persisted server id.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio::sync::{Mutex, watch};
use tokio_util::sync::CancellationToken;

use super::{UpstreamConnection, UpstreamConnectionPool};
use crate::{common::server::ServerType, core::models::MCPServerConfig};

const PREVIEW_RETENTION: Duration = Duration::from_secs(180);

fn preview_owner_acquisition_timeout(policy: crate::core::transport::timeout_policy::McpTimeoutPolicy) -> Duration {
    policy.startup.saturating_add(policy.capability_operation)
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PreviewOwnerKey {
    namespace: String,
    config_fingerprint: String,
}

#[derive(Clone, Debug)]
pub(crate) struct PreviewOwnerEntry {
    pub(crate) connection: UpstreamConnection,
    pub(crate) cancellation: Option<CancellationToken>,
    pub(crate) runtime_fingerprint: String,
    pub(crate) expires_at: std::time::Instant,
}

#[derive(Clone, Debug)]
pub(crate) enum PreviewAttemptOutcome {
    Published,
    Failed(String),
}

#[derive(Clone, Debug)]
pub(crate) struct PreviewAttemptEntry {
    pub(crate) attempt_id: u64,
    pub(crate) runtime_fingerprint: String,
    pub(crate) outcome_tx: watch::Sender<Option<PreviewAttemptOutcome>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UpstreamSubject {
    namespace: String,
    /// Stable persisted/draft identity that excludes resolved secrets.
    config_fingerprint: String,
    /// Effective launch materialization used only to decide transport reuse.
    runtime_fingerprint: String,
}

impl UpstreamSubject {
    pub(crate) fn preview(
        namespace: String,
        config_fingerprint: String,
        runtime_fingerprint: String,
    ) -> Self {
        Self {
            namespace,
            config_fingerprint,
            runtime_fingerprint,
        }
    }

    fn preview_key(&self) -> PreviewOwnerKey {
        PreviewOwnerKey {
            namespace: self.namespace.clone(),
            config_fingerprint: self.config_fingerprint.clone(),
        }
    }
}

enum PreviewAcquisition {
    Reuse(Box<UpstreamConnection>),
    Join(watch::Receiver<Option<PreviewAttemptOutcome>>),
    JoinProduction(watch::Receiver<Option<super::startup::StartupAttemptOutcome>>),
    Own {
        attempt_id: u64,
        outcome_rx: watch::Receiver<Option<PreviewAttemptOutcome>>,
    },
}

enum PreviewPromotion {
    None,
    Join(watch::Receiver<Option<PreviewAttemptOutcome>>),
    Promoted {
        instance_id: String,
        replaced_connection: Option<Box<UpstreamConnection>>,
        replaced_cancellation: Option<CancellationToken>,
    },
}

fn schedule_preview_cleanup(
    shared: &Arc<Mutex<UpstreamConnectionPool>>,
    key: PreviewOwnerKey,
    instance_id: String,
    delay: Duration,
) {
    let pool = Arc::downgrade(shared);
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        let Some(pool) = pool.upgrade() else {
            return;
        };
        let expired = {
            let mut pool = pool.lock().await;
            let should_remove = pool.preview_owners.get(&key).is_some_and(|entry| {
                entry.connection.id == instance_id && entry.expires_at <= std::time::Instant::now()
            });
            should_remove
                .then(|| pool.preview_owners.remove(&key))
                .flatten()
                .map(|entry| (entry.connection, entry.cancellation))
        };
        if let Some((connection, cancellation)) = expired {
            UpstreamConnectionPool::discard_startup_result(Some(connection), cancellation).await;
        }
    });
}

impl UpstreamConnectionPool {
    pub(crate) fn clear_preview_runtime_state(&mut self) {
        self.preview_owners.clear();
        self.preview_attempts.clear();
    }

    fn prepare_preview_acquisition(
        &mut self,
        subject: &UpstreamSubject,
        production_server_id: Option<&str>,
    ) -> (PreviewAcquisition, Vec<(UpstreamConnection, Option<CancellationToken>)>) {
        let key = subject.preview_key();
        let now = std::time::Instant::now();
        let expired_keys = self
            .preview_owners
            .iter()
            .filter_map(|(key, entry)| (entry.expires_at <= now).then_some(key.clone()))
            .collect::<Vec<_>>();
        let mut expired = Vec::with_capacity(expired_keys.len());
        for expired_key in expired_keys {
            if let Some(entry) = self.preview_owners.remove(&expired_key) {
                expired.push((entry.connection, entry.cancellation));
            }
        }

        if let Some(entry) = self.preview_owners.get_mut(&key) {
            let reusable = entry.connection.is_connected()
                && entry.runtime_fingerprint == subject.runtime_fingerprint
                && entry
                    .connection
                    .service
                    .as_ref()
                    .is_some_and(|service| !service.is_closed());
            if reusable {
                entry.expires_at = now + PREVIEW_RETENTION;
                return (PreviewAcquisition::Reuse(Box::new(entry.connection.clone())), expired);
            }
            let stale = self
                .preview_owners
                .remove(&key)
                .expect("observed preview owner remains present");
            expired.push((stale.connection, stale.cancellation));
        }

        if let Some(attempt) = self.preview_attempts.get(&key) {
            if attempt.runtime_fingerprint == subject.runtime_fingerprint {
                return (PreviewAcquisition::Join(attempt.outcome_tx.subscribe()), expired);
            }
        }
        if let Some(attempt) = self.preview_attempts.remove(&key) {
            let _ = attempt.outcome_tx.send(Some(PreviewAttemptOutcome::Failed(
                "Preview runtime materialization was superseded".to_string(),
            )));
        }

        if let Some(server_id) = production_server_id
            && self
                .config
                .mcp_servers
                .get(server_id)
                .is_some_and(|config| config.source_fingerprint.as_deref() == Some(subject.config_fingerprint.as_str()))
        {
            let route_key = super::ProductionRouteKey::shareable(server_id.to_string());
            if let Some(instance_id) = self.production_routes.get(&route_key)
                && let Some(connection) = self.connections.get(server_id).and_then(|items| items.get(instance_id))
                && connection.config_fingerprint.as_deref() == Some(subject.config_fingerprint.as_str())
                && connection.runtime_fingerprint.as_deref() == Some(subject.runtime_fingerprint.as_str())
                && connection.is_connected()
                && connection.service.as_ref().is_some_and(|service| !service.is_closed())
            {
                return (PreviewAcquisition::Reuse(Box::new(connection.clone())), expired);
            }
            if let Some(attempt) = self.startup_attempts.get(&route_key)
                && attempt.runtime_fingerprint == subject.runtime_fingerprint
            {
                return (
                    PreviewAcquisition::JoinProduction(attempt.outcome_tx.subscribe()),
                    expired,
                );
            }
        }

        let attempt_id = self
            .next_preview_attempt
            .fetch_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |current| current.checked_add(1),
            )
            .expect("preview attempt identifier exhausted");
        let (outcome_tx, outcome_rx) = watch::channel(None);
        self.preview_attempts.insert(
            key,
            PreviewAttemptEntry {
                attempt_id,
                runtime_fingerprint: subject.runtime_fingerprint.clone(),
                outcome_tx,
            },
        );
        (PreviewAcquisition::Own { attempt_id, outcome_rx }, expired)
    }

    async fn resolve_preview_production_server(
        shared: &Arc<Mutex<Self>>,
        subject: &UpstreamSubject,
    ) -> Result<Option<String>> {
        let (database, candidates) = {
            let guard = shared.lock().await;
            let candidates = guard
                .config
                .mcp_servers
                .iter()
                .filter_map(|(server_id, config)| {
                    (config.source_fingerprint.as_deref() == Some(subject.config_fingerprint.as_str()))
                        .then_some(server_id.clone())
                })
                .collect::<Vec<_>>();
            (guard.database.clone(), candidates)
        };
        let Some(database) = database else {
            return Ok(None);
        };
        for server_id in candidates {
            let server = crate::config::server::get_server_by_id(&database.pool, &server_id).await?;
            if server.is_some_and(|server| server.name == subject.namespace) {
                return Ok(Some(server_id));
            }
        }
        Ok(None)
    }

    pub(crate) fn has_preview_candidate(
        &self,
        config_fingerprint: &str,
    ) -> bool {
        self.preview_owners
            .keys()
            .chain(self.preview_attempts.keys())
            .any(|key| key.config_fingerprint == config_fingerprint)
    }

    fn prepare_preview_promotion(
        &mut self,
        subject: &UpstreamSubject,
        selection: &crate::core::capability::ConnectionSelection,
    ) -> PreviewPromotion {
        if !matches!(selection.affinity_key, crate::core::capability::AffinityKey::Default) {
            return PreviewPromotion::None;
        }
        let key = subject.preview_key();
        if let Some(entry) = self.preview_owners.get(&key) {
            let reusable = entry.expires_at > std::time::Instant::now()
                && entry.connection.is_connected()
                && entry.runtime_fingerprint == subject.runtime_fingerprint
                && entry
                    .connection
                    .service
                    .as_ref()
                    .is_some_and(|service| !service.is_closed());
            if reusable {
                let route_key = crate::core::pool::ProductionRouteKey::shareable(selection.server_id.clone());
                let config_matches = self.config.mcp_servers.get(&selection.server_id).is_some_and(|config| {
                    config.source_fingerprint.as_deref() == Some(subject.config_fingerprint.as_str())
                        && crate::config::server::fingerprint::materialized_runtime_fingerprint(config)
                            .is_ok_and(|fingerprint| fingerprint == subject.runtime_fingerprint)
                });
                let production_ready = self
                    .select_ready_instance_id(selection)
                    .ok()
                    .flatten()
                    .and_then(|instance_id| self.get_instance(&selection.server_id, &instance_id).ok())
                    .is_some_and(|connection| {
                        connection.config_fingerprint.as_deref() == Some(subject.config_fingerprint.as_str())
                            && connection.runtime_fingerprint.as_deref() == Some(subject.runtime_fingerprint.as_str())
                    });
                if config_matches && !production_ready && !self.startup_attempts.contains_key(&route_key) {
                    let entry = self
                        .preview_owners
                        .remove(&key)
                        .expect("observed preview owner remains present");
                    let instance_id = self
                        .resolve_production_route(selection)
                        .unwrap_or_else(|| self.allocate_production_route(selection));
                    let mut connection = entry.connection;
                    connection.id = instance_id.clone();
                    connection.server_name = subject.namespace.clone();
                    connection.config_fingerprint = Some(subject.config_fingerprint.clone());
                    connection.runtime_fingerprint = Some(subject.runtime_fingerprint.clone());
                    let replaced_connection = self
                        .connections
                        .entry(selection.server_id.clone())
                        .or_default()
                        .insert(instance_id.clone(), connection);
                    let tokens = self.cancellation_tokens.entry(selection.server_id.clone()).or_default();
                    let replaced_cancellation = tokens.remove(&instance_id);
                    if let Some(cancellation) = entry.cancellation {
                        tokens.insert(instance_id.clone(), cancellation);
                    }
                    self.clear_failure_state(&selection.server_id);
                    return PreviewPromotion::Promoted {
                        instance_id,
                        replaced_connection: replaced_connection.map(Box::new),
                        replaced_cancellation,
                    };
                }
            }
        }
        if let Some(attempt) = self.preview_attempts.get(&key)
            && attempt.runtime_fingerprint == subject.runtime_fingerprint
        {
            return PreviewPromotion::Join(attempt.outcome_tx.subscribe());
        }
        PreviewPromotion::None
    }

    fn claim_preview_attempt(
        &mut self,
        key: &PreviewOwnerKey,
        attempt_id: u64,
    ) -> Option<PreviewAttemptEntry> {
        let owns = self
            .preview_attempts
            .get(key)
            .is_some_and(|attempt| attempt.attempt_id == attempt_id);
        owns.then(|| self.preview_attempts.remove(key)).flatten()
    }

    fn spawn_preview_attempt(
        shared: &Arc<Mutex<Self>>,
        subject: UpstreamSubject,
        config: MCPServerConfig,
        server_type: ServerType,
        http_client: Option<reqwest::Client>,
        operation_timeout: Option<Duration>,
        key: PreviewOwnerKey,
        attempt_id: u64,
    ) {
        let pool = shared.clone();
        tokio::spawn(async move {
            let owner_namespace = subject.namespace.clone();
            let startup = tokio::spawn(async move {
                crate::config::server::capabilities::connect_preview_owner(
                    &subject.namespace,
                    &config,
                    server_type,
                    http_client,
                    operation_timeout,
                )
                .await
            })
            .await;

            let result = match startup {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("Preview owner task failed: {error}")),
            };
            match result {
                Ok((owner, cancellation)) => {
                    let instance_id = owner.id.clone();
                    let mut connection = Some(owner);
                    let published = {
                        let mut pool_guard = pool.lock().await;
                        if let Some(attempt) = pool_guard.claim_preview_attempt(&key, attempt_id) {
                            pool_guard.preview_owners.insert(
                                key.clone(),
                                PreviewOwnerEntry {
                                    connection: connection.take().expect("preview connection is present"),
                                    cancellation: cancellation.clone(),
                                    runtime_fingerprint: subject.runtime_fingerprint.clone(),
                                    expires_at: std::time::Instant::now() + PREVIEW_RETENTION,
                                },
                            );
                            let _ = attempt.outcome_tx.send(Some(PreviewAttemptOutcome::Published));
                            true
                        } else {
                            false
                        }
                    };
                    if let Some(connection) = connection {
                        Self::discard_startup_result(Some(connection), cancellation).await;
                    }
                    if published {
                        schedule_preview_cleanup(&pool, key, instance_id, PREVIEW_RETENTION);
                    }
                }
                Err(error) => {
                    let mut pool_guard = pool.lock().await;
                    if let Some(attempt) = pool_guard.claim_preview_attempt(&key, attempt_id) {
                        let _ = attempt
                            .outcome_tx
                            .send(Some(PreviewAttemptOutcome::Failed(format!("{error:#}"))));
                    }
                    tracing::debug!(namespace = owner_namespace, error = %error, "Preview owner acquisition failed");
                }
            }
        });
    }

    async fn wait_for_preview_attempt(
        mut outcome_rx: watch::Receiver<Option<PreviewAttemptOutcome>>,
        startup_timeout: Duration,
    ) -> Result<PreviewAttemptOutcome> {
        tokio::time::timeout(startup_timeout, outcome_rx.wait_for(|value| value.is_some()))
            .await
            .map_err(|_| {
                anyhow::anyhow!(
                    "Timed out waiting for preview owner after {}ms",
                    startup_timeout.as_millis()
                )
            })?
            .context("preview attempt channel closed while waiting")?
            .clone()
            .context("preview outcome missing after wait")
    }

    pub(crate) async fn preview_capabilities_coordinated(
        shared: &Arc<Mutex<Self>>,
        subject: UpstreamSubject,
        mut config: MCPServerConfig,
        server_type: ServerType,
        http_client: Option<reqwest::Client>,
        operation_timeout: Option<Duration>,
    ) -> Result<crate::config::server::capabilities::CapabilitySnapshot> {
        let key = subject.preview_key();
        config.source_fingerprint = Some(subject.config_fingerprint.clone());
        let timeout_policy = crate::core::transport::timeout_policy::McpTimeoutPolicy::for_server(
            server_type,
            config.command.as_deref(),
            operation_timeout,
        );
        let acquisition_timeout = preview_owner_acquisition_timeout(timeout_policy);

        loop {
            let production_server_id = Self::resolve_preview_production_server(shared, &subject).await?;
            let (acquisition, expired) = shared
                .lock()
                .await
                .prepare_preview_acquisition(&subject, production_server_id.as_deref());
            for (connection, cancellation) in expired {
                Self::discard_startup_result(Some(connection), cancellation).await;
            }

            match acquisition {
                PreviewAcquisition::Reuse(connection) => {
                    schedule_preview_cleanup(shared, key.clone(), connection.id.clone(), PREVIEW_RETENTION);
                    return crate::config::server::capabilities::discover_from_preview_connection(
                        &connection,
                        operation_timeout,
                    )
                    .await;
                }
                PreviewAcquisition::Join(outcome_rx) => {
                    match Self::wait_for_preview_attempt(outcome_rx, acquisition_timeout).await? {
                        PreviewAttemptOutcome::Published => continue,
                        PreviewAttemptOutcome::Failed(error) => return Err(anyhow::anyhow!(error)),
                    }
                }
                PreviewAcquisition::JoinProduction(mut outcome_rx) => {
                    let outcome =
                        tokio::time::timeout(acquisition_timeout, outcome_rx.wait_for(|value| value.is_some()))
                            .await
                            .map_err(|_| {
                                anyhow::anyhow!(
                                    "Timed out waiting for production owner after {}ms",
                                    acquisition_timeout.as_millis()
                                )
                            })?
                            .context("production startup channel closed while preview was waiting")?
                            .clone()
                            .context("production startup outcome missing after wait")?;
                    match outcome {
                        super::startup::StartupAttemptOutcome::Published(_) => continue,
                        super::startup::StartupAttemptOutcome::Failed(error) => return Err(anyhow::anyhow!(error)),
                        super::startup::StartupAttemptOutcome::Superseded => {
                            return Err(anyhow::anyhow!(
                                "Production owner was superseded while preview was waiting"
                            ));
                        }
                    }
                }
                PreviewAcquisition::Own { attempt_id, outcome_rx } => {
                    Self::spawn_preview_attempt(
                        shared,
                        subject.clone(),
                        config.clone(),
                        server_type,
                        http_client.clone(),
                        operation_timeout,
                        key.clone(),
                        attempt_id,
                    );
                    match Self::wait_for_preview_attempt(outcome_rx, acquisition_timeout).await? {
                        PreviewAttemptOutcome::Published => continue,
                        PreviewAttemptOutcome::Failed(error) => return Err(anyhow::anyhow!(error)),
                    }
                }
            }
        }
    }

    pub(crate) async fn promote_preview_owner_to_production(
        shared: &Arc<Mutex<Self>>,
        subject: &UpstreamSubject,
        selection: &crate::core::capability::ConnectionSelection,
        acquisition_timeout: Duration,
    ) -> Result<Option<String>> {
        loop {
            let promotion = shared.lock().await.prepare_preview_promotion(subject, selection);
            match promotion {
                PreviewPromotion::None => return Ok(None),
                PreviewPromotion::Promoted {
                    instance_id,
                    replaced_connection,
                    replaced_cancellation,
                } => {
                    Self::discard_startup_result(
                        replaced_connection.map(|connection| *connection),
                        replaced_cancellation,
                    )
                    .await;
                    return Ok(Some(instance_id));
                }
                PreviewPromotion::Join(outcome_rx) => {
                    match Self::wait_for_preview_attempt(outcome_rx, acquisition_timeout).await? {
                        PreviewAttemptOutcome::Published => continue,
                        PreviewAttemptOutcome::Failed(error) => return Err(anyhow::anyhow!(error)),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use rmcp::{ServerHandler, ServiceExt};
    use tokio::sync::Mutex;

    use super::*;
    use crate::core::{models::Config, pool::UpstreamConnection};

    #[derive(Clone, Default)]
    struct TestServer;

    impl ServerHandler for TestServer {}

    async fn ready_connection() -> (UpstreamConnection, tokio::task::JoinHandle<anyhow::Result<()>>) {
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_handle = tokio::spawn(async move {
            let service = TestServer.serve(server_transport).await?;
            service.waiting().await?;
            Ok(())
        });
        let service = crate::core::transport::client::UpstreamClientHandler::new("everything".to_string())
            .serve(client_transport)
            .await
            .expect("preview cleanup client should initialize");
        let mut connection = UpstreamConnection::new("everything".to_string());
        connection.update_connected(service, Vec::new(), Some(rmcp::model::ServerCapabilities::default()));
        (connection, server_handle)
    }

    #[tokio::test]
    async fn preview_owner_cleanup_closes_the_expired_matching_owner() {
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
            Arc::new(Config::default()),
            None,
        )));
        let key = PreviewOwnerKey {
            namespace: "everything".to_string(),
            config_fingerprint: "sha256:test".to_string(),
        };
        let (connection, server_handle) = ready_connection().await;
        let instance_id = connection.id.clone();
        pool.lock().await.preview_owners = HashMap::from([(
            key.clone(),
            PreviewOwnerEntry {
                connection,
                cancellation: None,
                runtime_fingerprint: "sha256:runtime-a".to_string(),
                expires_at: std::time::Instant::now() + Duration::from_millis(10),
            },
        )]);

        schedule_preview_cleanup(&pool, key.clone(), instance_id, Duration::from_millis(20));
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(!pool.lock().await.preview_owners.contains_key(&key));
        tokio::time::timeout(Duration::from_secs(1), server_handle)
            .await
            .expect("expired preview service should close")
            .expect("preview server task should finish")
            .expect("preview server should stop cleanly");
    }

    #[tokio::test]
    async fn changed_runtime_materialization_replaces_the_retained_owner() {
        let mut pool = UpstreamConnectionPool::new(Arc::new(Config::default()), None);
        let (connection, server_handle) = ready_connection().await;
        let subject = UpstreamSubject::preview(
            "everything".to_string(),
            "sha256:canonical".to_string(),
            "sha256:runtime-new".to_string(),
        );
        pool.preview_owners.insert(
            subject.preview_key(),
            PreviewOwnerEntry {
                connection,
                cancellation: None,
                runtime_fingerprint: "sha256:runtime-old".to_string(),
                expires_at: std::time::Instant::now() + Duration::from_secs(60),
            },
        );

        let (acquisition, stale) = pool.prepare_preview_acquisition(&subject, None);

        assert!(matches!(acquisition, PreviewAcquisition::Own { .. }));
        assert_eq!(stale.len(), 1, "changed credentials must retire the retained owner");
        let (connection, cancellation) = stale.into_iter().next().expect("stale preview owner");
        UpstreamConnectionPool::discard_startup_result(Some(connection), cancellation).await;
        server_handle
            .await
            .expect("preview server task should finish")
            .expect("preview server should stop cleanly");
    }

    #[tokio::test]
    async fn production_workers_do_not_clone_preview_registry_ownership() {
        let mut pool = UpstreamConnectionPool::new(Arc::new(Config::default()), None);
        let (connection, server_handle) = ready_connection().await;
        let key = PreviewOwnerKey {
            namespace: "everything".to_string(),
            config_fingerprint: "sha256:canonical".to_string(),
        };
        pool.preview_owners.insert(
            key.clone(),
            PreviewOwnerEntry {
                connection,
                cancellation: None,
                runtime_fingerprint: "sha256:runtime".to_string(),
                expires_at: std::time::Instant::now() + Duration::from_secs(60),
            },
        );

        let worker = pool.runtime_worker();

        assert!(worker.preview_owners.is_empty());
        assert!(worker.preview_attempts.is_empty());
        let connection = pool
            .preview_owners
            .remove(&key)
            .expect("shared pool retains preview owner")
            .connection;
        UpstreamConnectionPool::discard_startup_result(Some(connection), None).await;
        server_handle
            .await
            .expect("preview server task should finish")
            .expect("preview server should stop cleanly");
    }

    #[test]
    fn acquisition_wait_preserves_startup_and_initial_capability_budgets() {
        let policy = crate::core::transport::timeout_policy::McpTimeoutPolicy::for_server(
            ServerType::Stdio,
            Some("python3"),
            Some(Duration::from_secs(17)),
        );

        assert_eq!(preview_owner_acquisition_timeout(policy), Duration::from_secs(77));
    }
}
