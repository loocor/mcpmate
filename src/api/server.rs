// MCP Proxy API server
// Contains the API server implementation

use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use crate::http::pool::UpstreamConnectionPool;

use super::routes::create_router;

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
    ) -> Result<(), anyhow::Error> {
        let router = create_router(connection_pool);

        tracing::info!("Starting API server on {}", self.address);

        let listener = tokio::net::TcpListener::bind(self.address).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}
