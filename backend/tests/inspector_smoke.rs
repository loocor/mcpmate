use std::{path::PathBuf, sync::Arc};

use axum::body::to_bytes;
use axum::routing::{Router, get, post};
use futures_util::StreamExt;
use hyper::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tower::ServiceExt;

use mcpmate::api::handlers::{inspector, server};
use mcpmate::api::routes::AppState;
use mcpmate::common::constants::protocol;
use mcpmate::config::{database::Database, initialization::run_initialization};
use mcpmate::core::cache::{RedbCacheManager, manager::CacheConfig};
use mcpmate::core::models::Config;
use mcpmate::core::pool::UpstreamConnectionPool;
use mcpmate::core::profile::ConfigApplicationStateManager;
use mcpmate::inspector::{
    calls::InspectorCallRegistry, service as inspector_service, sessions::InspectorSessionManager,
};
use mcpmate::system::metrics::MetricsCollector;

const CREATE_SERVER_PATH: &str = "/api/mcp/servers/create";
const PREVIEW_SERVER_PATH: &str = "/api/mcp/servers/preview";
const TOOL_LIST_PATH: &str = "/api/mcp/inspector/tool/list";
const TOOL_CALL_PATH: &str = "/api/mcp/inspector/tool/call";
const RESOURCE_LIST_PATH: &str = "/api/mcp/inspector/resource/list";
const RESOURCE_READ_PATH: &str = "/api/mcp/inspector/resource/read";
const TEMPLATE_LIST_PATH: &str = "/api/mcp/inspector/template/list";
const TEMPLATE_READ_PATH: &str = "/api/mcp/inspector/template/read";
const SESSION_OPEN_PATH: &str = "/api/mcp/inspector/session/open";
const SESSION_CLOSE_PATH: &str = "/api/mcp/inspector/session/close";
const TEMPORARY_NATIVE_VALIDATION_SESSION_PREFIX: &str = "INSPNATIVE";

struct EnvVarGuard {
    key: &'static str,
}

impl EnvVarGuard {
    fn set(
        key: &'static str,
        value: &str,
    ) -> Self {
        // SAFETY: mutates process environment; native-off test uses `serial_test::serial` for this key.
        unsafe { std::env::set_var(key, value) };
        Self { key }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // SAFETY: pairs with `set`; same `serial` scope as above.
        unsafe { std::env::remove_var(self.key) };
    }
}

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
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache: redb,
        unified_query: None,
        client_service: None,
        inspector_calls,
        inspector_sessions,
        oauth_manager: RwLock::new(None),
        secret_store: RwLock::new(None),
        secret_store_readiness: RwLock::new(mcpmate::api::routes::unavailable_secret_store_readiness(
            "test_unavailable",
        )),
    })
}

async fn build_database_state(temp_dir: &TempDir) -> Arc<AppState> {
    let db_pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect("sqlite::memory:")
        .await
        .expect("sqlite pool");

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&db_pool)
        .await
        .expect("enable foreign keys");
    run_initialization(&db_pool).await.expect("initialize schema");
    mcpmate::core::capability::naming::initialize(db_pool.clone());
    mcpmate::core::capability::resolver::clear_cache().await;

    let database = Arc::new(Database {
        pool: db_pool,
        path: temp_dir.path().join("mcpmate-test.db"),
    });
    let cache_path = temp_dir.path().join("capability.redb");
    let redb_cache = Arc::new(RedbCacheManager::new(cache_path, CacheConfig::default()).expect("redb"));
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());

    Arc::new(AppState {
        connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
            Arc::new(Config::default()),
            Some(database.clone()),
        ))),
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database: Some(database),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache,
        unified_query: None,
        client_service: None,
        inspector_calls,
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: RwLock::new(None),
        secret_store: RwLock::new(None),
        secret_store_readiness: RwLock::new(mcpmate::api::routes::unavailable_secret_store_readiness(
            "test_unavailable",
        )),
    })
}

fn write_stdio_fixture(temp_dir: &TempDir) -> PathBuf {
    let path = temp_dir.path().join("stdio_mcp_fixture.py");
    let script = r#"
import json
import sys

def reply(request_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    req = json.loads(line)
    request_id = req.get("id")
    method = req.get("method")
    if request_id is None:
        continue
    if method == "initialize":
        reply(request_id, {
            "protocolVersion": "__PROTOCOL_VERSION__",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {"name": "inspector-fixture", "version": "1.0.0"}
        })
    elif method == "tools/list":
        reply(request_id, {
            "tools": [{
                "name": "echo",
                "description": "Echo a message.",
                "inputSchema": {
                    "type": "object",
                    "properties": {"message": {"type": "string"}},
                    "required": ["message"]
                }
            }]
        })
    elif method == "tools/call":
        message = req.get("params", {}).get("arguments", {}).get("message", "")
        reply(request_id, {
            "content": [{"type": "text", "text": "echo: " + message}],
            "isError": False
        })
    elif method == "resources/list":
        reply(request_id, {
            "resources": [{
                "uri": "test://hello",
                "name": "hello",
                "mimeType": "text/plain"
            }]
        })
    elif method == "resources/read":
        uri = req.get("params", {}).get("uri", "")
        resource_id = uri.rsplit("/", 1)[-1] if uri.startswith("test://dynamic/") else None
        reply(request_id, {
            "contents": [{
                "uri": uri,
                "mimeType": "text/plain",
                "text": "dynamic resource " + resource_id if resource_id is not None else "hello from resource"
            }]
        })
    elif method == "prompts/list":
        reply(request_id, {"prompts": []})
    elif method == "resources/templates/list":
        reply(request_id, {
            "resourceTemplates": [{
                "uriTemplate": "test://dynamic/{resourceId}",
                "name": "dynamic",
                "mimeType": "text/plain"
            }]
        })
    else:
        sys.stdout.write(json.dumps({
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {"code": -32601, "message": "method not found"}
        }) + "\n")
        sys.stdout.flush()
"#
    .replace("__PROTOCOL_VERSION__", protocol::CURRENT_VERSION);

    std::fs::write(&path, script).expect("write stdio fixture");
    path
}

fn write_empty_tool_fixture(temp_dir: &TempDir) -> PathBuf {
    let path = temp_dir.path().join("empty_tool_fixture.py");
    let script = r#"
import json
import sys

def reply(request_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    req = json.loads(line)
    request_id = req.get("id")
    method = req.get("method")
    if request_id is None:
        continue
    if method == "initialize":
        reply(request_id, {
            "protocolVersion": "__PROTOCOL_VERSION__",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "empty-tool-fixture", "version": "1.0.0"}
        })
    elif method == "tools/list":
        reply(request_id, {"tools": []})
"#
    .replace("__PROTOCOL_VERSION__", protocol::CURRENT_VERSION);
    std::fs::write(&path, script).expect("write empty tool fixture");
    path
}

fn write_slow_preview_fixture(temp_dir: &TempDir) -> PathBuf {
    let path = temp_dir.path().join("slow_preview_fixture.py");
    let script = r#"
import json
import sys
import time

delay_seconds = float(sys.argv[1])
slow_methods = sys.argv[2].split(",")
method_counts = {}
delay_overrides = {}
if len(sys.argv) > 3:
    for override in sys.argv[3].split(","):
        method, delay = override.rsplit("=", 1)
        delay_overrides[method] = float(delay)

def reply(request_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()

def wait_for(method):
    method_counts[method] = method_counts.get(method, 0) + 1
    indexed_method = method + ":" + str(method_counts[method])
    if "all" in slow_methods or method in slow_methods or indexed_method in slow_methods:
        delay = delay_overrides.get(indexed_method, delay_overrides.get(method, delay_seconds))
        time.sleep(delay)

for line in sys.stdin:
    if not line.strip():
        continue
    req = json.loads(line)
    request_id = req.get("id")
    method = req.get("method")
    if request_id is None:
        continue
    wait_for(method)
    if method == "initialize":
        reply(request_id, {
            "protocolVersion": "__PROTOCOL_VERSION__",
            "capabilities": {"tools": {}, "resources": {}, "prompts": {}},
            "serverInfo": {"name": "slow-preview-fixture", "version": "1.0.0"}
        })
    elif method == "tools/list":
        reply(request_id, {"tools": [{
            "name": "echo",
            "description": "Echo a message.",
            "inputSchema": {"type": "object", "properties": {}}
        }]})
    elif method == "prompts/list":
        reply(request_id, {"prompts": [{"name": "hello", "description": "Say hello."}]})
    elif method == "resources/list":
        reply(request_id, {"resources": [{"uri": "test://hello", "name": "hello"}]})
    elif method == "resources/templates/list":
        reply(request_id, {"resourceTemplates": [{
            "uriTemplate": "test://item/{id}",
            "name": "item"
        }]})
    else:
        sys.stdout.write(json.dumps({
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {"code": -32601, "message": "method not found"}
        }) + "\n")
        sys.stdout.flush()
"#
    .replace("__PROTOCOL_VERSION__", protocol::CURRENT_VERSION);

    std::fs::write(&path, script).expect("write slow preview fixture");
    path
}

async fn read_json_response(response: axum::response::Response) -> Value {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("response body");
    let body: Value = serde_json::from_slice(&bytes).expect("json response");
    assert!(status.is_success(), "unexpected status {status}: {body}");
    body
}

async fn read_json_response_with_status(response: axum::response::Response) -> (axum::http::StatusCode, Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("response body");
    let body: Value = serde_json::from_slice(&bytes).expect("json response");
    (status, body)
}

fn json_post_request(
    uri: &str,
    body: Value,
) -> Request<axum::body::Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn get_request(uri: String) -> Request<axum::body::Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(axum::body::Body::empty())
        .unwrap()
}

fn assert_api_success(body: &Value) {
    assert_eq!(body.pointer("/success").and_then(Value::as_bool), Some(true));
}

fn data_str<'a>(
    body: &'a Value,
    pointer: &str,
) -> &'a str {
    body.pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("expected string at {pointer}: {body}"))
}

fn data_u64(
    body: &Value,
    pointer: &str,
) -> u64 {
    body.pointer(pointer)
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("expected u64 at {pointer}: {body}"))
}

async fn create_stdio_fixture_server(
    app: &Router,
    temp_dir: &TempDir,
) -> String {
    let fixture = write_stdio_fixture(temp_dir);
    let python = which::which("python3").expect("python3 is required for stdio MCP fixture");
    let create_req = json_post_request(
        CREATE_SERVER_PATH,
        json!({
            "name": "inspector_fixture",
            "server_type": "stdio",
            "command": python.to_string_lossy(),
            "args": [fixture.to_string_lossy()]
        }),
    );

    let create_body = read_json_response(app.clone().oneshot(create_req).await.unwrap()).await;
    assert_api_success(&create_body);
    data_str(&create_body, "/data/id").to_string()
}

async fn seed_enabled_tool_server(
    state: &Arc<AppState>,
    server_id: &str,
    server_name: &str,
    command: &str,
) {
    let pool = &state.database.as_ref().expect("database state").pool;
    let profile_id = format!("profile-{server_id}");
    let profile_server_id = format!("profile-server-{server_id}");

    sqlx::query("INSERT INTO profile (id, name, type, is_active) VALUES (?, ?, 'user', 1)")
        .bind(&profile_id)
        .bind(format!("Profile {server_id}"))
        .execute(pool)
        .await
        .expect("insert active profile");
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, command, capabilities, enabled) \
         VALUES (?, ?, 'stdio', ?, 'tools', 1)",
    )
    .bind(server_id)
    .bind(server_name)
    .bind(command)
    .execute(pool)
    .await
    .expect("insert enabled server");
    sqlx::query(
        "INSERT INTO profile_server (id, profile_id, server_id, server_name, enabled) \
         VALUES (?, ?, ?, ?, 1)",
    )
    .bind(profile_server_id)
    .bind(profile_id)
    .bind(server_id)
    .bind(server_name)
    .execute(pool)
    .await
    .expect("insert enabled profile server");
}

async fn open_native_session(
    app: &Router,
    server_id: &str,
) -> String {
    let open_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                SESSION_OPEN_PATH,
                json!({
                    "server_id": server_id,
                    "mode": "native"
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&open_body);
    data_str(&open_body, "/data/session_id").to_string()
}

async fn close_inspector_session(
    app: &Router,
    session_id: &str,
) {
    let close_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                SESSION_CLOSE_PATH,
                json!({
                    "session_id": session_id
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&close_body);
    assert_eq!(close_body.pointer("/data/closed").and_then(Value::as_bool), Some(true));
}

async fn call_native_echo(
    app: &Router,
    server_id: &str,
    session_id: &str,
    message: &str,
) {
    let response = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                TOOL_CALL_PATH,
                json!({
                    "tool": "echo",
                    "server_id": server_id,
                    "mode": "native",
                    "session_id": session_id,
                    "timeout_ms": 5000,
                    "arguments": { "message": message }
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&response);
    assert_eq!(
        data_str(&response, "/data/result/content/0/text"),
        format!("echo: {message}")
    );
}

fn native_validation_session_id(session_id: &str) -> String {
    format!("inspector_native_session::{session_id}")
}

#[tokio::test]
async fn preview_timeout_is_applied_per_protocol_operation() {
    let temp_dir = TempDir::new().expect("temp dir");
    let fixture = write_slow_preview_fixture(&temp_dir);
    let python = which::which("python3").expect("python3 is required for stdio MCP fixture");
    let state = build_database_state(&temp_dir).await;
    let app = Router::new()
        .route(PREVIEW_SERVER_PATH, post(server::preview_servers))
        .with_state(state);

    let body = read_json_response(
        app.oneshot(json_post_request(
            PREVIEW_SERVER_PATH,
            json!({
                "timeout_ms": 300,
                "include_details": true,
                "servers": [
                    {
                        "name": "per_operation_deadline",
                        "kind": "stdio",
                        "command": python.to_string_lossy(),
                        "args": [fixture.to_string_lossy(), "0.10", "all"]
                    },
                    {
                        "name": "template_timeout",
                        "kind": "stdio",
                        "command": python.to_string_lossy(),
                        "args": [fixture.to_string_lossy(), "0.60", "resources/templates/list"]
                    }
                ]
            }),
        ))
        .await
        .expect("preview response"),
    )
    .await;

    assert_api_success(&body);
    assert_eq!(
        body.pointer("/data/items/0/ok").and_then(Value::as_bool),
        Some(true),
        "each successful protocol operation should receive the full timeout: {body}"
    );
    for pointer in [
        "/data/items/0/tools/items/0/name",
        "/data/items/0/prompts/items/0/name",
        "/data/items/0/resources/items/0/name",
        "/data/items/0/resource_templates/items/0/name",
    ] {
        assert!(body.pointer(pointer).is_some(), "missing {pointer}: {body}");
    }

    assert_eq!(body.pointer("/data/items/1/ok").and_then(Value::as_bool), Some(false));
    let error = data_str(&body, "/data/items/1/error");
    assert!(error.contains("resources/templates/list"), "unexpected error: {error}");
}

#[tokio::test]
#[serial_test::serial]
async fn proxy_list_connect_and_protocol_operation_receive_independent_timeouts() {
    let temp_dir = TempDir::new().expect("temp dir");
    let fixture = write_slow_preview_fixture(&temp_dir);
    let python = which::which("python3").expect("python3 is required for stdio MCP fixture");
    let state = build_database_state(&temp_dir).await;
    let app = Router::new()
        .route(CREATE_SERVER_PATH, post(server::create_server))
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .with_state(state.clone());

    let create_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                CREATE_SERVER_PATH,
                json!({
                    "name": "independent_list_timeout",
                    "server_type": "stdio",
                    "command": python.to_string_lossy(),
                    "args": [
                        fixture.to_string_lossy(),
                        "0.00",
                        "initialize,tools/list:2",
                        "initialize=0.40,tools/list:2=1.20"
                    ]
                }),
            ))
            .await
            .expect("create response"),
    )
    .await;
    assert_api_success(&create_body);
    let server_id = data_str(&create_body, "/data/id").to_string();

    disconnect_server_instances(&state, &server_id).await;

    let list_body = read_json_response(
        app.clone()
            .oneshot(get_request(format!(
                "{TOOL_LIST_PATH}?server_id={server_id}&mode=proxy&refresh=true&timeout_ms=1500"
            )))
            .await
            .expect("list response"),
    )
    .await;

    assert_api_success(&list_body);
    assert_eq!(data_u64(&list_body, "/data/total"), 1);

    disconnect_server_instances(&state, &server_id).await;
    let (timeout_status, timeout_body) = read_json_response_with_status(
        app.oneshot(get_request(format!(
            "{TOOL_LIST_PATH}?server_id={server_id}&mode=proxy&refresh=true&timeout_ms=100"
        )))
        .await
        .expect("timeout response"),
    )
    .await;
    assert_eq!(timeout_status, axum::http::StatusCode::REQUEST_TIMEOUT);
    assert_eq!(timeout_body.pointer("/error/status").and_then(Value::as_u64), Some(408));
    assert!(
        data_str(&timeout_body, "/error/message").contains("server connect exceeded 100 ms"),
        "unexpected timeout response: {timeout_body}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn proxy_aggregate_list_fails_when_every_eligible_server_fails() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_database_state(&temp_dir).await;
    seed_enabled_tool_server(
        &state,
        "aggregate-failure-server",
        "aggregate_failure_server",
        "/definitely/missing/mcp-server",
    )
    .await;
    let app = Router::new()
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .with_state(state);

    let (status, body) = read_json_response_with_status(
        app.oneshot(get_request(format!(
            "{TOOL_LIST_PATH}?mode=proxy&refresh=true&timeout_ms=100"
        )))
        .await
        .expect("aggregate list response"),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "unexpected response: {body}");
    assert!(
        data_str(&body, "/error/message").contains("All eligible upstream servers failed to list tools"),
        "unexpected aggregate error: {body}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn proxy_aggregate_list_keeps_successful_servers_when_one_server_fails() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_database_state(&temp_dir).await;
    let app = Router::new()
        .route(CREATE_SERVER_PATH, post(server::create_server))
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .with_state(state.clone());
    let healthy_server_id = create_stdio_fixture_server(&app, &temp_dir).await;
    let pool = &state.database.as_ref().expect("database state").pool;
    sqlx::query("INSERT INTO profile (id, name, type, is_active) VALUES ('profile-healthy', 'Healthy', 'user', 1)")
        .execute(pool)
        .await
        .expect("insert healthy profile");
    sqlx::query(
        "INSERT INTO profile_server (id, profile_id, server_id, server_name, enabled) \
         VALUES ('profile-server-healthy', 'profile-healthy', ?, 'inspector_fixture', 1)",
    )
    .bind(&healthy_server_id)
    .execute(pool)
    .await
    .expect("insert healthy profile server");
    seed_enabled_tool_server(
        &state,
        "aggregate-partial-failure",
        "aggregate_partial_failure",
        "/definitely/missing/mcp-server",
    )
    .await;

    let body = read_json_response(
        app.oneshot(get_request(format!(
            "{TOOL_LIST_PATH}?mode=proxy&refresh=true&timeout_ms=1000"
        )))
        .await
        .expect("aggregate list response"),
    )
    .await;

    assert_api_success(&body);
    assert_eq!(data_u64(&body, "/data/total"), 1);
    assert_eq!(data_str(&body, "/data/tools/0/name"), "inspector_fixture_echo");
    assert!(
        body.pointer("/data/meta")
            .and_then(Value::as_array)
            .is_some_and(|entries| entries.iter().any(|entry| entry.get("error").is_some())),
        "failed upstream should remain visible in Inspector metadata: {body}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn proxy_aggregate_list_rejects_empty_results_from_partial_inventory() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_database_state(&temp_dir).await;
    let python = which::which("python3").expect("python3 is required for stdio MCP fixture");
    let empty_fixture = write_empty_tool_fixture(&temp_dir);
    let app = Router::new()
        .route(CREATE_SERVER_PATH, post(server::create_server))
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .with_state(state.clone());
    let create_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                CREATE_SERVER_PATH,
                json!({
                    "name": "empty_tool_fixture",
                    "server_type": "stdio",
                    "command": python.to_string_lossy(),
                    "args": [empty_fixture.to_string_lossy()]
                }),
            ))
            .await
            .expect("create empty fixture response"),
    )
    .await;
    assert_api_success(&create_body);
    let empty_server_id = data_str(&create_body, "/data/id");
    let pool = &state.database.as_ref().expect("database state").pool;
    sqlx::query("INSERT INTO profile (id, name, type, is_active) VALUES ('profile-empty', 'Empty', 'user', 1)")
        .execute(pool)
        .await
        .expect("insert empty profile");
    sqlx::query(
        "INSERT INTO profile_server (id, profile_id, server_id, server_name, enabled) \
         VALUES ('profile-server-empty', 'profile-empty', ?, 'empty_tool_fixture', 1)",
    )
    .bind(empty_server_id)
    .execute(pool)
    .await
    .expect("insert empty profile server");
    seed_enabled_tool_server(
        &state,
        "aggregate-partial-empty-failure",
        "aggregate_partial_empty_failure",
        "/definitely/missing/mcp-server",
    )
    .await;

    let (status, body) = read_json_response_with_status(
        app.oneshot(get_request(format!(
            "{TOOL_LIST_PATH}?mode=proxy&refresh=true&timeout_ms=1000"
        )))
        .await
        .expect("aggregate list response"),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "unexpected response: {body}");
    assert!(
        data_str(&body, "/error/message").contains("listing is incomplete"),
        "unexpected incomplete aggregate error: {body}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn proxy_aggregate_list_surfaces_server_query_failures() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_database_state(&temp_dir).await;
    let pool = &state.database.as_ref().expect("database state").pool;
    sqlx::query("DROP TABLE profile_server")
        .execute(pool)
        .await
        .expect("drop profile server table");
    let app = Router::new()
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .with_state(state);

    let (status, body) = read_json_response_with_status(
        app.oneshot(get_request(format!("{TOOL_LIST_PATH}?mode=proxy")))
            .await
            .expect("aggregate list response"),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "unexpected response: {body}");
    assert!(
        data_str(&body, "/error/message").contains("profile_server"),
        "unexpected aggregate query error: {body}"
    );
}

async fn disconnect_server_instances(
    state: &Arc<AppState>,
    server_id: &str,
) {
    let mut pool = state.connection_pool.lock().await;
    let instance_ids = pool
        .get_all_server_instances()
        .remove(server_id)
        .unwrap_or_default()
        .into_iter()
        .map(|(instance_id, _)| instance_id)
        .collect::<Vec<_>>();
    for instance_id in instance_ids {
        pool.disconnect(server_id, &instance_id)
            .await
            .expect("disconnect prepared instance");
    }
}

async fn validation_session_exists(
    state: &Arc<AppState>,
    session_id: &str,
) -> bool {
    let pool = state.connection_pool.lock().await;
    pool.validation_sessions.contains_key(session_id)
}

async fn validation_session_contains_server(
    state: &Arc<AppState>,
    session_id: &str,
    server_id: &str,
) -> bool {
    let pool = state.connection_pool.lock().await;
    pool.validation_sessions
        .get(session_id)
        .is_some_and(|servers| servers.contains_key(server_id))
}

async fn temporary_validation_session_count(state: &Arc<AppState>) -> usize {
    let pool = state.connection_pool.lock().await;
    pool.validation_sessions
        .keys()
        .filter(|session_id| session_id.starts_with(TEMPORARY_NATIVE_VALIDATION_SESSION_PREFIX))
        .count()
}

#[tokio::test]
#[serial_test::serial]
async fn inspector_create_server_is_immediately_usable_without_restart() {
    let temp_dir = TempDir::new().expect("temp dir");
    let fixture = write_stdio_fixture(&temp_dir);
    let python = which::which("python3").expect("python3 is required for stdio MCP fixture");
    let state = build_database_state(&temp_dir).await;

    let app = Router::new()
        .route(CREATE_SERVER_PATH, post(server::create_server))
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .route(RESOURCE_LIST_PATH, get(inspector::resources_list))
        .route(RESOURCE_READ_PATH, get(inspector::resource_read))
        .route(TEMPLATE_LIST_PATH, get(inspector::templates_list))
        .route(TEMPLATE_READ_PATH, post(inspector::template_read))
        .with_state(state);

    let create_req = json_post_request(
        CREATE_SERVER_PATH,
        json!({
            "name": "inspector_fixture",
            "server_type": "stdio",
            "command": python.to_string_lossy(),
            "args": [fixture.to_string_lossy()]
        }),
    );

    let create_body = read_json_response(app.clone().oneshot(create_req).await.unwrap()).await;
    assert_api_success(&create_body);
    let server_id = data_str(&create_body, "/data/id").to_string();
    assert_eq!(
        data_str(&create_body, "/data/protocol_version"),
        protocol::CURRENT_VERSION
    );

    let tools_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=proxy&refresh=true"
    ));
    let tools_body = read_json_response(app.clone().oneshot(tools_req).await.unwrap()).await;
    assert_api_success(&tools_body);
    assert_eq!(data_u64(&tools_body, "/data/total"), 1);
    let tool_name = data_str(&tools_body, "/data/tools/0/name");
    assert!(
        tool_name.ends_with("_echo"),
        "proxy Inspector should expose the upstream echo tool with a stable unique name, got {tool_name}"
    );

    let resources_req = get_request(format!(
        "{RESOURCE_LIST_PATH}?server_id={server_id}&mode=proxy&refresh=true"
    ));
    let resources_body = read_json_response(app.clone().oneshot(resources_req).await.unwrap()).await;
    assert_api_success(&resources_body);
    assert_eq!(data_u64(&resources_body, "/data/total"), 1);
    let resource_uri = data_str(&resources_body, "/data/resources/0/uri");
    let expected_resource_uri = "mcpmate://resources/inspector_fixture/test/hello";
    assert_eq!(resource_uri, expected_resource_uri);

    let resource_read_query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("server_id", &server_id)
        .append_pair("mode", "proxy")
        .append_pair("uri", resource_uri)
        .finish();
    let resource_read_req = get_request(format!("{RESOURCE_READ_PATH}?{resource_read_query}"));
    let resource_read_body = read_json_response(app.clone().oneshot(resource_read_req).await.unwrap()).await;
    assert_eq!(
        data_str(&resource_read_body, "/data/result/contents/0/text"),
        "hello from resource"
    );

    let native_templates_req = get_request(format!(
        "{TEMPLATE_LIST_PATH}?server_id={server_id}&mode=native&refresh=true"
    ));
    let native_templates_body = read_json_response(app.clone().oneshot(native_templates_req).await.unwrap()).await;
    assert_api_success(&native_templates_body);
    assert_eq!(
        data_str(&native_templates_body, "/data/templates/0/uriTemplate"),
        "test://dynamic/{resourceId}"
    );

    let proxy_templates_req = get_request(format!(
        "{TEMPLATE_LIST_PATH}?server_id={server_id}&mode=proxy&refresh=true"
    ));
    let proxy_templates_body = read_json_response(app.clone().oneshot(proxy_templates_req).await.unwrap()).await;
    assert_api_success(&proxy_templates_body);
    let canonical_template = data_str(&proxy_templates_body, "/data/templates/0/uriTemplate");
    assert_eq!(
        canonical_template,
        "mcpmate://resources/template/inspector_fixture/test/dynamic/{resourceId}"
    );

    let template_read_response = app
        .clone()
        .oneshot(json_post_request(
            TEMPLATE_READ_PATH,
            json!({
                "uri_template": canonical_template,
                "arguments": { "resourceId": 42 },
                "mode": "proxy",
                "server_id": server_id,
            }),
        ))
        .await
        .expect("proxy template read response");
    assert_eq!(template_read_response.status(), StatusCode::OK);
    let template_read_body = read_json_response(template_read_response).await;
    assert_api_success(&template_read_body);
    assert_eq!(
        data_str(&template_read_body, "/data/expanded_uri"),
        "mcpmate://resources/template/inspector_fixture/test/dynamic/42"
    );
    assert_eq!(
        data_str(&template_read_body, "/data/result/contents/0/uri"),
        "mcpmate://resources/template/inspector_fixture/test/dynamic/42"
    );
    assert_eq!(
        data_str(&template_read_body, "/data/result/contents/0/text"),
        "dynamic resource 42"
    );

    let unknown_template_response = app
        .clone()
        .oneshot(json_post_request(
            TEMPLATE_READ_PATH,
            json!({
                "uri_template": "mcpmate://resources/template/inspector_fixture/test/dynamic/42",
                "arguments": {},
                "mode": "proxy",
                "server_id": server_id,
            }),
        ))
        .await
        .expect("unknown proxy template response");
    assert_eq!(unknown_template_response.status(), StatusCode::BAD_REQUEST);

    let proxy_dynamic_uri = canonical_template.replace("{resourceId}", "42");
    let proxy_dynamic_query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("server_id", &server_id)
        .append_pair("mode", "proxy")
        .append_pair("uri", &proxy_dynamic_uri)
        .finish();
    let proxy_dynamic_req = get_request(format!("{RESOURCE_READ_PATH}?{proxy_dynamic_query}"));
    let proxy_dynamic_body = read_json_response(app.clone().oneshot(proxy_dynamic_req).await.unwrap()).await;
    assert_api_success(&proxy_dynamic_body);
    assert_eq!(
        data_str(&proxy_dynamic_body, "/data/result/contents/0/uri"),
        proxy_dynamic_uri
    );
    assert_eq!(
        data_str(&proxy_dynamic_body, "/data/result/contents/0/text"),
        "dynamic resource 42"
    );

    let native_dynamic_query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("server_id", &server_id)
        .append_pair("mode", "native")
        .append_pair("uri", "test://dynamic/42")
        .finish();
    let native_dynamic_req = get_request(format!("{RESOURCE_READ_PATH}?{native_dynamic_query}"));
    let native_dynamic_body = read_json_response(app.oneshot(native_dynamic_req).await.unwrap()).await;
    assert_api_success(&native_dynamic_body);
    assert_eq!(
        data_str(&native_dynamic_body, "/data/result/contents/0/uri"),
        "test://dynamic/42"
    );
    assert_eq!(
        data_str(&native_dynamic_body, "/data/result/contents/0/text"),
        "dynamic resource 42"
    );

    mcpmate::core::capability::resolver::clear_cache().await;
}

#[tokio::test]
#[serial_test::serial]
async fn inspector_template_read_expands_and_reads_the_native_template() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_database_state(&temp_dir).await;

    let app = Router::new()
        .route(CREATE_SERVER_PATH, post(server::create_server))
        .route(TEMPLATE_READ_PATH, post(inspector::template_read))
        .with_state(state);
    let server_id = create_stdio_fixture_server(&app, &temp_dir).await;

    let response = app
        .oneshot(json_post_request(
            TEMPLATE_READ_PATH,
            json!({
                "uri_template": "test://dynamic/{resourceId}",
                "arguments": { "resourceId": 42 },
                "mode": "native",
                "server_id": server_id,
            }),
        ))
        .await
        .expect("template read response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json_response(response).await;

    assert_api_success(&body);
    assert_eq!(data_str(&body, "/data/expanded_uri"), "test://dynamic/42");
    assert_eq!(data_str(&body, "/data/result/contents/0/text"), "dynamic resource 42");
}

#[tokio::test]
#[serial_test::serial]
async fn inspector_template_read_rejects_invalid_expansion_inputs_as_bad_requests() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_database_state(&temp_dir).await;
    let app = Router::new()
        .route(CREATE_SERVER_PATH, post(server::create_server))
        .route(TEMPLATE_READ_PATH, post(inspector::template_read))
        .with_state(state);
    let server_id = create_stdio_fixture_server(&app, &temp_dir).await;

    for (uri_template, arguments) in [
        ("test://dynamic/{resourceId", json!({ "resourceId": 42 })),
        ("test://dynamic/{resourceId}", json!({ "unexpected": 42 })),
        ("test://dynamic/{resourceId}", json!({ "resourceId": [["nested"]] })),
        ("relative/{resourceId}", json!({ "resourceId": 42 })),
    ] {
        let response = app
            .clone()
            .oneshot(json_post_request(
                TEMPLATE_READ_PATH,
                json!({
                    "uri_template": uri_template,
                    "arguments": arguments,
                    "mode": "native",
                    "server_id": server_id,
                }),
            ))
            .await
            .expect("invalid template read response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "template: {uri_template}");
    }
}

#[tokio::test]
async fn inspector_proxy_template_read_names_its_database_requirement() {
    let app = Router::new()
        .route(TEMPLATE_READ_PATH, post(inspector::template_read))
        .with_state(build_test_state());

    let response = app
        .oneshot(json_post_request(
            TEMPLATE_READ_PATH,
            json!({
                "uri_template": "test://dynamic/{resourceId}",
                "mode": "proxy",
                "server_id": "missing-database",
            }),
        ))
        .await
        .expect("template read response");
    let (status, body) = read_json_response_with_status(response).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        data_str(&body, "/error/message"),
        "Proxy Inspector template routing requires database access"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn inspector_native_list_and_call_reuse_explicit_session() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_database_state(&temp_dir).await;

    let app = Router::new()
        .route(CREATE_SERVER_PATH, post(server::create_server))
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .route(TOOL_CALL_PATH, post(inspector::tool_call))
        .route(SESSION_OPEN_PATH, post(inspector::session_open))
        .route(SESSION_CLOSE_PATH, post(inspector::session_close))
        .with_state(state.clone());

    let server_id = create_stdio_fixture_server(&app, &temp_dir).await;
    let session_id = open_native_session(&app, &server_id).await;
    let validation_session = native_validation_session_id(&session_id);
    assert!(validation_session_exists(&state, &validation_session).await);
    assert!(validation_session_contains_server(&state, &validation_session, &server_id).await);
    assert_eq!(temporary_validation_session_count(&state).await, 0);

    let session_list_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=native&session_id={session_id}&refresh=true"
    ));
    let session_list_body = read_json_response(app.clone().oneshot(session_list_req).await.unwrap()).await;
    assert_api_success(&session_list_body);
    assert_eq!(data_u64(&session_list_body, "/data/total"), 1);
    assert!(validation_session_exists(&state, &validation_session).await);
    assert!(validation_session_contains_server(&state, &validation_session, &server_id).await);
    assert_eq!(temporary_validation_session_count(&state).await, 0);

    let stateless_list_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=native&refresh=true"
    ));
    let stateless_list_body = read_json_response(app.clone().oneshot(stateless_list_req).await.unwrap()).await;
    assert_api_success(&stateless_list_body);
    assert_eq!(data_u64(&stateless_list_body, "/data/total"), 1);
    assert!(validation_session_exists(&state, &validation_session).await);
    assert_eq!(temporary_validation_session_count(&state).await, 0);

    call_native_echo(&app, &server_id, &session_id, "session-reuse").await;

    close_inspector_session(&app, &session_id).await;
    assert!(!validation_session_exists(&state, &validation_session).await);

    let closed_session_list_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=native&session_id={session_id}&refresh=true"
    ));
    let (closed_status, closed_body) =
        read_json_response_with_status(app.clone().oneshot(closed_session_list_req).await.unwrap()).await;
    assert_eq!(closed_status, axum::http::StatusCode::NOT_FOUND);
    assert!(
        closed_body
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("not found or expired"),
        "expected explicit closed session error: {closed_body}"
    );

    mcpmate::core::capability::resolver::clear_cache().await;
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
#[serial_test::serial]
async fn inspector_native_mode_disabled_returns_forbidden() {
    let _native_off = EnvVarGuard::set("MCPMATE_INSPECTOR_NATIVE", "0");
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

/// Unknown `call_id`: server completes WebSocket handshake then closes (no SSE; stable path for Tauri/WKWebView).
#[tokio::test]
async fn inspector_tool_call_events_ws_unknown_call_closes() {
    let state = build_test_state();
    let app = Router::new()
        .route("/ws/inspector/events", get(inspector::tool_call_events_ws))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let url = format!(
        "ws://127.0.0.1:{}/ws/inspector/events?call_id=no-such-call-id",
        addr.port()
    );
    let (mut ws, response) = tokio_tungstenite::connect_async(url).await.expect("websocket connect");
    assert_eq!(response.status().as_u16(), 101, "expected 101 Switching Protocols");

    let first = ws.next().await.expect("stream item").expect("ws message");
    assert!(
        matches!(first, WsMessage::Close(_)),
        "expected server to close immediately for unknown call_id, got {first:?}"
    );

    let _ = ws.close(None).await;
    server.abort();
}
