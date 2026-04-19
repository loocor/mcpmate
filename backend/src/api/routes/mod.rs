// MCP Proxy API routes module
// Contains route definitions for the API server

#[cfg(feature = "ai")]
pub mod ai;
pub mod audit;
pub mod client;
pub mod inspector;
pub mod openapi;
pub mod profile;
pub mod registry;
pub mod runtime;
pub mod server;
pub mod system;

use std::sync::Arc;

use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::http::{Request, Response};
use axum::{Router, routing::get};
use std::time::Duration as StdDuration;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing::Level;

use crate::clients::ClientConfigService;
use crate::{
    core::{pool::UpstreamConnectionPool, proxy::ProxyServer},
    inspector::{calls::InspectorCallRegistry, service as inspector_service, sessions::InspectorSessionManager},
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
    pub audit_database: Option<Arc<crate::config::audit_database::AuditDatabase>>,
    pub audit_service: Option<Arc<crate::audit::AuditService>>,
    /// Configuration application state manager
    pub config_application_state: Arc<crate::core::profile::ConfigApplicationStateManager>,
    /// Redb cache manager (unified capabilities cache)
    pub redb_cache: Arc<crate::core::cache::RedbCacheManager>,
    /// Unified query adapter (optional, for gradual migration)
    pub unified_query: Option<Arc<crate::core::capability::UnifiedQueryAdapter>>,
    /// Client configuration service (template-driven)
    pub client_service: Option<Arc<ClientConfigService>>,
    /// Inspector call registry (long-running tool calls)
    pub inspector_calls: Arc<InspectorCallRegistry>,
    /// Inspector session manager
    pub inspector_sessions: Arc<InspectorSessionManager>,
    pub oauth_manager: Option<Arc<crate::core::oauth::OAuthManager>>,
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

    let audit_database = if let Some(ref proxy) = http_proxy {
        proxy.audit_database.clone()
    } else {
        None
    };

    let audit_service = if let Some(ref proxy) = http_proxy {
        proxy.audit_service.clone()
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

    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    let inspector_sessions = Arc::new(InspectorSessionManager::new());
    inspector_service::set_call_registry(inspector_calls.clone());

    // Create unified query adapter (optional, for incremental migration)
    let unified_query = if database.is_some() {
        crate::core::capability::UnifiedQueryIntegration::create_adapter(&AppState {
            connection_pool: connection_pool.clone(),
            metrics_collector: metrics_collector.clone(),
            http_proxy: http_proxy.clone(),
            profile_merge_service: profile_merge_service.clone(),
            database: database.clone(),
            audit_database: audit_database.clone(),
            audit_service: audit_service.clone(),
            config_application_state: config_application_state.clone(),
            redb_cache: redb_cache.clone(),
            unified_query: None, // avoid recursion
            client_service: None,
            inspector_calls: inspector_calls.clone(),
            inspector_sessions: inspector_sessions.clone(),
            oauth_manager: None,
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

    let oauth_manager = database
        .as_ref()
        .map(|db| Arc::new(crate::core::oauth::OAuthManager::new(db.pool.clone())));

    let state = Arc::new(AppState {
        connection_pool,
        metrics_collector,
        http_proxy,
        profile_merge_service,
        database,
        audit_database,
        audit_service,
        config_application_state,
        redb_cache,
        unified_query,
        client_service,
        inspector_calls,
        inspector_sessions,
        oauth_manager,
    });

    // Create OpenAPI specification
    let mut api = OpenApi::default();

    // Create API router with aide support
    let api_router = ApiRouter::new()
        .merge(audit::routes(state.clone()))
        .merge(server::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(profile::routes(state.clone()))
        .merge(runtime::routes(state.clone()))
        .merge(inspector::routes(state.clone()))
        .merge(client::routes(state.clone()))
        .merge(registry::routes(state.clone()));

    #[cfg(feature = "ai")]
    let api_router = api_router.merge(ai::routes(state.clone()));

    let api_router = api_router
        .finish_api_with(&mut api, openapi::api_docs)
        .layer(axum::middleware::from_fn(crate::common::env::origin_guard_middleware));

    let inspector_ws = Router::new()
        .route(
            "/inspector/events",
            get(crate::api::handlers::inspector::tool_call_events_ws),
        )
        .route("/audit/events", get(crate::api::handlers::audit::audit_events_ws))
        .with_state(state.clone());

    Router::new()
        .nest("/api", api_router)
        .nest("/ws", inspector_ws)
        .merge(openapi::openapi_routes(api))
        // Lightweight request/response logging for debugging 5xx issues
        // Logs method, path, status, and latency. 5xx at ERROR, 4xx at WARN, others at DEBUG.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<_>| {
                    tracing::span!(
                        Level::INFO,
                        "http",
                        method = %req.method(),
                        path = %req.uri().path()
                    )
                })
                .on_request(|_req: &Request<_>, _span: &tracing::Span| {
                    // Intentionally quiet; we log on response with status and latency
                })
                .on_response(|res: &Response<_>, latency: StdDuration, span: &tracing::Span| {
                    let status = res.status();
                    if status.is_server_error() {
                        tracing::error!(
                            parent: span,
                            status = %status,
                            latency_ms = %latency.as_millis(),
                            "HTTP response completed with 5xx"
                        );
                    } else if status.is_client_error() {
                        tracing::warn!(
                            parent: span,
                            status = %status,
                            latency_ms = %latency.as_millis(),
                            "HTTP response completed with 4xx"
                        );
                    } else {
                        tracing::debug!(
                            parent: span,
                            status = %status,
                            latency_ms = %latency.as_millis(),
                            "HTTP response completed"
                        );
                    }
                }),
        )
}
