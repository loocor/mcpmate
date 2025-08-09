// MCP Proxy API routes module
// Contains route definitions for the API server

pub mod board;
pub mod clients;
// pub mod inspect;  // Removed: Inspect functionality integrated into server module
pub mod notifs;
pub mod runtime;
pub mod server;
pub mod suits;
pub mod system;

use std::sync::Arc;

use axum::Router;
use tokio::sync::Mutex;

use crate::{
    core::{pool::UpstreamConnectionPool, proxy::ProxyServer},
    system::metrics::MetricsCollector,
};

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// System metrics collector
    pub metrics_collector: Arc<MetricsCollector>,
    /// HTTP proxy server reference
    pub http_proxy: Option<Arc<ProxyServer>>,
    /// Config Suit merge service
    pub suit_merge_service: Option<Arc<crate::core::suit::SuitService>>,
    /// Database reference for API operations
    pub database: Option<Arc<crate::config::database::Database>>,
    /// Configuration application state manager
    pub config_application_state: Arc<crate::core::suit::ConfigApplicationStateManager>,
    /// Redb cache manager (unified capabilities cache)
    pub redb_cache: Arc<crate::core::cache::RedbCacheManager>,
}

/// Create the API router with all routes
pub fn create_router(connection_pool: Arc<Mutex<UpstreamConnectionPool>>) -> Router {
    create_router_internal(connection_pool, None)
}

/// Create the API router with all routes and HTTP proxy server reference
pub fn create_router_with_proxy(
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    http_proxy: Arc<ProxyServer>,
) -> Router {
    create_router_internal(connection_pool, Some(http_proxy))
}

/// Internal function to create router with optional HTTP proxy
fn create_router_internal(
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    http_proxy: Option<Arc<ProxyServer>>,
) -> Router {
    // Create system metrics collector with 5 second update interval
    let metrics_collector = Arc::new(MetricsCollector::new(std::time::Duration::from_secs(5)));

    // Start background refresh task
    MetricsCollector::start_background_refresh(metrics_collector.clone());

    // Create Config Suit merge service if HTTP proxy and database are available
    let config_suit_merge_service = if let Some(ref proxy) = http_proxy {
        if let Some(db) = proxy.database.clone() {
            let merge_service = Arc::new(crate::core::suit::SuitService::new(db));
            Some(merge_service)
        } else {
            tracing::warn!("Database not available, Config Suit merge service will not be initialized");
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

    // Create and initialize configuration application state manager
    let config_application_state = Arc::new(crate::core::suit::ConfigApplicationStateManager::new());

    // Initialize the state manager in the background
    let state_manager_clone = config_application_state.clone();
    tokio::spawn(async move {
        state_manager_clone.initialize().await;
    });

    // Initialize Redb cache manager
    // Allow override via MCPMATE_REDB_CACHE_PATH for tests; default to temp dir
    let redb_cache_path = if let Ok(p) = std::env::var("MCPMATE_REDB_CACHE_PATH") {
        std::path::PathBuf::from(p)
    } else {
        std::env::temp_dir().join("mcpmate").join("cache.redb")
    };
    let redb_cache = Arc::new(
        crate::core::cache::RedbCacheManager::new(redb_cache_path, crate::core::cache::manager::CacheConfig::default())
            .expect("Failed to initialize Redb cache manager"),
    );

    let state = Arc::new(AppState {
        connection_pool,
        metrics_collector,
        http_proxy,
        suit_merge_service: config_suit_merge_service,
        database,
        config_application_state,
        redb_cache,
    });

    // Create API router
    let api_router = Router::new()
        .merge(server::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(notifs::routes(state.clone()))
        .merge(suits::routes(state.clone()))
        .merge(runtime::routes(state.clone()))
        .merge(clients::routes(state.clone()));
    // .merge(inspect::routes(state.clone()));  // Removed: Inspect functionality integrated into server module

    // Create main router with API routes and board static files
    // Note: API routes must come first to avoid being intercepted by board fallback
    Router::new()
        .nest("/api", api_router)
        .merge(board::routes(state.clone()))
}
