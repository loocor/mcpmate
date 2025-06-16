// MCP Proxy API server
// Contains the API server implementation

use std::{net::SocketAddr, sync::Arc};

use tokio::sync::Mutex;
use tracing;

use super::routes::{create_router, create_router_with_proxy};
use crate::core::{pool::UpstreamConnectionPool, proxy::ProxyServer};

/// API server for the MCP Proxy
#[derive(Debug)]
pub struct ApiServer {
    /// Address to bind the server to
    address: SocketAddr,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(address: SocketAddr) -> Self {
        Self { address }
    }

    /// Start the API server
    pub async fn start(
        &self,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
        http_proxy: Option<Arc<ProxyServer>>,
    ) -> Result<(), anyhow::Error> {
        // Create the router with connection pool and HTTP proxy reference if available
        let router = if let Some(proxy) = http_proxy {
            create_router_with_proxy(connection_pool.clone(), proxy)
        } else {
            create_router(connection_pool.clone())
        };

        tracing::info!("Starting API server on {}", self.address);

        let listener = tokio::net::TcpListener::bind(self.address).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}
