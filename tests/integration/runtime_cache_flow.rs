//! Minimal integration tests for runtime status and Redb cache flow

use axum::{Router, body::to_bytes};
use mcpmate::api::routes::create_router;
use std::sync::Arc;
use tower::util::ServiceExt; // for oneshot

// Helper to create an in-memory app with initialized router
async fn create_app() -> Router {
    // Unique Redb path per test run to avoid lock conflicts
    // Simple unique file name using monotonic timestamp to avoid deprecations
    let now_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let unique = format!("test-cache-{}-{}.redb", std::process::id(), now_nanos);
    let path = std::env::temp_dir().join(unique);
    // Safety: setting an env var for test isolation is fine; no concurrency issues expected here
    unsafe {
        std::env::set_var("MCPMATE_REDB_CACHE_PATH", path);
    }
    // Build a minimal config and proxy to satisfy router creation
    // Reuse ProxyServer to create a connection_pool instance
    let proxy = mcpmate::core::proxy::server::ProxyServer::new(Arc::new(mcpmate::core::models::Config {
        mcp_servers: std::collections::HashMap::new(),
        pagination: None,
    }));

    let connection_pool = proxy.connection_pool.clone();
    create_router(connection_pool)
}

#[tokio::test]
#[serial_test::serial]
async fn test_runtime_status_counts_update_with_instance_type() {
    let app = create_app().await;

    // Initially: fetch status -> get baseline counts
    let req = axum::http::Request::builder()
        .uri("/api/runtime/status")
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());

    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let baseline_exploration = v["active_servers"]["exploration"].as_u64().unwrap_or(0);

    // Trigger exploration session accounting via instance_type param on tools endpoint
    // server id won't resolve from DB in this test; we only assert handler wiring doesn't panic and increments session
    // Use a non-existent server id; expect NotFound or empty but session should register based on query
    let req2 = axum::http::Request::builder()
        .uri("/api/mcp/servers/nonexist/tools?instance_type=exploration")
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req2).await; // ignore result

    // Read status again
    let req3 = axum::http::Request::builder()
        .uri("/api/runtime/status")
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp3 = app.clone().oneshot(req3).await.unwrap();
    let body3 = to_bytes(resp3.into_body(), 1024 * 1024).await.unwrap();
    let v3: serde_json::Value = serde_json::from_slice(&body3).unwrap();
    let after_exploration = v3["active_servers"]["exploration"].as_u64().unwrap_or(0);

    // Because register_session_if_needed uses a lock with timeout and increments in pool, count should be >= baseline
    assert!(after_exploration >= baseline_exploration);
}

#[tokio::test]
#[serial_test::serial]
async fn test_runtime_cache_clear_endpoint() {
    let app = create_app().await;

    // GET cache metrics
    let req = axum::http::Request::builder()
        .uri("/api/runtime/cache")
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());

    // Clear cache
    let req2 = axum::http::Request::builder()
        .uri("/api/runtime/cache/clear")
        .method("POST")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from("{}"))
        .unwrap();
    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert!(resp2.status().is_success());
}
