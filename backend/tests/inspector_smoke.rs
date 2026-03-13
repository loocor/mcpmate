use std::sync::Arc;

use axum::body::to_bytes;
use axum::routing::{Router, get, post};
use hyper::{Request, StatusCode};
use tokio::sync::Mutex;
use tower::ServiceExt; // for `oneshot`
// bring crate types into scope
use mcpmate::api::handlers::inspector;
use mcpmate::api::routes::AppState;
use mcpmate::core::cache::RedbCacheManager;
use mcpmate::core::models::Config;
use mcpmate::core::pool::UpstreamConnectionPool;
use mcpmate::core::profile::ConfigApplicationStateManager;
use mcpmate::inspector::{
    calls::InspectorCallRegistry, service as inspector_service, sessions::InspectorSessionManager,
};
use mcpmate::system::metrics::MetricsCollector;

fn build_test_state() -> Arc<AppState> {
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
        Arc::new(Config::default()),
        None,
    )));
    let metrics = Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1)));
    let redb = RedbCacheManager::global().expect("redb");
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());
    let inspector_sessions = Arc::new(InspectorSessionManager::new());

    Arc::new(AppState {
        connection_pool: pool,
        metrics_collector: metrics,
        http_proxy: None,
        profile_merge_service: None,
        database: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache: redb,
        unified_query: None,
        client_service: None,
        inspector_calls,
        inspector_sessions,
    })
}

#[tokio::test]
async fn inspector_tool_call_inline_error_or_accept() {
    let state = build_test_state();

    let app = Router::new()
        .route("/api/mcp/inspector/tool/call", post(inspector::tool_call))
        .with_state(state);
    let req = Request::builder()
        .method("POST")
        .uri("/api/mcp/inspector/tool/call")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({
                "tool":"noop",
                "mode":"proxy"
            }))
            .unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), 200);
    let bytes = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let success = body.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    if success {
        // accepted path
        let data = body.get("data").cloned().unwrap_or_default();
        assert!(data.get("call_id").is_some());
        assert!(data.get("message").is_some());
    } else {
        // inline error path
        let err = body.get("error").cloned().unwrap_or_default();
        assert!(err.get("message").is_some());
    }
}

#[tokio::test]
async fn inspector_native_mode_disabled_returns_forbidden() {
    let state = build_test_state();
    let app = Router::new()
        .route("/api/mcp/inspector/tool/list", get(inspector::tools_list))
        .with_state(state);

    let req = Request::builder()
        .method("GET")
        .uri("/api/mcp/inspector/tool/list?mode=native&server_id=test")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
