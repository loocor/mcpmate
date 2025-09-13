//! Unified connection entry point - Core MCP protocol unification
//!
//! Implements the unified entry point `ensure_affinitized_connection` as specified in the refactoring guide.
//! This provides a single interface for connection management with isolation strategies and lifecycle control.
//!
//! Key features:
//! - `ensure_affinitized_connection(server, mode, affinity_key)` - unified entry point
//! - Ready → Shutdown → New instance lifecycle management
//! - Isolation strategies: shareable, per-client, per-session
//! - Resource control and monitoring

use anyhow::Result;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing;

use crate::core::{
    capability::domain::{AffinityKey, CapabilityError, ConnectionMode, IsolationMode},
    connection::UpstreamConnection,
    models::Config,
};

// Constants for unified connection management
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 90;
const DEFAULT_MAX_INSTANCES: usize = 6;
// TODO: Implement session-based timeout management for long-running connections
#[allow(dead_code)]
const DEFAULT_SESSION_TIMEOUT_SECS: u64 = 3600;
// TODO: Implement health check multiplier for adaptive health monitoring intervals
#[allow(dead_code)]
const HEALTH_CHECK_MULTIPLIER: u32 = 2;
const MAX_AUTO_REVIVE_INSTANCES: usize = 2;
// TODO: Implement periodic maintenance scheduling for connection pool optimization
#[allow(dead_code)]
const DEFAULT_MAINTENANCE_INTERVAL_SECS: u64 = 60;

/// Connection instance metadata for lifecycle management
#[derive(Debug, Clone)]
pub struct InstanceMetadata {
    /// Instance ID
    pub instance_id: String,
    /// Server ID
    pub server_id: String,
    /// Connection mode
    pub mode: ConnectionMode,
    /// Current status
    pub status: InstanceStatus,
    /// Created at timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Shutdown timestamp (if applicable)
    pub shutdown_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Session ID (for per-session mode)
    pub session_id: Option<String>,
}

/// Instance status with lifecycle states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceStatus {
    /// Initializing
    Initializing,
    /// Ready and active
    Ready,
    /// Shutdown but can be revived
    Shutdown,
    /// Error state
    Error(String),
}

/// Connection statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Total connections created
    pub total_created: u64,
    /// Active connections
    pub active_count: usize,
    /// Shutdown connections
    pub shutdown_count: usize,
    /// Cache hits (revived connections)
    pub cache_hits: u64,
    /// Connection failures
    pub failures: u64,
    /// Health check failures
    pub health_check_failures: u64,
    /// Cleanup operations performed
    pub cleanup_operations: u64,
    /// Instances revived from shutdown
    pub revived_instances: u64,
}

/// Isolation strategy configuration
#[derive(Debug, Clone)]
pub struct IsolationConfig {
    /// Default mode for server type
    pub default_mode: IsolationMode,
    /// Maximum instances per server (resource control)
    pub max_instances: usize,
    /// Idle timeout for connection cleanup
    pub idle_timeout: Duration,
    /// Whether to enable automatic connection revival
    pub enable_revival: bool,
    /// Whether to enable load balancing across instances
    pub enable_load_balancing: bool,
    /// Session timeout for per-session mode
    pub session_timeout: Option<Duration>,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            default_mode: IsolationMode::Shareable,
            max_instances: 6, // Default from refactoring guide for stdio
            idle_timeout: Duration::from_secs(90),
            enable_revival: true,
            enable_load_balancing: false, // Disabled by default for affinity
            session_timeout: Some(Duration::from_secs(3600)), // 1 hour
        }
    }
}

impl IsolationConfig {
    /// Get configuration for server type
    pub fn for_server_type(server_type: &crate::common::server::ServerType) -> Self {
        match server_type {
            crate::common::server::ServerType::Stdio => Self {
                default_mode: IsolationMode::PerClient,
                max_instances: 6, // As per refactoring guide
                idle_timeout: Duration::from_secs(90),
                enable_revival: true,
                enable_load_balancing: false,
                session_timeout: Some(Duration::from_secs(3600)),
            },
            crate::common::server::ServerType::Sse => Self {
                default_mode: IsolationMode::Shareable,
                max_instances: 3, // HTTP servers can handle more concurrent connections
                idle_timeout: Duration::from_secs(300), // Longer timeout for HTTP
                enable_revival: true,
                enable_load_balancing: true, // Enable load balancing for HTTP
                session_timeout: Some(Duration::from_secs(1800)), // 30 minutes for HTTP
            },
            crate::common::server::ServerType::StreamableHttp => Self {
                default_mode: IsolationMode::Shareable,
                max_instances: 3,
                idle_timeout: Duration::from_secs(300),
                enable_revival: true,
                enable_load_balancing: true,
                session_timeout: Some(Duration::from_secs(1800)),
            },
        }
    }

    /// Override specific configuration values
    pub fn with_max_instances(
        mut self,
        max_instances: usize,
    ) -> Self {
        self.max_instances = max_instances;
        self
    }

    pub fn with_idle_timeout(
        mut self,
        timeout: Duration,
    ) -> Self {
        self.idle_timeout = timeout;
        self
    }

    pub fn with_revival(
        mut self,
        enable: bool,
    ) -> Self {
        self.enable_revival = enable;
        self
    }

    pub fn with_load_balancing(
        mut self,
        enable: bool,
    ) -> Self {
        self.enable_load_balancing = enable;
        self
    }
}

/// Unified connection manager - implements ensure_affinitized_connection
pub struct UnifiedConnectionManager {
    /// Active instances: (server_id, affinity_key) -> InstanceMetadata
    active_instances: Arc<Mutex<HashMap<String, InstanceMetadata>>>,
    /// Connection pool reference
    pool: Arc<Mutex<crate::core::pool::UpstreamConnectionPool>>,
    /// Server configuration
    config: Arc<Config>,
    /// Statistics
    stats: Arc<Mutex<ConnectionStats>>,
    /// Isolation strategy configurations per server type
    isolation_configs: std::collections::HashMap<crate::common::server::ServerType, IsolationConfig>,
}

impl UnifiedConnectionManager {
    /// Create new unified connection manager
    pub fn new(
        pool: Arc<Mutex<crate::core::pool::UpstreamConnectionPool>>,
        config: Arc<Config>,
    ) -> Self {
        let isolation_configs = std::collections::HashMap::from([
            (
                crate::common::server::ServerType::Stdio,
                IsolationConfig::for_server_type(&crate::common::server::ServerType::Stdio),
            ),
            (
                crate::common::server::ServerType::Sse,
                IsolationConfig::for_server_type(&crate::common::server::ServerType::Sse),
            ),
            (
                crate::common::server::ServerType::StreamableHttp,
                IsolationConfig::for_server_type(&crate::common::server::ServerType::StreamableHttp),
            ),
        ]);

        Self {
            active_instances: Arc::new(Mutex::new(HashMap::new())),
            pool,
            config,
            stats: Arc::new(Mutex::new(ConnectionStats::default())),
            isolation_configs,
        }
    }

    /// Unified entry point: ensure affinitized connection
    ///
    /// This is the core unified entry point as specified in the refactoring guide:
    /// 1. Look for Ready instance in (server_id, affinity_key) → reuse
    /// 2. Otherwise look for Shutdown → revive (keep instance_id unchanged)
    /// 3. None → create new instance and register to (server_id, affinity_key)
    ///
    /// # Arguments
    /// * `server_id` - Server identifier
    /// * `mode` - Connection mode with isolation strategy and affinity key
    ///
    /// # Returns
    /// * `Result<String>` - Instance ID of the ensured connection
    pub async fn ensure_affinitized_connection(
        &self,
        server_id: &str,
        mode: ConnectionMode,
    ) -> Result<String> {
        let affinity_key = mode.affinity_key_string();
        let routing_key = format!("{}:{}", server_id, affinity_key);

        tracing::debug!(
            "ensure_affinitized_connection called for server '{}' with mode {:?}, key '{}'",
            server_id,
            mode.isolation_mode,
            routing_key
        );

        // Step 1: Look for Ready instance - early return pattern
        if let Some(instance_id) = self.find_ready_instance(&routing_key).await? {
            return self.handle_ready_instance(&routing_key, server_id, instance_id).await;
        }

        // Step 2: Look for Shutdown instance - early return pattern
        if let Some(instance_id) = self.find_shutdown_instance(&routing_key).await? {
            if (self.revive_instance(&routing_key, server_id).await).is_ok() {
                return self.handle_revived_instance(&routing_key, server_id, instance_id).await;
            }
        }

        // Step 3: Create new instance - final return
        self.create_and_register_instance(server_id, &routing_key, mode.clone())
            .await
    }

    /// Handle ready instance found - early return helper
    async fn handle_ready_instance(
        &self,
        routing_key: &str,
        server_id: &str,
        instance_id: String,
    ) -> Result<String> {
        self.update_activity(routing_key).await;
        self.increment_cache_hits().await;

        tracing::info!(
            "Reusing ready instance '{}' for server '{}' with key '{}'",
            instance_id,
            server_id,
            routing_key
        );
        Ok(instance_id)
    }

    /// Handle revived instance - early return helper
    async fn handle_revived_instance(
        &self,
        routing_key: &str,
        server_id: &str,
        instance_id: String,
    ) -> Result<String> {
        self.update_activity(routing_key).await;

        tracing::info!(
            "Revived shutdown instance '{}' for server '{}' with key '{}'",
            instance_id,
            server_id,
            routing_key
        );
        Ok(instance_id)
    }

    /// Create and register new instance - final step helper
    async fn create_and_register_instance(
        &self,
        server_id: &str,
        routing_key: &str,
        mode: ConnectionMode,
    ) -> Result<String> {
        let instance_id = self.create_new_instance(server_id, routing_key, mode).await?;

        tracing::info!(
            "Created new instance '{}' for server '{}' with key '{}'",
            instance_id,
            server_id,
            routing_key
        );

        Ok(instance_id)
    }

    /// Find ready instance by routing key
    async fn find_ready_instance(
        &self,
        routing_key: &str,
    ) -> Result<Option<String>> {
        let instances = self.active_instances.lock().await;

        if let Some(metadata) = instances.get(routing_key) {
            if matches!(metadata.status, InstanceStatus::Ready) {
                return Ok(Some(metadata.instance_id.clone()));
            }
        }

        Ok(None)
    }

    /// Find shutdown instance by routing key
    async fn find_shutdown_instance(
        &self,
        routing_key: &str,
    ) -> Result<Option<String>> {
        let instances = self.active_instances.lock().await;

        if let Some(metadata) = instances.get(routing_key) {
            if matches!(metadata.status, InstanceStatus::Shutdown) {
                return Ok(Some(metadata.instance_id.clone()));
            }
        }

        Ok(None)
    }

    /// Revive a shutdown instance
    async fn revive_instance(
        &self,
        routing_key: &str,
        server_id: &str,
    ) -> Result<()> {
        let mut instances = self.active_instances.lock().await;

        if let Some(metadata) = instances.get_mut(routing_key) {
            if matches!(metadata.status, InstanceStatus::Shutdown) {
                // Update status to ready
                metadata.status = InstanceStatus::Ready;
                metadata.last_activity = chrono::Utc::now();
                metadata.shutdown_at = None;

                // Trigger actual connection revival through pool
                let _pool = self.pool.lock().await;
                // Note: Actual connection revival logic would go here
                // This would involve re-establishing the connection to the server

                tracing::debug!("Revived instance '{}' for server '{}'", metadata.instance_id, server_id);
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "No shutdown instance found for routing key: {}",
            routing_key
        ))
    }

    /// Create new instance and register
    async fn create_new_instance(
        &self,
        server_id: &str,
        routing_key: &str,
        mode: ConnectionMode,
    ) -> Result<String> {
        // Check resource limits
        self.check_resource_limits(server_id).await?;

        // Generate instance ID
        let instance_id = self.generate_instance_id(server_id, &mode).await;

        // Create metadata
        let metadata = InstanceMetadata {
            instance_id: instance_id.clone(),
            server_id: server_id.to_string(),
            mode: mode.clone(),
            status: InstanceStatus::Initializing,
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            shutdown_at: None,
            session_id: match mode.affinity_key {
                AffinityKey::PerSession(ref id) => Some(id.clone()),
                _ => None,
            },
        };

        // Register instance
        {
            let mut instances = self.active_instances.lock().await;
            instances.insert(routing_key.to_string(), metadata);
        }

        // Increment statistics
        self.increment_total_created().await;

        // Trigger actual connection creation through pool
        self.trigger_pool_connection(server_id, &instance_id).await?;

        // Update status to ready
        {
            let mut instances = self.active_instances.lock().await;
            if let Some(metadata) = instances.get_mut(routing_key) {
                metadata.status = InstanceStatus::Ready;
            }
        }

        Ok(instance_id)
    }

    /// Get server type for server ID
    async fn get_server_type(
        &self,
        server_id: &str,
    ) -> Option<crate::common::server::ServerType> {
        // First try to get server type from configuration
        if let Some(server_config) = self.config.mcp_servers.get(server_id) {
            // Determine server type from configuration
            if server_config.command.is_some() {
                Some(crate::common::server::ServerType::Stdio)
            } else if server_config.url.is_some() {
                // Check if it's SSE or Streamable HTTP based on URL pattern
                if let Some(url) = &server_config.url {
                    if url.contains("/sse") || url.ends_with("/sse") {
                        Some(crate::common::server::ServerType::Sse)
                    } else {
                        Some(crate::common::server::ServerType::StreamableHttp)
                    }
                } else {
                    Some(crate::common::server::ServerType::StreamableHttp)
                }
            } else {
                None
            }
        } else {
            // Fallback to connection pool inference
            let pool = self.pool.lock().await;
            if let Some(server_connections) = pool.connections.get(server_id) {
                // Try to infer server type from connection characteristics
                if server_connections.len() > 3 {
                    Some(crate::common::server::ServerType::Sse) // HTTP servers handle more connections
                } else {
                    Some(crate::common::server::ServerType::Stdio)
                }
            } else {
                None
            }
        }
    }

    /// Get isolation configuration for server
    async fn get_isolation_config(
        &self,
        server_id: &str,
    ) -> Option<&IsolationConfig> {
        if let Some(server_type) = self.get_server_type(server_id).await {
            self.isolation_configs.get(&server_type)
        } else {
            // Fallback to stdio config
            self.isolation_configs.get(&crate::common::server::ServerType::Stdio)
        }
    }

    /// Check resource limits before creating new instance
    async fn check_resource_limits(
        &self,
        server_id: &str,
    ) -> Result<()> {
        let instances = self.active_instances.lock().await;

        // Count active instances for this server
        let server_active_count = instances
            .values()
            .filter(|metadata| metadata.server_id == server_id && matches!(metadata.status, InstanceStatus::Ready))
            .count();

        // Get isolation configuration for this server
        let max_instances = self
            .get_isolation_config(server_id)
            .await
            .map(|config| config.max_instances)
            .unwrap_or(DEFAULT_MAX_INSTANCES);

        if server_active_count >= max_instances {
            return Err(CapabilityError::NoAvailableInstances(server_id.to_string()).into());
        }

        Ok(())
    }

    /// Generate instance ID based on server and mode
    async fn generate_instance_id(
        &self,
        server_id: &str,
        mode: &ConnectionMode,
    ) -> String {
        let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        match mode.isolation_mode {
            IsolationMode::Shareable => format!("{}-shareable-{}", server_id, timestamp),
            IsolationMode::PerClient => match &mode.affinity_key {
                AffinityKey::PerClient(client_id) => format!("{}-client-{}", server_id, client_id),
                _ => format!("{}-client-{}", server_id, timestamp),
            },
            IsolationMode::PerSession => match &mode.affinity_key {
                AffinityKey::PerSession(session_id) => format!("{}-session-{}", server_id, session_id),
                _ => format!("{}-session-{}", server_id, timestamp),
            },
        }
    }

    /// Trigger actual connection creation through the pool
    async fn trigger_pool_connection(
        &self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        let mut pool = self.pool.lock().await;

        // Ensure server exists in pool
        if !pool.connections.contains_key(server_id) {
            // Create new connection in pool
            let connection = UpstreamConnection::new(instance_id.to_string());
            let instances = pool.connections.entry(server_id.to_string()).or_default();
            instances.insert(instance_id.to_string(), connection);
        }

        // Trigger connection
        pool.trigger_connect(server_id, instance_id).await?;

        Ok(())
    }

    /// Update last activity timestamp
    async fn update_activity(
        &self,
        routing_key: &str,
    ) {
        let mut instances = self.active_instances.lock().await;

        if let Some(metadata) = instances.get_mut(routing_key) {
            metadata.last_activity = chrono::Utc::now();
        }
    }

    /// Shutdown instance by routing key
    pub async fn shutdown_instance(
        &self,
        routing_key: &str,
    ) -> Result<()> {
        let mut instances = self.active_instances.lock().await;

        if let Some(metadata) = instances.get_mut(routing_key) {
            if matches!(metadata.status, InstanceStatus::Ready) {
                metadata.status = InstanceStatus::Shutdown;
                metadata.shutdown_at = Some(chrono::Utc::now());

                // Trigger actual shutdown through pool
                let mut pool = self.pool.lock().await;
                if let Err(e) = pool.disconnect(&metadata.server_id, &metadata.instance_id).await {
                    tracing::warn!("Failed to disconnect instance '{}': {}", metadata.instance_id, e);
                }

                tracing::info!("Shutdown instance '{}' for key '{}'", metadata.instance_id, routing_key);
            }
        }

        Ok(())
    }

    /// Cleanup idle instances
    pub async fn cleanup_idle_instances(&self) -> Result<usize> {
        let now = chrono::Utc::now();
        let ready_instances = self.get_ready_instances_metadata().await;

        if ready_instances.is_empty() {
            return Ok(0);
        }

        // Check each instance for idle timeout
        let idle_keys = self.find_idle_instances(&ready_instances, now).await;

        // Shutdown idle instances
        self.shutdown_idle_instances(&idle_keys, now).await
    }

    /// Get metadata for all ready instances
    async fn get_ready_instances_metadata(&self) -> Vec<(String, InstanceMetadata)> {
        let instances = self.active_instances.lock().await;
        instances
            .iter()
            .filter(|(_, metadata)| matches!(metadata.status, InstanceStatus::Ready))
            .map(|(key, metadata)| (key.clone(), metadata.clone()))
            .collect()
    }

    /// Find instances that have exceeded idle timeout
    async fn find_idle_instances(
        &self,
        ready_instances: &[(String, InstanceMetadata)],
        now: chrono::DateTime<chrono::Utc>,
    ) -> Vec<String> {
        let mut idle_keys = Vec::new();

        for (key, metadata) in ready_instances {
            let timeout = self.get_idle_timeout(&metadata.server_id).await;
            let idle_duration = now.signed_duration_since(metadata.last_activity);

            if idle_duration.num_seconds() > timeout.as_secs() as i64 {
                idle_keys.push(key.clone());
            }
        }

        idle_keys
    }

    /// Get idle timeout for a server
    async fn get_idle_timeout(
        &self,
        server_id: &str,
    ) -> Duration {
        self.get_isolation_config(server_id)
            .await
            .map(|config| config.idle_timeout)
            .unwrap_or(Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS))
    }

    /// Shutdown idle instances
    async fn shutdown_idle_instances(
        &self,
        idle_keys: &[String],
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize> {
        let mut instances = self.active_instances.lock().await;
        let mut cleaned_count = 0;

        for key in idle_keys {
            if let Some(metadata) = instances.get_mut(key) {
                metadata.status = InstanceStatus::Shutdown;
                metadata.shutdown_at = Some(now);
                cleaned_count += 1;

                // Trigger shutdown through pool
                let mut pool = self.pool.lock().await;
                if let Err(e) = pool.disconnect(&metadata.server_id, &metadata.instance_id).await {
                    tracing::warn!("Failed to disconnect idle instance '{}': {}", metadata.instance_id, e);
                }
            }
        }

        Ok(cleaned_count)
    }

    /// Get connection statistics
    pub async fn get_stats(&self) -> ConnectionStats {
        self.stats.lock().await.clone()
    }

    /// Get all active instances for monitoring
    pub async fn get_active_instances(&self) -> Vec<InstanceMetadata> {
        let instances = self.active_instances.lock().await;
        instances
            .values()
            .filter(|metadata| matches!(metadata.status, InstanceStatus::Ready))
            .cloned()
            .collect()
    }

    // Statistics helper methods
    async fn increment_total_created(&self) {
        let mut stats = self.stats.lock().await;
        stats.total_created += 1;
        stats.active_count += 1;
    }

    async fn increment_cache_hits(&self) {
        let mut stats = self.stats.lock().await;
        stats.cache_hits += 1;
    }

    async fn increment_revived_instances(&self) {
        let mut stats = self.stats.lock().await;
        stats.revived_instances += 1;
    }

    /// Get default isolation mode for server type
    pub fn get_default_isolation_mode(server_type: &crate::common::server::ServerType) -> IsolationMode {
        IsolationConfig::for_server_type(server_type).default_mode
    }

    /// Get connection mode from server type and context
    pub fn get_connection_mode(
        server_type: &crate::common::server::ServerType,
        client_id: Option<String>,
        session_id: Option<String>,
    ) -> ConnectionMode {
        let isolation_mode = Self::get_default_isolation_mode(server_type);

        match isolation_mode {
            IsolationMode::PerClient => {
                if let Some(id) = client_id {
                    ConnectionMode::per_client(id)
                } else {
                    ConnectionMode::shareable() // Fallback to shareable
                }
            }
            IsolationMode::PerSession => {
                if let Some(id) = session_id {
                    ConnectionMode::per_session(id)
                } else {
                    ConnectionMode::shareable() // Fallback to shareable
                }
            }
            IsolationMode::Shareable => ConnectionMode::shareable(),
        }
    }

    /// Get isolation configuration for server type
    pub fn get_isolation_config_for_server(
        &self,
        server_type: &crate::common::server::ServerType,
    ) -> Option<&IsolationConfig> {
        self.isolation_configs.get(server_type)
    }

    /// Update isolation configuration for server type
    pub fn update_isolation_config(
        &mut self,
        server_type: crate::common::server::ServerType,
        config: IsolationConfig,
    ) {
        self.isolation_configs.insert(server_type, config);
    }

    /// Health check for all active instances
    pub async fn health_check_all(&self) -> Result<HealthCheckResult> {
        let instances = self.active_instances.lock().await;
        let now = chrono::Utc::now();
        let mut healthy_count = 0;
        let mut unhealthy_count = 0;
        let mut unhealthy_instances = Vec::new();

        for (routing_key, metadata) in instances.iter() {
            if matches!(metadata.status, InstanceStatus::Ready) {
                // Check if instance has been active too long without health check
                let time_since_activity = now.signed_duration_since(metadata.last_activity);
                let max_activity_time = if let Some(config) = self.get_isolation_config(&metadata.server_id).await {
                    config.idle_timeout * 2 // Double the idle timeout for health checks
                } else {
                    Duration::from_secs(180) // 3 minutes default
                };

                if time_since_activity.num_seconds() > max_activity_time.as_secs() as i64 {
                    unhealthy_count += 1;
                    unhealthy_instances.push(HealthCheckItem {
                        routing_key: routing_key.clone(),
                        instance_id: metadata.instance_id.clone(),
                        server_id: metadata.server_id.clone(),
                        issue: HealthIssue::InactiveTooLong,
                        last_activity: metadata.last_activity,
                    });
                } else {
                    healthy_count += 1;
                }
            }
        }

        Ok(HealthCheckResult {
            total_instances: instances.len(),
            healthy_instances: healthy_count,
            unhealthy_instances: unhealthy_count,
            unhealthy_details: unhealthy_instances,
            checked_at: now,
        })
    }

    /// Perform health check and cleanup for unhealthy instances
    pub async fn health_check_and_cleanup(&self) -> Result<HealthCheckResult> {
        let health_result = self.health_check_all().await?;

        // Cleanup unhealthy instances
        if !health_result.unhealthy_details.is_empty() {
            let mut instances = self.active_instances.lock().await;
            let now = chrono::Utc::now();

            for unhealthy in &health_result.unhealthy_details {
                if let Some(metadata) = instances.get_mut(&unhealthy.routing_key) {
                    match unhealthy.issue {
                        HealthIssue::InactiveTooLong => {
                            metadata.status = InstanceStatus::Shutdown;
                            metadata.shutdown_at = Some(now);

                            // Trigger shutdown through pool
                            let mut pool = self.pool.lock().await;
                            if let Err(e) = pool.disconnect(&metadata.server_id, &metadata.instance_id).await {
                                tracing::warn!(
                                    "Failed to disconnect unhealthy instance '{}': {}",
                                    metadata.instance_id,
                                    e
                                );
                            }
                        }
                        HealthIssue::ConnectionFailed | HealthIssue::ResourceExhausted => {
                            // Mark as error state
                            metadata.status =
                                InstanceStatus::Error(format!("Health check failed: {:?}", unhealthy.issue));
                            metadata.shutdown_at = Some(now);

                            // Trigger shutdown through pool
                            let mut pool = self.pool.lock().await;
                            if let Err(e) = pool.disconnect(&metadata.server_id, &metadata.instance_id).await {
                                tracing::warn!(
                                    "Failed to disconnect failed instance '{}': {}",
                                    metadata.instance_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            // Update statistics
            let mut stats = self.stats.lock().await;
            stats.cleanup_operations += 1;
        }

        Ok(health_result)
    }

    /// Periodic maintenance task - combines cleanup and health checks
    pub async fn periodic_maintenance(&self) -> Result<MaintenanceResult> {
        let start_time = chrono::Utc::now();

        // Perform cleanup
        let cleaned_count = self.cleanup_idle_instances().await.unwrap_or(0);

        // Perform health check
        let health_result = self.health_check_all().await?;

        // Auto-revive some shutdown instances if needed
        let revived_count = self.auto_revive_critical_instances().await.unwrap_or(0);

        let duration = start_time.signed_duration_since(chrono::Utc::now());

        Ok(MaintenanceResult {
            cleaned_instances: cleaned_count,
            revived_instances: revived_count,
            health_result,
            maintenance_duration: duration,
            performed_at: start_time,
        })
    }

    /// Auto-revive critical instances for high-priority servers
    async fn auto_revive_critical_instances(&self) -> Result<usize> {
        let instances = self.active_instances.lock().await;

        // Find shutdown instances that might need revival
        let shutdown_instances: Vec<(String, InstanceMetadata)> = instances
            .iter()
            .filter(|(_, metadata)| matches!(metadata.status, InstanceStatus::Shutdown))
            .map(|(key, metadata)| (key.clone(), metadata.clone()))
            .collect();

        drop(instances); // Release lock

        let mut revived_count = 0;

        // Try to revive some instances
        for (key, metadata) in shutdown_instances {
            if revived_count >= MAX_AUTO_REVIVE_INSTANCES {
                break;
            }

            // Check if this server type is configured for auto-revival
            let should_revive = self
                .get_isolation_config(&metadata.server_id)
                .await
                .map(|config| config.enable_revival)
                .unwrap_or(false);

            if should_revive {
                // Extract server_id from routing key
                if let Some(server_id) = key.split(':').next() {
                    if (self.revive_instance(&key, server_id).await).is_ok() {
                        revived_count += 1;
                        self.increment_revived_instances().await;
                    }
                }
            }
        }

        Ok(revived_count)
    }

    /// Get instances by status
    pub async fn get_instances_by_status(
        &self,
        status: InstanceStatus,
    ) -> Vec<InstanceMetadata> {
        let instances = self.active_instances.lock().await;
        instances
            .values()
            .filter(|metadata| metadata.status == status)
            .cloned()
            .collect()
    }

    /// Get instance by routing key
    pub async fn get_instance(
        &self,
        routing_key: &str,
    ) -> Option<InstanceMetadata> {
        let instances = self.active_instances.lock().await;
        instances.get(routing_key).cloned()
    }

    /// Force shutdown of all instances for a server
    pub async fn force_shutdown_server(
        &self,
        server_id: &str,
    ) -> Result<usize> {
        let mut instances = self.active_instances.lock().await;
        let now = chrono::Utc::now();
        let mut shutdown_count = 0;

        let keys_to_shutdown: Vec<String> = instances
            .iter()
            .filter(|(_, metadata)| metadata.server_id == server_id && matches!(metadata.status, InstanceStatus::Ready))
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_shutdown {
            if let Some(metadata) = instances.get_mut(&key) {
                metadata.status = InstanceStatus::Shutdown;
                metadata.shutdown_at = Some(now);
                shutdown_count += 1;

                // Trigger shutdown through pool
                let mut pool = self.pool.lock().await;
                if let Err(e) = pool.disconnect(&metadata.server_id, &metadata.instance_id).await {
                    tracing::warn!("Failed to force shutdown instance '{}': {}", metadata.instance_id, e);
                }
            }
        }

        Ok(shutdown_count)
    }

    /// Get server statistics
    pub async fn get_server_stats(
        &self,
        server_id: &str,
    ) -> Option<ServerStats> {
        let instances = self.active_instances.lock().await;
        let server_instances: Vec<&InstanceMetadata> = instances
            .values()
            .filter(|metadata| metadata.server_id == server_id)
            .collect();

        if server_instances.is_empty() {
            return None;
        }

        let ready_count = server_instances
            .iter()
            .filter(|metadata| matches!(metadata.status, InstanceStatus::Ready))
            .count();

        let shutdown_count = server_instances
            .iter()
            .filter(|metadata| matches!(metadata.status, InstanceStatus::Shutdown))
            .count();

        let error_count = server_instances
            .iter()
            .filter(|metadata| matches!(metadata.status, InstanceStatus::Error(_)))
            .count();

        Some(ServerStats {
            server_id: server_id.to_string(),
            total_instances: server_instances.len(),
            ready_instances: ready_count,
            shutdown_instances: shutdown_count,
            error_instances: error_count,
        })
    }
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub total_instances: usize,
    pub healthy_instances: usize,
    pub unhealthy_instances: usize,
    pub unhealthy_details: Vec<HealthCheckItem>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// Health check item
#[derive(Debug, Clone)]
pub struct HealthCheckItem {
    pub routing_key: String,
    pub instance_id: String,
    pub server_id: String,
    pub issue: HealthIssue,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Health issue types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthIssue {
    InactiveTooLong,
    ConnectionFailed,
    ResourceExhausted,
}

/// Maintenance result
#[derive(Debug, Clone)]
pub struct MaintenanceResult {
    pub cleaned_instances: usize,
    pub revived_instances: usize,
    pub health_result: HealthCheckResult,
    pub maintenance_duration: chrono::Duration,
    pub performed_at: chrono::DateTime<chrono::Utc>,
}

/// Server statistics
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub server_id: String,
    pub total_instances: usize,
    pub ready_instances: usize,
    pub shutdown_instances: usize,
    pub error_instances: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::server::ServerType;
    use crate::core::pool::UpstreamConnectionPool;

    #[tokio::test]
    async fn test_connection_mode_creation() {
        let shareable = ConnectionMode::shareable();
        assert_eq!(shareable.isolation_mode, IsolationMode::Shareable);

        let per_client = ConnectionMode::per_client("test-client".to_string());
        assert_eq!(per_client.isolation_mode, IsolationMode::PerClient);

        let per_session = ConnectionMode::per_session("test-session".to_string());
        assert_eq!(per_session.isolation_mode, IsolationMode::PerSession);
    }

    #[tokio::test]
    async fn test_affinity_key_string() {
        let shareable = ConnectionMode::shareable();
        assert_eq!(shareable.affinity_key_string(), "default");

        let per_client = ConnectionMode::per_client("client123".to_string());
        assert_eq!(per_client.affinity_key_string(), "client:client123");

        let per_session = ConnectionMode::per_session("session456".to_string());
        assert_eq!(per_session.affinity_key_string(), "session:session456");
    }

    #[tokio::test]
    async fn test_default_isolation_mode() {
        assert_eq!(
            UnifiedConnectionManager::get_default_isolation_mode(&ServerType::Stdio),
            IsolationMode::PerClient
        );
        assert_eq!(
            UnifiedConnectionManager::get_default_isolation_mode(&ServerType::Sse),
            IsolationMode::Shareable
        );
        assert_eq!(
            UnifiedConnectionManager::get_default_isolation_mode(&ServerType::StreamableHttp),
            IsolationMode::Shareable
        );
    }

    #[tokio::test]
    async fn test_get_connection_mode() {
        let mode = UnifiedConnectionManager::get_connection_mode(&ServerType::Stdio, Some("client1".to_string()), None);
        assert_eq!(mode.isolation_mode, IsolationMode::PerClient);

        let mode = UnifiedConnectionManager::get_connection_mode(&ServerType::Sse, None, None);
        assert_eq!(mode.isolation_mode, IsolationMode::Shareable);
    }

    #[tokio::test]
    async fn test_unified_entry_point_functionality() {
        // Create a mock pool for testing
        let config = Arc::new(Config::default());
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(config.clone(), None)));

        // Create the unified connection manager
        let manager = UnifiedConnectionManager::new(pool, config);

        // Test 1: Test different isolation modes
        let shareable_mode = ConnectionMode::shareable();
        let per_client_mode = ConnectionMode::per_client("test-client-123".to_string());
        let per_session_mode = ConnectionMode::per_session("test-session-456".to_string());

        assert_eq!(shareable_mode.isolation_mode, IsolationMode::Shareable);
        assert_eq!(per_client_mode.isolation_mode, IsolationMode::PerClient);
        assert_eq!(per_session_mode.isolation_mode, IsolationMode::PerSession);

        // Test 2: Test affinity key generation
        assert_eq!(shareable_mode.affinity_key_string(), "default");
        assert_eq!(per_client_mode.affinity_key_string(), "client:test-client-123");
        assert_eq!(per_session_mode.affinity_key_string(), "session:test-session-456");

        // Test 3: Test isolation configuration
        let stdio_config = IsolationConfig::for_server_type(&ServerType::Stdio);
        let sse_config = IsolationConfig::for_server_type(&ServerType::Sse);

        assert_eq!(stdio_config.default_mode, IsolationMode::PerClient);
        assert_eq!(sse_config.default_mode, IsolationMode::Shareable);
        assert_eq!(stdio_config.max_instances, 6);
        assert_eq!(sse_config.max_instances, 3);

        // Test 4: Test getting instances by status
        let ready_instances = manager.get_instances_by_status(InstanceStatus::Ready).await;
        let shutdown_instances = manager.get_instances_by_status(InstanceStatus::Shutdown).await;

        // Should be empty initially
        assert_eq!(ready_instances.len(), 0);
        assert_eq!(shutdown_instances.len(), 0);

        // Test 5: Test statistics
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_created, 0);
        assert_eq!(stats.active_count, 0);
        assert_eq!(stats.cache_hits, 0);

        // Test 6: Test health check functionality
        let health_result = manager.health_check_all().await.unwrap();
        assert_eq!(health_result.total_instances, 0);
        assert_eq!(health_result.healthy_instances, 0);
        assert_eq!(health_result.unhealthy_instances, 0);
        assert!(health_result.unhealthy_details.is_empty());

        // Test 7: Test periodic maintenance
        let maintenance_result = manager.periodic_maintenance().await.unwrap();
        assert_eq!(maintenance_result.cleaned_instances, 0);
        assert_eq!(maintenance_result.revived_instances, 0);

        // Test 8: Test configuration methods
        let default_mode = UnifiedConnectionManager::get_default_isolation_mode(&ServerType::Stdio);
        assert_eq!(default_mode, IsolationMode::PerClient);

        let connection_mode =
            UnifiedConnectionManager::get_connection_mode(&ServerType::Stdio, Some("client123".to_string()), None);
        assert_eq!(connection_mode.isolation_mode, IsolationMode::PerClient);

        println!("🎉 All unified entry point tests passed!");
    }

    #[tokio::test]
    async fn test_isolation_config_builder() {
        let config = IsolationConfig::for_server_type(&ServerType::Stdio);

        // Test builder pattern methods
        let modified_config = config
            .with_max_instances(10)
            .with_idle_timeout(std::time::Duration::from_secs(120))
            .with_revival(false)
            .with_load_balancing(true);

        assert_eq!(modified_config.max_instances, 10);
        assert_eq!(modified_config.idle_timeout, std::time::Duration::from_secs(120));
        assert!(!modified_config.enable_revival);
        assert!(modified_config.enable_load_balancing);
    }

    #[tokio::test]
    async fn test_connection_mode_from_server_type() {
        // Test stdio server type
        let stdio_mode = UnifiedConnectionManager::get_connection_mode(
            &ServerType::Stdio,
            Some("client1".to_string()),
            Some("session1".to_string()),
        );
        assert_eq!(stdio_mode.isolation_mode, IsolationMode::PerClient);

        // Test SSE server type
        let sse_mode = UnifiedConnectionManager::get_connection_mode(&ServerType::Sse, None, None);
        assert_eq!(sse_mode.isolation_mode, IsolationMode::Shareable);

        // Test fallback behavior
        let fallback_mode = UnifiedConnectionManager::get_connection_mode(&ServerType::Stdio, None, None);
        assert_eq!(fallback_mode.isolation_mode, IsolationMode::Shareable); // Should fallback
    }
}
