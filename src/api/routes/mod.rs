// MCP Proxy API routes module
// Contains route definitions for the API server

pub mod mcp;
pub mod notifications;
pub mod specs;
pub mod system;
pub mod tool;

use axum::Router;
use std::sync::Arc;

use crate::http::{pool::UpstreamConnectionPool, HttpProxyServer};
use crate::system::SystemMetricsCollector;
use tokio::sync::Mutex;

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// System metrics collector
    pub metrics_collector: Arc<SystemMetricsCollector>,
    /// HTTP proxy server reference
    pub http_proxy: Option<Arc<HttpProxyServer>>,
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
        http_proxy: None,
    });

    Router::new()
        .merge(mcp::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(notifications::routes(state.clone()))
        .merge(specs::routes(state.clone()))
        .merge(tool::routes(state))
}

/// Create the API router with all routes and HTTP proxy server reference
pub fn create_router_with_proxy(
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    http_proxy: Arc<HttpProxyServer>,
) -> Router {
    // Create system metrics collector with 5 second update interval
    let metrics_collector = Arc::new(SystemMetricsCollector::new(std::time::Duration::from_secs(
        5,
    )));

    // Start background refresh task
    SystemMetricsCollector::start_background_refresh(metrics_collector.clone());

    let state = Arc::new(AppState {
        connection_pool,
        metrics_collector,
        http_proxy: Some(http_proxy),
    });

    Router::new()
        .merge(mcp::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(notifications::routes(state.clone()))
        .merge(specs::routes(state.clone()))
        .merge(tool::routes(state))
}
