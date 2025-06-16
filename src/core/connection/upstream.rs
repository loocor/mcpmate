// Upstream MCP server connection management
// Contains the UpstreamConnection struct and related functionality

use std::time::{Duration, Instant};

use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    service::RunningService,
};

use crate::generate_id;
use crate::core::foundation::types::{
    ConnectionOperation, // action to perform on the connection
    ConnectionStatus,    // status of the connection
    ErrorDetails,        // details of the error
    ErrorType,           // type of the error
};

/// Connection to an upstream MCP server
#[derive(Debug)]
pub struct UpstreamConnection {
    /// Unique instance ID
    pub id: String,
    /// Name of the server
    pub server_name: String,
    /// Active service connection
    pub service: Option<RunningService<RoleClient, ()>>,
    /// Server capabilities (resources, tools, etc.)
    pub capabilities: Option<ServerCapabilities>,
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
            capabilities: self.capabilities.clone(),
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
            id: generate_id!("upsv"),
            server_name,
            service: None,
            capabilities: None,
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
        service: RunningService<RoleClient, ()>,
        tools: Vec<Tool>,
        capabilities: Option<ServerCapabilities>,
    ) {
        self.service = Some(service);
        self.tools = tools;
        self.capabilities = capabilities;
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
            ConnectionStatus::Error(details) => {
                (details.failure_count + 1, details.first_failure_time)
            }
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

    /// Update connection status to ready
    pub fn update_ready(&mut self) {
        self.status = ConnectionStatus::Ready;
        self.last_connected = Instant::now();
    }

    /// Update connection status to error (alias for compatibility)
    pub fn update_error(
        &mut self,
        error_msg: String,
    ) {
        self.update_failed(error_msg);
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
        self.status
            .can_perform_operation(ConnectionOperation::Connect)
    }

    /// Check if the connection should be monitored
    pub fn should_monitor(&self) -> bool {
        matches!(
            self.status,
            ConnectionStatus::Ready | ConnectionStatus::Busy
        )
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
}
