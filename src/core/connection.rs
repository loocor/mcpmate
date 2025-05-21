// MCP Proxy connection module
// Contains the UpstreamConnection struct and related functionality

use std::time::{Duration, Instant};

use nanoid::nanoid;
use rmcp::{RoleClient, model::Tool, service::RunningService};

use super::types::ConnectionStatus;

/// Connection to an upstream MCP server
#[derive(Debug)]
pub struct UpstreamConnection {
    /// Unique instance ID
    pub id: String,
    /// Name of the server
    pub server_name: String,
    /// Active service connection
    pub service: Option<RunningService<RoleClient, ()>>,
    /// Tools provided by this server
    pub tools: Vec<Tool>,
    /// Time when the connection was created
    pub created_at: Instant,
    /// Last time the server was connected
    pub last_connected: Instant,
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
}

// Manual implementation of Clone for UpstreamConnection
// We can't derive Clone because RunningService doesn't implement Clone
impl Clone for UpstreamConnection {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            server_name: self.server_name.clone(),
            service: None, // We don't clone the service
            tools: self.tools.clone(),
            created_at: self.created_at,
            last_connected: self.last_connected,
            connection_attempts: self.connection_attempts,
            status: self.status.clone(),
            last_health_check: self.last_health_check,
            process_id: self.process_id,
            cpu_usage: self.cpu_usage,
            memory_usage: self.memory_usage,
        }
    }
}

impl UpstreamConnection {
    /// Create a new upstream connection
    pub fn new(server_name: String) -> Self {
        let now = Instant::now();
        Self {
            id: format!("upsv{}", nanoid!(12)),
            server_name,
            service: None,
            tools: Vec::new(),
            created_at: now,
            last_connected: now,
            connection_attempts: 0,
            status: ConnectionStatus::Shutdown,
            last_health_check: now,
            process_id: None,
            cpu_usage: None,
            memory_usage: None,
        }
    }

    /// Check if the connection is active
    pub fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Ready)
    }

    /// Update connection with successful connection details
    pub fn update_connected(
        &mut self,
        service: RunningService<RoleClient, ()>,
        tools: Vec<Tool>,
    ) {
        self.service = Some(service);
        self.tools = tools;
        self.status = ConnectionStatus::Ready;
        self.last_connected = Instant::now();
    }

    /// Update connection status to error
    pub fn update_failed(
        &mut self,
        error_msg: String,
    ) {
        // Check if we're already in an error state
        let (failure_count, first_failure_time) = match &self.status {
            ConnectionStatus::Error(details) =>
                (details.failure_count + 1, details.first_failure_time),
            _ => (1, chrono::Local::now().timestamp() as u64),
        };

        // Create error details
        let error_details = super::types::ErrorDetails {
            message: error_msg,
            error_type: super::types::ErrorType::Temporary, // Default to temporary
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
        let error_details = super::types::ErrorDetails {
            message: error_msg,
            error_type: super::types::ErrorType::Permanent,
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

    /// Update connection status to shutdown
    pub fn update_disconnected(&mut self) {
        self.service = None;
        self.tools = Vec::new();
        self.status = ConnectionStatus::Shutdown;
    }

    /// Update connection status to shutdown (disabled)
    pub fn update_disabled(&mut self) {
        self.service = None;
        self.tools = Vec::new();
        self.status = ConnectionStatus::Shutdown;
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

    /// Get the time elapsed since the last connection
    pub fn time_since_last_connection(&self) -> Duration {
        self.last_connected.elapsed()
    }

    /// Get the time elapsed since the connection was created
    pub fn time_since_creation(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Check if the connection is in a state that allows connection attempts
    pub fn can_connect(&self) -> bool {
        self.status.can_connect()
    }

    /// Check if the connection is in a state that should be monitored by health checks
    pub fn should_monitor(&self) -> bool {
        self.status.should_monitor()
    }

    /// Check if a specific operation is allowed in the current state
    pub fn can_perform_operation(
        &self,
        operation: &str,
    ) -> bool {
        self.status.can_perform_operation(operation)
    }

    /// Get the allowed operations for this connection
    pub fn allowed_operations(&self) -> Vec<String> {
        self.status
            .allowed_operations()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Reset connection attempts counter
    pub fn reset_connection_attempts(&mut self) {
        self.connection_attempts = 0;
    }
}
