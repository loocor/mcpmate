// MCP Proxy API routes module
// Contains route definitions for the API server

pub mod mcp;
pub mod notifs;
pub mod specs;
pub mod suit;
pub mod system;

use std::sync::Arc;

use axum::Router;
use tokio::sync::Mutex;

use crate::{
    http::{HttpProxyServer, pool::UpstreamConnectionPool},
    system::SystemMetricsCollector,
};

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// System metrics collector
    pub metrics_collector: Arc<SystemMetricsCollector>,
    /// HTTP proxy server reference
    pub http_proxy: Option<Arc<HttpProxyServer>>,
    /// Config Suit merge service
    pub suit_merge_service: Option<Arc<crate::core::suit::ConfigSuitMergeService>>,
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
        suit_merge_service: None,
    });

    Router::new()
        .merge(mcp::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(notifs::routes(state.clone()))
        .merge(specs::routes(state.clone()))
        .merge(suit::routes(state.clone()))
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

    // Create Config Suit merge service if database is available
    let config_suit_merge_service = if let Some(db) = http_proxy.database.clone() {
        let merge_service = Arc::new(crate::core::suit::ConfigSuitMergeService::new(db));
        Some(merge_service)
    } else {
        tracing::warn!("Database not available, Config Suit merge service will not be initialized");
        None
    };

    let state = Arc::new(AppState {
        connection_pool,
        metrics_collector,
        http_proxy: Some(http_proxy),
        suit_merge_service: config_suit_merge_service,
    });

    Router::new()
        .merge(mcp::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(notifs::routes(state.clone()))
        .merge(specs::routes(state.clone()))
        .merge(suit::routes(state.clone()))
}
