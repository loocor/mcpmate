//! Runtime configuration for MCPMate
//!
//! This module provides global runtime configuration that can be set from
//! command line arguments and accessed throughout the application.

use anyhow::{Context, Result};
use std::{
    net::SocketAddr,
    sync::{OnceLock, RwLock},
};

use crate::common::constants::ports;

const LOOPBACK_BIND_HOST: &str = "127.0.0.1";
const LOOPBACK_URL_HOST: &str = "localhost";

pub fn loopback_bind_host() -> &'static str {
    LOOPBACK_BIND_HOST
}

pub fn loopback_url_host() -> &'static str {
    LOOPBACK_URL_HOST
}

pub fn bind_socket_addr(port: u16) -> Result<SocketAddr> {
    format!("{}:{}", loopback_bind_host(), port)
        .parse()
        .with_context(|| format!("Failed to parse loopback bind address for port {}", port))
}

pub fn api_url_from_port(port: u16) -> String {
    format!("http://{}:{}", loopback_url_host(), port)
}

pub fn mcp_http_url_from_port(port: u16) -> String {
    format!("{}/mcp", api_url_from_port(port))
}

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
        api_url_from_port(self.api_port)
    }

    /// Get the MCP HTTP endpoint URL
    pub fn mcp_http_url(&self) -> String {
        mcp_http_url_from_port(self.mcp_port)
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

static RUNTIME_PORT_STORAGE: OnceLock<RwLock<RuntimePortConfig>> = OnceLock::new();

fn runtime_ports() -> &'static RwLock<RuntimePortConfig> {
    RUNTIME_PORT_STORAGE.get_or_init(|| RwLock::new(RuntimePortConfig::default()))
}

/// Set (or update) the global runtime port configuration.
///
/// Safe to call on every embedded backend start (e.g. Tauri in-process restart): values always match
/// the current listener ports.
pub fn init_port_config(
    api_port: u16,
    mcp_port: u16,
) {
    let config = RuntimePortConfig::new(api_port, mcp_port);
    let mut guard = runtime_ports().write().expect("runtime port config RwLock poisoned");
    let changed = guard.api_port != config.api_port || guard.mcp_port != config.mcp_port;
    *guard = config;
    if changed {
        tracing::info!(
            "Runtime port config set: API={}, MCP={}",
            guard.api_port,
            guard.mcp_port
        );
    }
}

/// Snapshot of the global runtime port configuration.
pub fn get_runtime_port_config() -> RuntimePortConfig {
    runtime_ports()
        .read()
        .expect("runtime port config RwLock poisoned")
        .clone()
}

/// Whether the port storage has been allocated (after any call to [`init_port_config`] or
/// [`get_runtime_port_config`]).
pub fn is_runtime_port_config_initialized() -> bool {
    RUNTIME_PORT_STORAGE.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_port_config_urls() {
        let config = RuntimePortConfig::new(9080, 9000);

        assert_eq!(config.api_url(), "http://localhost:9080");
        assert_eq!(config.mcp_http_url(), "http://localhost:9000/mcp");
    }

    #[test]
    fn test_default_config() {
        let config = RuntimePortConfig::default();

        assert_eq!(config.api_port, 8080);
        assert_eq!(config.mcp_port, 8000);
    }

    #[test]
    #[serial_test::serial]
    fn init_port_config_updates_on_second_call() {
        init_port_config(18080, 18000);
        let first = get_runtime_port_config();
        assert_eq!(first.api_port, 18080);
        assert_eq!(first.mcp_port, 18000);

        init_port_config(28080, 28000);
        let second = get_runtime_port_config();
        assert_eq!(second.api_port, 28080);
        assert_eq!(second.mcp_port, 28000);

        // Restore defaults so other tests in the same binary see consistent state.
        init_port_config(ports::API_PORT, ports::MCP_PORT);
    }
}
