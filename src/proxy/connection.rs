// MCP Proxy connection module
// Contains the UpstreamConnection struct and related functionality

use rmcp::{model::Tool, service::RunningService, RoleClient};
use std::time::Instant;

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
        self.status == ConnectionStatus::Connected
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
}
