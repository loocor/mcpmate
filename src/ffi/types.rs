//! FFI data types
//!
//! Defines data structures that can be safely passed between Rust and Swift

use serde::{Deserialize, Serialize};

/// Startup progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupProgress {
    /// Progress percentage (0.0 to 1.0)
    pub percentage: f32,
    /// Current step description
    pub current_step: String,
    /// Number of connected servers
    pub connected_servers: u32,
    /// Total number of servers to connect
    pub total_servers: u32,
    /// Whether startup is complete
    pub is_complete: bool,
    /// Error message if any
    pub error_message: Option<String>,
}

impl Default for StartupProgress {
    fn default() -> Self {
        Self {
            percentage: 0.0,
            current_step: "Initializing...".to_string(),
            connected_servers: 0,
            total_servers: 0,
            is_complete: false,
            error_message: None,
        }
    }
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// MCPMate version
    pub version: String,
    /// API server port
    pub api_port: u16,
    /// MCP server port
    pub mcp_port: u16,
    /// Service uptime in seconds
    pub uptime_seconds: u64,
    /// Whether service is running
    pub is_running: bool,
    /// Number of active connections
    pub active_connections: u32,
}

impl Default for ServiceInfo {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            api_port: 8080,
            mcp_port: 3000,
            uptime_seconds: 0,
            is_running: false,
            active_connections: 0,
        }
    }
}

/// Service status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Service is not initialized
    Unknown,
    /// Service is starting up
    Starting,
    /// Service is running normally
    Running,
    /// Service is stopping
    Stopping,
    /// Service is stopped
    Stopped,
    /// Service encountered an error
    Error,
}

impl Default for ServiceStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl ServiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Error => "error",
        }
    }
}
