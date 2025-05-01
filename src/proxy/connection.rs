// MCP Proxy connection module
// Contains the UpstreamConnection struct and related functionality

use rmcp::{model::Tool, service::RunningService, RoleClient};
use std::time::{Duration, Instant};

use super::types::ConnectionStatus;

/// Connection to an upstream MCP server
#[derive(Debug)]
pub struct UpstreamConnection {
    /// Name of the server
    pub server_name: String,
    /// Active service connection
    pub service: Option<RunningService<RoleClient, ()>>,
    /// Tools provided by this server
    pub tools: Vec<Tool>,
    /// Last time the server was connected
    pub last_connected: Instant,
    /// Number of connection attempts
    pub connection_attempts: u32,
    /// Current connection status
    pub status: ConnectionStatus,
}

impl UpstreamConnection {
    /// Create a new upstream connection
    pub fn new(server_name: String) -> Self {
        Self {
            server_name,
            service: None,
            tools: Vec::new(),
            last_connected: Instant::now(),
            connection_attempts: 0,
            status: ConnectionStatus::Disconnected,
        }
    }

    /// Check if the connection is active
    pub fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected)
    }

    /// Update connection with successful connection details
    pub fn update_connected(&mut self, service: RunningService<RoleClient, ()>, tools: Vec<Tool>) {
        self.service = Some(service);
        self.tools = tools;
        self.status = ConnectionStatus::Connected;
        self.last_connected = Instant::now();
    }

    /// Update connection status to failed
    pub fn update_failed(&mut self, error_msg: String) {
        self.status = ConnectionStatus::Failed(error_msg);
    }

    /// Update connection status to connecting
    pub fn update_connecting(&mut self) {
        self.status = ConnectionStatus::Connecting;
        self.connection_attempts += 1;
    }

    /// Update connection status to disconnected
    pub fn update_disconnected(&mut self) {
        self.service = None;
        self.tools = Vec::new();
        self.status = ConnectionStatus::Disconnected;
    }

    /// Update connection status to disabled
    pub fn update_disabled(&mut self) {
        self.service = None;
        self.tools = Vec::new();
        self.status = ConnectionStatus::Disabled;
    }

    /// Update connection status to paused
    pub fn update_paused(&mut self) {
        self.status = ConnectionStatus::Paused;
    }

    /// Update connection status to reconnecting
    pub fn update_reconnecting(&mut self) {
        self.status = ConnectionStatus::Reconnecting;
    }

    /// Get a string representation of the connection status
    pub fn status_string(&self) -> String {
        self.status.to_string()
    }

    /// Get the time elapsed since the last connection
    pub fn time_since_last_connection(&self) -> Duration {
        self.last_connected.elapsed()
    }

    /// Check if the connection is in a state that allows connection attempts
    pub fn can_connect(&self) -> bool {
        self.status.can_connect()
    }

    /// Check if the connection is in a state that should be monitored by health checks
    pub fn should_monitor(&self) -> bool {
        self.status.should_monitor()
    }
}
