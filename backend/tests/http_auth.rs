use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use mcpmate::common::server::ServerType;
use mcpmate::core::models::MCPServerConfig;
use mcpmate::core::transport::{TransportType, connect_http_server};
use tokio::net::TcpListener;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Default)]
struct CapturedAuth {
    post: Arc<tokio::sync::Mutex<Vec<Option<String>>>>,
    get: Arc<tokio::sync::Mutex<Vec<Option<String>>>>,
    delete: Arc<tokio::sync::Mutex<Vec<Option<String>>>>,
}

async fn capture_auth_middleware(
    State(captured): State<CapturedAuth>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let auth = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match method {
        Method::POST => captured.post.lock().await.push(auth),
        Method::GET => captured.get.lock().await.push(auth),
        Method::DELETE => captured.delete.lock().await.push(auth),
        _ => {}
    }

    next.run(req).await
}

#[derive(Clone, Default)]
struct DummyServer;

impl rmcp::ServerHandler for DummyServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo::default()
    }
}

#[tokio::test]
async fn streamable_http_carries_bearer_on_init_sse_delete() -> anyhow::Result<()> {
    let captured = CapturedAuth::default();
    let service: rmcp::transport::streamable_http_server::tower::StreamableHttpService<
        DummyServer,
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
    > = rmcp::transport::streamable_http_server::tower::StreamableHttpService::new(
        || Ok(DummyServer),
        Default::default(),
        rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default()
            .with_stateful_mode(true)
            .with_sse_keep_alive(None)
            .with_sse_retry(Some(Duration::from_secs(3)))
            .with_json_response(false)
            .with_cancellation_token(CancellationToken::new()),
    );

    let router = Router::new()
        .nest_service("/mcp", service)
        .layer(from_fn_with_state(captured.clone(), capture_auth_middleware));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr: SocketAddr = listener.local_addr()?;
    let shutdown = CancellationToken::new();
    let shutdown_task = {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { shutdown.cancelled_owned().await })
                .await;
        })
    };

    let url = format!("http://{addr}/mcp");
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer SECRET-XYZ".to_string());
    let server_cfg = MCPServerConfig {
        kind: ServerType::StreamableHttp,
        command: None,
        args: None,
        url: Some(url),
        env: None,
        headers: Some(headers),
    };

    let (service, _tools, _caps) = connect_http_server("auth-test", &server_cfg, TransportType::StreamableHttp).await?;

    sleep(Duration::from_millis(200)).await;

    let _ = service.cancel().await;

    sleep(Duration::from_millis(200)).await;
    shutdown.cancel();
    let _ = shutdown_task.await;

    let posts = captured.post.lock().await.clone();
    let gets = captured.get.lock().await.clone();
    let deletes = captured.delete.lock().await.clone();

    assert!(posts.iter().any(|h| matches!(h.as_deref(), Some("Bearer SECRET-XYZ"))));
    assert!(gets.iter().any(|h| matches!(h.as_deref(), Some("Bearer SECRET-XYZ"))));
    assert!(
        deletes
            .iter()
            .any(|h| matches!(h.as_deref(), Some("Bearer SECRET-XYZ")))
    );

    Ok(())
}
