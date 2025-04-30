// MCP Proxy server module
// Contains the ProxyServer struct and related functionality

use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use super::pool::UpstreamConnectionPool;
use crate::config::Config;

/// MCP Proxy Server that aggregates tools from multiple MCP servers
#[derive(Debug, Clone)]
pub struct ProxyServer {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

#[tool(tool_box)]
impl ProxyServer {
    pub fn new(config: Arc<Config>, rule_config: Arc<HashMap<String, bool>>) -> Self {
        // Create connection pool
        let mut pool = UpstreamConnectionPool::new(config, rule_config);

        // Initialize the pool
        pool.initialize();

        let connection_pool = Arc::new(Mutex::new(pool));

        // Start health check task
        UpstreamConnectionPool::start_health_check(connection_pool.clone());

        Self { connection_pool }
    }
}

#[tool(tool_box)]
impl ServerHandler for ProxyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP Proxy Server that aggregates tools from multiple MCP servers".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
