// MCP Proxy API routes module
// Contains route definitions for the API server

pub mod mcp;
pub mod system;

use axum::Router;
use std::sync::Arc;

use crate::proxy::pool::UpstreamConnectionPool;
use tokio::sync::Mutex;

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

/// Create the API router with all routes
pub fn create_router(connection_pool: Arc<Mutex<UpstreamConnectionPool>>) -> Router {
    let state = Arc::new(AppState { connection_pool });

    Router::new()
        .merge(mcp::routes(state.clone()))
        .merge(system::routes(state))
}
