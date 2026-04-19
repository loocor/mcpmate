use aide::{
    openapi::{OpenApi, Server, Tag},
    transform::TransformOpenApi,
};
use axum::{
    Router,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use base64::Engine; // for BASE64_STANDARD.decode
use once_cell::sync::OnceCell;
use std::sync::Arc;

// Cached OpenAPI payload for serving behind a lightweight authorization gate.
static OPENAPI_JSON: OnceCell<Arc<String>> = OnceCell::new();

/// Create OpenAPI documentation routes
pub fn openapi_routes(api: OpenApi) -> Router {
    // Serialize once to avoid cloning OpenApi structures at runtime.
    let json = serde_json::to_string(&api).unwrap_or_else(|_| "{}".to_string());
    let _ = OPENAPI_JSON.set(Arc::new(json));

    Router::new()
        .route("/docs", get(serve_docs))
        .route("/openapi.json", get(openapi_json_guarded))
}

/// Configure OpenAPI documentation
pub fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("MCPMate Management API")
        .description("API for managing MCP servers and instances")
        .version("0.1.0")
        .server(Server {
            url: "/api".into(),
            description: Some("MCPMate API Server".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "system".into(),
            description: Some("System management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "runtime".into(),
            description: Some("Runtime management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "profile".into(),
            description: Some("Profile management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "client".into(),
            description: Some("Client management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "server".into(),
            description: Some("Server management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "instance".into(),
            description: Some("Instance management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "capabilities".into(),
            description: Some("Cache management endpoints".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "ai".into(),
            description: Some("AI configuration management endpoints".into()),
            ..Default::default()
        })
}

/// Serve API documentation using Scalar UI
pub async fn serve_docs() -> Html<String> {
    let locked = std::env::var("MCPMATE_OPENAPI_PASSWORD_HASH")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        || std::env::var("MCPMATE_OPENAPI_PASSWORD")
            .map(|v| !v.is_empty())
            .unwrap_or(false);

    let html = if !locked {
        r#"<!DOCTYPE html>
<html>
<head>
  <title>MCPMate API Documentation</title>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <link rel="icon" href="data:," />
  <style>:root { --scalar-sidebar-width: 360px; }</style>
  <meta name="robots" content="noindex" />
</head>
<body>
  <script id="api-reference" data-url="/openapi.json"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.36.0"></script>
</body>
</html>"#
            .to_string()
    } else {
        r#"<!DOCTYPE html>
<html>
<head>
  <title>MCPMate API Documentation</title>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <link rel="icon" href="data:," />
  <meta name="robots" content="noindex" />
  <style id="lock-style">
    :root { --scalar-sidebar-width: 360px; }
    body { font-family: -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Helvetica, Arial, sans-serif; margin: 0; padding: 0; background: #0b1020; color: #e6e8f0; }
    .wrap { display: flex; align-items: center; justify-content: center; height: 100vh; }
    .card { width: 420px; padding: 28px; border-radius: 12px; background: #12172a; box-shadow: 0 8px 24px rgba(0,0,0,0.35); }
    h1 { margin: 0 0 8px; font-size: 20px; }
    p { margin: 0 0 16px; color: #aab2c5; font-size: 13px; }
    input { width: 100%; box-sizing: border-box; padding: 10px 12px; border: 1px solid #2a3456; border-radius: 8px; background: #0e1428; color: #e6e8f0; }
    button { margin-top: 12px; width: 100%; padding: 10px 12px; border: 0; border-radius: 8px; background: #3b82f6; color: white; font-weight: 600; cursor: pointer; }
    .err { color: #fca5a5; margin-top: 8px; min-height: 18px; font-size: 12px; }
  </style>
  </style>
</head>
<body>
  <div id="lock-root" class="wrap">
    <div class="card">
      <h1>MCPMate API Docs</h1>
      <p>Enter the preview password to unlock endpoints.</p>
      <input id="pw" type="password" autocomplete="current-password" placeholder="Password" />
      <button id="go">Unlock</button>
      <div id="err" class="err"></div>
    </div>
  </div>

  <script>
    const unlock = async () => {
      const pw = document.getElementById('pw').value || '';
      const resp = await fetch('/openapi.json', { headers: { 'X-OpenAPI-Password': pw } });
      if (!resp.ok) {
        document.getElementById('err').textContent = 'Incorrect password';
        return;
      }
      const basic = btoa('openapi:' + pw);
      const headers = {
        Authorization: 'Basic ' + basic,
        'X-OpenAPI-Password': pw, // fallback for any client that doesn't forward Authorization
      };
      // Also set a short-lived cookie as a last-resort transport for the password
      // so same-origin fetches (like Scalar) can succeed without custom header plumbing.
      document.cookie = 'MCPMATE_OPENAPI_PW=' + encodeURIComponent(pw) + '; Path=/; Max-Age=1800; SameSite=Lax';
      // Remove lock styles and DOM to avoid leaking CSS to Scalar UI
      const s = document.getElementById('lock-style'); if (s) s.remove();
      const r = document.getElementById('lock-root'); if (r) r.remove();
      document.body.innerHTML = '';
      // Re-apply only Scalar-specific variable
      const scalarStyle = document.createElement('style');
      scalarStyle.textContent = ':root { --scalar-sidebar-width: 360px; }';
      document.head.appendChild(scalarStyle);
      const api = document.createElement('script');
      api.id = 'api-reference';
      api.setAttribute('data-url', '/openapi.json');
      api.setAttribute('data-headers', JSON.stringify(headers));
      document.body.appendChild(api);
      const cdn = document.createElement('script');
      cdn.src = 'https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.36.0';
      document.body.appendChild(cdn);
    };
    document.getElementById('go').addEventListener('click', unlock);
    document.getElementById('pw').addEventListener('keydown', (e)=>{ if(e.key==='Enter') unlock(); });
  </script>
</body>
</html>"#.to_string()
    };

    Html(html)
}

/// Serve OpenAPI JSON with an optional password guard.
///
/// Behavior:
/// - If env `MCPMATE_OPENAPI_ENABLED` is set to `false`, returns 404.
/// - If env `MCPMATE_OPENAPI_PASSWORD` is set (non-empty), requires either:
///   - `Authorization: Basic base64("openapi:<password>")`, or
///   - header `X-OpenAPI-Password: <password>`.
///     On failure, returns 401 with `WWW-Authenticate: Basic` to indicate lock.
/// - Otherwise, returns the OpenAPI JSON body.
async fn openapi_json_guarded(headers: HeaderMap) -> impl IntoResponse {
    // Global enable/disable gate
    if std::env::var("MCPMATE_OPENAPI_ENABLED")
        .ok()
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("false") || v == "0")
    {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Some(json) = OPENAPI_JSON.get().cloned() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let guard_hash = std::env::var("MCPMATE_OPENAPI_PASSWORD_HASH").unwrap_or_default();
    let guard = std::env::var("MCPMATE_OPENAPI_PASSWORD").unwrap_or_default();
    if guard_hash.is_empty() && guard.is_empty() {
        return ([(header::CONTENT_TYPE, "application/json")], json.as_str().to_owned()).into_response();
    }

    // Accept custom header for non-interactive tooling
    if let Some(h) = headers.get("X-OpenAPI-Password") {
        let ok = if !guard_hash.is_empty() {
            h.to_str().map(|pw| verify_hash(pw, &guard_hash)).unwrap_or(false)
        } else {
            h.to_str().map(|s| s == guard).unwrap_or(false)
        };
        if ok {
            return ([(header::CONTENT_TYPE, "application/json")], json.as_str().to_owned()).into_response();
        }
    }

    // Accept HTTP Basic: any username, password must match guard.
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(encoded) = auth_str.strip_prefix("Basic ") {
                if let Ok(decoded) = base64::prelude::BASE64_STANDARD.decode(encoded) {
                    if let Ok(pair) = String::from_utf8(decoded) {
                        if let Some((_, pw)) = pair.split_once(':') {
                            let ok = if !guard_hash.is_empty() {
                                verify_hash(pw, &guard_hash)
                            } else {
                                pw == guard
                            };
                            if ok {
                                return ([(header::CONTENT_TYPE, "application/json")], json.as_str().to_owned())
                                    .into_response();
                            }
                        }
                    }
                }
            }
        }
    }

    // Accept password from cookie as a fallback (for embedded UIs that cannot attach headers).
    if let Some(cookie) = headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie.to_str() {
            for part in cookie_str.split(';') {
                let kv = part.trim();
                if let Some((k, v)) = kv.split_once('=') {
                    if k == "MCPMATE_OPENAPI_PW" {
                        let ok = if !guard_hash.is_empty() {
                            verify_hash(v, &guard_hash)
                        } else {
                            v == guard
                        };
                        if ok {
                            return ([(header::CONTENT_TYPE, "application/json")], json.as_str().to_owned())
                                .into_response();
                        }
                    }
                }
            }
        }
    }

    // Unauthorized without `WWW-Authenticate` to prevent the browser's basic auth popup.
    // Clients should retry with either `X-OpenAPI-Password` or `Authorization: Basic`.
    (
        StatusCode::UNAUTHORIZED,
        [(header::CONTENT_TYPE, "application/json")],
        "{\n  \"error\": \"unauthorized\"\n}",
    )
        .into_response()
}

fn verify_hash(
    pw: &str,
    encoded: &str,
) -> bool {
    if let Some(expect_hex) = encoded.strip_prefix("sha256:") {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(pw.as_bytes());
        let got = h.finalize();
        let got_hex = hex_lower(&got);
        return got_hex.eq_ignore_ascii_case(expect_hex);
    }
    false
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Macro to generate aide-compatible wrapper and documentation functions
///
/// This macro automatically generates both the `_aide` wrapper function and `_docs`
/// documentation function for legacy handlers that return `Result<Json<T>, ApiError>`.
///
/// # Usage
/// ```rust
/// aide_wrapper!(
///     system::get_status,         // Full handler path (enables VS Code jump + auto tag)
///     StatusResponse,             // Response type
///     "Get system status"         // Description (tag auto-extracted from module name)
/// );
/// ```
///
/// This generates:
/// - `get_status_aide` function compatible with aide
/// - `get_status_docs` function for OpenAPI documentation
///
/// Then use in routes:
/// ```rust
/// .api_route("/system/status", get_with(get_status_aide, get_status_docs))
/// ```
#[macro_export]
macro_rules! aide_wrapper {
    ($module:ident :: $handler:ident, $response_type:ty, $description:expr) => {
        paste::paste! {
            /// Aide-compatible wrapper function
            pub async fn [<$handler _aide>](
                axum::extract::State(state): axum::extract::State<std::sync::Arc<$crate::api::routes::AppState>>
            ) -> impl aide::axum::IntoApiResponse {
                use axum::response::IntoResponse;
                match $module::$handler(axum::extract::State(state)).await {
                    Ok(json_response) => json_response.into_response(),
                    Err(api_error) => api_error.into_response(),
                }
            }

            /// Documentation function for OpenAPI generation
            pub fn [<$handler _docs>](
                op: aide::transform::TransformOperation
            ) -> aide::transform::TransformOperation {
                op.description($description)
                    .tag(stringify!($module))  // auto-extract tag from module name
                    .response::<200, axum::Json<$response_type>>()
                    .response::<500, ()>()
            }
        }
    };
}

/// Macro for GET endpoints with query parameters
///
/// Usage:
/// ```rust
/// aide_get_with_query!(
///     capabilities::details,                      // Handler function path
///     DetailsQuery,                               // Query parameter type
///     serde_json::Value,                          // Response type
///     "Get cache details with filtering options"  // Description
/// );
/// ```
#[macro_export]
macro_rules! aide_wrapper_query {
    ($module:ident :: $handler:ident, $query_type:ty, $response_type:ty, $description:expr) => {
        paste::paste! {
            /// Aide-compatible wrapper function for GET with query parameters
            pub async fn [<$handler _aide>](
                axum::extract::Query(query): axum::extract::Query<$query_type>,
                axum::extract::State(state): axum::extract::State<std::sync::Arc<$crate::api::routes::AppState>>
            ) -> impl aide::axum::IntoApiResponse {
                use axum::response::IntoResponse;
                match $module::$handler(axum::extract::State(state), axum::extract::Query(query)).await {
                    Ok(json_response) => json_response.into_response(),
                    Err(api_error) => api_error.into_response(),
                }
            }

            /// Documentation function for GET with query parameters
            pub fn [<$handler _docs>](
                op: aide::transform::TransformOperation
            ) -> aide::transform::TransformOperation {
                op.description($description)
                    .tag(stringify!($module))
                    .response::<200, axum::Json<$response_type>>()
                    .response::<400, ()>()
                    .response::<500, ()>()
            }
        }
    };
}

/// Macro for POST endpoints with payload body
///
/// Usage:
/// ```rust
/// aide_wrapper_payload!(
///     runtime::install,                                                   // Handler function path
///     RuntimeInstallReq,                                                  // Payload body type
///     RuntimeInstallResp,                                                 // Response type
///     "Install runtime package (UV or Bun) with optional configuration"   // Description
/// );
/// ```
#[macro_export]
macro_rules! aide_wrapper_payload {
    ($module:ident :: $handler:ident, $json_type:ty, $response_type:ty, $description:expr) => {
        paste::paste! {
            /// Aide-compatible wrapper function for POST with payload body
            pub async fn [<$handler _aide>](
                axum::extract::State(state): axum::extract::State<std::sync::Arc<$crate::api::routes::AppState>>,
                axum::extract::Json(json): axum::extract::Json<$json_type>
            ) -> impl aide::axum::IntoApiResponse {
                use axum::response::IntoResponse;
                match $module::$handler(axum::extract::State(state), axum::extract::Json(json)).await {
                    Ok(json_response) => json_response.into_response(),
                    Err(api_error) => api_error.into_response(),
                }
            }

            /// Documentation function for POST with payload body
            pub fn [<$handler _docs>](
                op: aide::transform::TransformOperation
            ) -> aide::transform::TransformOperation {
                op.description($description)
                    .tag(stringify!($module))
                    .input::<axum::Json<$json_type>>()
                    .response::<200, axum::Json<$response_type>>()
                    .response::<400, ()>()
                    .response::<500, ()>()
            }
        }
    };
}
