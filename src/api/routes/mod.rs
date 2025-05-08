// MCP Proxy API routes module
// Contains route definitions for the API server

pub mod mcp;
pub mod notifications;
pub mod system;

use axum::Router;
use std::sync::Arc;

use crate::http::pool::UpstreamConnectionPool;
use crate::system::SystemMetricsCollector;
use tokio::sync::Mutex;

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// System metrics collector
    pub metrics_collector: Arc<SystemMetricsCollector>,
}

/// Create the API router with all routes
pub fn create_router(connection_pool: Arc<Mutex<UpstreamConnectionPool>>) -> Router {
    // Create system metrics collector with 5 second update interval
    let metrics_collector = Arc::new(SystemMetricsCollector::new(std::time::Duration::from_secs(
        5,
    )));

    // Start background refresh task
    SystemMetricsCollector::start_background_refresh(metrics_collector.clone());

    let state = Arc::new(AppState {
        connection_pool,
        metrics_collector,
    });

    Router::new()
        .merge(mcp::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(notifications::routes(state))
}
