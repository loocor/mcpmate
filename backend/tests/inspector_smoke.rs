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
use mcpmate::common::server::ServerType;
use mcpmate::config::{database::Database, initialization::run_initialization};
use mcpmate::core::cache::{RedbCacheManager, manager::CacheConfig};
use mcpmate::core::models::{Config, MCPServerConfig};
use mcpmate::core::pool::UpstreamConnectionPool;
use mcpmate::core::profile::ConfigApplicationStateManager;
use mcpmate::core::proxy::server::ProxyServer;
use mcpmate::inspector::{
    calls::InspectorCallRegistry,
    service as inspector_service,
    sessions::InspectorSessionManager,
    workspace::{InspectorServerProvenance, InspectorServerRecordInput, InspectorWorkspace},
};
use mcpmate::system::config::{RuntimePortConfig, get_runtime_port_config, init_port_config};
use mcpmate::system::metrics::MetricsCollector;

const CREATE_SERVER_PATH: &str = "/api/mcp/servers/create";
const TOOL_LIST_PATH: &str = "/api/mcp/inspector/tool/list";
const TOOL_CALL_PATH: &str = "/api/mcp/inspector/tool/call";
const TOOL_CALL_EVIDENCE_PATH: &str = "/api/mcp/inspector/tool/call/evidence";
const CAPABILITY_PATCH_UPSERT_PATH: &str = "/api/mcp/inspector/capability-patch/upsert";
const SCRATCH_SERVER_LIST_PATH: &str = "/api/mcp/inspector/scratch/server/list";
const SCRATCH_SERVER_CREATE_PATH: &str = "/api/mcp/inspector/scratch/server/create";
const SCRATCH_SERVER_DELETE_PATH: &str = "/api/mcp/inspector/scratch/server/delete";
const COMPATIBILITY_SNAPSHOT_PATH: &str = "/api/mcp/inspector/compatibility/snapshot";
const PACKAGE_SAFETY_SNAPSHOT_PATH: &str = "/api/mcp/inspector/package-safety/snapshot";
const PROMPT_GET_PATH: &str = "/api/mcp/inspector/prompt/get";
const RESOURCE_LIST_PATH: &str = "/api/mcp/inspector/resource/list";
const RESOURCE_READ_PATH: &str = "/api/mcp/inspector/resource/read";
const SESSION_OPEN_PATH: &str = "/api/mcp/inspector/session/open";
const SESSION_CLOSE_PATH: &str = "/api/mcp/inspector/session/close";

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
        inspector_workspace: Arc::new(InspectorWorkspace::new(mcpmate::common::paths::global_paths())),
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
        inspector_workspace: inspector_workspace_for(temp_dir),
        oauth_manager: RwLock::new(None),
        secret_store: RwLock::new(None),
        secret_store_readiness: RwLock::new(mcpmate::api::routes::unavailable_secret_store_readiness(
            "test_unavailable",
        )),
    })
}

async fn build_proxy_database_state(temp_dir: &TempDir) -> (Arc<AppState>, Arc<ProxyServer>) {
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

    let database = Database {
        pool: db_pool,
        path: temp_dir.path().join("mcpmate-test.db"),
    };
    let redb_cache =
        Arc::new(RedbCacheManager::new(temp_dir.path().join("capability.redb"), CacheConfig::default()).expect("redb"));

    let mut proxy = ProxyServer::new(Arc::new(Config::default()));
    proxy.redb_cache = redb_cache.clone();
    proxy.set_database(database).await.expect("proxy database");
    let proxy = Arc::new(proxy);

    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());

    let state = Arc::new(AppState {
        connection_pool: proxy.connection_pool.clone(),
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: Some(proxy.clone()),
        profile_merge_service: proxy.profile_service.clone(),
        database: proxy.database.clone(),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache,
        unified_query: None,
        client_service: proxy.client_config_service.clone(),
        inspector_calls,
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        inspector_workspace: inspector_workspace_for(temp_dir),
        oauth_manager: RwLock::new(None),
        secret_store: RwLock::new(None),
        secret_store_readiness: RwLock::new(mcpmate::api::routes::unavailable_secret_store_readiness(
            "test_unavailable",
        )),
    });

    (state, proxy)
}

fn inspector_workspace_for(temp_dir: &TempDir) -> Arc<InspectorWorkspace> {
    Arc::new(InspectorWorkspace::from_servers_dir(
        temp_dir.path().join("inspector").join("servers"),
    ))
}

struct TestProxySurface {
    proxy: Arc<ProxyServer>,
    previous_ports: RuntimePortConfig,
    restored: bool,
}

impl TestProxySurface {
    async fn start(proxy: Arc<ProxyServer>) -> Self {
        let previous_ports = get_runtime_port_config();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy port");
        let port = listener.local_addr().expect("proxy addr").port();
        drop(listener);

        init_port_config(previous_ports.api_port, port);
        let handle = proxy
            .start_unified(format!("127.0.0.1:{port}").parse().expect("proxy socket addr"))
            .await
            .expect("start proxy surface");
        handle.await.expect("proxy start task").expect("proxy start result");
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

        Self {
            proxy,
            previous_ports,
            restored: false,
        }
    }

    async fn shutdown(mut self) {
        self.proxy.initiate_shutdown().await.expect("proxy shutdown");
        init_port_config(self.previous_ports.api_port, self.previous_ports.mcp_port);
        self.restored = true;
    }
}

impl Drop for TestProxySurface {
    fn drop(&mut self) {
        if self.restored {
            return;
        }
        self.proxy.cancellation_token.cancel();
        init_port_config(self.previous_ports.api_port, self.previous_ports.mcp_port);
    }
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
        reply(request_id, {
            "contents": [{
                "uri": "test://hello",
                "mimeType": "text/plain",
                "text": "hello from resource"
            }]
        })
    elif method == "prompts/list":
        reply(request_id, {"prompts": [{
            "name": "hello_prompt",
            "description": "Returns a greeting prompt."
        }]})
    elif method == "prompts/get":
        reply(request_id, {
            "description": "Returns a greeting prompt.",
            "messages": [{
                "role": "user",
                "content": {"type": "text", "text": "hello from prompt"}
            }]
        })
    elif method == "resources/templates/list":
        reply(request_id, {"resourceTemplates": []})
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

fn has_tool_named(
    body: &Value,
    name: &str,
) -> bool {
    body.pointer("/data/tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| {
            tools
                .iter()
                .any(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
        })
}

async fn create_stdio_fixture_server(
    app: &Router,
    temp_dir: &TempDir,
) -> String {
    create_stdio_fixture_server_named(app, temp_dir, "inspector-fixture").await
}

async fn create_stdio_fixture_server_named(
    app: &Router,
    temp_dir: &TempDir,
    name: &str,
) -> String {
    let fixture = write_stdio_fixture(temp_dir);
    let python = which::which("python3").expect("python3 is required for stdio MCP fixture");
    let create_req = json_post_request(
        CREATE_SERVER_PATH,
        json!({
            "name": name,
            "server_type": "stdio",
            "command": python.to_string_lossy(),
            "args": [fixture.to_string_lossy()]
        }),
    );

    let create_body = read_json_response(app.clone().oneshot(create_req).await.unwrap()).await;
    assert_api_success(&create_body);
    data_str(&create_body, "/data/id").to_string()
}

async fn mark_unify_direct_exposure_eligible(
    state: &Arc<AppState>,
    server_id: &str,
) {
    let database = state.database.as_ref().expect("database");
    sqlx::query("UPDATE server_config SET unify_direct_exposure_eligible = 1 WHERE id = ?")
        .bind(server_id)
        .execute(&database.pool)
        .await
        .expect("mark direct exposure eligible");
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

async fn validation_session_count(state: &Arc<AppState>) -> usize {
    let pool = state.connection_pool.lock().await;
    pool.validation_sessions.len()
}

#[tokio::test]
#[serial_test::serial]
async fn inspector_create_server_is_immediately_usable_without_restart() {
    let temp_dir = TempDir::new().expect("temp dir");
    let fixture = write_stdio_fixture(&temp_dir);
    let python = which::which("python3").expect("python3 is required for stdio MCP fixture");
    let (state, proxy) = build_proxy_database_state(&temp_dir).await;
    let proxy_surface = TestProxySurface::start(proxy).await;

    let app = Router::new()
        .route(CREATE_SERVER_PATH, post(server::create_server))
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .route(TOOL_CALL_PATH, post(inspector::tool_call))
        .route(PROMPT_GET_PATH, post(inspector::prompt_get))
        .route(RESOURCE_LIST_PATH, get(inspector::resources_list))
        .route(RESOURCE_READ_PATH, get(inspector::resource_read))
        .route(SESSION_OPEN_PATH, post(inspector::session_open))
        .route(SESSION_CLOSE_PATH, post(inspector::session_close))
        .with_state(state.clone());

    let create_req = json_post_request(
        CREATE_SERVER_PATH,
        json!({
            "name": "inspector-fixture",
            "server_type": "stdio",
            "command": python.to_string_lossy(),
            "args": [fixture.to_string_lossy()]
        }),
    );

    let create_body = read_json_response(app.clone().oneshot(create_req).await.unwrap()).await;
    assert_api_success(&create_body);
    let server_id = data_str(&create_body, "/data/id").to_string();
    mark_unify_direct_exposure_eligible(&state, &server_id).await;
    let second_server_id = create_stdio_fixture_server_named(&app, &temp_dir, "inspector-fixture-two").await;
    mark_unify_direct_exposure_eligible(&state, &second_server_id).await;
    assert_eq!(
        data_str(&create_body, "/data/protocol_version"),
        protocol::CURRENT_VERSION
    );

    let tools_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=proxy&refresh=true"
    ));
    let tools_body = read_json_response(app.clone().oneshot(tools_req).await.unwrap()).await;
    assert_api_success(&tools_body);
    assert_eq!(data_str(&tools_body, "/data/meta/0/proxy_mode"), "hosted");
    assert_eq!(data_str(&tools_body, "/data/meta/0/proxy_scope"), "isolated");
    assert_eq!(data_u64(&tools_body, "/data/total"), 1);
    assert_eq!(
        data_str(&tools_body, "/data/evidence/operation/kind"),
        "capability_list"
    );
    assert_eq!(
        data_str(&tools_body, "/data/evidence/platform_rows/0/layer"),
        "platform"
    );
    assert_eq!(data_str(&tools_body, "/data/evidence/mcp_rows/0/layer"), "mcp");
    assert_eq!(data_str(&tools_body, "/data/evidence/events/0/layer"), "platform");
    assert_eq!(data_str(&tools_body, "/data/evidence/events/1/layer"), "mcp");
    let tool_name = data_str(&tools_body, "/data/tools/0/name");
    assert!(
        tool_name.ends_with("_echo"),
        "proxy Inspector should expose the upstream echo tool with a stable unique name, got {tool_name}"
    );

    let proxy_call_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                TOOL_CALL_PATH,
                json!({
                    "tool": tool_name,
                    "server_id": server_id,
                    "mode": "proxy",
                    "proxy_mode": "hosted",
                    "proxy_scope": "isolated",
                    "timeout_ms": 5000,
                    "arguments": { "message": "proxy-call" }
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&proxy_call_body);
    assert_eq!(
        data_str(&proxy_call_body, "/data/result/content/0/text"),
        "echo: proxy-call"
    );

    let prompt_get_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                PROMPT_GET_PATH,
                json!({
                    "name": "hello_prompt",
                    "server_id": server_id,
                    "mode": "proxy"
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&prompt_get_body);
    assert_eq!(
        data_str(&prompt_get_body, "/data/result/messages/0/content/text"),
        "hello from prompt"
    );
    assert_eq!(
        data_str(&prompt_get_body, "/data/evidence/operation/kind"),
        "prompt_get"
    );
    assert_eq!(
        data_str(&prompt_get_body, "/data/evidence/platform_rows/0/layer"),
        "platform"
    );
    assert_eq!(data_str(&prompt_get_body, "/data/evidence/mcp_rows/0/layer"), "mcp");

    let resources_req = get_request(format!(
        "{RESOURCE_LIST_PATH}?server_id={server_id}&mode=proxy&refresh=true"
    ));
    let resources_body = read_json_response(app.clone().oneshot(resources_req).await.unwrap()).await;
    assert_api_success(&resources_body);
    assert_eq!(data_u64(&resources_body, "/data/total"), 1);

    let resource_read_req = get_request(format!(
        "{RESOURCE_READ_PATH}?server_id={server_id}&mode=proxy&uri=test%3A%2F%2Fhello"
    ));
    let resource_read_body = read_json_response(app.clone().oneshot(resource_read_req).await.unwrap()).await;
    assert_eq!(
        data_str(&resource_read_body, "/data/result/contents/0/text"),
        "hello from resource"
    );
    assert_eq!(
        data_str(&resource_read_body, "/data/evidence/operation/kind"),
        "resource_read"
    );
    assert_eq!(
        data_str(&resource_read_body, "/data/evidence/platform_rows/0/layer"),
        "platform"
    );
    assert_eq!(data_str(&resource_read_body, "/data/evidence/mcp_rows/0/layer"), "mcp");

    let second_tools_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={second_server_id}&mode=proxy&refresh=true"
    ));
    let second_tools_body = read_json_response(app.clone().oneshot(second_tools_req).await.unwrap()).await;
    assert_api_success(&second_tools_body);
    let second_tool_name = data_str(&second_tools_body, "/data/tools/0/name");

    let unify_active_req = get_request(format!(
        "{TOOL_LIST_PATH}?mode=proxy&proxy_mode=unify&proxy_scope=active_catalog&refresh=true"
    ));
    let unify_active_body = read_json_response(app.clone().oneshot(unify_active_req).await.unwrap()).await;
    assert_api_success(&unify_active_body);
    assert_eq!(data_str(&unify_active_body, "/data/meta/0/proxy_mode"), "unify");
    assert_eq!(
        data_str(&unify_active_body, "/data/meta/0/proxy_scope"),
        "active_catalog"
    );
    assert!(
        has_tool_named(&unify_active_body, tool_name),
        "active-catalog Unify surface should expose direct fixture tool: {unify_active_body}"
    );
    assert!(
        has_tool_named(&unify_active_body, second_tool_name),
        "active-catalog Unify surface should include every eligible direct fixture tool: {unify_active_body}"
    );

    let unify_active_with_server_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=proxy&proxy_mode=unify&proxy_scope=active_catalog&refresh=true"
    ));
    let unify_active_with_server_body =
        read_json_response(app.clone().oneshot(unify_active_with_server_req).await.unwrap()).await;
    assert_api_success(&unify_active_with_server_body);
    assert!(
        has_tool_named(&unify_active_with_server_body, second_tool_name),
        "active-catalog Unify surface must not be narrowed by a caller-provided server_id: {unify_active_with_server_body}"
    );

    let active_session_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                SESSION_OPEN_PATH,
                json!({
                    "mode": "proxy",
                    "proxy_mode": "unify",
                    "proxy_scope": "active_catalog"
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&active_session_body);
    assert_eq!(data_str(&active_session_body, "/data/mode"), "proxy");
    assert_eq!(data_str(&active_session_body, "/data/target/mode"), "proxy");
    assert_eq!(data_str(&active_session_body, "/data/target/proxy_mode"), "unify");
    assert_eq!(
        data_str(&active_session_body, "/data/target/proxy_scope"),
        "active_catalog"
    );
    assert!(
        active_session_body.pointer("/data/server_id").is_none(),
        "active-catalog proxy sessions must not be bound to one server: {active_session_body}"
    );
    let active_session_id = data_str(&active_session_body, "/data/session_id").to_string();
    let active_session_list_req = get_request(format!(
        "{TOOL_LIST_PATH}?mode=proxy&session_id={active_session_id}&refresh=true"
    ));
    let active_session_list_body =
        read_json_response(app.clone().oneshot(active_session_list_req).await.unwrap()).await;
    assert_api_success(&active_session_list_body);
    assert!(
        has_tool_named(&active_session_list_body, second_tool_name),
        "proxy session list should reuse the active-catalog runtime surface: {active_session_list_body}"
    );
    let active_session_call_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                TOOL_CALL_PATH,
                json!({
                    "tool": tool_name,
                    "mode": "proxy",
                    "session_id": active_session_id,
                    "timeout_ms": 5000,
                    "arguments": { "message": "active-session-call" }
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&active_session_call_body);
    assert_eq!(
        data_str(&active_session_call_body, "/data/result/content/0/text"),
        "echo: active-session-call"
    );
    let mutated_session_list_req = get_request(format!(
        "{TOOL_LIST_PATH}?mode=proxy&session_id={active_session_id}&proxy_scope=isolated&refresh=true"
    ));
    let (mutated_status, mutated_body) =
        read_json_response_with_status(app.clone().oneshot(mutated_session_list_req).await.unwrap()).await;
    assert_eq!(mutated_status, axum::http::StatusCode::BAD_REQUEST);
    assert!(
        mutated_body
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("cannot be changed"),
        "expected immutable proxy session surface error: {mutated_body}"
    );
    close_inspector_session(&app, &active_session_id).await;

    let unify_isolated_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=proxy&proxy_mode=unify&proxy_scope=isolated&refresh=true"
    ));
    let unify_isolated_body = read_json_response(app.clone().oneshot(unify_isolated_req).await.unwrap()).await;
    assert_api_success(&unify_isolated_body);
    assert_eq!(data_str(&unify_isolated_body, "/data/meta/0/proxy_mode"), "unify");
    assert_eq!(data_str(&unify_isolated_body, "/data/meta/0/proxy_scope"), "isolated");
    assert!(
        has_tool_named(&unify_isolated_body, tool_name),
        "isolated Unify surface should expose direct fixture tool: {unify_isolated_body}"
    );

    mcpmate::core::capability::resolver::clear_cache().await;
    proxy_surface.shutdown().await;
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
    let baseline_validation_sessions = validation_session_count(&state).await;
    let session_id = open_native_session(&app, &server_id).await;
    assert_eq!(validation_session_count(&state).await, baseline_validation_sessions);

    let session_list_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=native&session_id={session_id}&refresh=true"
    ));
    let session_list_body = read_json_response(app.clone().oneshot(session_list_req).await.unwrap()).await;
    assert_api_success(&session_list_body);
    assert_eq!(data_u64(&session_list_body, "/data/total"), 1);
    assert_eq!(validation_session_count(&state).await, baseline_validation_sessions);

    let stateless_list_req = get_request(format!(
        "{TOOL_LIST_PATH}?server_id={server_id}&mode=native&refresh=true"
    ));
    let stateless_list_body = read_json_response(app.clone().oneshot(stateless_list_req).await.unwrap()).await;
    assert_api_success(&stateless_list_body);
    assert_eq!(data_u64(&stateless_list_body, "/data/total"), 1);
    assert_eq!(validation_session_count(&state).await, baseline_validation_sessions);

    call_native_echo(&app, &server_id, &session_id, "session-reuse").await;

    close_inspector_session(&app, &session_id).await;
    assert_eq!(validation_session_count(&state).await, baseline_validation_sessions);

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
#[serial_test::serial]
async fn inspector_native_scratch_record_runs_without_registry_server() {
    let temp_dir = TempDir::new().expect("temp dir");
    let fixture = write_stdio_fixture(&temp_dir);
    let python = which::which("python3").expect("python3 is required for stdio MCP fixture");
    let state = build_database_state(&temp_dir).await;

    let record = state
        .inspector_workspace
        .create_server_record(InspectorServerRecordInput {
            name: "Scratch Fixture".to_string(),
            config: MCPServerConfig {
                kind: ServerType::Stdio,
                command: Some(python.to_string_lossy().to_string()),
                args: Some(vec![fixture.to_string_lossy().to_string()]),
                url: None,
                env: None,
                headers: None,
            },
            provenance: InspectorServerProvenance::Scratch {
                origin: Some("inspector_smoke".to_string()),
            },
        })
        .expect("create scratch record");

    let app = Router::new()
        .route(TOOL_LIST_PATH, get(inspector::tools_list))
        .route(TOOL_CALL_PATH, post(inspector::tool_call))
        .route(CAPABILITY_PATCH_UPSERT_PATH, post(inspector::capability_patch_upsert))
        .route(COMPATIBILITY_SNAPSHOT_PATH, get(inspector::compatibility_snapshot))
        .route(PACKAGE_SAFETY_SNAPSHOT_PATH, get(inspector::package_safety_snapshot))
        .route(SESSION_OPEN_PATH, post(inspector::session_open))
        .route(SESSION_CLOSE_PATH, post(inspector::session_close))
        .with_state(state.clone());

    let list_req = get_request(format!(
        "{TOOL_LIST_PATH}?scratch_id={}&mode=native&refresh=true",
        record.id
    ));
    let list_body = read_json_response(app.clone().oneshot(list_req).await.unwrap()).await;
    assert_api_success(&list_body);
    assert_eq!(data_u64(&list_body, "/data/total"), 1);
    assert_eq!(data_str(&list_body, "/data/meta/0/scratch_id"), record.id);
    assert!(
        has_tool_named(&list_body, "echo"),
        "scratch native surface should expose fixture tool: {list_body}"
    );

    let patch_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                CAPABILITY_PATCH_UPSERT_PATH,
                json!({
                    "scratch_id": record.id,
                    "mode": "native",
                    "capability_kind": "tools",
                    "capability_key": "echo",
                    "patch": {
                        "name": "echo_refined",
                        "description": "Refined echo tool"
                    }
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&patch_body);
    assert_eq!(data_str(&patch_body, "/data/record/capability_key"), "echo");

    let patched_list_req = get_request(format!(
        "{TOOL_LIST_PATH}?scratch_id={}&mode=native&refresh=true",
        record.id
    ));
    let patched_list_body = read_json_response(app.clone().oneshot(patched_list_req).await.unwrap()).await;
    assert_api_success(&patched_list_body);
    assert!(
        has_tool_named(&patched_list_body, "echo_refined"),
        "patch overlay should rename tool in Inspector list output: {patched_list_body}"
    );
    assert_eq!(
        data_str(&patched_list_body, "/data/tools/0/description"),
        "Refined echo tool"
    );

    let snapshot_req = get_request(format!(
        "{COMPATIBILITY_SNAPSHOT_PATH}?scratch_id={}&mode=native",
        record.id
    ));
    let snapshot_body = read_json_response(app.clone().oneshot(snapshot_req).await.unwrap()).await;
    assert_api_success(&snapshot_body);
    assert_eq!(
        data_str(&snapshot_body, "/data/snapshot/target/source"),
        "scratch_workspace"
    );
    assert_eq!(data_str(&snapshot_body, "/data/snapshot/target/transport"), "stdio");
    assert_eq!(data_u64(&snapshot_body, "/data/snapshot/capabilities/counts/tools"), 1);

    let safety_req = get_request(format!(
        "{PACKAGE_SAFETY_SNAPSHOT_PATH}?scratch_id={}&mode=native",
        record.id
    ));
    let safety_body = read_json_response(app.clone().oneshot(safety_req).await.unwrap()).await;
    assert_api_success(&safety_body);
    assert_eq!(
        data_str(&safety_body, "/data/snapshot/target/source"),
        "scratch_workspace"
    );
    assert_eq!(
        data_str(&safety_body, "/data/snapshot/scanner/status"),
        "not_configured"
    );
    assert!(
        data_str(&safety_body, "/data/snapshot/inventory/fingerprint").starts_with("cmd:"),
        "stdio scratch inventory should expose a command fingerprint: {safety_body}"
    );

    let open_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                SESSION_OPEN_PATH,
                json!({
                    "scratch_id": record.id,
                    "mode": "native"
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&open_body);
    assert_eq!(data_str(&open_body, "/data/scratch_id"), record.id);
    assert_eq!(data_str(&open_body, "/data/target/mode"), "native");
    assert_eq!(data_str(&open_body, "/data/target/scratch_id"), record.id);
    assert!(
        open_body.pointer("/data/server_id").is_none(),
        "scratch sessions must not report a production server_id: {open_body}"
    );
    let session_id = data_str(&open_body, "/data/session_id").to_string();

    let call_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                TOOL_CALL_PATH,
                json!({
                    "tool": "echo",
                    "mode": "native",
                    "session_id": session_id,
                    "timeout_ms": 5000,
                    "arguments": { "message": "scratch-session" }
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&call_body);
    assert_eq!(
        data_str(&call_body, "/data/result/content/0/text"),
        "echo: scratch-session"
    );

    let patched_call_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                TOOL_CALL_PATH,
                json!({
                    "tool": "echo_refined",
                    "scratch_id": record.id,
                    "mode": "native",
                    "timeout_ms": 5000,
                    "arguments": { "message": "patched-name" }
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&patched_call_body);
    assert_eq!(
        data_str(&patched_call_body, "/data/result/content/0/text"),
        "echo: patched-name"
    );

    close_inspector_session(&app, &session_id).await;

    let registry_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM server_config")
        .fetch_one(&state.database.as_ref().expect("database").pool)
        .await
        .expect("count server registry");
    assert_eq!(registry_count.0, 0, "scratch record must not create registry rows");
}

#[tokio::test]
#[serial_test::serial]
async fn inspector_scratch_server_records_use_workspace_api() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_database_state(&temp_dir).await;

    let app = Router::new()
        .route(SCRATCH_SERVER_LIST_PATH, get(inspector::scratch_server_list))
        .route(SCRATCH_SERVER_CREATE_PATH, post(inspector::scratch_server_create))
        .route(SCRATCH_SERVER_DELETE_PATH, post(inspector::scratch_server_delete))
        .with_state(state.clone());

    let initial_list = read_json_response(
        app.clone()
            .oneshot(get_request(SCRATCH_SERVER_LIST_PATH.to_string()))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&initial_list);
    assert_eq!(data_u64(&initial_list, "/data/total"), 0);

    let create_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                SCRATCH_SERVER_CREATE_PATH,
                json!({
                    "name": "Scratch Fetch",
                    "origin": "smoke",
                    "config": {
                        "type": "stdio",
                        "command": "node",
                        "args": ["scratch-server.js"]
                    }
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&create_body);
    let record_id = data_str(&create_body, "/data/record/id").to_string();
    assert_eq!(data_str(&create_body, "/data/record/name"), "Scratch Fetch");
    assert_eq!(data_str(&create_body, "/data/record/provenance/kind"), "scratch");
    assert_eq!(data_str(&create_body, "/data/record/provenance/origin"), "smoke");

    let list_body = read_json_response(
        app.clone()
            .oneshot(get_request(SCRATCH_SERVER_LIST_PATH.to_string()))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&list_body);
    assert_eq!(data_u64(&list_body, "/data/total"), 1);
    assert_eq!(data_str(&list_body, "/data/records/0/id"), record_id);

    let delete_body = read_json_response(
        app.clone()
            .oneshot(json_post_request(
                SCRATCH_SERVER_DELETE_PATH,
                json!({
                    "record_id": record_id,
                }),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&delete_body);
    assert_eq!(
        delete_body.pointer("/data/deleted").and_then(Value::as_bool),
        Some(true)
    );

    let final_list = read_json_response(
        app.clone()
            .oneshot(get_request(SCRATCH_SERVER_LIST_PATH.to_string()))
            .await
            .unwrap(),
    )
    .await;
    assert_api_success(&final_list);
    assert_eq!(data_u64(&final_list, "/data/total"), 0);

    let registry_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM server_config")
        .fetch_one(&state.database.as_ref().expect("database").pool)
        .await
        .expect("count server registry");
    assert_eq!(registry_count.0, 0, "scratch API must not create registry rows");
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
async fn inspector_tool_call_evidence_unknown_call_returns_not_found() {
    let state = build_test_state();

    let app = Router::new()
        .route(TOOL_CALL_EVIDENCE_PATH, get(inspector::tool_call_evidence))
        .with_state(state);
    let req = get_request(format!("{TOOL_CALL_EVIDENCE_PATH}?call_id=no-such-call-id"));
    let (status, body) = read_json_response_with_status(app.oneshot(req).await.unwrap()).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        body.pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("no-such-call-id"),
        "expected missing call id in evidence error body: {body}"
    );
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
