//! Core connection pool implementation separated from module index.

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{self, Result};
use rmcp::service::{Peer, RoleClient};
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing;

use crate::core::{
    foundation::{monitor::ProcessMonitor, types::ConnectionStatus},
    models::Config,
};

const STANDARD_INSTANCE_IDLE_SECS: u64 = 5 * 60;

use super::config::PoolConfigManager;
use super::sync::ServerSyncManager;
use super::types::{self, FailureKind};

type InstanceSnapshot = (String, ConnectionStatus, bool, bool, Option<Peer<RoleClient>>);
type SnapshotMap = HashMap<String, Vec<InstanceSnapshot>>;

/// Build-in lightweight HTTP client registry kept at pool layer.
///
/// The registry maps an origin (scheme://host[:port]) to a shared reqwest::Client
/// enabling TCP/TLS/HTTP2 reuse across requests to the same upstream.
/// It's intentionally located in the pool to keep strategy/state close to
/// connection/backoff logic while keeping transport layer stateless.
#[derive(Clone, Debug)]
pub(crate) struct HttpClientRegistry {
    inner: Arc<tokio::sync::RwLock<std::collections::HashMap<String, reqwest::Client>>>,
}

impl HttpClientRegistry {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Derive a reuse key from URL string (scheme://host[:port])
    pub(crate) fn origin_key(url: &str) -> Option<String> {
        if let Ok(parsed) = url::Url::parse(url) {
            let scheme = parsed.scheme();
            if let Some(host) = parsed.host_str() {
                let port = parsed.port().map(|p| format!(":{}", p)).unwrap_or_default();
                return Some(format!("{}://{}{}", scheme, host, port));
            }
        }
        None
    }

    /// Get or create a reqwest Client for the given origin key
    pub(crate) async fn get_or_create(
        &self,
        origin: &str,
    ) -> reqwest::Client {
        {
            let map = self.inner.read().await;
            if let Some(c) = map.get(origin) {
                return c.clone();
            }
        }

        let client = Self::build_client();
        let mut map = self.inner.write().await;
        let entry = map.entry(origin.to_string()).or_insert_with(|| client.clone());
        entry.clone()
    }

    fn build_client() -> reqwest::Client {
        // Environment-configurable HTTP pooling and timeouts (with sensible defaults)
        let idle_ms = std::env::var("MCPMATE_HTTP_POOL_IDLE_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(45_000);
        let max_idle = std::env::var("MCPMATE_HTTP_POOL_MAX_IDLE_PER_HOST")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8);
        let keepalive_ms = std::env::var("MCPMATE_HTTP_TCP_KEEPALIVE_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30_000);
        let connect_ms = std::env::var("MCPMATE_HTTP_CONNECT_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60_000);
        let request_ms = std::env::var("MCPMATE_HTTP_REQUEST_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60_000);
        let accept_invalid = std::env::var("MCPMATE_ACCEPT_INVALID_CERTS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let user_agent = std::env::var("MCPMATE_USER_AGENT").unwrap_or_else(|_| "MCPMate/1.0".to_string());

        reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_millis(idle_ms))
            .pool_max_idle_per_host(max_idle)
            .tcp_keepalive(std::time::Duration::from_millis(keepalive_ms))
            .connect_timeout(std::time::Duration::from_millis(connect_ms))
            .danger_accept_invalid_certs(accept_invalid)
            .timeout(std::time::Duration::from_millis(request_ms))
            .user_agent(user_agent)
            .build()
            .expect("Failed to build shared reqwest Client")
    }
}

/// Pool of connections to upstream MCP servers
///
/// This is the core connection pool that manages active connections to upstream MCP servers.
/// It focuses purely on connection storage, access, and basic lifecycle management.
/// Business logic for configuration synchronization and server management is handled
/// by dedicated managers (PoolConfigManager and ServerSyncManager).
#[derive(Debug, Clone)]
pub struct UpstreamConnectionPool {
    /// Map of server ID to map of instance ID to connection
    pub connections: HashMap<String, HashMap<String, types::UpstreamConnection>>,
    /// Exploration sessions: session_id -> map of server_id to connection (minimal skeleton)
    pub exploration_sessions: HashMap<String, HashMap<String, types::UpstreamConnection>>,
    /// Validation sessions: session_id -> map of server_id to connection (minimal skeleton)
    pub validation_sessions: HashMap<String, HashMap<String, types::UpstreamConnection>>,
    /// Exploration session expirations
    pub exploration_expirations: HashMap<String, std::time::Instant>,
    /// Validation session expirations
    pub validation_expirations: HashMap<String, std::time::Instant>,
    /// Server configuration
    pub config: Arc<Config>,
    /// Map of server ID to map of instance ID to cancellation token
    pub cancellation_tokens: HashMap<String, HashMap<String, CancellationToken>>,
    /// Process monitor for tracking resource usage
    pub process_monitor: Option<Arc<ProcessMonitor>>,
    /// Database reference for checking server status (used by sync manager)
    pub database: Option<Arc<crate::config::database::Database>>,
    /// Runtime cache for fast runtime queries
    pub runtime_cache: Option<Arc<crate::runtime::RuntimeCache>>,
    /// Failure tracking for circuit breaking/backoff
    pub failure_states: HashMap<String, types::FailureState>,
    /// Optional shared HTTP client registry for transport reuse
    pub(crate) http_clients: Option<Arc<crate::core::pool::connection::HttpClientRegistry>>,
}

impl UpstreamConnectionPool {
    /// Create a new connection pool
    ///
    /// # Arguments
    /// * `config` - The server configuration
    /// * `database` - Optional database reference for checking server status
    pub fn new(
        config: Arc<Config>,
        database: Option<Arc<crate::config::database::Database>>,
    ) -> Self {
        // Create process monitor with 5 second update interval
        let process_monitor = Arc::new(ProcessMonitor::new(Duration::from_secs(5)));

        // Start process monitoring
        ProcessMonitor::start_monitoring(process_monitor.clone());

        Self {
            connections: HashMap::new(),
            exploration_sessions: HashMap::new(),
            validation_sessions: HashMap::new(),
            exploration_expirations: HashMap::new(),
            validation_expirations: HashMap::new(),
            config,
            cancellation_tokens: HashMap::new(),
            process_monitor: Some(process_monitor),
            database,
            runtime_cache: None, // Will be set by the proxy server
            failure_states: HashMap::new(),
            http_clients: {
                // Prefer new env name MCPMATE_HTTP_CLIENT_REUSE, fallback to legacy MCMP_MATE_HTTP_CLIENT_REUSE
                let reuse = std::env::var("MCPMATE_HTTP_CLIENT_REUSE")
                    .ok()
                    .or_else(|| std::env::var("MCMP_MATE_HTTP_CLIENT_REUSE").ok());
                let enabled = match reuse.as_deref() {
                    Some("0") | Some("false") | Some("off") => false,
                    _ => true, // default ON unless explicitly disabled
                };
                if enabled {
                    Some(Arc::new(crate::core::pool::connection::HttpClientRegistry::new()))
                } else {
                    None
                }
            },
        }
    }

    // (moved to module scope) HttpClientRegistry definition

    /// Update the configuration using the configuration manager
    ///
    /// This method delegates to PoolConfigManager for the actual configuration logic.
    /// It maintains the public API while separating business logic concerns.
    ///
    /// Returns Ok(()) on success, or Err(CoreError) if configuration update fails.
    pub fn set_config(
        &mut self,
        config: Arc<Config>,
    ) -> Result<(), crate::core::foundation::error::CoreError> {
        // Use the configuration manager to handle the complex logic
        PoolConfigManager::update_configuration(&mut self.connections, &mut self.cancellation_tokens, config.clone())?;

        // Update the stored configuration reference
        self.config = config;
        tracing::info!("Pool configuration updated successfully");
        Ok(())
    }

    /// Set the database reference
    pub fn set_database(
        &mut self,
        database: Option<Arc<crate::config::database::Database>>,
    ) {
        self.database = database;
        tracing::info!("Database reference updated for connection pool");
    }

    /// Set the runtime cache reference
    pub fn set_runtime_cache(
        &mut self,
        runtime_cache: Option<Arc<crate::runtime::RuntimeCache>>,
    ) {
        self.runtime_cache = runtime_cache;
        tracing::info!("Runtime cache reference updated for connection pool");
    }

    /// Idle timeout for standard instances (may become configurable later)
    pub fn standard_instance_idle_timeout() -> Duration {
        let env_override = std::env::var("MCPMATE_INSTANCE_IDLE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok());
        Duration::from_secs(env_override.unwrap_or(STANDARD_INSTANCE_IDLE_SECS))
    }

    /// Disconnect instances that have been idle beyond the configured timeout.
    pub async fn reap_idle_instances(&mut self) {
        let idle_timeout = Self::standard_instance_idle_timeout();
        let now = Instant::now();
        let mut targets: Vec<(String, String)> = Vec::new();

        for (server_id, instances) in &self.connections {
            for (instance_id, conn) in instances {
                if matches!(conn.status, ConnectionStatus::Ready)
                    && now.duration_since(conn.last_activity) >= idle_timeout
                {
                    targets.push((server_id.clone(), instance_id.clone()));
                }
            }
        }

        for (server_id, instance_id) in targets {
            match self.disconnect_non_blocking(&server_id, &instance_id).await {
                Ok(()) => tracing::info!(
                    server_id = %server_id,
                    instance_id = %instance_id,
                    idle_timeout_secs = idle_timeout.as_secs(),
                    "Disconnected idle instance after timeout"
                ),
                Err(e) => tracing::warn!(
                    server_id = %server_id,
                    instance_id = %instance_id,
                    error = %e,
                    "Failed to disconnect idle instance"
                ),
            }
        }
    }

    fn failure_state_mut(
        &mut self,
        server_id: &str,
    ) -> &mut types::FailureState {
        self.failure_states.entry(server_id.to_string()).or_default()
    }

    pub fn register_failure(
        &mut self,
        server_id: &str,
        kind: FailureKind,
        reason: Option<String>,
    ) -> Duration {
        let backoff = self
            .failure_state_mut(server_id)
            .register_failure(Instant::now(), kind, reason.clone());

        tracing::warn!(
            server_id = server_id,
            failure_kind = kind.as_str(),
            backoff_secs = backoff.as_secs_f32(),
            reason = reason.as_deref().unwrap_or("<none>"),
            "Registering failure for server, entering backoff"
        );

        backoff
    }

    pub fn clear_failure_state(
        &mut self,
        server_id: &str,
    ) {
        if let Some(state) = self.failure_states.get_mut(server_id) {
            if state.consecutive_failures > 0 {
                tracing::debug!(
                    server_id = server_id,
                    "Clearing failure state after successful operation"
                );
            }
            state.record_success();
        }
    }

    pub fn remaining_backoff(
        &self,
        server_id: &str,
    ) -> Option<Duration> {
        let now = Instant::now();
        self.failure_states
            .get(server_id)
            .and_then(|state| state.remaining_backoff(now))
    }

    /// Initialize the connection pool with all servers
    ///
    /// This method delegates to PoolConfigManager for the initialization logic.
    pub fn initialize(&mut self) {
        PoolConfigManager::initialize_connections(&mut self.connections, &self.config);
    }

    // Instance helper methods are now in instance_helpers.rs

    /// Sync all servers based on active profile
    ///
    /// This method delegates to ServerSyncManager for the complex synchronization logic.
    /// It maintains the public API while separating business logic concerns.
    pub async fn sync_servers_from_active_profile(&mut self) -> Result<()> {
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not available for server sync"))?;

        let sync_manager = ServerSyncManager::new(db.clone());
        sync_manager.sync_servers_from_active_profile(self).await
    }

    /// Create or refresh an exploration session with TTL
    pub fn upsert_exploration_session(
        &mut self,
        session_id: &str,
        ttl: Duration,
    ) {
        use std::time::Instant;
        self.exploration_sessions.entry(session_id.to_string()).or_default();
        self.exploration_expirations
            .insert(session_id.to_string(), Instant::now() + ttl);
    }

    /// Create or refresh a validation session with TTL
    pub fn upsert_validation_session(
        &mut self,
        session_id: &str,
        ttl: Duration,
    ) {
        use std::time::Instant;
        self.validation_sessions.entry(session_id.to_string()).or_default();
        self.validation_expirations
            .insert(session_id.to_string(), Instant::now() + ttl);
    }

    /// Cleanup expired exploration/validation sessions
    pub fn cleanup_expired_sessions(&mut self) {
        use std::time::Instant;
        let now = Instant::now();
        let expired_exploration: Vec<String> = self
            .exploration_expirations
            .iter()
            .filter_map(|(sid, &exp)| if exp <= now { Some(sid.clone()) } else { None })
            .collect();
        for sid in expired_exploration {
            self.exploration_expirations.remove(&sid);
            self.exploration_sessions.remove(&sid);
        }

        let expired_validation: Vec<String> = self
            .validation_expirations
            .iter()
            .filter_map(|(sid, &exp)| if exp <= now { Some(sid.clone()) } else { None })
            .collect();
        for sid in expired_validation {
            self.validation_expirations.remove(&sid);
            self.validation_sessions.remove(&sid);
        }
    }

    /// Get active instance counts for runtime status
    pub fn active_instance_counts(&self) -> (usize, usize, usize) {
        let production = self.connections.iter().filter(|(_, m)| !m.is_empty()).count();
        let exploration = self.exploration_sessions.len();
        let validation = self.validation_sessions.len();
        (production, exploration, validation)
    }

    /// Get connection pool snapshot for read-only operations (optimized for concurrent access)
    ///
    /// This method provides a fast, consistent snapshot of the connection pool state
    /// without requiring exclusive access. It's designed for API queries and monitoring.
    pub fn get_connection_snapshot(&self) -> HashMap<String, Vec<(String, types::UpstreamConnection)>> {
        let mut result = HashMap::new();
        let snapshot_time = Instant::now();

        for (server_id, instances) in &self.connections {
            let instance_clones: Vec<(String, types::UpstreamConnection)> = instances
                .iter()
                .map(|(id, conn)| {
                    // Create a lightweight clone for snapshot
                    let mut conn_clone = conn.clone();
                    // Add snapshot metadata
                    conn_clone.last_snapshot = Some(snapshot_time);
                    (id.clone(), conn_clone)
                })
                .collect();

            if !instance_clones.is_empty() {
                result.insert(server_id.clone(), instance_clones);
            }
        }

        tracing::debug!(
            "Created connection pool snapshot with {} servers and {} total instances",
            result.len(),
            result.values().map(|instances| instances.len()).sum::<usize>()
        );

        result
    }

    /// Get a lightweight snapshot for read-only operations (minimal cloning, no tool vectors)
    /// Returns: server_id -> Vec of (instance_id, status, supports_resources, supports_prompts, service_peer)
    pub fn get_snapshot(&self) -> SnapshotMap {
        let mut result: SnapshotMap = HashMap::new();

        for (server_id, instances) in &self.connections {
            let mut vec = Vec::with_capacity(instances.len());
            for (id, conn) in instances.iter() {
                let supports_resources = conn.supports_resources();
                let supports_prompts = conn.supports_prompts();
                let peer = conn.service.as_ref().map(|svc| svc.peer().clone());
                vec.push((
                    id.clone(),
                    conn.status.clone(),
                    supports_resources,
                    supports_prompts,
                    peer,
                ));
            }
            if !vec.is_empty() {
                result.insert(server_id.clone(), vec);
            }
        }

        result
    }

    /// Get server status summary for quick API responses
    ///
    /// Returns a lightweight summary without full connection details
    pub fn get_server_status_summary(&self) -> HashMap<String, (usize, usize, String)> {
        let mut summary = HashMap::new();

        for (server_id, instances) in &self.connections {
            let total_instances = instances.len();
            let connected_instances = instances
                .values()
                .filter(|conn| matches!(conn.status, ConnectionStatus::Ready))
                .count();

            let overall_status = if connected_instances == 0 {
                "disconnected".to_string()
            } else if connected_instances == total_instances {
                "connected".to_string()
            } else {
                "partial".to_string()
            };

            summary.insert(
                server_id.clone(),
                (total_instances, connected_instances, overall_status),
            );
        }

        summary
    }

    /// Get or create an exploration instance for a server
    pub fn get_or_create_exploration_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
        ttl: Duration,
    ) -> Result<Option<&types::UpstreamConnection>, anyhow::Error> {
        // Ensure session exists
        self.upsert_exploration_session(session_id, ttl);

        // Check if server connection already exists in this session
        if let Some(session_servers) = self.exploration_sessions.get(session_id) {
            if let Some(connection) = session_servers.get(server_name) {
                return Ok(Some(connection));
            }
        }

        // For now, return None - full implementation would create actual connection
        // This would involve:
        // 1. Get server config from database
        // 2. Create new UpstreamConnection
        // 3. Initialize connection to server
        // 4. Store in exploration_sessions

        tracing::debug!(
            "Exploration instance for server '{}' in session '{}' not implemented yet",
            server_name,
            session_id
        );
        Ok(None)
    }

    /// Get or create a validation instance for a server
    ///
    /// This method implements "create-use-destroy" lifecycle for validation instances:
    /// 1. Check if validation instance already exists in session
    /// 2. If not, create temporary validation instance
    /// 3. Instance will be destroyed after use (handled by caller)
    pub async fn get_or_create_validation_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
        _ttl: Duration, // TTL not used for validation instances per requirements
    ) -> Result<Option<&types::UpstreamConnection>, anyhow::Error> {
        // Check if server connection already exists in this session
        if let Some(session_servers) = self.validation_sessions.get(session_id) {
            if session_servers.contains_key(server_name) {
                return Ok(self
                    .validation_sessions
                    .get(session_id)
                    .and_then(|session| session.get(server_name)));
            }
        }

        // Create temporary validation instance
        let connection = self
            .create_temporary_validation_instance(server_name, session_id)
            .await?;

        // Store in validation_sessions
        let session_servers = self.validation_sessions.entry(session_id.to_string()).or_default();
        session_servers.insert(server_name.to_string(), connection);

        // Return reference to the stored connection
        Ok(self
            .validation_sessions
            .get(session_id)
            .and_then(|session| session.get(server_name)))
    }

    /// Create a temporary validation instance for a server
    ///
    /// This creates a temporary connection that will be used for capability inspection
    /// and then immediately destroyed. It does not affect the server's enabled status.
    async fn create_temporary_validation_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
    ) -> Result<types::UpstreamConnection, anyhow::Error> {
        tracing::info!("Creating temporary validation instance for server: {}", server_name);

        // Get database connection
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not available"))?;

        // Get server configuration from database
        let server = crate::config::server::get_server(&db.pool, server_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

        // Use server_id if available; fall back to server_name for failure tracking
        let failure_key: String = server.id.clone().unwrap_or_else(|| server_name.to_string());

        // Respect backoff window for faulty upstreams to avoid repeated connection storms
        if let Some(remaining) = self.remaining_backoff(&failure_key) {
            tracing::warn!(
                server = %server_name,
                backoff_secs = remaining.as_secs_f32(),
                "Skipping validation instance creation due to active backoff"
            );
            return Err(anyhow::anyhow!(
                "Server '{}' is backing off for {:.1}s",
                server_name,
                remaining.as_secs_f32()
            ));
        }

        // Convert database Server to MCPServerConfig (reusing existing conversion logic)
        let server_config = self.convert_server_to_config(&server, &db.pool).await?;

        // Create temporary connection instance with validation prefix (unified helper)
        let instance_id = crate::core::pool::helpers::format_validation_instance_id(server_name, session_id);
        let mut connection = types::UpstreamConnection::new(server_name.to_string());
        connection.id = instance_id;

        // Set validation status to distinguish from production instances
        connection.status = ConnectionStatus::Validating;

        // Connect to server using unified transport interface with a short timeout to avoid startup stalls
        // Determine transport type strictly from server_type (DB no longer stores transport_type)
        let effective_transport = match server.server_type {
            crate::common::server::ServerType::StreamableHttp => crate::common::server::TransportType::StreamableHttp,
            crate::common::server::ServerType::Sse => crate::common::server::TransportType::Sse,
            crate::common::server::ServerType::Stdio => crate::common::server::TransportType::Stdio,
        };

        let connect_fut = crate::core::transport::unified::connect_server(
            server_name,
            &server_config,
            server.server_type,
            effective_transport,
            None, // No cancellation token needed for short-lived validation
            Some(&db.pool),
            self.runtime_cache.as_ref().map(|rc| rc.as_ref()),
        );

        // Validation connect timeout: configurable via MCPMATE_VALIDATION_CONNECT_TIMEOUT_MS (default 60000ms)
        let timeout_ms: u64 = std::env::var("MCPMATE_VALIDATION_CONNECT_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60_000); // Increased from 10s to 60s for consistency
        let (service, tools, capabilities, _process_id) =
            match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), connect_fut).await {
                Ok(Ok(ok)) => ok,
                Ok(Err(e)) => {
                    // Register failure and enter backoff to protect startup/import flows
                    let reason = format!("{}", e);
                    let _ = self.register_failure(&failure_key, FailureKind::Connect, Some(reason));
                    return Err(e);
                }
                Err(_elapsed) => {
                    let reason = format!("validation connect timeout ({}ms)", timeout_ms);
                    let _ = self.register_failure(&failure_key, FailureKind::Connect, Some(reason));
                    return Err(anyhow::anyhow!(
                        "Timed out creating validation instance for server '{}'",
                        server_name
                    ));
                }
            };

        // Update connection with service and capabilities
        connection.update_connected(service, tools, capabilities);

        tracing::info!("Created temporary validation instance for server '{}'", server_name);
        Ok(connection)
    }

    /// Convert database Server model to MCPServerConfig
    ///
    /// Reuses the conversion logic from core/foundation/loader.rs
    async fn convert_server_to_config(
        &self,
        server: &crate::config::models::Server,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> Result<crate::core::models::MCPServerConfig, anyhow::Error> {
        // Get server arguments (reusing existing logic)
        let args = if let Some(id) = &server.id {
            let server_args = crate::config::server::get_server_args(pool, id).await?;
            if server_args.is_empty() {
                None
            } else {
                let mut sorted_args: Vec<_> = server_args.into_iter().collect();
                sorted_args.sort_by_key(|arg| arg.arg_index);
                Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect())
            }
        } else {
            None
        };

        // Get server environment variables (reusing existing logic)
        let env = if let Some(id) = &server.id {
            let env_map = crate::config::server::get_server_env(pool, id).await?;
            if env_map.is_empty() { None } else { Some(env_map) }
        } else {
            None
        };

        // Load default HTTP headers if any (validation path previously missed headers)
        let headers = if let Some(id) = &server.id {
            match crate::config::server::get_server_headers(pool, id).await {
                Ok(map) if !map.is_empty() => Some(map),
                _ => None,
            }
        } else {
            None
        };

        // Create MCPServerConfig (reusing existing structure)
        Ok(crate::core::models::MCPServerConfig {
            kind: server.server_type,
            command: server.command.clone(),
            args,
            url: server.url.clone(),
            env,
            headers,
        })
    }

    /// Destroy a validation instance after use
    ///
    /// This implements the "immediate cleanup" part of the create-use-destroy lifecycle
    pub async fn destroy_validation_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
    ) -> Result<(), anyhow::Error> {
        if let Some(session_servers) = self.validation_sessions.get_mut(session_id) {
            if let Some(mut connection) = session_servers.remove(server_name) {
                // Best-effort graceful shutdown: ask running service to cancel before dropping pipes.
                if let Some(service) = connection.service.as_ref() {
                    // Use cancellation token to request shutdown without taking ownership
                    service.cancellation_token().cancel();
                }
                // Disconnect the service if still connected (clears handles/status)
                if connection.is_connected() { connection.update_disconnected(); }
                tracing::info!("Destroyed validation instance for server '{}'", server_name);
            }
        }

        Ok(())
    }
}
