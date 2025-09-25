use std::sync::Arc;

use axum::routing::{post, Router};
use tower::ServiceExt; // for `oneshot`
use axum::body::to_bytes;
use hyper::Request;
use tokio::sync::Mutex;
// bring crate types into scope
use mcpmate::api::handlers::inspector;
use mcpmate::api::routes::AppState;
use mcpmate::core::cache::RedbCacheManager;
use mcpmate::core::models::Config;
use mcpmate::core::pool::UpstreamConnectionPool;
use mcpmate::core::profile::ConfigApplicationStateManager;
use mcpmate::system::metrics::MetricsCollector;

#[tokio::test]
async fn inspector_tool_call_inline_error_or_accept() {
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
        Arc::new(Config::default()),
        None,
    )));
    let metrics = Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1)));
    let redb = RedbCacheManager::global().expect("redb");
    let state = Arc::new(AppState {
        connection_pool: pool,
        metrics_collector: metrics,
        http_proxy: None,
        profile_merge_service: None,
        database: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache: redb,
        unified_query: None,
        client_service: None,
    });

    let app = Router::new()
        .route(
            "/api/mcp/inspector/tool/call",
            post(inspector::tool_call),
        )
        .with_state(state);
    let req = Request::builder()
        .method("POST")
        .uri("/api/mcp/inspector/tool/call")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(serde_json::to_vec(&serde_json::json!({
            "tool":"noop",
            "mode":"proxy"
        })).unwrap()))
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
    } else {
        // inline error path
        let err = body.get("error").cloned().unwrap_or_default();
        assert!(err.get("message").is_some());
    }
}
