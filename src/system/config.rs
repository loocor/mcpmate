//! Runtime configuration for MCPMate
//!
//! This module provides global runtime configuration that can be set from
//! command line arguments and accessed throughout the application.

use crate::common::profile::ports;
use std::sync::OnceLock;

/// Global runtime port configuration
#[derive(Debug, Clone)]
pub struct RuntimePortConfig {
    /// API server port
    pub api_port: u16,
    /// MCP proxy server port
    pub mcp_port: u16,
}

impl RuntimePortConfig {
    /// Create a new runtime port configuration
    pub fn new(
        api_port: u16,
        mcp_port: u16,
    ) -> Self {
        Self { api_port, mcp_port }
    }

    /// Get the API server URL
    pub fn api_url(&self) -> String {
        format!("http://localhost:{}", self.api_port)
    }

    /// Get the MCP SSE endpoint URL
    pub fn mcp_sse_url(&self) -> String {
        format!("http://localhost:{}/sse", self.mcp_port)
    }

    /// Get the MCP HTTP endpoint URL
    pub fn mcp_http_url(&self) -> String {
        format!("http://localhost:{}/mcp", self.mcp_port)
    }
}

impl Default for RuntimePortConfig {
    fn default() -> Self {
        Self {
            api_port: ports::API_PORT,
            mcp_port: ports::MCP_PORT,
        }
    }
}

/// Global runtime port configuration instance
static RUNTIME_PORT_CONFIG: OnceLock<RuntimePortConfig> = OnceLock::new();

/// Initialize the global runtime port configuration
pub fn init_port_config(
    api_port: u16,
    mcp_port: u16,
) {
    let config = RuntimePortConfig::new(api_port, mcp_port);

    if RUNTIME_PORT_CONFIG.set(config.clone()).is_err() {
        tracing::warn!("Runtime port config was already initialized, ignoring new values");
    } else {
        tracing::info!(
            "Runtime port config initialized: API={}, MCP={}",
            config.api_port,
            config.mcp_port
        );
    }
}

/// Get the global runtime port configuration
pub fn get_runtime_port_config() -> &'static RuntimePortConfig {
    RUNTIME_PORT_CONFIG.get_or_init(|| {
        tracing::warn!("Runtime port config not initialized, using defaults");
        RuntimePortConfig::default()
    })
}

/// Check if runtime port configuration has been initialized
pub fn is_runtime_port_config_initialized() -> bool {
    RUNTIME_PORT_CONFIG.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_port_config_urls() {
        let config = RuntimePortConfig::new(9080, 9000);

        assert_eq!(config.api_url(), "http://localhost:9080");
        assert_eq!(config.mcp_sse_url(), "http://localhost:9000/sse");
        assert_eq!(config.mcp_http_url(), "http://localhost:9000/mcp");
    }

    #[test]
    fn test_default_config() {
        let config = RuntimePortConfig::default();

        assert_eq!(config.api_port, 8080);
        assert_eq!(config.mcp_port, 8000);
    }
}
