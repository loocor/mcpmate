// MCP Proxy API routes module
// Contains route definitions for the API server

pub mod ai;
pub mod board;
pub mod cache;
pub mod client;
pub mod notifs;
pub mod openapi;
pub mod profile;
pub mod runtime;
pub mod server;
pub mod system;

use std::sync::Arc;

use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::Router;
use tokio::sync::Mutex;

use crate::clients::ClientConfigService;
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
    /// Profile merge service
    pub profile_merge_service: Option<Arc<crate::core::profile::ProfileService>>,
    /// Database reference for API operations
    pub database: Option<Arc<crate::config::database::Database>>,
    /// Configuration application state manager
    pub config_application_state: Arc<crate::core::profile::ConfigApplicationStateManager>,
    /// Redb cache manager (unified capabilities cache)
    pub redb_cache: Arc<crate::core::cache::RedbCacheManager>,
    /// Unified query adapter (optional, for gradual migration)
    pub unified_query: Option<Arc<crate::core::capability::UnifiedQueryAdapter>>,
    /// Client configuration service (template-driven)
    pub client_service: Option<Arc<ClientConfigService>>,
}

/// Create the API router with all routes
pub async fn create_router(connection_pool: Arc<Mutex<UpstreamConnectionPool>>) -> Router {
    create_router_internal(connection_pool, None).await
}

/// Create the API router with all routes and HTTP proxy server reference
pub async fn create_router_with_proxy(
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    http_proxy: Arc<ProxyServer>,
) -> Router {
    create_router_internal(connection_pool, Some(http_proxy)).await
}

/// Internal function to create router with optional HTTP proxy
async fn create_router_internal(
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    http_proxy: Option<Arc<ProxyServer>>,
) -> Router {
    // Create system metrics collector with 5 second update interval
    let metrics_collector = Arc::new(MetricsCollector::new(std::time::Duration::from_secs(5)));

    // Start background refresh task
    MetricsCollector::start_background_refresh(metrics_collector.clone());

    // Create Profile merge service if HTTP proxy and database are available
    let profile_merge_service = if let Some(ref proxy) = http_proxy {
        if let Some(db) = proxy.database.clone() {
            let merge_service = Arc::new(crate::core::profile::ProfileService::new(db));
            Some(merge_service)
        } else {
            tracing::warn!("Database not available, Profile merge service will not be initialized");
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
    let config_application_state = Arc::new(crate::core::profile::ConfigApplicationStateManager::new());

    // Initialize the state manager in the background
    let state_manager_clone = config_application_state.clone();
    tokio::spawn(async move {
        state_manager_clone.initialize().await;
    });

    // Initialize standard Redb cache manager for API operations using global singleton
    // Note: EventHandlers now uses a lightweight capability manager without RedbCacheManager
    // This eliminates file lock conflicts while maintaining API query performance
    let redb_cache = crate::core::cache::RedbCacheManager::global()
        .expect("Failed to initialize standard Redb cache manager for API operations");

    // 创建统一查询适配器（可选，用于渐进式迁移）
    let unified_query = if database.is_some() {
        crate::core::capability::UnifiedQueryIntegration::create_adapter(&AppState {
            connection_pool: connection_pool.clone(),
            metrics_collector: metrics_collector.clone(),
            http_proxy: http_proxy.clone(),
            profile_merge_service: profile_merge_service.clone(),
            database: database.clone(),
            config_application_state: config_application_state.clone(),
            redb_cache: redb_cache.clone(),
            unified_query: None, // 避免递归
            client_service: None,
        })
    } else {
        None
    };

    // Initialize client configuration service when database is available
    let client_service = if let Some(db) = database.clone() {
        let pool = Arc::new(db.pool.clone());
        match ClientConfigService::bootstrap(pool).await {
            Ok(service) => Some(Arc::new(service)),
            Err(err) => {
                tracing::error!("Failed to bootstrap client configuration service: {}", err);
                None
            }
        }
    } else {
        tracing::warn!("Database not available, client configuration service disabled");
        None
    };

    let state = Arc::new(AppState {
        connection_pool,
        metrics_collector,
        http_proxy,
        profile_merge_service,
        database,
        config_application_state,
        redb_cache,
        unified_query,
        client_service,
    });

    // Create OpenAPI specification
    let mut api = OpenApi::default();

    // Create API router with aide support
    let api_router = ApiRouter::new()
        .merge(ai::routes(state.clone()))
        .merge(server::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(cache::routes(state.clone()))
        .merge(notifs::routes(state.clone()))
        .merge(profile::routes(state.clone()))
        .merge(runtime::routes(state.clone()))
        .merge(client::routes(state.clone()))
        .finish_api_with(&mut api, openapi::api_docs)
        .layer(axum::middleware::from_fn(
            crate::common::env::origin_guard_middleware,
        ));

    // Create main router with API routes, docs, and board static files
    // Note: API routes must come first to avoid being intercepted by board fallback
    Router::new()
        .nest("/api", api_router)
        .merge(openapi::openapi_routes(api))
        .merge(board::routes(state.clone()))
}
