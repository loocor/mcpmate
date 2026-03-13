use std::net::SocketAddr;

use anyhow::Result;
use bytes::Bytes;
use futures_util::{StreamExt, TryStreamExt};
use http::{HeaderMap, Method, StatusCode, header};
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hyper::body::Incoming;
use hyper::{Request, Response, server::conn::http1, service::service_fn};
use hyper_util::rt::tokio::TokioIo;
use once_cell::sync::Lazy;
use reqwest::{Client, redirect::Policy};
use tokio::{net::TcpListener, task::JoinHandle};
use tracing::{error, info, trace};
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
        let _ = writeln!(f, "[{}][stream] {}", ts, line);
    }
}

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

static ABSOLUTE_ASSET_PREFIXES: &[&str] = &["/_next/", "/static/", "/assets/", "/images/"];

fn find_portal<'a>(segments: &'a [&'a str]) -> Option<(PortalDef, usize)> {
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

fn forward_headers(orig: &HeaderMap) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (k, v) in orig.iter() {
        // Strip hop-by-hop headers and Accept-Encoding to avoid upstream compression
        if k == header::HOST
            || k == header::ORIGIN
            || k == header::CONNECTION
            || k == header::ACCEPT_ENCODING
        {
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
            header::HeaderValue::from_static("mcpmate-tauri-stream-proxy"),
        );
    }
    // Explicitly ask for identity to be safe when upstream ignores missing header
    map.insert(
        header::ACCEPT_ENCODING,
        header::HeaderValue::from_static("identity"),
    );
    map
}

fn build_head_injection(prefix: &str, _portal: &PortalDef) -> String {
    // Inject style + config + shim after <head> tag
    format!(
        "\n<link id=\"mcpmate-market-outline-style\" rel=\"stylesheet\" href=\"{prefix}/scripts/market/market-style.css\" />\n<script id=\"mcpmate-market-config\" src=\"{prefix}/scripts/market/config.js\"></script>\n<script id=\"mcpmate-market-shim\" src=\"{prefix}/scripts/market/market-shim.js\"></script>\n"
    )
}

fn proxy_base(port: u16) -> String {
    format!("http://127.0.0.1:{}/market-proxy", port)
}

fn http_client() -> &'static Client {
    static CL: Lazy<Client> = Lazy::new(|| {
        Client::builder()
            .redirect(Policy::limited(8))
            .cookie_store(true)
            .build()
            .expect("client")
    });
    &CL
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type Resp = Response<BoxBody<Bytes, BoxError>>;

async fn handle(req: Request<Incoming>, port: u16) -> Result<Resp> {
    let path = req.uri().path().to_string();
    let method_dbg = req.method().clone();
    let referer_dbg = req
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    trace!(target: "mcpmate_tauri::market_stream", %method_dbg, %path, referer=%referer_dbg, "incoming request");
    let segments: Vec<_> = path.trim_start_matches('/').split('/').collect();
    // Fallback for escaped absolute asset paths like /_next/* by using Referer to pick the portal
    let req_id = DIAG_REQ_ID.fetch_add(1, Ordering::Relaxed);
    if diag_enabled() {
        let ae = req.headers().get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
        diag_log(&format!("#{} IN {} {} AE='{}'", req_id, method_dbg, path, ae));
    }
    if ABSOLUTE_ASSET_PREFIXES.iter().any(|p| path.starts_with(p)) {
        let method = req.method().clone();
        let headers_snapshot = req.headers().clone();
        let query_snapshot = req.uri().query().map(|s| s.to_string());
        if let Some(referer) = headers_snapshot
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok())
            && let Ok(u) = url::Url::parse(referer)
        {
            let segs: Vec<_> = u.path().trim_start_matches('/').split('/').collect();
            if let Some((portal, _)) = find_portal(&segs) {
                trace!(target: "mcpmate_tauri::market_stream", portal=%portal.id, %path, "fallback absolute asset via Referer");
                let mut target = reqwest::Url::parse(portal.remote_origin)?;
                target.set_path(&path);
                target.set_query(query_snapshot.as_deref());
                let headers = forward_headers(&headers_snapshot);
                trace!(target: "mcpmate_tauri::market_stream", to=%target.as_str(), "proxy upstream asset");
                let upstream = http_client()
                    .request(method, target)
                    .headers(headers)
                    .send()
                    .await?;
                let status = upstream.status();
                let content_type = upstream
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                let ce = upstream.headers().get(header::CONTENT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
                if diag_enabled() { diag_log(&format!("#{} ASSET {} CT='{}' CE='{}'", req_id, status, content_type, ce)); }
                let set_cookie_vals: Vec<header::HeaderValue> = upstream
                    .headers()
                    .get_all(header::SET_COOKIE)
                    .iter()
                    .cloned()
                    .collect();
                trace!(target: "mcpmate_tauri::market_stream", %status, %content_type, "upstream asset response");
                // Body is decompressed by reqwest when compression features are enabled.
                let bytes = upstream.bytes().await?;
                let base = Full::new(Bytes::copy_from_slice(bytes.as_ref()))
                    .map_err(|never| match never {});
                let mapped =
                    <_ as http_body_util::BodyExt>::map_err(base, |never| -> BoxError { match never {} });
                let body: BoxBody<Bytes, BoxError> = http_body_util::BodyExt::boxed(mapped);
                let mut resp = Response::new(body);
                *resp.status_mut() = status;
                resp.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-store"),
                );
                if let Ok(hv) = header::HeaderValue::from_str(&content_type) {
                    resp.headers_mut().insert(header::CONTENT_TYPE, hv);
                }
                // Propagate Set-Cookie if present
                for v in set_cookie_vals.into_iter() {
                    resp.headers_mut().append(header::SET_COOKIE, v);
                }
                return Ok(resp);
            }
        }
    }
    if let Some((portal, base)) = find_portal(&segments) {
        // Serve assets
        if segments.len() >= base + 3
            && segments[base] == "scripts"
            && segments[base + 1] == "market"
        {
            let name = segments[base + 2];
            let (mime, body) = match name {
                "market-style.css" => ("text/css; charset=utf-8", MARKET_STYLE.as_bytes().to_vec()),
                "market-shim.js" => (
                    "application/javascript; charset=utf-8",
                    MARKET_SHIM.as_bytes().to_vec(),
                ),
                "config.js" => {
                    let js = format!(
                        "window.__MCPMATE_PORTAL__={{portalId:'{id}',prefix:'{prefix}',remoteOrigin:'{origin}',adapter:'{adapter}'}};",
                        id = portal.id,
                        prefix = proxy_base(port) + "/" + portal.id,
                        origin = portal.remote_origin,
                        adapter = portal.adapter,
                    );
                    ("application/javascript; charset=utf-8", js.into_bytes())
                }
                _ => ("text/plain; charset=utf-8", b"Not Found".to_vec()),
            };
            let base = Full::new(Bytes::from(body)).map_err(|never| match never {});
            let mapped = <_ as http_body_util::BodyExt>::map_err(base, |never| -> BoxError {
                match never {}
            });
            let body: BoxBody<Bytes, BoxError> = http_body_util::BodyExt::boxed(mapped);
            let mut resp = Response::new(body);
            *resp.status_mut() = StatusCode::OK;
            resp.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("no-store"),
            );
            resp.headers_mut()
                .insert(header::CONTENT_TYPE, header::HeaderValue::from_static(mime));
            return Ok(resp);
        }

        // Proxy to upstream
        let mut rel = "/".to_string();
        if segments.len() > base {
            rel.push_str(&segments[base..].join("/"));
        }
        let query_snapshot = req.uri().query().map(|s| s.to_string());
        let method = match *req.method() {
            Method::GET => Method::GET,
            Method::HEAD => Method::HEAD,
            Method::POST => Method::POST,
            Method::PUT => Method::PUT,
            Method::DELETE => Method::DELETE,
            Method::PATCH => Method::PATCH,
            _ => Method::GET,
        };
        let headers_snapshot = req.headers().clone();
        let body_bytes = req.collect().await?.to_bytes();
        let mut target = reqwest::Url::parse(portal.remote_origin)?;
        target.set_path(&rel);
        target.set_query(query_snapshot.as_deref());

        let headers = forward_headers(&headers_snapshot);
        trace!(target: "mcpmate_tauri::market_stream", portal=%portal.id, to=%target.as_str(), %method, "proxy upstream page");
        let upstream = http_client()
            .request(method, target)
            .headers(headers)
            .body(body_bytes)
            .send()
            .await?;
        let status = upstream.status();
        let content_type = upstream
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let ce = upstream.headers().get(header::CONTENT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
        if diag_enabled() { diag_log(&format!("#{} PAGE portal={} {} CT='{}' CE='{}'", req_id, portal.id, status, content_type, ce)); }
        let set_cookie_vals: Vec<header::HeaderValue> = upstream
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .cloned()
            .collect();
        trace!(target: "mcpmate_tauri::market_stream", %status, %content_type, "upstream page response");

        // For mcp.so HTML, stream and inject after <head>
        if portal.id == "mcpso" && content_type.to_ascii_lowercase().starts_with("text/html") {
            let prefix = format!("{}/{}", proxy_base(port), portal.id);
            let injection = build_head_injection(&prefix, &portal);
            let mut injected = false;
            let mut buf = Vec::new();
            let mut stream = upstream.bytes_stream();
            if diag_enabled() { diag_log(&format!("#{} STREAM inject-after-<head> start", req_id)); }

            let s = async_stream::try_stream! {
                while let Some(chunk) = stream.next().await {
                    let bytes = chunk?;
                    tracing::trace!(target: "mcpmate_tauri::market_stream", chunk=bytes.len(), injected=?injected, "upstream chunk");

                    if !injected {
                        buf.extend_from_slice(&bytes);

                        // Try to find and inject after <head>
                        if let Some(pos) = twoway::find_bytes(&buf, b"<head>") {
                            let head_end = pos + 6;
                            let before = buf[..head_end].to_vec();
                            let after = buf[head_end..].to_vec();
                            tracing::trace!(target: "mcpmate_tauri::market_stream", head_index=head_end, before=before.len(), after=after.len(), "inject after <head>");
                            yield Bytes::from(before);
                            yield Bytes::from(injection.clone());
                            yield Bytes::from(after);
                            buf.clear();
                            injected = true;
                            if diag_enabled() { diag_log(&format!("#{} STREAM injected at {}", req_id, head_end)); }
                        } else if buf.len() > 512 {
                            // Write buffered content but keep small buffer for pattern matching
                            let keep = 256;
                            let split = buf.len().saturating_sub(keep);
                            if split > 0 {
                                let out = buf[..split].to_vec();
                                tracing::trace!(target: "mcpmate_tauri::market_stream", out=out.len(), keep=keep, "flush buffered bytes");
                                yield Bytes::from(out);
                                buf = buf[split..].to_vec();
                            }
                        }
                        continue;
                    }

                    // After injection, stream through directly
                    yield bytes;
                }

                // Write remaining buffer
                if !buf.is_empty() {
                    // Last chance to inject if <head> wasn't found earlier
                    if !injected {
                        if let Some(pos) = twoway::find_bytes(&buf, b"<head>") {
                            let head_end = pos + 6;
                            let before = buf[..head_end].to_vec();
                            let after = buf[head_end..].to_vec();
                            tracing::trace!(target: "mcpmate_tauri::market_stream", head_index=head_end, before=before.len(), after=after.len(), "late inject at end");
                            yield Bytes::from(before);
                            yield Bytes::from(injection.clone());
                            yield Bytes::from(after);
                            if diag_enabled() { diag_log(&format!("#{} STREAM late-injected at end", req_id)); }
                        } else {
                            tracing::trace!(target: "mcpmate_tauri::market_stream", remaining=buf.len(), "write remaining without head");
                            yield Bytes::from(buf.clone());
                            if diag_enabled() { diag_log(&format!("#{} STREAM ended without <head> match", req_id)); }
                        }
                    } else {
                        tracing::trace!(target: "mcpmate_tauri::market_stream", remaining=buf.len(), "write final buffer");
                        yield Bytes::from(buf.clone());
                    }
                }
            };

            let sb = StreamBody::new(s.map_ok(hyper::body::Frame::data));
            let b = <StreamBody<_> as http_body_util::BodyExt>::map_err(
                sb,
                |e: reqwest::Error| -> BoxError { Box::new(e) },
            );
            let body: BoxBody<Bytes, BoxError> = http_body_util::BodyExt::boxed(b);
            let mut resp = Response::new(body);
            *resp.status_mut() = status;
            resp.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("no-store"),
            );
            if let Ok(hv) = header::HeaderValue::from_str(&content_type) {
                resp.headers_mut().insert(header::CONTENT_TYPE, hv);
            }
            return Ok(resp);
        }

        // Non-HTML or other portals: simple relay as bytes
        // Fetch decompressed bytes (reqwest handles gzip/br/deflate/zstd when enabled)
        let bytes = upstream.bytes().await?;
        let base =
            Full::new(Bytes::copy_from_slice(bytes.as_ref())).map_err(|never| match never {});
        let mapped =
            <_ as http_body_util::BodyExt>::map_err(base, |never| -> BoxError { match never {} });
        let body: BoxBody<Bytes, BoxError> = http_body_util::BodyExt::boxed(mapped);
        let mut resp = Response::new(body);
        *resp.status_mut() = status;
        resp.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        );
        if let Ok(hv) = header::HeaderValue::from_str(&content_type) {
            resp.headers_mut().insert(header::CONTENT_TYPE, hv);
        }
        // Propagate Set-Cookie for session consistency
        for v in set_cookie_vals.into_iter() {
            resp.headers_mut().append(header::SET_COOKIE, v);
        }
        return Ok(resp);
    }

    let base = Full::new(Bytes::from_static(b"Not Found")).map_err(|never| match never {});
    let mapped =
        <_ as http_body_util::BodyExt>::map_err(base, |never| -> BoxError { match never {} });
    let body: BoxBody<Bytes, BoxError> = http_body_util::BodyExt::boxed(mapped);
    let mut resp = Response::new(body);
    *resp.status_mut() = StatusCode::NOT_FOUND;
    Ok(resp)
}

pub async fn start_streaming_proxy() -> Result<(JoinHandle<()>, u16)> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    let addr: SocketAddr = listener.local_addr()?;
    let port = addr.port();
    let server = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    let svc = service_fn(move |req| handle(req, port));
                    tokio::spawn(async move {
                        let io = TokioIo::new(stream);
                        if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                            error!(error = %err, "stream-proxy connection error");
                        }
                    });
                }
                Err(err) => {
                    error!(error = %err, "stream-proxy accept error");
                    break;
                }
            }
        }
    });
    info!(port, "started streaming market proxy on 127.0.0.1");
    Ok((server, port))
}
