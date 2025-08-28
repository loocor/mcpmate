//! Interop data types
//!
//! Defines data structures that can be safely passed between different languages

use crate::common::config::ports;
use serde::{Deserialize, Serialize};

/// Port configuration for MCPMate services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    /// API server port (default: 8080)
    pub api_port: u16,
    /// MCP proxy server port (default: 8000)
    pub mcp_port: u16,
}

impl Default for PortConfig {
    fn default() -> Self {
        Self {
            api_port: ports::API_PORT,
            mcp_port: ports::MCP_PORT,
        }
    }
}

impl PortConfig {
    /// Create a new port configuration
    pub fn new(
        api_port: u16,
        mcp_port: u16,
    ) -> Self {
        Self { api_port, mcp_port }
    }

    /// Validate port configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.api_port == 0 {
            return Err("API port cannot be 0".to_string());
        }
        if self.mcp_port == 0 {
            return Err("MCP port cannot be 0".to_string());
        }
        if self.api_port == self.mcp_port {
            return Err("API port and MCP port cannot be the same".to_string());
        }
        if self.api_port < 1024 || self.mcp_port < 1024 {
            return Err("Ports below 1024 require root privileges".to_string());
        }
        Ok(())
    }
}

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
            api_port: ports::API_PORT,
            mcp_port: ports::MCP_PORT,
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

/// Startup configuration for MCPMate service
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StartupConfig {
    /// Port configuration (API and MCP ports)
    #[serde(flatten)]
    pub ports: PortConfig,
    /// Configuration suites to load (None = default, Some(empty) = none, Some(list) = specific)
    pub config_suites: Option<Vec<String>>,
    /// Start in minimal mode (API only, no config suites loaded)
    pub minimal: bool,
}

impl StartupConfig {
    /// Create a new startup configuration
    pub fn new(
        api_port: u16,
        mcp_port: u16,
        config_suites: Option<Vec<String>>,
        minimal: bool,
    ) -> Self {
        Self {
            ports: PortConfig::new(api_port, mcp_port),
            config_suites,
            minimal,
        }
    }

    /// Create a minimal startup configuration (API only)
    pub fn minimal(api_port: u16) -> Self {
        Self {
            ports: PortConfig::new(api_port, 0), // MCP port not used in minimal mode
            config_suites: None,
            minimal: true,
        }
    }

    /// Create a configuration with specific suites
    pub fn with_suites(
        api_port: u16,
        mcp_port: u16,
        suites: Vec<String>,
    ) -> Self {
        Self {
            ports: PortConfig::new(api_port, mcp_port),
            config_suites: Some(suites),
            minimal: false,
        }
    }

    /// Create a configuration with no suites
    pub fn no_suites(
        api_port: u16,
        mcp_port: u16,
    ) -> Self {
        Self {
            ports: PortConfig::new(api_port, mcp_port),
            config_suites: Some(Vec::new()),
            minimal: false,
        }
    }

    /// Validate startup configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate API port (always required)
        if self.ports.api_port == 0 {
            return Err("API port cannot be 0".to_string());
        }

        // Validate MCP port only in non-minimal mode
        if !self.minimal {
            // Use PortConfig validation for comprehensive port checks
            self.ports.validate()?;
        }

        // Validate config suites if provided
        if let Some(ref suites) = self.config_suites {
            for suite in suites {
                if suite.trim().is_empty() {
                    return Err("Configuration suite ID cannot be empty".to_string());
                }
            }
        }

        Ok(())
    }

    /// Convert to StartupMode for internal use
    pub fn to_startup_mode(&self) -> crate::core::proxy::args::StartupMode {
        use crate::core::proxy::args::StartupMode;

        if self.minimal {
            StartupMode::Minimal
        } else if let Some(ref suites) = self.config_suites {
            if suites.is_empty() {
                StartupMode::NoSuites
            } else {
                StartupMode::SpecificSuites(suites.clone())
            }
        } else {
            StartupMode::Default
        }
    }

    /// Get API port (convenience method)
    pub fn api_port(&self) -> u16 {
        self.ports.api_port
    }

    /// Get MCP port (convenience method)
    pub fn mcp_port(&self) -> u16 {
        self.ports.mcp_port
    }
}
