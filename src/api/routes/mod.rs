// MCP Proxy API routes module
// Contains route definitions for the API server

pub mod board;
pub mod notifs;
pub mod runtime;
pub mod server;
pub mod specs;
pub mod suits;
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
    /// Database reference for API operations
    pub database: Option<Arc<crate::conf::database::Database>>,
}

/// Create the API router with all routes
pub fn create_router(connection_pool: Arc<Mutex<UpstreamConnectionPool>>) -> Router {
    create_router_internal(connection_pool, None)
}

/// Create the API router with all routes and HTTP proxy server reference
pub fn create_router_with_proxy(
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    http_proxy: Arc<HttpProxyServer>,
) -> Router {
    create_router_internal(connection_pool, Some(http_proxy))
}

/// Internal function to create router with optional HTTP proxy
fn create_router_internal(
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    http_proxy: Option<Arc<HttpProxyServer>>,
) -> Router {
    // Create system metrics collector with 5 second update interval
    let metrics_collector = Arc::new(SystemMetricsCollector::new(std::time::Duration::from_secs(
        5,
    )));

    // Start background refresh task
    SystemMetricsCollector::start_background_refresh(metrics_collector.clone());

    // Create Config Suit merge service if HTTP proxy and database are available
    let config_suit_merge_service = if let Some(ref proxy) = http_proxy {
        if let Some(db) = proxy.database.clone() {
            let merge_service = Arc::new(crate::core::suit::ConfigSuitMergeService::new(db));
            Some(merge_service)
        } else {
            tracing::warn!(
                "Database not available, Config Suit merge service will not be initialized"
            );
            None
        }
    } else {
        None
    };

    // Get database reference from HTTP proxy if available
    let database = if let Some(ref proxy) = http_proxy {
        proxy.database.clone()
    } else {
        None
    };

    let state = Arc::new(AppState {
        connection_pool,
        metrics_collector,
        http_proxy,
        suit_merge_service: config_suit_merge_service,
        database,
    });

    // Create API router
    let api_router = Router::new()
        .merge(server::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(notifs::routes(state.clone()))
        .merge(specs::routes(state.clone()))
        .merge(suits::routes(state.clone()))
        .merge(runtime::routes(state.clone()));

    // Create main router with API routes and board static files
    // Note: API routes must come first to avoid being intercepted by board fallback
    Router::new()
        .nest("/api", api_router)
        .merge(board::routes(state.clone()))
}
