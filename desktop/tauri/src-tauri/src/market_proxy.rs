use std::time::Duration;

use anyhow::{Context, Result};
use http::{HeaderMap, Method, StatusCode, header};
use http::{Request as HttpRequest, Response as HttpResponse};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{Client, redirect::Policy};
use tracing::error;
use url::Url;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

static DIAG_REQ_ID: AtomicU64 = AtomicU64::new(1);

fn diag_enabled() -> bool {
    std::env::var("MCPMATE_MARKET_DIAG").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
}

fn diag_log(line: &str) {
    if !diag_enabled() { return; }
    let ts = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => format!("{:.3}", d.as_secs_f64()),
        Err(_) => "0".to_string(),
    };
    let mut path = std::env::var("TMPDIR").ok().map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    path.push("mcpmate-market-diag.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{}][scheme] {}", ts, line);
    }
}

// Embed the same assets used by the Vite dev middleware for injection.
// Keep a single source of truth by referencing files under board/.
// Note: path is relative to this file.
const MARKET_STYLE: &str = include_str!("../../../../board/scripts/market/market-style.css");
const MARKET_SHIM: &str = include_str!("../../../../board/scripts/market/market-shim.js");

#[derive(Clone)]
struct PortalDef {
    id: &'static str,
    remote_origin: &'static str,
    adapter: &'static str,
}

static PORTALS: &[PortalDef] = &[
    PortalDef {
        id: "mcpmarket",
        remote_origin: "https://mcpmarket.cn",
        adapter: "mcpmarket",
    },
    PortalDef {
        id: "mcpso",
        remote_origin: "https://mcp.so",
        adapter: "mcpso",
    },
];

// Absolute asset prefixes that remote portals commonly use.
static ABSOLUTE_ASSET_PREFIXES: &[&str] = &["/_next/", "/static/", "/assets/", "/images/"];

static ATTR_REWRITE_RE: Lazy<Regex> = Lazy::new(|| {
    // (href|src|action)="/..." or '...'
    Regex::new(r#"(?i)(href|src|action)=([\"'])/([^\"'>]*)"#).expect("valid regex")
});

static CSS_URL_RE: Lazy<Regex> = Lazy::new(|| {
    // url('/...') or url("/...")
    Regex::new(r#"(?i)url\((['"])\/([^)]+)\)"#).expect("valid regex")
});

fn find_portal<'a>(segments: &'a [&'a str]) -> Option<(PortalDef, usize)> {
    // Expecting path like: /market-proxy/{portal}/{rest...}
    if segments.first() != Some(&"market-proxy") {
        return None;
    }
    if let Some(id) = segments.get(1)
        && let Some(p) = PORTALS.iter().find(|p| &p.id == id)
    {
        return Some((p.clone(), 2));
    }
    None
}

fn portal_prefix(portal_id: &str) -> String {
    // Build the absolute prefix for this scheme: mcpmate://localhost/market-proxy/{portal}
    format!("mcpmate://localhost/market-proxy/{}", portal_id)
}

fn build_injected_head(portal: &PortalDef) -> String {
    let prefix = portal_prefix(portal.id);
    // Use external resources to avoid inline-script/style CSP issues.
    format!(
        r#"
<link id="mcpmate-market-outline-style" rel="stylesheet" href="{prefix}/scripts/market/market-style.css" />
<script id="mcpmate-market-config" src="{prefix}/scripts/market/config.js" ></script>
<script id="mcpmate-market-shim" src="{prefix}/scripts/market/market-shim.js" defer></script>
"#
    )
}

fn rewrite_html(mut html: String, portal: &PortalDef) -> String {
    // 1) Absolute attribute URLs -> prefix under our proxy
    html = ATTR_REWRITE_RE
        .replace_all(&html, |caps: &regex::Captures| {
            let attr = caps.get(1).unwrap().as_str();
            let quote = caps.get(2).unwrap().as_str();
            let rest = caps.get(3).unwrap().as_str();
            format!(
                "{}={}{}//localhost/market-proxy/{}/{}",
                attr, quote, "mcpmate:", portal.id, rest
            )
        })
        .into_owned();

    // 2) CSS url(/...) -> prefix
    html = CSS_URL_RE
        .replace_all(&html, |caps: &regex::Captures| {
            let quote = caps.get(1).unwrap().as_str();
            let rest = caps.get(2).unwrap().as_str();
            format!(
                "url({}mcpmate://localhost/market-proxy/{}/{})",
                quote, portal.id, rest
            )
        })
        .into_owned();

    // 3) Inject style + shim right after <head> or before </head>
    let inject = build_injected_head(portal);
    if let Some(idx) = html.find("<head>") {
        let insert_at = idx + "<head>".len();
        let mut buf = String::with_capacity(html.len() + inject.len() + 2);
        buf.push_str(&html[..insert_at]);
        buf.push('\n');
        buf.push_str(&inject);
        buf.push_str(&html[insert_at..]);
        return buf;
    }
    if let Some(idx) = html.find("</head>") {
        let mut buf = String::with_capacity(html.len() + inject.len() + 2);
        buf.push_str(&html[..idx]);
        buf.push_str(&inject);
        buf.push_str(&html[idx..]);
        return buf;
    }
    // Fallback: prepend
    format!("{}{}", inject, html)
}

fn is_escaped_asset(path: &str) -> bool {
    ABSOLUTE_ASSET_PREFIXES.iter().any(|p| path.starts_with(p))
}

fn portal_from_referer(headers: &HeaderMap) -> Option<PortalDef> {
    let referer = headers.get(header::REFERER)?.to_str().ok()?;
    if let Ok(url) = Url::parse(referer) {
        let segments: Vec<_> = url.path().trim_start_matches('/').split('/').collect();
        if let Some((portal, _)) = find_portal(&segments) {
            return Some(portal);
        }
    }
    None
}

fn forward_headers(orig: &HeaderMap) -> HeaderMap {
    static HOP_HEADERS: Lazy<Vec<header::HeaderName>> = Lazy::new(|| {
        vec![
            header::CONNECTION,
            header::HOST,
            header::PROXY_AUTHENTICATE,
            header::PROXY_AUTHORIZATION,
            header::TE,
            header::TRAILER,
            header::TRANSFER_ENCODING,
            header::UPGRADE,
            header::ORIGIN,
            header::ACCEPT_ENCODING,
        ]
    });
    let mut map = HeaderMap::new();
    for (k, v) in orig.iter() {
        if HOP_HEADERS.iter().any(|h| h == k) {
            continue;
        }
        map.append(k.clone(), v.clone());
    }
    if !map.contains_key(header::ACCEPT) {
        map.insert(header::ACCEPT, header::HeaderValue::from_static("*/*"));
    }
    if !map.contains_key(header::USER_AGENT) {
        map.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("mcpmate-tauri-market-proxy"),
        );
    }
    // Ensure identity encoding to keep bodies plain text for HTML injection
    map.insert(
        header::ACCEPT_ENCODING,
        header::HeaderValue::from_static("identity"),
    );
    map
}

pub fn register<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.register_asynchronous_uri_scheme_protocol("mcpmate", |_ctx, req, responder| {
        tauri::async_runtime::spawn(async move {
            match handle_request(req).await {
                Ok(resp) => responder.respond(resp),
                Err(err) => {
                    error!(error = %err, "market proxy error");
                    responder.respond(
                        HttpResponse::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                            .header(header::CACHE_CONTROL, "no-store")
                            .body("Market proxy error".as_bytes().to_vec())
                            .expect("failed to build error response"),
                    );
                }
            }
        });
    })
}

async fn handle_request(req: HttpRequest<Vec<u8>>) -> Result<HttpResponse<Vec<u8>>> {
    let uri = req.uri().clone();
    let path = uri.path();
    let method = req.method().clone();
    let req_id = DIAG_REQ_ID.fetch_add(1, Ordering::Relaxed);
    if diag_enabled() {
        let ae = req.headers().get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
        diag_log(&format!("#{} IN {} {} AE='{}'", req_id, method, path, ae));
    }

    // Serve embedded assets: /market-proxy/{portal}/scripts/market/*
    if let Some((portal, base_index)) = {
        let segs: Vec<_> = path.trim_start_matches('/').split('/').collect();
        find_portal(&segs)
    } {
        let segs: Vec<_> = path.trim_start_matches('/').split('/').collect();
        if segs.len() >= base_index + 3
            && segs[base_index] == "scripts"
            && segs[base_index + 1] == "market"
        {
            match segs[base_index + 2] {
                "market-style.css" => {
                    return Ok(HttpResponse::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
                        .header(header::CACHE_CONTROL, "no-store")
                        .body(MARKET_STYLE.as_bytes().to_vec())?);
                }
                "config.js" => {
                    // Build portal-aware bootstrap config without inline script.
                    let js = format!(
                        "window.__MCPMATE_PORTAL__={{portalId:'{id}',prefix:'{prefix}',remoteOrigin:'{origin}',adapter:'{adapter}'}};",
                        id = portal.id,
                        prefix = portal_prefix(portal.id),
                        origin = portal.remote_origin,
                        adapter = portal.adapter,
                    );
                    return Ok(HttpResponse::builder()
                        .status(StatusCode::OK)
                        .header(
                            header::CONTENT_TYPE,
                            "application/javascript; charset=utf-8",
                        )
                        .header(header::CACHE_CONTROL, "no-store")
                        .body(js.into_bytes())?);
                }
                "market-shim.js" => {
                    return Ok(HttpResponse::builder()
                        .status(StatusCode::OK)
                        .header(
                            header::CONTENT_TYPE,
                            "application/javascript; charset=utf-8",
                        )
                        .header(header::CACHE_CONTROL, "no-store")
                        .body(MARKET_SHIM.as_bytes().to_vec())?);
                }
                _ => {}
            }
        }

        // General proxying under /market-proxy/{portal}/...
        return proxy_to_portal(&req, &method, &portal).await;
    }

    // Escaped absolute assets like mcpmate://localhost/_next/*
    if is_escaped_asset(path)
        && let Some(portal) = portal_from_referer(req.headers())
    {
        return proxy_raw_asset(&req, &method, &portal, path).await;
    }

    // Not found
    Ok(HttpResponse::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(b"Not Found".to_vec())?)
}

async fn proxy_to_portal(
    req: &HttpRequest<Vec<u8>>,
    method: &Method,
    portal: &PortalDef,
) -> Result<HttpResponse<Vec<u8>>> {
    let uri = req.uri().clone();
    let req_id = DIAG_REQ_ID.fetch_add(1, Ordering::Relaxed);
    if diag_enabled() {
        let ae = req.headers().get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
        diag_log(&format!("#{} PROXY portal={} {} {} AE='{}'", req_id, portal.id, method, uri.path(), ae));
    }
    let path = uri.path();
    let path = {
        // strip /market-proxy/{id}
        let mut segs = path.trim_start_matches('/').split('/');
        let _ = segs.next(); // market-proxy
        let _ = segs.next(); // portal id
        format!("/{}", segs.collect::<Vec<_>>().join("/"))
    };

    let mut target = Url::parse(portal.remote_origin).context("invalid portal origin")?;
    target.set_path(&path);
    target.set_query(uri.query());

    let client = http_client();
    let mut builder = client.request(method.clone(), target);

    // Body
    if method != Method::GET && method != Method::HEAD {
        builder = builder.body(req.body().clone());
    }

    // Headers
    let headers = forward_headers(req.headers());
    builder = builder.headers(headers.clone());

    let upstream = builder.send().await.context("upstream fetch failed")?;
    let status = upstream.status();
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let ce = upstream.headers().get(header::CONTENT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
    if diag_enabled() { diag_log(&format!("#{} PAGE CT='{}' CE='{}' status={} portal={}", req_id, content_type, ce, status, portal.id)); }

    if content_type.to_ascii_lowercase().starts_with("text/html") {
        // Capture Set-Cookie before consuming body
        let set_cookie_vals: Vec<header::HeaderValue> = upstream
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .cloned()
            .collect();
        // reqwest decodes compressed bodies when features are enabled
        let text = upstream.text().await.unwrap_or_default();
        let body = rewrite_html(text, portal);
        let mut resp = HttpResponse::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CACHE_CONTROL, "no-store")
            .body(body.into_bytes())?;
        // Propagate Set-Cookie if present
        for v in set_cookie_vals.into_iter() {
            resp.headers_mut().append(header::SET_COOKIE, v.clone());
        }
        return Ok(resp);
    }

    let set_cookie_vals: Vec<header::HeaderValue> = upstream
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .cloned()
        .collect();
    let bytes = upstream.bytes().await.unwrap_or_default();
    let mut resp = HttpResponse::builder()
        .status(status)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_TYPE, content_type)
        .body(bytes.to_vec())?;
    for v in set_cookie_vals.into_iter() {
        resp.headers_mut().append(header::SET_COOKIE, v);
    }
    Ok(resp)
}

async fn proxy_raw_asset(
    req: &HttpRequest<Vec<u8>>,
    method: &Method,
    portal: &PortalDef,
    upstream_path: &str,
) -> Result<HttpResponse<Vec<u8>>> {
    let req_id = DIAG_REQ_ID.fetch_add(1, Ordering::Relaxed);
    if diag_enabled() { diag_log(&format!("#{} ASSET portal={} {} {}", req_id, portal.id, method, upstream_path)); }
    let mut target = Url::parse(portal.remote_origin).context("invalid portal origin")?;
    target.set_path(upstream_path);
    target.set_query(req.uri().query());

    let client = http_client();
    let mut builder = client.request(method.clone(), target);
    if method != Method::GET && method != Method::HEAD {
        builder = builder.body(req.body().clone());
    }
    builder = builder.headers(forward_headers(req.headers()));
    let upstream = builder.send().await.context("asset fetch failed")?;

    let status = upstream.status();
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let set_cookie_vals: Vec<header::HeaderValue> = upstream
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .cloned()
        .collect();
    let bytes = upstream.bytes().await.unwrap_or_default();
    let mut resp = HttpResponse::builder()
        .status(status)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_TYPE, content_type)
        .body(bytes.to_vec())?;
    for v in set_cookie_vals.into_iter() {
        resp.headers_mut().append(header::SET_COOKIE, v);
    }
    Ok(resp)
}

fn http_client() -> &'static Client {
    static CLIENT: Lazy<Client> = Lazy::new(|| {
        Client::builder()
            .redirect(Policy::limited(8))
            .timeout(Duration::from_secs(30))
            .cookie_store(true)
            .build()
            .expect("reqwest client")
    });
    &CLIENT
}
