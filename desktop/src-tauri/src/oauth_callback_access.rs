use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::{Mutex, oneshot};
use tokio::time::{Duration, sleep};
use tracing::warn;

const CALLBACK_EVENT_NAME: &str = "mcp-oauth/callback";
const CALLBACK_PATH: &str = "/oauth/callback";
const CALLBACK_FLOW_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Clone, Default)]
pub struct OAuthCallbackAccessState {
    flows: Arc<Mutex<HashMap<String, PendingLoopbackFlow>>>,
}

struct PendingLoopbackFlow {
    shutdown_tx: oneshot::Sender<()>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthCallbackAccessContract {
    pub kind: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OAuthCallbackNotificationPayload {
    r#type: String,
    server_id: Option<String>,
    error: Option<String>,
    timestamp: u64,
}

#[derive(Clone)]
struct LoopbackHandlerState {
    app: AppHandle,
    access_state: OAuthCallbackAccessState,
    server_id: String,
    api_base_url: String,
}

#[derive(Debug, Deserialize)]
struct LoopbackCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Serialize)]
struct CompleteOAuthRequest {
    state: String,
    code: String,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct OAuthStatusPayload {
    server_id: String,
}

impl OAuthCallbackAccessState {
    async fn replace_flow(&self, server_id: String, flow: PendingLoopbackFlow) {
        let mut guard = self.flows.lock().await;
        if let Some(existing) = guard.remove(&server_id) {
            let _ = existing.shutdown_tx.send(());
        }
        guard.insert(server_id, flow);
    }

    pub async fn finish_flow(&self, server_id: &str) -> bool {
        let mut guard = self.flows.lock().await;
        if let Some(existing) = guard.remove(server_id) {
            let _ = existing.shutdown_tx.send(());
            return true;
        }

        false
    }
}

pub async fn prepare_callback_access(
    app: AppHandle,
    access_state: OAuthCallbackAccessState,
    server_id: String,
    api_base_url: String,
) -> Result<OAuthCallbackAccessContract, String> {
    access_state.finish_flow(&server_id).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| error.to_string())?;
    let port = listener.local_addr().map_err(|error| error.to_string())?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    access_state
        .replace_flow(server_id.clone(), PendingLoopbackFlow { shutdown_tx })
        .await;

    let handler_state = LoopbackHandlerState {
        app,
        access_state: access_state.clone(),
        server_id,
        api_base_url: normalize_api_base_url(&api_base_url),
    };
    let timeout_app = handler_state.app.clone();
    let timeout_server_id = handler_state.server_id.clone();
    let router = Router::new()
        .route(CALLBACK_PATH, get(handle_loopback_callback))
        .with_state(handler_state);

    tauri::async_runtime::spawn(async move {
        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

        if let Err(error) = server.await {
            warn!(error = %error, "Desktop OAuth loopback callback server stopped with error");
        }
    });

    let timeout_state = access_state.clone();
    tauri::async_runtime::spawn(async move {
        sleep(CALLBACK_FLOW_TIMEOUT).await;
        if timeout_state.finish_flow(&timeout_server_id).await {
            let _ = timeout_app.emit(
                CALLBACK_EVENT_NAME,
                error_payload(
                    Some(timeout_server_id),
                    "OAuth authorization timed out before the callback completed.".to_string(),
                ),
            );
        }
    });

    Ok(OAuthCallbackAccessContract {
        kind: "desktop_loopback".to_string(),
        redirect_uri,
    })
}

pub fn open_authorization_url(app: &AppHandle, authorization_url: &str) -> Result<(), String> {
    app.opener()
        .open_url(authorization_url, None::<String>)
        .map_err(|error| error.to_string())
}

async fn handle_loopback_callback(
    State(state): State<LoopbackHandlerState>,
    Query(query): Query<LoopbackCallbackQuery>,
) -> Html<String> {
    let (payload, html) = if let Some(error) = query.error.or(query.error_description) {
        (
            error_payload(Some(state.server_id.clone()), error),
            render_result_html(false),
        )
    } else {
        match (query.code, query.state) {
            (Some(code), Some(flow_state)) => {
                match complete_oauth_callback(&state.api_base_url, &flow_state, &code).await {
                    Ok(server_id) => (success_payload(server_id), render_result_html(true)),
                    Err(error) => (
                        error_payload(Some(state.server_id.clone()), error),
                        render_result_html(false),
                    ),
                }
            }
            _ => (
                error_payload(
                    Some(state.server_id.clone()),
                    "Missing required OAuth callback parameters.".to_string(),
                ),
                render_result_html(false),
            ),
        }
    };

    let _ = state.app.emit(CALLBACK_EVENT_NAME, payload);
    state.access_state.finish_flow(&state.server_id).await;

    Html(html)
}

async fn complete_oauth_callback(
    api_base_url: &str,
    oauth_state: &str,
    code: &str,
) -> Result<String, String> {
    let endpoint = format!("{api_base_url}/api/mcp/servers/oauth/callback");
    let response = reqwest::Client::new()
        .post(endpoint)
        .json(&CompleteOAuthRequest {
            state: oauth_state.to_string(),
            code: code.to_string(),
        })
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;

    let envelope = response
        .json::<ApiEnvelope<OAuthStatusPayload>>()
        .await
        .map_err(|error| error.to_string())?;

    envelope
        .data
        .map(|payload| payload.server_id)
        .ok_or_else(|| "OAuth callback response missing server_id.".to_string())
}

fn normalize_api_base_url(api_base_url: &str) -> String {
    let trimmed = api_base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return "http://127.0.0.1:8080".to_string();
    }
    trimmed.to_string()
}

fn success_payload(server_id: String) -> OAuthCallbackNotificationPayload {
    OAuthCallbackNotificationPayload {
        r#type: "OAUTH_CALLBACK_SUCCESS".to_string(),
        server_id: Some(server_id),
        error: None,
        timestamp: unix_timestamp_ms(),
    }
}

fn error_payload(server_id: Option<String>, error: String) -> OAuthCallbackNotificationPayload {
    OAuthCallbackNotificationPayload {
        r#type: "OAUTH_CALLBACK_ERROR".to_string(),
        server_id,
        error: Some(error),
        timestamp: unix_timestamp_ms(),
    }
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn render_result_html(success: bool) -> String {
    let title = if success {
        "Authorization complete"
    } else {
        "Authorization failed"
    };
    let message = if success {
        "You can close this window and return to MCPMate."
    } else {
        "You can close this window and return to MCPMate to retry."
    };

    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
    <style>
      body {{
        margin: 0;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        background: #0f172a;
        color: #e2e8f0;
        display: flex;
        min-height: 100vh;
        align-items: center;
        justify-content: center;
        padding: 24px;
      }}
      main {{
        max-width: 420px;
        text-align: center;
        background: rgba(15, 23, 42, 0.88);
        border: 1px solid rgba(148, 163, 184, 0.25);
        border-radius: 16px;
        padding: 24px;
        box-shadow: 0 20px 45px rgba(15, 23, 42, 0.35);
      }}
      h1 {{ margin: 0 0 12px; font-size: 24px; }}
      p {{ margin: 0; line-height: 1.5; color: #cbd5e1; }}
    </style>
  </head>
  <body>
    <main>
      <h1>{title}</h1>
      <p>{message}</p>
    </main>
    <script>
      setTimeout(function () {{
        window.close();
      }}, 500);
    </script>
  </body>
</html>"#
    )
}
