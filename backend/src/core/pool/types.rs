// Pool types - connection and related data structures
// Contains the UpstreamConnection struct and related functionality

use std::sync::Arc;
use std::time::{Duration, Instant};

use rmcp::model::{ServerCapabilities, Tool};

use crate::core::foundation::types::{
    ConnectionOperation, // action to perform on the connection
    ConnectionStatus,    // status of the connection
    DisabledDetails,     // details of the disabled state
    ErrorDetails,        // details of the error
    ErrorType,           // type of the error
};
use crate::generate_id;

const FAILURE_DECAY_WINDOW_SECS: u64 = 120;
const FAILURE_BACKOFF_SCHEDULE_SECS: [u64; 3] = [5, 15, 60];

/// Production route key for affinity-aware connection routing.
///
/// This key combines server_id with affinity_key to uniquely identify
/// a production connection route. It enables the pool to maintain
/// separate connection instances for different client/session affinities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProductionRouteKey {
    pub server_id: String,
    pub affinity_key: crate::core::capability::AffinityKey,
}

impl ProductionRouteKey {
    pub fn new(
        server_id: impl Into<String>,
        affinity_key: crate::core::capability::AffinityKey,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            affinity_key,
        }
    }

    /// Create a route key for shareable (default) affinity.
    pub fn shareable(server_id: impl Into<String>) -> Self {
        Self::new(server_id, crate::core::capability::AffinityKey::Default)
    }

    /// Create a route key for per-client affinity.
    pub fn per_client(
        server_id: impl Into<String>,
        client_id: impl Into<String>,
    ) -> Self {
        Self::new(
            server_id,
            crate::core::capability::AffinityKey::PerClient(client_id.into()),
        )
    }

    /// Create a route key for per-session affinity.
    pub fn per_session(
        server_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self::new(
            server_id,
            crate::core::capability::AffinityKey::PerSession(session_id.into()),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Connect,
    RuntimeGone,
    RuntimeTimeout,
    RuntimeOther,
}

impl FailureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FailureKind::Connect => "connect",
            FailureKind::RuntimeGone => "runtime_gone",
            FailureKind::RuntimeTimeout => "runtime_timeout",
            FailureKind::RuntimeOther => "runtime_other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FailureState {
    pub consecutive_failures: u32,
    pub last_failure_at: Option<Instant>,
    pub next_retry_at: Option<Instant>,
    pub last_error: Option<String>,
    pub last_kind: Option<FailureKind>,
}

impl FailureState {
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
            last_failure_at: None,
            next_retry_at: None,
            last_error: None,
            last_kind: None,
        }
    }

    pub fn register_failure(
        &mut self,
        now: Instant,
        kind: FailureKind,
        reason: Option<String>,
    ) -> Duration {
        if let Some(last) = self.last_failure_at {
            if now.duration_since(last) > Duration::from_secs(FAILURE_DECAY_WINDOW_SECS) {
                self.consecutive_failures = 0;
            }
        }

        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.last_failure_at = Some(now);
        self.last_kind = Some(kind);
        self.last_error = reason;

        let index = self
            .consecutive_failures
            .saturating_sub(1)
            .try_into()
            .unwrap_or(usize::MAX);
        let backoff_secs = FAILURE_BACKOFF_SCHEDULE_SECS
            .get(index)
            .copied()
            .unwrap_or_else(|| *FAILURE_BACKOFF_SCHEDULE_SECS.last().unwrap_or(&60));
        let backoff = Duration::from_secs(backoff_secs);
        self.next_retry_at = Some(now + backoff);

        backoff
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.last_failure_at = None;
        self.next_retry_at = None;
        self.last_error = None;
        self.last_kind = None;
    }

    pub fn remaining_backoff(
        &self,
        now: Instant,
    ) -> Option<Duration> {
        self.next_retry_at
            .and_then(|deadline| deadline.checked_duration_since(now))
    }
}

impl Default for FailureState {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection to an upstream MCP server
#[derive(Debug)]
pub struct UpstreamConnection {
    /// Unique instance ID
    pub id: String,
    /// Name of the server
    pub server_name: String,
    /// Active service connection (using Arc for cheap cloning)
    pub service: Option<Arc<crate::core::transport::ClientService>>,
    /// Server capabilities (resources, tools, etc.)
    pub capabilities: Option<ServerCapabilities>,
    /// Tools provided by this server
    pub tools: Vec<Tool>,
    /// Time when the connection was created
    pub created_at: Instant,
    /// Last time the server was connected
    pub last_connected: Instant,
    /// Last time the connection served a request
    pub last_activity: Instant,
    /// Number of connection attempts
    pub connection_attempts: u32,
    /// Current connection status
    pub status: ConnectionStatus,
    /// Last time the health check was performed
    pub last_health_check: Instant,
    /// Process ID of the server process
    pub process_id: Option<u32>,
    /// CPU usage of the process (percentage)
    pub cpu_usage: Option<f32>,
    /// Memory usage of the process (bytes)
    pub memory_usage: Option<u64>,
    /// Last time a snapshot was taken (for API performance tracking)
    pub last_snapshot: Option<Instant>,
}

// Manual implementation of Clone for UpstreamConnection
// Using Arc for service allows cheap cloning while preserving the connection
impl Clone for UpstreamConnection {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            server_name: self.server_name.clone(),
            service: self.service.clone(), // Arc clone is cheap and preserves service
            capabilities: self.capabilities.clone(),
            tools: self.tools.clone(),
            created_at: self.created_at,
            last_connected: self.last_connected,
            last_activity: self.last_activity,
            connection_attempts: self.connection_attempts,
            status: self.status.clone(),
            last_health_check: self.last_health_check,
            process_id: self.process_id,
            cpu_usage: self.cpu_usage,
            memory_usage: self.memory_usage,
            last_snapshot: self.last_snapshot,
        }
    }
}

impl UpstreamConnection {
    /// Create a new upstream connection
    pub fn new(server_name: String) -> Self {
        let now = Instant::now();
        Self {
            id: generate_id!("upsv"),
            server_name,
            service: None,
            capabilities: None,
            tools: Vec::new(),
            created_at: now,
            last_connected: now,
            last_activity: now,
            connection_attempts: 0,
            status: ConnectionStatus::Idle,
            last_health_check: now,
            process_id: None,
            cpu_usage: None,
            memory_usage: None,
            last_snapshot: None,
        }
    }

    /// Check if the connection is active
    pub fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Ready)
    }

    /// Check if the server supports resources
    pub fn supports_resources(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|caps| caps.resources.as_ref())
            .is_some()
    }

    /// Check if the server supports prompts
    pub fn supports_prompts(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|caps| caps.prompts.as_ref())
            .is_some()
    }

    /// Update connection with successful connection details
    pub fn update_connected(
        &mut self,
        service: crate::core::transport::ClientService,
        tools: Vec<Tool>,
        capabilities: Option<ServerCapabilities>,
    ) {
        self.service = Some(Arc::new(service));
        self.tools = tools;
        self.capabilities = capabilities;
        self.status = ConnectionStatus::Ready;
        self.last_connected = Instant::now();
        self.last_activity = self.last_connected;
    }

    /// Update connection status to error (basic error handling without escalation)
    ///
    /// Use this method for simple error reporting without progressive failure escalation.
    /// For production use with fault tolerance, prefer `update_error_with_escalation`.
    pub fn update_failed(
        &mut self,
        error_msg: String,
    ) {
        // Check if we're already in an error state
        let (failure_count, first_failure_time) = match &self.status {
            ConnectionStatus::Error(details) => (details.failure_count + 1, details.first_failure_time),
            _ => (1, chrono::Local::now().timestamp() as u64),
        };

        // Create error details
        let error_details = ErrorDetails {
            message: error_msg,
            error_type: ErrorType::Temporary, // Default to temporary
            failure_count,
            first_failure_time,
            last_failure_time: chrono::Local::now().timestamp() as u64,
        };

        self.status = ConnectionStatus::Error(error_details);
    }

    /// Update connection status to permanent error (requires manual intervention)
    pub fn update_permanent_error(
        &mut self,
        error_msg: String,
    ) {
        // Create error details
        let error_details = ErrorDetails {
            message: error_msg,
            error_type: ErrorType::Permanent,
            failure_count: 1,
            first_failure_time: chrono::Local::now().timestamp() as u64,
            last_failure_time: chrono::Local::now().timestamp() as u64,
        };

        self.status = ConnectionStatus::Error(error_details);
    }

    /// Update connection status to initializing
    pub fn update_connecting(&mut self) {
        self.status = ConnectionStatus::Initializing;
        self.connection_attempts += 1;
    }

    /// Update connection status to initializing (alias for compatibility)
    pub fn update_initializing(&mut self) {
        self.update_connecting();
    }

    /// Transition from idle placeholder to shutdown when explicitly disconnected
    /// Update connection status to ready
    pub fn update_ready(&mut self) {
        self.status = ConnectionStatus::Ready;
        let now = Instant::now();
        self.last_connected = now;
        self.last_activity = now;
    }

    /// Update connection status with progressive failure escalation
    ///
    /// This method implements the progressive failure escalation strategy:
    /// - 1st failure: 60s backoff
    /// - 2nd failure: 120s backoff
    /// - 3rd failure: 360s backoff
    /// - 4th+ failure: auto-disable
    pub fn update_error_with_escalation(
        &mut self,
        error_msg: String,
    ) {
        // Check if we're already in an error state
        let (failure_count, first_failure_time) = match &self.status {
            ConnectionStatus::Error(details) => (details.failure_count + 1, details.first_failure_time),
            _ => (1, chrono::Local::now().timestamp() as u64),
        };

        // Check if we should disable the server (4+ consecutive failures)
        if failure_count >= 4 {
            self.update_disabled_due_to_failures(error_msg, failure_count);
            return;
        }

        // Create error details with progressive escalation
        let error_details = ErrorDetails {
            message: error_msg,
            error_type: ErrorType::Temporary,
            failure_count,
            first_failure_time,
            last_failure_time: chrono::Local::now().timestamp() as u64,
        };

        tracing::warn!(
            "Server '{}' failure #{}: {} (progressive escalation active)",
            self.server_name,
            failure_count,
            error_details.message
        );

        self.status = ConnectionStatus::Error(error_details);
    }

    /// Update connection status to disabled due to repeated failures
    pub fn update_disabled_due_to_failures(
        &mut self,
        last_error: String,
        total_failures: u32,
    ) {
        let disabled_details = DisabledDetails {
            reason: format!("Auto-disabled after {} consecutive failures", total_failures),
            total_failures,
            disabled_time: chrono::Local::now().timestamp() as u64,
            last_error,
        };

        self.service = None;
        self.tools = Vec::new();
        self.status = ConnectionStatus::Disabled(disabled_details);

        tracing::error!(
            "Server '{}' has been auto-disabled after {} consecutive failures",
            self.server_name,
            total_failures
        );
    }

    /// Update connection status to shutdown
    pub fn update_disconnected(&mut self) {
        self.service = None;
        self.tools = Vec::new();
        self.status = ConnectionStatus::Shutdown;
        self.last_activity = Instant::now();
    }

    /// Update connection status to shutdown (legacy method, use update_disconnected instead)
    #[deprecated(note = "Use update_disconnected instead to avoid confusion with disabled state")]
    pub fn update_disabled(&mut self) {
        self.update_disconnected();
    }

    /// Update connection status to busy
    pub fn update_busy(&mut self) {
        self.status = ConnectionStatus::Busy;
    }

    /// Update connection status to initializing (for reconnection)
    pub fn update_reconnecting(&mut self) {
        self.status = ConnectionStatus::Initializing;
    }

    /// Get a string representation of the connection status
    pub fn status_string(&self) -> String {
        self.status.to_string()
    }

    /// Get time since last connection
    pub fn time_since_last_connection(&self) -> Duration {
        Instant::now().duration_since(self.last_connected)
    }

    /// Get time since creation
    pub fn time_since_creation(&self) -> Duration {
        Instant::now().duration_since(self.created_at)
    }

    /// Check if the connection can be established
    pub fn can_connect(&self) -> bool {
        self.status.can_perform_operation(ConnectionOperation::Connect)
    }

    /// Check if the connection should be monitored
    pub fn should_monitor(&self) -> bool {
        matches!(self.status, ConnectionStatus::Ready | ConnectionStatus::Busy)
    }

    /// Check if the connection can perform a specific typed operation
    pub fn can_perform_typed_operation(
        &self,
        operation: ConnectionOperation,
    ) -> bool {
        self.status.can_perform_operation(operation)
    }

    /// Get all allowed typed operations for the current status
    pub fn allowed_typed_operations(&self) -> Vec<ConnectionOperation> {
        self.status.allowed_operations()
    }

    /// Reset connection attempts counter
    pub fn reset_connection_attempts(&mut self) {
        self.connection_attempts = 0;
    }

    /// Manually re-enable a disabled server (resets failure count and status)
    pub fn manual_re_enable(&mut self) -> Result<(), String> {
        match &self.status {
            ConnectionStatus::Disabled(details) => {
                tracing::info!(
                    "Manually re-enabling server '{}' (was disabled: {})",
                    self.server_name,
                    details.reason
                );

                // Reset to shutdown state and clear failure counters
                self.status = ConnectionStatus::Shutdown;
                self.connection_attempts = 0;
                self.service = None;
                self.tools = Vec::new();

                tracing::info!(
                    "Server '{}' has been manually re-enabled and is ready for connection",
                    self.server_name
                );

                Ok(())
            }
            _ => Err(format!(
                "Server '{}' is not in disabled state (current: {})",
                self.server_name, self.status
            )),
        }
    }

    /// Check if the server is disabled
    pub fn is_disabled(&self) -> bool {
        self.status.is_disabled()
    }

    /// Check if the server should be included in API responses
    pub fn is_available(&self) -> bool {
        self.status.is_available()
    }

    /// Get the progressive backoff time based on failure count
    pub fn get_progressive_backoff_seconds(&self) -> u64 {
        match &self.status {
            ConnectionStatus::Error(details) => match details.failure_count {
                1 => 60,  // 1 minute
                2 => 120, // 2 minutes
                3 => 360, // 6 minutes
                _ => 600, // 10 minutes (fallback, though should be disabled at 4+)
            },
            _ => 60, // Default 1 minute
        }
    }
}
