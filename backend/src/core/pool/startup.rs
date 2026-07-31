//! Production startup ownership coordinator for demand-driven connections.
//!
//! Coordinates the startup transaction so that slow transport initialization
//! (database/secret materialization, process spawn, protocol initialize) never
//! runs while the shared pool lock is held:
//!
//!   1. prepare under lock - resolve/allocate the production route, capture
//!      the identity and generation facts, register an in-flight attempt for
//!      single-flight joining.
//!   2. start outside the lock - run the existing transport startup on a
//!      detached worker pool clone.
//!   3. conditional publish - reacquire the lock and publish only when the
//!      route, identity and generation still match.
//!   4. discard outside lock - the owning attempt explicitly cancels and shuts
//!      down stale or superseded temporary transports.
//!
//! The coordinator reuses the existing `ProductionRouteKey` identity, connection
//! `created_at` generation fact, and `config_fingerprint` checks instead of
//! inventing a broader lease hierarchy. Joiners wait for the same typed outcome
//! and never obtain shutdown authority over the temporary transport.

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::{Mutex, watch};
use tokio_util::sync::CancellationToken;

use crate::core::capability::{AffinityKey, ConnectionSelection};
use crate::core::foundation::types::ConnectionStatus;
use crate::core::pool::{ProductionRouteKey, UpstreamConnection, UpstreamConnectionPool};

/// Typed outcome shared by all demands waiting on one startup attempt.
#[derive(Clone, Debug)]
pub(crate) enum StartupAttemptOutcome {
    /// The attempt published its instance under the captured generation facts.
    Published(String),
    /// The attempt failed; every joiner receives the same error text.
    Failed(String),
    /// The attempt was superseded before publication; demands should retry.
    Superseded,
}

/// Registry entry for one in-flight production startup attempt.
///
/// The attempt identity is the production route key plus a monotonic attempt id.
/// The watch channel carries the typed outcome from the owning attempt to every
/// joiner; a `None` value means the attempt is still starting.
#[derive(Debug, Clone)]
pub(crate) struct StartupAttemptEntry {
    pub(crate) attempt_id: u64,
    pub(crate) outcome_tx: watch::Sender<Option<StartupAttemptOutcome>>,
}

/// Owner-side handle to a registered startup attempt.
pub(crate) struct StartupAttempt {
    route_key: ProductionRouteKey,
    server_id: String,
    instance_id: String,
    created_at: std::time::Instant,
    config_fingerprint: Option<String>,
    attempt_id: u64,
    worker: UpstreamConnectionPool,
}

/// Result of the locked prepare phase.
pub(crate) enum StartupPrepare {
    /// A ready instance already satisfies the demand.
    Ready(String),
    /// Another attempt for the same identity is in flight; wait for its outcome.
    Join(watch::Receiver<Option<StartupAttemptOutcome>>),
    /// This demand owns the new attempt and must run the startup.
    Own(Box<StartupAttempt>),
}

/// Result of the locked conditional publication phase.
pub(crate) enum StartupPublish {
    /// The attempt published; the replaced old generation is discarded outside
    /// the lock.
    Published {
        replaced_connection: Option<UpstreamConnection>,
        replaced_token: Option<CancellationToken>,
    },
    /// The attempt failed; its temporary result is discarded outside the lock.
    /// The error is the exact typed outcome shared with every joiner.
    Failed {
        connection: Option<UpstreamConnection>,
        cancellation: Option<CancellationToken>,
        error: String,
    },
    /// The attempt was superseded; its temporary result is discarded outside
    /// the lock and demands retry.
    Invalidated {
        connection: Option<UpstreamConnection>,
        cancellation: Option<CancellationToken>,
    },
}

/// Maximum number of startup attempts before a demand gives up on repeated
/// route/configuration changes during startup.
const MAX_STARTUP_ATTEMPTS: usize = 4;

impl UpstreamConnectionPool {
    /// Production startup entry point coordinating one attempt per connection
    /// identity while keeping transport startup outside the shared pool lock.
    pub async fn ensure_connected_coordinated(
        pool: &Arc<Mutex<Self>>,
        selection: &ConnectionSelection,
    ) -> Result<String> {
        let mut attempts = 0;
        loop {
            attempts += 1;
            let prepared = {
                let mut guard = pool.lock().await;
                guard.prepare_startup_attempt(selection)
            }?;
            match prepared {
                StartupPrepare::Ready(instance_id) => return Ok(instance_id),
                StartupPrepare::Join(mut outcome_rx) => {
                    let outcome = outcome_rx
                        .wait_for(|value| value.is_some())
                        .await
                        .context("startup attempt channel closed while waiting")?
                        .clone()
                        .expect("startup outcome present after wait");
                    match outcome {
                        StartupAttemptOutcome::Published(instance_id) => return Ok(instance_id),
                        StartupAttemptOutcome::Failed(error) => return Err(anyhow::anyhow!("{error}")),
                        StartupAttemptOutcome::Superseded => {
                            if attempts >= MAX_STARTUP_ATTEMPTS {
                                return Err(anyhow::anyhow!(
                                    "Startup for server '{}' was superseded {} times by route or configuration changes",
                                    selection.server_id,
                                    attempts
                                ));
                            }
                        }
                    }
                }
                StartupPrepare::Own(attempt) => {
                    let cleanup_route_key = attempt.route_key.clone();
                    let cleanup_attempt_id = attempt.attempt_id;
                    let cleanup_server_id = attempt.server_id.clone();
                    let cleanup_instance_id = attempt.instance_id.clone();
                    let cleanup_created_at = attempt.created_at;
                    let task_pool = pool.clone();
                    let outcome = match tokio::spawn(async move { run_startup_attempt(&task_pool, *attempt).await })
                        .await
                    {
                        Ok(outcome) => outcome,
                        Err(join_error) => {
                            // A panic inside the attempt must never strand the
                            // registry entry: claim it and fail every joiner.
                            let mut guard = pool.lock().await;
                            if let Some(entry) = guard.claim_startup_attempt(&cleanup_route_key, cleanup_attempt_id) {
                                let _ = entry.outcome_tx.send(Some(StartupAttemptOutcome::Failed(format!(
                                    "Startup attempt task failed: {join_error}"
                                ))));
                            }
                            if let Ok(connection) = guard.get_instance_mut(&cleanup_server_id, &cleanup_instance_id)
                                && connection.created_at == cleanup_created_at
                            {
                                connection.update_failed("Startup attempt task failed".to_string());
                            }
                            drop(guard);
                            return Err(anyhow::anyhow!(
                                "Startup attempt task failed for server '{}': {join_error}",
                                selection.server_id
                            ));
                        }
                    };
                    match outcome {
                        StartupAttemptOutcome::Published(instance_id) => return Ok(instance_id),
                        StartupAttemptOutcome::Failed(error) => return Err(anyhow::anyhow!("{error}")),
                        StartupAttemptOutcome::Superseded => {
                            if attempts >= MAX_STARTUP_ATTEMPTS {
                                return Err(anyhow::anyhow!(
                                    "Startup for server '{}' was superseded {} times by route or configuration changes",
                                    selection.server_id,
                                    attempts
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Locked prepare phase: resolve the production route, register or join an
    /// in-flight attempt, and capture the generation facts needed for
    /// conditional publication.
    pub(crate) fn prepare_startup_attempt(
        &mut self,
        selection: &ConnectionSelection,
    ) -> Result<StartupPrepare> {
        if let Some(instance_id) = self.resolve_production_route(selection)
            && let Ok(Some(ready_id)) = self.select_ready_instance_id(selection)
            && ready_id == instance_id
        {
            return Ok(StartupPrepare::Ready(instance_id));
        }

        if let Some(remaining) = self.remaining_backoff(&selection.server_id) {
            tracing::warn!(
                server_id = %selection.server_id,
                wait_secs = remaining.as_secs_f32(),
                "Connection attempt blocked due to active backoff"
            );
            return Err(anyhow::anyhow!(
                "Server '{}' is backing off for {:.1}s",
                selection.server_id,
                remaining.as_secs_f32()
            ));
        }
        if !self.config.mcp_servers.contains_key(&selection.server_id) {
            return Err(anyhow::anyhow!(
                "Server '{}' not found in configuration",
                selection.server_id
            ));
        }

        let route_key = ProductionRouteKey::new(selection.server_id.clone(), selection.affinity_key.clone());
        if let Some(entry) = self.startup_attempts.get(&route_key) {
            return Ok(StartupPrepare::Join(entry.outcome_tx.subscribe()));
        }

        let instance_id = match self.resolve_production_route(selection) {
            Some(instance_id) => instance_id,
            None => self.allocate_production_route(selection),
        };
        self.invalidate_health_reconnect(&selection.server_id, &instance_id);
        let created_at = self
            .get_instance(&selection.server_id, &instance_id)
            .expect("allocated production instance exists")
            .created_at;
        let config_fingerprint = self
            .config
            .mcp_servers
            .get(&selection.server_id)
            .and_then(|config| config.source_fingerprint.clone());
        let attempt_id = self
            .next_startup_attempt
            .fetch_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |current| current.checked_add(1),
            )
            .expect("startup attempt identifier exhausted");
        self.get_instance_mut(&selection.server_id, &instance_id)
            .expect("allocated production instance exists")
            .update_initializing();
        let (outcome_tx, _) = watch::channel(None);
        self.startup_attempts
            .insert(route_key.clone(), StartupAttemptEntry { attempt_id, outcome_tx });
        let worker = self.clone();
        Ok(StartupPrepare::Own(Box::new(StartupAttempt {
            route_key,
            server_id: selection.server_id.clone(),
            instance_id,
            created_at,
            config_fingerprint,
            attempt_id,
            worker,
        })))
    }

    /// Locked claim: remove the attempt registry entry only when this attempt
    /// still owns it. The entry is returned so the owner can send its typed
    /// outcome after the registry slot has been claimed.
    fn claim_startup_attempt(
        &mut self,
        route_key: &ProductionRouteKey,
        attempt_id: u64,
    ) -> Option<StartupAttemptEntry> {
        let owns = self
            .startup_attempts
            .get(route_key)
            .is_some_and(|entry| entry.attempt_id == attempt_id);
        if owns {
            self.startup_attempts.remove(route_key)
        } else {
            None
        }
    }

    /// Locked conditional publication for one attempt.
    #[allow(clippy::too_many_arguments)]
    fn publish_startup_attempt(
        &mut self,
        route_key: &ProductionRouteKey,
        server_id: &str,
        instance_id: &str,
        created_at: std::time::Instant,
        config_fingerprint: Option<&str>,
        attempt_id: u64,
        result: Result<()>,
        worker_connection: Option<UpstreamConnection>,
        worker_token: Option<CancellationToken>,
    ) -> StartupPublish {
        let Some(entry) = self.claim_startup_attempt(route_key, attempt_id) else {
            return StartupPublish::Invalidated {
                connection: worker_connection,
                cancellation: worker_token,
            };
        };

        let route_owned = self.production_routes.get(route_key).map(String::as_str) == Some(instance_id);
        let instance_owned = self
            .get_instance(server_id, instance_id)
            .ok()
            .is_some_and(|connection| {
                connection.created_at == created_at && matches!(connection.status, ConnectionStatus::Initializing)
            });
        let config_owned = self
            .config
            .mcp_servers
            .get(server_id)
            .and_then(|config| config.source_fingerprint.as_deref())
            == config_fingerprint;
        if !(route_owned && instance_owned && config_owned) {
            if let Ok(connection) = self.get_instance_mut(server_id, instance_id)
                && connection.created_at == created_at
                && matches!(connection.status, ConnectionStatus::Initializing)
            {
                connection.update_failed("Startup attempt superseded before publication".to_string());
            }
            let _ = entry.outcome_tx.send(Some(StartupAttemptOutcome::Superseded));
            return StartupPublish::Invalidated {
                connection: worker_connection,
                cancellation: worker_token,
            };
        }

        match result {
            Ok(()) => {
                self.clear_failure_state(server_id);
                let connection = worker_connection.expect("successful startup attempt retains its worker connection");
                let replaced_connection = self
                    .instance_map_mut(route_key)
                    .insert(instance_id.to_string(), connection);
                // Only stdio transports register a cancellation token; HTTP/SSE
                // transports do not, matching the pre-existing pool behavior.
                let replaced_token = worker_token.and_then(|token| {
                    self.cancellation_tokens
                        .entry(server_id.to_string())
                        .or_default()
                        .insert(instance_id.to_string(), token)
                });
                let _ = entry
                    .outcome_tx
                    .send(Some(StartupAttemptOutcome::Published(instance_id.to_string())));
                StartupPublish::Published {
                    replaced_connection,
                    replaced_token,
                }
            }
            Err(error) => {
                let requires_manual_intervention =
                    crate::core::capability::connection_provider::PoolCapabilityConnectionProvider::authentication_failure_code(
                        &error,
                    )
                    .is_some();
                if requires_manual_intervention {
                    self.clear_failure_state(server_id);
                } else {
                    self.register_failure(
                        server_id,
                        crate::core::pool::FailureKind::Connect,
                        Some(error.to_string()),
                    );
                }
                if let Ok(connection) = self.get_instance_mut(server_id, instance_id)
                    && connection.created_at == created_at
                {
                    let message = format!("Connection failed: {}", error);
                    if requires_manual_intervention {
                        connection.update_permanent_error(message);
                    } else {
                        connection.update_failed(message);
                    }
                }
                let outcome_error = format!("{error:?}");
                let _ = entry
                    .outcome_tx
                    .send(Some(StartupAttemptOutcome::Failed(outcome_error.clone())));
                StartupPublish::Failed {
                    connection: worker_connection,
                    cancellation: worker_token,
                    error: outcome_error,
                }
            }
        }
    }

    /// Mutable instance map for the route's affinity partition.
    fn instance_map_mut(
        &mut self,
        route_key: &ProductionRouteKey,
    ) -> &mut std::collections::HashMap<String, UpstreamConnection> {
        match &route_key.affinity_key {
            AffinityKey::Default => self.connections.entry(route_key.server_id.clone()).or_default(),
            AffinityKey::PerClient(bound_id) | AffinityKey::PerSession(bound_id) => self
                .client_bound_connections
                .entry((route_key.server_id.clone(), bound_id.clone()))
                .or_default(),
        }
    }

    /// Explicitly discard a startup attempt's temporary transport outside the
    /// pool lock. The attempt owns the transport until conditional publication,
    /// so discarding never touches a newer published instance.
    pub(crate) async fn discard_startup_result(
        connection: Option<UpstreamConnection>,
        cancellation: Option<CancellationToken>,
    ) {
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
        let Some(mut connection) = connection else {
            return;
        };
        let Some(service) = connection.service.take() else {
            return;
        };
        service.cancellation_token().cancel();
        match Arc::try_unwrap(service) {
            Ok(service) => {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(5), service.cancel()).await;
            }
            Err(_) => {
                tracing::warn!(
                    instance_id = %connection.id,
                    "Discarded startup service still has another owner"
                );
            }
        }
    }
}

/// Start the attempt's transport outside the pool lock and publish under lock.
async fn run_startup_attempt(
    pool: &Arc<Mutex<UpstreamConnectionPool>>,
    attempt: StartupAttempt,
) -> StartupAttemptOutcome {
    let mut worker = attempt.worker;
    let result = worker
        .connect_transport_for_startup(&attempt.server_id, &attempt.instance_id)
        .await;
    let worker_connection = match &attempt.route_key.affinity_key {
        AffinityKey::Default => worker
            .connections
            .get_mut(&attempt.server_id)
            .and_then(|instances| instances.remove(&attempt.instance_id)),
        AffinityKey::PerClient(bound_id) | AffinityKey::PerSession(bound_id) => worker
            .client_bound_connections
            .get_mut(&(attempt.server_id.clone(), bound_id.clone()))
            .and_then(|instances| instances.remove(&attempt.instance_id)),
    };
    let worker_token = worker
        .cancellation_tokens
        .get_mut(&attempt.server_id)
        .and_then(|tokens| tokens.remove(&attempt.instance_id));

    let failure_message = result.as_ref().err().map(ToString::to_string);
    let publish = {
        let mut guard = pool.lock().await;
        guard.publish_startup_attempt(
            &attempt.route_key,
            &attempt.server_id,
            &attempt.instance_id,
            attempt.created_at,
            attempt.config_fingerprint.as_deref(),
            attempt.attempt_id,
            result,
            worker_connection,
            worker_token,
        )
    };

    match publish {
        StartupPublish::Published {
            replaced_connection,
            replaced_token,
        } => {
            UpstreamConnectionPool::discard_startup_result(replaced_connection, replaced_token).await;
            let event_database = pool.lock().await.database.clone();
            UpstreamConnectionPool::publish_startup_result(event_database, &attempt.server_id, true, None).await;
            StartupAttemptOutcome::Published(attempt.instance_id)
        }
        StartupPublish::Failed {
            connection,
            cancellation,
            error,
        } => {
            UpstreamConnectionPool::discard_startup_result(connection, cancellation).await;
            let event_database = pool.lock().await.database.clone();
            UpstreamConnectionPool::publish_startup_result(event_database, &attempt.server_id, false, failure_message)
                .await;
            StartupAttemptOutcome::Failed(error)
        }
        StartupPublish::Invalidated {
            connection,
            cancellation,
        } => {
            UpstreamConnectionPool::discard_startup_result(connection, cancellation).await;
            StartupAttemptOutcome::Superseded
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use rmcp::{ServerHandler, ServiceExt};
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::common::{constants::protocol, server::ServerType};
    use crate::core::capability::AffinityKey;
    use crate::core::models::{Config, MCPServerConfig};
    use crate::core::pool::UpstreamConnection;

    const SLOW_STARTUP_FIXTURE: &str = r#"
import json
import os
import sys
import time

marker = os.environ.get("STARTUP_MARKER")
delay = float(os.environ.get("STARTUP_DELAY_SECS", "0"))
for line in sys.stdin:
    request = json.loads(line)
    request_id = request.get("id")
    if request_id is None:
        continue
    if request.get("method") == "initialize":
        if marker:
            with open(marker, "a") as f:
                f.write("init\n")
        time.sleep(delay)
        result = {
            "protocolVersion": "__PROTOCOL_VERSION__",
            "capabilities": {},
            "serverInfo": {"name": "startup-fixture", "version": "1.0.0"},
        }
    elif request.get("method") == "tools/list":
        result = {"tools": []}
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()
"#;

    const SLOW_FAILING_STARTUP_FIXTURE: &str = r#"
import json
import os
import sys
import time

counter = os.environ.get("STARTUP_COUNTER")
if counter:
    with open(counter, "a") as f:
        f.write("start\n")
marker = os.environ.get("STARTUP_MARKER")
delay = float(os.environ.get("STARTUP_DELAY_SECS", "0"))
for line in sys.stdin:
    request = json.loads(line)
    request_id = request.get("id")
    if request_id is None:
        continue
    if request.get("method") == "initialize":
        if marker:
            with open(marker, "a") as f:
                f.write("init\n")
        time.sleep(delay)
        error = {"code": -32000, "message": "slow initialize failure"}
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "error": error}) + "\n")
        sys.stdout.flush()
        break
"#;

    fn write_fixture(
        dir: &TempDir,
        name: &str,
        source: &str,
    ) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, source.replace("__PROTOCOL_VERSION__", protocol::CURRENT_VERSION))
            .expect("write stdio fixture");
        path
    }

    fn stdio_server_config(
        python: &Path,
        script: &Path,
        fingerprint: Option<&str>,
        env: Option<HashMap<String, String>>,
    ) -> MCPServerConfig {
        MCPServerConfig {
            source_fingerprint: fingerprint.map(ToOwned::to_owned),
            kind: ServerType::Stdio,
            command: Some(python.to_string_lossy().into_owned()),
            args: Some(vec![script.to_string_lossy().into_owned()]),
            url: None,
            env,
            headers: None,
        }
    }

    fn selection(server_id: &str) -> ConnectionSelection {
        ConnectionSelection {
            server_id: server_id.to_string(),
            affinity_key: AffinityKey::Default,
        }
    }

    async fn wait_for_marker(
        path: &Path,
        timeout: Duration,
    ) {
        tokio::time::timeout(timeout, async {
            loop {
                if path.exists() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("fixture marker should appear within timeout");
    }

    #[derive(Clone, Default)]
    struct TestServer;

    impl ServerHandler for TestServer {}

    async fn ready_connection(
        server_id: &str,
        instance_id: &str,
    ) -> (UpstreamConnection, tokio::task::JoinHandle<anyhow::Result<()>>) {
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_handle = tokio::spawn(async move {
            let service = TestServer.serve(server_transport).await?;
            service.waiting().await?;
            Ok(())
        });
        let service = crate::core::transport::client::UpstreamClientHandler::new(server_id.to_string())
            .serve(client_transport)
            .await
            .expect("startup test client should initialize");
        let mut connection = UpstreamConnection::new(server_id.to_string());
        connection.id = instance_id.to_string();
        connection.update_connected(service, Vec::new(), Some(rmcp::model::ServerCapabilities::default()));
        (connection, server_handle)
    }

    /// Contract 1: a slow server A (blocked during initialize) must not block
    /// server B's prepare, status reads, or independent startup.
    #[tokio::test]
    async fn slow_server_startup_does_not_block_other_server_prepare_or_start() {
        let temp = TempDir::new().expect("temp dir");
        let python = which::which("python3").expect("python3 is required for the stdio fixture");
        let slow_script = write_fixture(&temp, "slow_startup.py", SLOW_STARTUP_FIXTURE);
        let fast_script = write_fixture(&temp, "fast_startup.py", SLOW_STARTUP_FIXTURE);
        let marker = temp.path().join("slow-init.marker");
        let slow_env = HashMap::from([
            ("STARTUP_MARKER".to_string(), marker.to_string_lossy().into_owned()),
            ("STARTUP_DELAY_SECS".to_string(), "5.0".to_string()),
        ]);

        let mut config = Config::default();
        config.mcp_servers.insert(
            "server-a".to_string(),
            stdio_server_config(&python, &slow_script, Some("v1"), Some(slow_env)),
        );
        config.mcp_servers.insert(
            "server-b".to_string(),
            stdio_server_config(&python, &fast_script, Some("v1"), None),
        );
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(Arc::new(config), None)));

        let pool_a = pool.clone();
        let demand_a = tokio::spawn(async move {
            UpstreamConnectionPool::ensure_connected_coordinated(&pool_a, &selection("server-a")).await
        });
        wait_for_marker(&marker, Duration::from_secs(10)).await;

        let pool_b = pool.clone();
        let t_begin = Instant::now();
        let result_b = tokio::time::timeout(Duration::from_secs(2), async move {
            UpstreamConnectionPool::ensure_connected_coordinated(&pool_b, &selection("server-b")).await
        })
        .await
        .expect("server-b must start while server-a is still initializing");
        assert!(
            t_begin.elapsed() < Duration::from_secs(3),
            "server-b startup must not wait for slow server-a"
        );
        assert!(result_b.is_ok(), "server-b must reach a connected instance");

        let result_a = demand_a.await.expect("server-a demand task should complete");
        assert!(result_a.is_ok(), "server-a must eventually connect");
    }

    /// Contract 2: concurrent demands for the same current connection identity
    /// create exactly one upstream transport and share the typed outcome.
    #[tokio::test]
    async fn concurrent_demands_for_same_identity_share_one_startup_attempt() {
        let temp = TempDir::new().expect("temp dir");
        let python = which::which("python3").expect("python3 is required for the stdio fixture");
        let failing_script = write_fixture(&temp, "slow_failing_startup.py", SLOW_FAILING_STARTUP_FIXTURE);
        let marker = temp.path().join("slow-fail-init.marker");
        let counter = temp.path().join("starts.txt");
        let counter_env = HashMap::from([
            ("STARTUP_COUNTER".to_string(), counter.to_string_lossy().into_owned()),
            ("STARTUP_MARKER".to_string(), marker.to_string_lossy().into_owned()),
            ("STARTUP_DELAY_SECS".to_string(), "2.0".to_string()),
        ]);

        let mut config = Config::default();
        config.mcp_servers.insert(
            "server-a".to_string(),
            stdio_server_config(&python, &failing_script, Some("v1"), Some(counter_env)),
        );
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(Arc::new(config), None)));

        // The first demand starts and blocks in initialize; the second demand
        // must join the same in-flight attempt deterministically.
        let pool_one = pool.clone();
        let demand_one = tokio::spawn(async move {
            UpstreamConnectionPool::ensure_connected_coordinated(&pool_one, &selection("server-a")).await
        });
        wait_for_marker(&marker, Duration::from_secs(10)).await;

        let pool_two = pool.clone();
        let demand_two = tokio::spawn(async move {
            UpstreamConnectionPool::ensure_connected_coordinated(&pool_two, &selection("server-a")).await
        });

        let result_one = demand_one.await.expect("first demand task should complete");
        let result_two = demand_two.await.expect("second demand task should complete");
        let error_one = result_one.expect_err("first demand must observe the startup failure");
        let error_two = result_two.expect_err("second demand must share the same startup failure");
        assert_eq!(
            format!("{error_one:?}"),
            format!("{error_two:?}"),
            "concurrent demands must wait for the same typed outcome"
        );

        let starts = std::fs::read_to_string(&counter).unwrap_or_default();
        assert_eq!(
            starts.matches("start\n").count(),
            1,
            "concurrent demands must create exactly one upstream transport"
        );
    }

    /// Contract 3: when the route/configuration/generation changes during
    /// startup, the stale result must not be published.
    #[tokio::test]
    async fn stale_startup_result_is_not_published_after_config_fingerprint_change() {
        let temp = TempDir::new().expect("temp dir");
        let python = which::which("python3").expect("python3 is required for the stdio fixture");
        let slow_script = write_fixture(&temp, "stale_startup.py", SLOW_STARTUP_FIXTURE);
        let marker = temp.path().join("stale-init.marker");
        let slow_env = HashMap::from([
            ("STARTUP_MARKER".to_string(), marker.to_string_lossy().into_owned()),
            ("STARTUP_DELAY_SECS".to_string(), "1.5".to_string()),
        ]);

        let mut config = Config::default();
        config.mcp_servers.insert(
            "server-a".to_string(),
            stdio_server_config(&python, &slow_script, Some("v1"), Some(slow_env)),
        );
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(Arc::new(config), None)));

        let pool_a = pool.clone();
        let demand_a = tokio::spawn(async move {
            UpstreamConnectionPool::ensure_connected_coordinated(&pool_a, &selection("server-a")).await
        });
        wait_for_marker(&marker, Duration::from_secs(10)).await;

        let pool_cfg = pool.clone();
        tokio::spawn(async move {
            let mut guard = pool_cfg.lock().await;
            let cfg = Arc::make_mut(&mut guard.config);
            cfg.mcp_servers
                .get_mut("server-a")
                .expect("server-a config exists")
                .source_fingerprint = Some("v2".to_string());
        })
        .await
        .expect("config change task should complete");

        let instance_id = demand_a
            .await
            .expect("demand task should complete")
            .expect("startup must retry under the new configuration and publish");

        let guard = pool.lock().await;
        let connection = guard
            .get_instance("server-a", &instance_id)
            .expect("published instance exists");
        assert_eq!(
            connection.config_fingerprint.as_deref(),
            Some("v2"),
            "stale startup result must not be published under the old configuration"
        );
    }

    /// Contract 5: after a newer instance is published, the stale attempt's
    /// cleanup must not cancel or close the newer service.
    #[tokio::test]
    async fn late_startup_cleanup_does_not_cancel_or_close_a_newer_instance() {
        let (new_connection, new_server_handle) = ready_connection("server-a", "instance-a").await;
        let new_token = CancellationToken::new();
        let mut pool = UpstreamConnectionPool::new(Arc::new(Config::default()), None);
        pool.connections.insert(
            "server-a".to_string(),
            HashMap::from([("instance-a".to_string(), new_connection.clone())]),
        );
        pool.cancellation_tokens.insert(
            "server-a".to_string(),
            HashMap::from([("instance-a".to_string(), new_token.clone())]),
        );

        let (old_connection, old_server_handle) = ready_connection("server-a", "instance-a").await;
        let old_token = CancellationToken::new();

        UpstreamConnectionPool::discard_startup_result(Some(old_connection), Some(old_token)).await;

        assert!(
            !new_token.is_cancelled(),
            "stale attempt cleanup must not cancel the newer instance token"
        );
        let published = pool
            .get_instance("server-a", "instance-a")
            .expect("newer instance remains published");
        let service = published
            .service
            .as_ref()
            .expect("newer instance service remains published");
        assert!(!service.is_closed(), "stale cleanup must not close the newer service");
        service.cancellation_token().cancel();
        let _ = service;
        drop(pool);

        tokio::time::timeout(Duration::from_secs(5), new_server_handle)
            .await
            .expect("new server should stop after cancellation")
            .expect("new server task should join")
            .expect("new server should stop");
        tokio::time::timeout(Duration::from_secs(5), old_server_handle)
            .await
            .expect("old server should stop after discard")
            .expect("old server task should join")
            .expect("old server should stop");
    }

    /// Contract 1/6 for HTTP transports: a successful startup with no pool
    /// cancellation token (HTTP transports never register one) must publish
    /// without panicking or blocking the outcome (C1 regression guard).
    #[tokio::test]
    async fn startup_publish_tolerates_missing_pool_token() {
        let (connection, server_handle) = ready_connection("server-a", "instance-a").await;
        let mut config = Config::default();
        config.mcp_servers.insert(
            "server-a".to_string(),
            MCPServerConfig {
                source_fingerprint: Some("v1".to_string()),
                kind: ServerType::StreamableHttp,
                command: None,
                args: None,
                url: Some("http://127.0.0.1:1/mcp".to_string()),
                env: None,
                headers: None,
            },
        );
        let mut pool = UpstreamConnectionPool::new(Arc::new(config), None);
        let mut instance = UpstreamConnection::new("server-a".to_string());
        instance.id = "instance-a".to_string();
        instance.update_initializing();
        let created_at = instance.created_at;
        pool.connections.insert(
            "server-a".to_string(),
            HashMap::from([("instance-a".to_string(), instance)]),
        );
        let route_key = ProductionRouteKey::shareable("server-a");
        pool.production_routes
            .insert(route_key.clone(), "instance-a".to_string());
        let (outcome_tx, outcome_rx) = watch::channel(None);
        pool.startup_attempts.insert(
            route_key.clone(),
            StartupAttemptEntry {
                attempt_id: 11,
                outcome_tx,
            },
        );

        let publish = pool.publish_startup_attempt(
            &route_key,
            "server-a",
            "instance-a",
            created_at,
            Some("v1"),
            11,
            Ok(()),
            Some(connection),
            None,
        );
        assert!(
            matches!(
                publish,
                StartupPublish::Published {
                    replaced_token: None,
                    ..
                }
            ),
            "missing pool token must not panic or block publication"
        );
        let outcome = outcome_rx.borrow().clone().expect("outcome must be published");
        assert!(
            matches!(outcome, StartupAttemptOutcome::Published(instance) if instance == "instance-a"),
            "joiners must receive the published instance id"
        );
        let published = pool
            .get_instance("server-a", "instance-a")
            .expect("published instance exists");
        assert!(published.service.is_some(), "service must be published");
        assert!(published.is_connected(), "instance must be ready");
        let service = published.service.clone().expect("service published");
        service.cancellation_token().cancel();
        let _ = published;
        drop(pool);

        tokio::time::timeout(Duration::from_secs(5), server_handle)
            .await
            .expect("server should stop after cancellation")
            .expect("server task should join")
            .expect("server should stop");
    }

    /// Affinity-bound production routes allocate instances into
    /// `client_bound_connections`; their startup must publish from that map
    /// instead of panicking on a missing shared connection (Copilot finding).
    #[tokio::test]
    async fn affinity_bound_startup_publishes_from_client_bound_instances() {
        let temp = TempDir::new().expect("temp dir");
        let python = which::which("python3").expect("python3 is required for the stdio fixture");
        let fast_script = write_fixture(&temp, "affinity_fast.py", SLOW_STARTUP_FIXTURE);

        let mut config = Config::default();
        config.mcp_servers.insert(
            "server-a".to_string(),
            stdio_server_config(&python, &fast_script, Some("v1"), None),
        );
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(Arc::new(config), None)));

        let selection = ConnectionSelection {
            server_id: "server-a".to_string(),
            affinity_key: AffinityKey::PerClient("client-1".to_string()),
        };
        let pool_a = pool.clone();
        let instance_id = tokio::time::timeout(Duration::from_secs(10), async move {
            UpstreamConnectionPool::ensure_connected_coordinated(&pool_a, &selection).await
        })
        .await
        .expect("affinity-bound startup must complete within timeout")
        .expect("affinity-bound startup must succeed");

        let guard = pool.lock().await;
        let bound_instances = guard
            .client_bound_connections
            .get(&("server-a".to_string(), "client-1".to_string()))
            .expect("affinity-bound instance must live in client_bound_connections");
        let bound = bound_instances
            .get(&instance_id)
            .expect("published affinity-bound instance exists");
        assert!(bound.is_connected(), "affinity-bound instance must be ready");
        assert!(bound.service.is_some(), "affinity-bound service must be published");
        drop(guard);

        pool.lock()
            .await
            .disconnect("server-a", &instance_id)
            .await
            .expect("disconnect affinity-bound instance");
    }
}
