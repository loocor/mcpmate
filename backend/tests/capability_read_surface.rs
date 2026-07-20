use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use axum::{
    Router,
    body::to_bytes,
    routing::{get, post},
};
use hyper::{Request, StatusCode};
use mcpmate::{
    api::{handlers::server as server_handlers, models::server::ServersImportConfig, routes::AppState},
    common::{constants::protocol, profile::ProfileType},
    config::{
        database::Database,
        initialization::run_initialization,
        models::{Profile, Server},
        profile as profile_config, server as server_config,
    },
    core::{
        events::{Event, EventBus, EventDrivenCapabilityManager, EventHandlers},
        foundation::load_server_config_strict,
        models::Config,
        pool::{CapSyncFlags, UpstreamConnectionPool},
        profile::ConfigApplicationStateManager,
        proxy::server::{
            ClientContext, ClientIdentitySource, ClientTransport, ManagedClientContextResolver, ProxyServer,
        },
    },
    inspector::{calls::InspectorCallRegistry, service as inspector_service, sessions::InspectorSessionManager},
    system::metrics::MetricsCollector,
};
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, DeclarationState, DerivedCapabilityCache, InventoryState, KindObservation,
    SnapshotState, SqliteCapabilityCatalog,
};
use rmcp::{
    ServerHandler, ServiceExt as _,
    model::RequestId,
    service::{RequestContext, RoleClient, RoleServer, RunningService},
};
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tempfile::TempDir;
use tokio::{
    io::duplex,
    sync::{Mutex, RwLock},
};
use tower::ServiceExt as _;

struct EnvVarGuard {
    key: &'static str,
}

impl EnvVarGuard {
    fn set(
        key: &'static str,
        value: &str,
    ) -> Self {
        // SAFETY: these process-wide import retry keys are isolated by serial_test.
        unsafe { std::env::set_var(key, value) };
        Self { key }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // SAFETY: paired with set under the same serial test scope.
        unsafe { std::env::remove_var(self.key) };
    }
}

#[derive(Clone, Copy, Debug)]
enum SurfaceKind {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

impl SurfaceKind {
    const ALL: [Self; 4] = [Self::Tools, Self::Prompts, Self::Resources, Self::ResourceTemplates];

    const fn label(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Prompts => "prompts",
            Self::Resources => "resources",
            Self::ResourceTemplates => "resource templates",
        }
    }

    const fn rest_path(self) -> &'static str {
        match self {
            Self::Tools => "/tools",
            Self::Prompts => "/prompts",
            Self::Resources => "/resources",
            Self::ResourceTemplates => "/resource-templates",
        }
    }

    const fn protocol_items_pointer(self) -> &'static str {
        match self {
            Self::Tools => "/tools",
            Self::Prompts => "/prompts",
            Self::Resources => "/resources",
            Self::ResourceTemplates => "/resourceTemplates",
        }
    }
}

#[derive(Clone)]
struct DownstreamContextServer;

impl ServerHandler for DownstreamContextServer {}

async fn open_database(path: PathBuf) -> Arc<Database> {
    let database_url = format!("sqlite://{}", path.display());
    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("parse test database URL")
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .expect("open test database");
    run_initialization(&pool).await.expect("initialize test database");
    mcpmate::core::capability::naming::initialize(pool.clone());
    mcpmate::core::capability::resolver::clear_cache().await;

    Arc::new(Database {
        pool,
        path,
        capability_cache: Arc::new(DerivedCapabilityCache::default()),
    })
}

fn build_proxy(database: Arc<Database>) -> ProxyServer {
    let config = Arc::new(Config::default());
    let mut proxy = ProxyServer::new(config.clone());
    proxy.connection_pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(config, Some(database.clone()))));
    proxy.database = Some(database);
    proxy
}

fn build_app_state(database: Arc<Database>) -> Arc<AppState> {
    let config = Arc::new(Config::default());
    let connection_pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(config, Some(database.clone()))));
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());

    Arc::new(AppState {
        connection_pool,
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database: Some(database),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
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

fn write_counted_stdio_fixture(temp_dir: &TempDir) -> PathBuf {
    let path = temp_dir.path().join("capability_read_fixture.py");
    let script = r#"
import json
import sys

counter_path = sys.argv[1]
label = sys.argv[2]
protocol_version = sys.argv[3]
mode = sys.argv[4] if len(sys.argv) > 4 else "normal"

def reply(request_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    request = json.loads(line)
    request_id = request.get("id")
    method = request.get("method")
    if request_id is None:
        continue
    if mode in (
        "count_methods",
        "method_not_found_templates_count_methods",
        "paginated_tools_method_not_found_templates_count_methods",
    ):
        with open(counter_path, "a", encoding="utf-8") as counter:
            counter.write(method + "\n")
            if mode == "paginated_tools_method_not_found_templates_count_methods" and method == "tools/list":
                cursor = (request.get("params") or {}).get("cursor", "<none>")
                counter.write("tools/list.cursor=" + cursor + "\n")
            counter.flush()
    if method == "initialize":
        if mode not in (
            "count_methods",
            "method_not_found_templates_count_methods",
            "paginated_tools_method_not_found_templates_count_methods",
        ):
            with open(counter_path, "a", encoding="utf-8") as counter:
                counter.write("start\n")
                counter.flush()
        reply(request_id, {
            "protocolVersion": protocol_version,
            "capabilities": {"tools": {}, "prompts": {}, "resources": {}},
            "serverInfo": {"name": label, "version": "1.0.0"}
        })
    elif method == "tools/list":
        if mode == "paginated_tools_method_not_found_templates_count_methods":
            cursor = (request.get("params") or {}).get("cursor")
            if cursor is None:
                reply(request_id, {
                    "tools": [{
                        "name": label + "_tool_page_one",
                        "description": "page one",
                        "inputSchema": {"type": "object"}
                    }],
                    "nextCursor": "page-2"
                })
            elif cursor == "page-2":
                reply(request_id, {"tools": [{
                    "name": label + "_tool_page_two",
                    "description": "page two",
                    "inputSchema": {"type": "object"}
                }]})
            else:
                raise RuntimeError("unexpected tools cursor: " + cursor)
        else:
            reply(request_id, {"tools": [{
                "name": label + "_tool",
                "description": "revision two",
                "inputSchema": {"type": "object"}
            }]})
    elif method == "prompts/list":
        reply(request_id, {"prompts": [{"name": label + "_prompt"}]})
    elif method == "resources/list":
        if mode == "fail_resources":
            sys.stdout.write(json.dumps({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32603, "message": "resource inventory failed"}
            }) + "\n")
            sys.stdout.flush()
        else:
            reply(request_id, {"resources": [{
                "uri": "fixture://" + label + "/item",
                "name": label + "_resource"
            }]})
    elif method == "resources/templates/list":
        if mode in (
            "method_not_found_templates",
            "method_not_found_templates_count_methods",
            "paginated_tools_method_not_found_templates_count_methods",
        ):
            sys.stdout.write(json.dumps({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32601, "message": "method not found"}
            }) + "\n")
            sys.stdout.flush()
        else:
            reply(request_id, {"resourceTemplates": [{
                "uriTemplate": "fixture://" + label + "/{item}",
                "name": label + "_template"
            }]})
    else:
        sys.stdout.write(json.dumps({
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {"code": -32601, "message": "method not found"}
        }) + "\n")
        sys.stdout.flush()
"#;
    std::fs::write(&path, script).expect("write counted stdio fixture");
    path
}

async fn insert_stdio_server(
    database: &Database,
    script: &Path,
    counter: &Path,
    server_id: &str,
    server_name: &str,
) {
    insert_stdio_server_with_mode(database, script, counter, server_id, server_name, "normal").await;
}

async fn insert_stdio_server_with_mode(
    database: &Database,
    script: &Path,
    counter: &Path,
    server_id: &str,
    server_name: &str,
    mode: &str,
) {
    let python = which::which("python3").expect("python3 is required for the stdio fixture");
    let mut server = Server::new_stdio(server_name.to_string(), Some(python.to_string_lossy().into_owned()));
    server.id = Some(server_id.to_string());
    let stored_id = server_config::upsert_server(&database.pool, &server)
        .await
        .expect("insert stdio server");
    assert_eq!(stored_id, server_id);
    server_config::upsert_server_args(
        &database.pool,
        server_id,
        &[
            script.to_string_lossy().into_owned(),
            counter.to_string_lossy().into_owned(),
            server_name.to_string(),
            protocol::CURRENT_VERSION.to_string(),
            mode.to_string(),
        ],
    )
    .await
    .expect("insert stdio server arguments");
}

#[derive(Debug, Default)]
struct CatalogEventCounts {
    commits: HashMap<String, usize>,
    changes: HashMap<String, usize>,
}

impl CatalogEventCounts {
    fn observe(
        &mut self,
        event: Event,
    ) {
        match event {
            Event::CapabilityCatalogCommitted { server_id, .. } => {
                *self.commits.entry(server_id).or_default() += 1;
            }
            Event::CapabilityCatalogChanged { server_id, .. } => {
                *self.changes.entry(server_id).or_default() += 1;
            }
            _ => {}
        }
    }

    fn transition_complete(
        &self,
        server_id: &str,
    ) -> bool {
        self.commits.get(server_id).copied().unwrap_or_default() >= 1
            && self.changes.get(server_id).copied().unwrap_or_default() >= 1
    }

    fn assert_exactly_one(
        &self,
        server_id: &str,
    ) {
        assert_eq!(
            self.commits.get(server_id).copied().unwrap_or_default(),
            1,
            "{server_id} must publish exactly one catalog revision event: {self:?}"
        );
        assert_eq!(
            self.changes.get(server_id).copied().unwrap_or_default(),
            1,
            "{server_id} must publish exactly one catalog change event: {self:?}"
        );
    }

    fn assert_no_events(
        &self,
        server_id: &str,
    ) {
        assert_eq!(self.commits.get(server_id), None, "unexpected revision event: {self:?}");
        assert_eq!(self.changes.get(server_id), None, "unexpected change event: {self:?}");
    }
}

async fn run_background_sync_and_collect_events(
    database: Arc<Database>,
    server_id: &str,
) -> CatalogEventCounts {
    let mut receiver = EventBus::global().subscribe_async();
    let (_, server_config) = load_server_config_strict(&database, server_id, None)
        .await
        .expect("load background server config");
    let config = Config {
        mcp_servers: HashMap::from([(server_id.to_string(), server_config)]),
        ..Default::default()
    };
    let mut pool = UpstreamConnectionPool::new(Arc::new(config), Some(database));
    let instance_id = pool
        .ensure_connected(server_id)
        .await
        .expect("connect background sync owner");
    let mut counts = CatalogEventCounts::default();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !counts.transition_complete(server_id) {
            counts.observe(receiver.recv().await.expect("receive background catalog event"));
        }
    })
    .await
    .unwrap_or_else(|_| panic!("background sync did not publish a complete catalog transition: {counts:?}"));
    tokio::time::sleep(Duration::from_millis(50)).await;
    while let Ok(event) = receiver.try_recv() {
        counts.observe(event);
    }
    pool.disconnect(server_id, &instance_id)
        .await
        .expect("disconnect background sync owner");
    counts
}

async fn call_rest_list(
    app: &Router,
    kind: SurfaceKind,
    server_id: &str,
) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("{}?id={server_id}", kind.rest_path()))
                .body(axum::body::Body::empty())
                .expect("build REST capability request"),
        )
        .await
        .expect("call REST capability route");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read REST capability response");
    let body_text = String::from_utf8_lossy(&bytes);
    assert_eq!(status, StatusCode::OK, "REST capability request failed: {body_text}");
    serde_json::from_slice(&bytes).expect("decode REST capability response")
}

async fn insert_inert_server(
    database: &Database,
    server_id: &str,
    server_name: &str,
) {
    let mut server = Server::new_stdio(server_name.to_string(), Some("must-not-start".to_string()));
    server.id = Some(server_id.to_string());
    let stored_id = server_config::upsert_server(&database.pool, &server)
        .await
        .expect("insert inert server");
    assert_eq!(stored_id, server_id);
}

async fn insert_active_profile(
    database: &Database,
    server_ids: &[&str],
) -> String {
    let mut profile = Profile::new("Capability Surface Profile".to_string(), ProfileType::Shared);
    profile.is_active = true;
    let profile_id = profile_config::upsert_profile(&database.pool, &profile)
        .await
        .expect("insert active profile");
    for server_id in server_ids {
        profile_config::add_server_to_profile(&database.pool, &profile_id, server_id, true)
            .await
            .expect("add server to active profile");
    }
    profile_id
}

fn initialize_result(server_name: &str) -> rmcp::model::InitializeResult {
    serde_json::from_value(json!({
        "protocolVersion": protocol::CURRENT_VERSION,
        "capabilities": {"tools": {}, "prompts": {}, "resources": {}},
        "serverInfo": {"name": server_name, "version": "1.0.0"}
    }))
    .expect("build initialize result")
}

fn protocol_items(
    label: &str
) -> (
    Vec<rmcp::model::Tool>,
    Vec<rmcp::model::Resource>,
    Vec<rmcp::model::Prompt>,
    Vec<rmcp::model::ResourceTemplate>,
) {
    let tools = vec![
        serde_json::from_value(json!({
            "name": format!("{label}_tool"),
            "description": "Capability surface fixture",
            "inputSchema": {"type": "object"}
        }))
        .expect("build tool"),
    ];
    let resources = vec![
        serde_json::from_value(json!({
            "uri": format!("fixture://{label}/item"),
            "name": format!("{label}_resource")
        }))
        .expect("build resource"),
    ];
    let prompts = vec![
        serde_json::from_value(json!({
            "name": format!("{label}_prompt")
        }))
        .expect("build prompt"),
    ];
    let templates = vec![
        serde_json::from_value(json!({
            "uriTemplate": format!("fixture://{label}/{{item}}"),
            "name": format!("{label}_template")
        }))
        .expect("build resource template"),
    ];
    (tools, resources, prompts, templates)
}

async fn commit_ready_catalog(
    database: &Database,
    server_id: &str,
    server_name: &str,
    with_items: bool,
) {
    let (tools, resources, prompts, templates) = if with_items {
        protocol_items(server_name)
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };
    server_config::capabilities::commit_protocol_items_for_kinds(
        &database.pool,
        server_id,
        server_name,
        Some(initialize_result(server_name)),
        tools,
        resources,
        prompts,
        templates,
        CapSyncFlags::ALL,
    )
    .await
    .expect("commit ready capability catalog");
    database.capability_cache.invalidate_server(server_id).await;
}

fn start_count(counter: &Path) -> usize {
    match std::fs::read_to_string(counter) {
        Ok(contents) => contents.lines().count(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => panic!("read {}: {error}", counter.display()),
    }
}

fn operation_count(
    counter: &Path,
    operation: &str,
) -> usize {
    std::fs::read_to_string(counter)
        .unwrap_or_default()
        .lines()
        .filter(|line| *line == operation)
        .count()
}

async fn downstream_request_context(
    client_id: &str,
    session_id: &str,
) -> (
    RequestContext<RoleServer>,
    RunningService<RoleClient, ()>,
    RunningService<RoleServer, DownstreamContextServer>,
) {
    let (server_transport, client_transport) = duplex(4096);
    let server_task = tokio::spawn(async move {
        DownstreamContextServer
            .serve(server_transport)
            .await
            .expect("serve downstream context peer")
    });
    let client_service = ().serve(client_transport).await.expect("connect downstream context client");
    let server_service = server_task.await.expect("join downstream context server");
    let mut context = RequestContext::new(
        RequestId::String("capability-surface-test".into()),
        server_service.peer().clone(),
    );
    let request = Request::builder()
        .uri(format!("/mcp?client_id={client_id}"))
        .header("mcp-session-id", session_id)
        .header(protocol::MCP_PROTOCOL_VERSION_HEADER, protocol::CURRENT_VERSION)
        .body(())
        .expect("build downstream request parts");
    context.extensions.insert(request.into_parts().0);
    (context, client_service, server_service)
}

async fn bind_client(
    proxy: &ProxyServer,
    client_id: &str,
    session_id: &str,
    profile_id: &str,
) {
    proxy
        .client_context_resolver
        .bind_session(
            session_id,
            &ClientContext {
                client_id: client_id.to_string(),
                session_id: Some(session_id.to_string()),
                profile_id: Some(profile_id.to_string()),
                config_mode: Some("hosted".to_string()),
                unify_workspace: None,
                surface_fingerprint: None,
                transport: ClientTransport::StreamableHttp,
                source: ClientIdentitySource::SessionBinding,
                observed_client_info: None,
            },
        )
        .await
        .expect("bind managed client session");
}

async fn call_managed_mcp_list(
    proxy: &ProxyServer,
    kind: SurfaceKind,
    context: RequestContext<RoleServer>,
) -> Value {
    match kind {
        SurfaceKind::Tools => serde_json::to_value(
            ServerHandler::list_tools(proxy, None, context)
                .await
                .expect("list managed tools"),
        )
        .expect("serialize tool list"),
        SurfaceKind::Prompts => serde_json::to_value(
            ServerHandler::list_prompts(proxy, None, context)
                .await
                .expect("list managed prompts"),
        )
        .expect("serialize prompt list"),
        SurfaceKind::Resources => serde_json::to_value(
            ServerHandler::list_resources(proxy, None, context)
                .await
                .expect("list managed resources"),
        )
        .expect("serialize resource list"),
        SurfaceKind::ResourceTemplates => serde_json::to_value(
            ServerHandler::list_resource_templates(proxy, None, context)
                .await
                .expect("list managed resource templates"),
        )
        .expect("serialize resource template list"),
    }
}

async fn read_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    serde_json::from_slice(&body).expect("decode JSON response")
}

#[test]
fn public_capability_surfaces_do_not_bypass_the_read_service() {
    for path in [
        "src/core/proxy/server/tools.rs",
        "src/core/proxy/server/prompts.rs",
        "src/core/proxy/server/resources.rs",
        "src/mcper/builtin/broker.rs",
        "src/api/handlers/server/tools.rs",
        "src/api/handlers/server/prompts.rs",
        "src/api/handlers/server/resources.rs",
        "src/api/handlers/server/capability.rs",
    ] {
        let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .unwrap_or_else(|error| panic!("read {path}: {error}"));
        assert!(
            !source.contains("runtime::list("),
            "{path} bypasses CapabilityReadService"
        );
        assert!(
            source.contains("CapabilityReadService::from_runtime"),
            "{path} does not call the unique CapabilityReadService"
        );
    }

    for path in [
        "src/api/handlers/server/tools.rs",
        "src/api/handlers/server/prompts.rs",
        "src/api/handlers/server/resources.rs",
        "src/api/handlers/server/capability.rs",
        "src/core/capability/query.rs",
    ] {
        let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .unwrap_or_else(|error| panic!("read {path}: {error}"));
        assert!(
            !source.contains("CAPABILITY_VALIDATION_SESSION"),
            "{path} reuses a shared validation session for an ordinary API read"
        );
        assert!(
            !source.contains("CapabilityService::new"),
            "{path} routes an ordinary API read through the legacy compatibility facade"
        );
    }

    let common =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/handlers/server/common.rs"))
            .expect("read server handler common source");
    assert!(
        !common.contains("fn check_capability_or_error("),
        "REST capability handlers retain a catalog pre-read outside CapabilityReadService"
    );

    for path in [
        "src/api/handlers/server/tools.rs",
        "src/api/handlers/server/prompts.rs",
        "src/api/handlers/server/resources.rs",
    ] {
        let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .unwrap_or_else(|error| panic!("read {path}: {error}"));
        assert!(
            !source.contains("unwrap_or(serde_json::Value::Null)"),
            "{path} silently converts a serialization failure into a null capability"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn background_sync_commit_invalidates_the_current_raw_snapshot() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("background-success.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("background-success-starts.log");
    let server_id = "server-background-success";
    let server_name = "background_fixture";
    let unrelated_id = "server-background-unrelated";
    insert_stdio_server(&database, &script, &counter, server_id, server_name).await;
    insert_inert_server(&database, unrelated_id, "unrelated_fixture").await;

    let (mut tools, resources, prompts, templates) = protocol_items(server_name);
    tools[0].description = Some("revision one".into());
    server_config::capabilities::commit_protocol_items_for_kinds(
        &database.pool,
        server_id,
        server_name,
        Some(initialize_result(server_name)),
        tools,
        resources,
        prompts,
        templates,
        CapSyncFlags::ALL,
    )
    .await
    .expect("commit revision one");
    database.capability_cache.invalidate_server(server_id).await;
    commit_ready_catalog(&database, unrelated_id, "unrelated_fixture", true).await;

    let app = Router::new()
        .route("/tools", get(server_handlers::server_tools))
        .with_state(build_app_state(database.clone()));
    let first = call_rest_list(&app, SurfaceKind::Tools, server_id).await;
    assert_eq!(first.pointer("/data/meta/source"), Some(&json!("sqlite_catalog")));
    assert!(first.to_string().contains("revision one"));
    let warmed = call_rest_list(&app, SurfaceKind::Tools, server_id).await;
    assert_eq!(warmed.pointer("/data/meta/source"), Some(&json!("memory_cache")));

    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());
    let before = catalog
        .load_snapshot(server_id)
        .await
        .expect("load revision one")
        .expect("revision one exists");
    let unrelated_before = catalog
        .load_snapshot(unrelated_id)
        .await
        .expect("load unrelated snapshot")
        .expect("unrelated snapshot exists");

    let events = run_background_sync_and_collect_events(database.clone(), server_id).await;
    events.assert_exactly_one(server_id);
    events.assert_no_events(unrelated_id);

    let after = call_rest_list(&app, SurfaceKind::Tools, server_id).await;
    assert_eq!(after.pointer("/data/meta/source"), Some(&json!("sqlite_catalog")));
    assert!(
        after.to_string().contains("revision two"),
        "new revision missing: {after}"
    );
    assert!(
        !after.to_string().contains("revision one"),
        "old LRU payload survived: {after}"
    );
    let committed = catalog
        .load_snapshot(server_id)
        .await
        .expect("load revision two")
        .expect("revision two exists");
    assert_ne!(committed.revision, before.revision);
    let unrelated_after = catalog
        .load_snapshot(unrelated_id)
        .await
        .expect("reload unrelated snapshot")
        .expect("unrelated snapshot remains");
    assert_eq!(unrelated_after.revision, unrelated_before.revision);
}

#[tokio::test]
#[serial_test::serial]
async fn background_sync_failure_hides_the_previous_ready_snapshot() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("background-failure.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("background-failure-starts.log");
    let server_id = "server-background-failure";
    let server_name = "background_failure_fixture";
    let unrelated_id = "server-failure-unrelated";
    insert_stdio_server_with_mode(&database, &script, &counter, server_id, server_name, "fail_resources").await;
    insert_inert_server(&database, unrelated_id, "unrelated_failure_fixture").await;

    let (mut tools, resources, prompts, templates) = protocol_items(server_name);
    tools[0].description = Some("revision one".into());
    server_config::capabilities::commit_protocol_items_for_kinds(
        &database.pool,
        server_id,
        server_name,
        Some(initialize_result(server_name)),
        tools,
        resources,
        prompts,
        templates,
        CapSyncFlags::ALL,
    )
    .await
    .expect("commit ready baseline");
    database.capability_cache.invalidate_server(server_id).await;
    commit_ready_catalog(&database, unrelated_id, "unrelated_failure_fixture", true).await;

    let app = Router::new()
        .route("/tools", get(server_handlers::server_tools))
        .with_state(build_app_state(database.clone()));
    let warmed = call_rest_list(&app, SurfaceKind::Tools, server_id).await;
    assert!(warmed.to_string().contains("revision one"));

    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());
    let unrelated_before = catalog
        .load_snapshot(unrelated_id)
        .await
        .expect("load unrelated snapshot")
        .expect("unrelated snapshot exists");
    let events = run_background_sync_and_collect_events(database.clone(), server_id).await;
    events.assert_exactly_one(server_id);
    events.assert_no_events(unrelated_id);

    let failed = catalog
        .load_snapshot(server_id)
        .await
        .expect("load failed snapshot")
        .expect("failed snapshot exists");
    assert_eq!(failed.state, SnapshotState::Unavailable);
    let resources_state = failed
        .kind_states
        .iter()
        .find(|state| state.kind == CapabilityKind::Resources)
        .expect("resources state exists");
    assert_eq!(resources_state.inventory, InventoryState::Failed);
    let reason = failed
        .last_error
        .as_deref()
        .expect("terminal failure reason is persisted");
    assert!(reason.contains(server_id), "reason omits server identity: {reason}");
    assert!(reason.contains("resources"), "reason omits kind scope: {reason}");
    assert!(reason.contains("instance="), "reason omits owner instance: {reason}");
    assert!(
        reason.contains("generation=None"),
        "reason fabricates or omits generation: {reason}"
    );

    let after_failure = call_rest_list(&app, SurfaceKind::Tools, server_id).await;
    assert!(
        !after_failure.to_string().contains("revision one"),
        "terminal failure left the old Ready payload visible: {after_failure}"
    );
    let unrelated_after = catalog
        .load_snapshot(unrelated_id)
        .await
        .expect("reload unrelated snapshot")
        .expect("unrelated snapshot remains");
    assert_eq!(unrelated_after.revision, unrelated_before.revision);
    assert_eq!(unrelated_after.state, SnapshotState::Ready);
}

#[tokio::test]
#[serial_test::serial]
async fn resource_template_method_not_found_has_one_state_across_sync_paths() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("template-method-not-found.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let active_counter = temp_dir.path().join("active-template-starts.log");
    let background_counter = temp_dir.path().join("background-template-starts.log");
    let active_id = "server-active-template-unsupported";
    let background_id = "server-background-template-unsupported";
    insert_stdio_server_with_mode(
        &database,
        &script,
        &active_counter,
        active_id,
        "active_template_fixture",
        "method_not_found_templates",
    )
    .await;
    insert_stdio_server_with_mode(
        &database,
        &script,
        &background_counter,
        background_id,
        "background_template_fixture",
        "method_not_found_templates",
    )
    .await;

    let app = Router::new()
        .route("/resource-templates", get(server_handlers::server_resource_templates))
        .with_state(build_app_state(database.clone()));
    let active_result = call_rest_list(&app, SurfaceKind::ResourceTemplates, active_id).await;
    assert_eq!(active_result.pointer("/data/items"), Some(&json!([])));
    let background_events = run_background_sync_and_collect_events(database.clone(), background_id).await;
    background_events.assert_exactly_one(background_id);

    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());
    let active = catalog
        .load_snapshot(active_id)
        .await
        .expect("load active observation")
        .expect("active observation exists");
    let background = catalog
        .load_snapshot(background_id)
        .await
        .expect("load background observation")
        .expect("background observation exists");
    let state = |snapshot: &mcpmate_capability_store::CatalogSnapshot| {
        snapshot
            .kind_states
            .iter()
            .find(|state| state.kind == CapabilityKind::ResourceTemplates)
            .cloned()
            .expect("resource template state exists")
    };
    let expected = KindObservation::new(
        CapabilityKind::ResourceTemplates,
        DeclarationState::Unsupported,
        InventoryState::Complete,
    );
    assert_eq!(state(&active), expected);
    assert_eq!(state(&background), expected);
}

#[tokio::test]
#[serial_test::serial]
async fn validation_sync_template_method_not_found_is_unsupported_complete() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("validation-template-unsupported.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("validation-template-unsupported.log");
    let server_id = "server-validation-template-unsupported";
    let server_name = "validation_template_fixture";
    insert_stdio_server_with_mode(
        &database,
        &script,
        &counter,
        server_id,
        server_name,
        "paginated_tools_method_not_found_templates_count_methods",
    )
    .await;
    let (_, server_config) = load_server_config_strict(&database, server_id, None)
        .await
        .expect("load validation config");
    let pool = Mutex::new(UpstreamConnectionPool::new(
        Arc::new(Config {
            mcp_servers: HashMap::from([(server_id.to_string(), server_config)]),
            ..Default::default()
        }),
        Some(database.clone()),
    ));

    server_config::capabilities::sync_via_connection_pool(
        &pool,
        &database.pool,
        database.capability_cache.as_ref(),
        server_id,
        server_name,
        5,
    )
    .await
    .expect("MethodNotFound must be a successful unsupported observation");

    let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(server_id)
        .await
        .expect("load validation snapshot")
        .expect("validation snapshot exists");
    let templates = snapshot
        .kind_states
        .iter()
        .find(|state| state.kind == CapabilityKind::ResourceTemplates)
        .expect("resource templates state exists");
    assert_eq!(templates.declaration, DeclarationState::Unsupported);
    assert_eq!(templates.inventory, InventoryState::Complete);
    let mut tool_keys = snapshot
        .records
        .iter()
        .filter(|record| record.kind() == CapabilityKind::Tools)
        .map(|record| record.upstream_key.as_str())
        .collect::<Vec<_>>();
    tool_keys.sort_unstable();
    assert_eq!(
        tool_keys,
        vec![
            "validation_template_fixture_tool_page_one",
            "validation_template_fixture_tool_page_two",
        ],
        "validation discovery must merge every tools page"
    );
    assert_eq!(
        operation_count(&counter, "tools/list"),
        2,
        "validation discovery must fetch every tools page after initialize"
    );
    assert_eq!(
        operation_count(&counter, "tools/list.cursor=<none>"),
        1,
        "first tools page must omit the cursor"
    );
    assert_eq!(
        operation_count(&counter, "tools/list.cursor=page-2"),
        1,
        "second tools page must forward nextCursor"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn validation_sync_terminal_failure_records_scoped_evidence() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("validation-terminal-failure.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("validation-terminal-failure.log");
    let server_id = "server-validation-terminal-failure";
    let server_name = "validation_failure_fixture";
    insert_stdio_server_with_mode(&database, &script, &counter, server_id, server_name, "fail_resources").await;
    let (_, server_config) = load_server_config_strict(&database, server_id, None)
        .await
        .expect("load validation config");
    let pool = Mutex::new(UpstreamConnectionPool::new(
        Arc::new(Config {
            mcp_servers: HashMap::from([(server_id.to_string(), server_config)]),
            ..Default::default()
        }),
        Some(database.clone()),
    ));
    let mut receiver = EventBus::global().subscribe_async();

    let error = server_config::capabilities::sync_via_connection_pool(
        &pool,
        &database.pool,
        database.capability_cache.as_ref(),
        server_id,
        server_name,
        5,
    )
    .await
    .expect_err("resource inventory failure must remain visible to CRUD callers");
    assert!(
        error.to_string().contains("resources/list"),
        "typed operation was lost: {error:#}"
    );

    let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(server_id)
        .await
        .expect("load validation failure")
        .expect("validation failure evidence exists");
    assert_eq!(snapshot.state, SnapshotState::Unavailable);
    assert_eq!(snapshot.server_name, server_name);
    let expected_identity = format!("instance=Some(\"validation-{server_name}-api\") generation=None");
    let reason = snapshot.last_error.as_deref().expect("failure reason exists");
    assert!(reason.contains(&format!(
        "server_id={server_id} server_name={server_name} kinds=[resources]"
    )));
    assert!(reason.contains(&expected_identity), "owner evidence mismatch: {reason}");
    assert!(
        reason.contains("resource inventory failed"),
        "upstream cause missing: {reason}"
    );

    let mut events = CatalogEventCounts::default();
    tokio::time::timeout(Duration::from_secs(1), async {
        while !events.transition_complete(server_id) {
            events.observe(receiver.recv().await.expect("receive validation failure event"));
        }
    })
    .await
    .expect("validation failure must publish a catalog transition");
    events.assert_exactly_one(server_id);
}

async fn event_manager_with_connected_fixture(
    database: Arc<Database>,
    server_id: &str,
) -> (
    Arc<Mutex<UpstreamConnectionPool>>,
    Arc<EventDrivenCapabilityManager>,
    String,
) {
    let (_, server_config) = load_server_config_strict(&database, server_id, None)
        .await
        .expect("load event-driven config");
    let mut pool = UpstreamConnectionPool::new(
        Arc::new(Config {
            mcp_servers: HashMap::from([(server_id.to_string(), server_config)]),
            ..Default::default()
        }),
        Some(database.clone()),
    );
    let connection = pool
        .get_or_create_validation_instance(server_id, "event-fixture", Duration::from_secs(60))
        .await
        .expect("create event fixture connection")
        .expect("event fixture connection exists")
        .clone();
    let instance_id = connection.id.clone();
    pool.connections
        .entry(server_id.to_string())
        .or_default()
        .insert(instance_id.clone(), connection);
    let pool = Arc::new(Mutex::new(pool));
    let manager = Arc::new(EventDrivenCapabilityManager::new(
        Arc::new(database.pool.clone()),
        database.capability_cache.clone(),
        pool.clone(),
    ));
    (pool, manager, instance_id)
}

#[tokio::test]
#[serial_test::serial]
async fn event_driven_validation_terminal_failure_records_scoped_evidence() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("event-terminal-failure.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let server_id = "server-event-terminal-failure";
    let server_name = "event_failure_fixture";
    insert_stdio_server_with_mode(
        &database,
        &script,
        &temp_dir.path().join("event-terminal-failure.log"),
        server_id,
        server_name,
        "fail_resources",
    )
    .await;
    let (_pool, manager, instance_id) = event_manager_with_connected_fixture(database.clone(), server_id).await;

    manager
        .sync_single_server(server_id)
        .await
        .expect_err("event-driven inventory failure must remain visible");

    let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(server_id)
        .await
        .expect("load event-driven failure")
        .expect("event-driven failure evidence exists");
    assert_eq!(snapshot.state, SnapshotState::Unavailable);
    assert_eq!(snapshot.server_name, server_name);
    let reason = snapshot.last_error.as_deref().expect("event failure reason exists");
    assert!(reason.contains(&format!(
        "server_id={server_id} server_name={server_name} kinds=[resources]"
    )));
    assert!(
        reason.contains(&format!("instance=Some(\"{instance_id}\") generation=None")),
        "event owner evidence mismatch: {reason}"
    );
    assert!(reason.contains("resource inventory failed"));
}

#[tokio::test]
#[serial_test::serial]
async fn event_driven_validation_template_method_not_found_is_unsupported_complete() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("event-template-unsupported.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let server_id = "server-event-template-unsupported";
    let server_name = "event_template_fixture";
    insert_stdio_server_with_mode(
        &database,
        &script,
        &temp_dir.path().join("event-template-unsupported.log"),
        server_id,
        server_name,
        "method_not_found_templates",
    )
    .await;
    let (_pool, manager, _instance_id) = event_manager_with_connected_fixture(database.clone(), server_id).await;

    manager
        .sync_single_server(server_id)
        .await
        .expect("event-driven MethodNotFound must commit unsupported observation");

    let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(server_id)
        .await
        .expect("load event template snapshot")
        .expect("event template snapshot exists");
    let state = snapshot
        .kind_states
        .iter()
        .find(|state| state.kind == CapabilityKind::ResourceTemplates)
        .expect("event template state exists");
    assert_eq!(state.declaration, DeclarationState::Unsupported);
    assert_eq!(state.inventory, InventoryState::Complete);
}

#[tokio::test]
#[serial_test::serial]
async fn import_retry_records_terminal_inventory_failure_once() {
    let _retry_guard = EnvVarGuard::set("MCPMATE_IMPORT_CAP_SYNC_RETRIES", "1");
    let _backoff_guard = EnvVarGuard::set("MCPMATE_IMPORT_CAP_SYNC_BACKOFF_MS", "1");
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("import-terminal-failure.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("import-terminal-failure.log");
    let server_name = "import_failure_fixture";
    let python = which::which("python3").expect("python3 is required for the import fixture");
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
        Arc::new(Config::default()),
        Some(database.clone()),
    )));
    let mut receiver = EventBus::global().subscribe_async();
    let outcome = server_config::import_batch(
        &database.pool,
        database.capability_cache.clone(),
        &pool,
        HashMap::from([(
            server_name.to_string(),
            ServersImportConfig {
                kind: "stdio".to_string(),
                command: Some(python.to_string_lossy().into_owned()),
                args: Some(vec![
                    script.to_string_lossy().into_owned(),
                    counter.to_string_lossy().into_owned(),
                    server_name.to_string(),
                    protocol::CURRENT_VERSION.to_string(),
                    "fail_resources".to_string(),
                ]),
                url: None,
                env: None,
                headers: None,
                source: None,
                meta: None,
            },
        )]),
        server_config::ImportOptions::dashboard_import(false, None),
    )
    .await
    .expect("schedule imported capability sync");
    assert!(outcome.scheduled);
    let server = server_config::get_server(&database.pool, server_name)
        .await
        .expect("load imported server")
        .expect("imported server exists");
    let server_id = server.id.expect("imported server has stable id");
    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());

    let snapshot = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(snapshot) = catalog.load_snapshot(&server_id).await.expect("poll import catalog") {
                break snapshot;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("terminal import failure must become durable evidence");
    assert_eq!(snapshot.state, SnapshotState::Unavailable);
    assert_eq!(
        snapshot.revision, 1,
        "intermediate retry failures must not commit evidence"
    );
    let reason = snapshot.last_error.as_deref().expect("import failure reason exists");
    assert!(reason.contains(&format!(
        "server_id={server_id} server_name={server_name} kinds=[resources]"
    )));
    assert!(reason.contains("generation=None"));
    assert!(reason.contains("resource inventory failed"));

    let mut events = CatalogEventCounts::default();
    tokio::time::timeout(Duration::from_secs(1), async {
        while !events.transition_complete(&server_id) {
            events.observe(receiver.recv().await.expect("receive import terminal event"));
        }
    })
    .await
    .expect("terminal import failure must publish one transition");
    tokio::time::sleep(Duration::from_millis(50)).await;
    while let Ok(event) = receiver.try_recv() {
        events.observe(event);
    }
    events.assert_exactly_one(&server_id);
}

#[tokio::test]
#[serial_test::serial]
async fn background_sync_transitions_publish_once_and_isolate_servers() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("background-events.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let success_id = "server-event-success";
    let failure_id = "server-event-failure";
    let unrelated_id = "server-event-unrelated";
    insert_stdio_server(
        &database,
        &script,
        &temp_dir.path().join("event-success-starts.log"),
        success_id,
        "event_success",
    )
    .await;
    insert_stdio_server_with_mode(
        &database,
        &script,
        &temp_dir.path().join("event-failure-starts.log"),
        failure_id,
        "event_failure",
        "fail_resources",
    )
    .await;
    insert_inert_server(&database, unrelated_id, "event_unrelated").await;
    for (server_id, server_name) in [
        (success_id, "event_success"),
        (failure_id, "event_failure"),
        (unrelated_id, "event_unrelated"),
    ] {
        commit_ready_catalog(&database, server_id, server_name, true).await;
    }
    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());
    let unrelated_revision = catalog
        .load_snapshot(unrelated_id)
        .await
        .expect("load unrelated snapshot")
        .expect("unrelated snapshot exists")
        .revision;

    let success_events = run_background_sync_and_collect_events(database.clone(), success_id).await;
    success_events.assert_exactly_one(success_id);
    success_events.assert_no_events(failure_id);
    success_events.assert_no_events(unrelated_id);
    let failure_events = run_background_sync_and_collect_events(database.clone(), failure_id).await;
    failure_events.assert_exactly_one(failure_id);
    failure_events.assert_no_events(success_id);
    failure_events.assert_no_events(unrelated_id);
    assert_eq!(
        catalog
            .load_snapshot(unrelated_id)
            .await
            .expect("reload unrelated snapshot")
            .expect("unrelated snapshot remains")
            .revision,
        unrelated_revision
    );
}

#[tokio::test]
#[serial_test::serial]
async fn successful_production_startup_has_one_capability_sync_owner() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("startup-owner.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let operations = temp_dir.path().join("startup-owner-operations.log");
    let server_id = "server-startup-owner";
    let server_name = "startup_owner_fixture";
    insert_stdio_server_with_mode(&database, &script, &operations, server_id, server_name, "count_methods").await;
    commit_ready_catalog(&database, server_id, server_name, true).await;
    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());
    let before_revision = catalog
        .load_snapshot(server_id)
        .await
        .expect("load startup baseline")
        .expect("startup baseline exists")
        .revision;

    let (_, server_config) = load_server_config_strict(&database, server_id, None)
        .await
        .expect("load startup server config");
    let config = Config {
        mcp_servers: HashMap::from([(server_id.to_string(), server_config)]),
        ..Default::default()
    };
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
        Arc::new(config),
        Some(database.clone()),
    )));
    let manager = Arc::new(EventDrivenCapabilityManager::new(
        Arc::new(database.pool.clone()),
        database.capability_cache.clone(),
        pool.clone(),
    ));
    let mut handlers = EventHandlers::new();
    handlers.set_connection_pool(pool.clone());
    handlers.set_event_capability_manager(manager);
    handlers.init().expect("install production event handlers");
    let mut receiver = EventBus::global().subscribe_async();

    let instance_id = pool
        .lock()
        .await
        .ensure_connected(server_id)
        .await
        .expect("connect startup owner");
    let mut events = CatalogEventCounts::default();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !events.transition_complete(server_id) {
            events.observe(receiver.recv().await.expect("receive startup catalog event"));
        }
    })
    .await
    .expect("startup must publish a catalog transition");
    tokio::time::sleep(Duration::from_millis(750)).await;
    while let Ok(event) = receiver.try_recv() {
        events.observe(event);
    }

    let after_revision = catalog
        .load_snapshot(server_id)
        .await
        .expect("load startup observation")
        .expect("startup observation exists")
        .revision;
    assert_eq!(
        after_revision,
        before_revision + 1,
        "startup committed more than one revision"
    );
    events.assert_exactly_one(server_id);
    for operation in [
        "initialize",
        "tools/list",
        "prompts/list",
        "resources/list",
        "resources/templates/list",
    ] {
        assert_eq!(
            operation_count(&operations, operation),
            1,
            "startup requested {operation} more than once"
        );
    }
    pool.lock()
        .await
        .disconnect(server_id, &instance_id)
        .await
        .expect("disconnect startup owner");
}

#[tokio::test]
#[serial_test::serial]
async fn missing_and_invalidated_catalog_recover_through_each_mcp_list_surface() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("mcp-recovery.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let target_counter = temp_dir.path().join("target-starts.log");
    let unrelated_counter = temp_dir.path().join("unrelated-starts.log");
    insert_stdio_server(&database, &script, &target_counter, "server-target", "target_fixture").await;
    insert_stdio_server(
        &database,
        &script,
        &unrelated_counter,
        "server-unrelated",
        "unrelated_fixture",
    )
    .await;
    let profile_id = insert_active_profile(&database, &["server-target", "server-unrelated"]).await;
    commit_ready_catalog(&database, "server-unrelated", "unrelated_fixture", false).await;

    let proxy = build_proxy(database.clone());
    let client_id = "capability-surface-client";
    let session_id = "capability-surface-session";
    bind_client(&proxy, client_id, session_id, &profile_id).await;
    let (context, client_service_guard, server_service_guard) = downstream_request_context(client_id, session_id).await;

    for kind in SurfaceKind::ALL {
        let starts_before = start_count(&target_counter);
        let first = call_managed_mcp_list(&proxy, kind, context.clone()).await;
        assert!(
            first
                .pointer(kind.protocol_items_pointer())
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty()),
            "{} missing-catalog recovery returned no protocol items: {first}",
            kind.label()
        );
        assert_eq!(
            start_count(&target_counter),
            starts_before + 1,
            "{} missing catalog should start only the target upstream once",
            kind.label()
        );
        assert_eq!(
            start_count(&unrelated_counter),
            0,
            "{} target recovery must not start an unrelated enabled server",
            kind.label()
        );

        SqliteCapabilityCatalog::new(database.pool.clone())
            .invalidate_server("server-target", "public surface invalidation test")
            .await
            .expect("invalidate target catalog");
        database.capability_cache.invalidate_server("server-target").await;

        let recovered = call_managed_mcp_list(&proxy, kind, context.clone()).await;
        assert_eq!(
            start_count(&target_counter),
            starts_before + 2,
            "{} invalidated catalog should start only the target upstream once",
            kind.label()
        );
        assert_eq!(
            start_count(&unrelated_counter),
            0,
            "{} invalidated recovery must leave the unrelated server stopped",
            kind.label()
        );
        assert_eq!(
            first,
            recovered,
            "{} recovery changed the protocol payload",
            kind.label()
        );
    }

    drop((client_service_guard, server_service_guard));
}

#[tokio::test]
#[serial_test::serial]
async fn ready_sqlite_catalog_survives_restart_through_each_mcp_list_surface_without_starting_upstream() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database_path = temp_dir.path().join("mcp-restart.db");
    let first_database = open_database(database_path.clone()).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("restart-starts.log");
    insert_stdio_server(&first_database, &script, &counter, "server-restart", "restart_fixture").await;
    let profile_id = insert_active_profile(&first_database, &["server-restart"]).await;
    commit_ready_catalog(&first_database, "server-restart", "restart_fixture", true).await;
    first_database.pool.close().await;

    let restarted_database = open_database(database_path).await;
    let proxy = build_proxy(restarted_database);
    let client_id = "restart-client";
    let session_id = "restart-session";
    bind_client(&proxy, client_id, session_id, &profile_id).await;
    let (context, client_service_guard, server_service_guard) = downstream_request_context(client_id, session_id).await;

    for kind in SurfaceKind::ALL {
        let payload = call_managed_mcp_list(&proxy, kind, context.clone()).await;
        assert!(
            payload
                .pointer(kind.protocol_items_pointer())
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty()),
            "{} restart read returned no protocol items: {payload}",
            kind.label()
        );
    }
    assert_eq!(
        start_count(&counter),
        0,
        "Ready SQLite restart reads must not start upstream"
    );

    drop((client_service_guard, server_service_guard));
}

#[tokio::test]
#[serial_test::serial]
async fn invalidated_full_catalog_does_not_resurrect_an_unselected_prompt_after_tool_recovery() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("mcp-invalidated-full.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("invalidated-full-starts.log");
    let server_id = "server-invalidated-full";
    let server_name = "current_fixture";
    insert_stdio_server(&database, &script, &counter, server_id, server_name).await;
    let profile_id = insert_active_profile(&database, &[server_id]).await;

    let (tools, resources, mut prompts, templates) = protocol_items(server_name);
    prompts[0].description = Some("Stale invalidated prompt".to_string());
    server_config::capabilities::commit_protocol_items_for_kinds(
        &database.pool,
        server_id,
        server_name,
        Some(initialize_result(server_name)),
        tools,
        resources,
        prompts,
        templates,
        CapSyncFlags::ALL,
    )
    .await
    .expect("commit full stale capability catalog");
    database.capability_cache.invalidate_server(server_id).await;
    let ready = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(server_id)
        .await
        .expect("load full ready snapshot")
        .expect("full ready snapshot exists");
    assert_eq!(ready.state, SnapshotState::Ready);
    assert!(
        ready
            .records
            .iter()
            .any(|record| record.kind() == CapabilityKind::Tools)
    );
    assert!(
        ready
            .records
            .iter()
            .any(|record| record.kind() == CapabilityKind::Prompts)
    );

    let proxy = build_proxy(database.clone());
    let client_id = "invalidated-full-client";
    let session_id = "invalidated-full-session";
    bind_client(&proxy, client_id, session_id, &profile_id).await;
    let (context, client_service_guard, server_service_guard) = downstream_request_context(client_id, session_id).await;

    let stale_prompt = call_managed_mcp_list(&proxy, SurfaceKind::Prompts, context.clone()).await;
    assert!(
        stale_prompt.to_string().contains("Stale invalidated prompt"),
        "full Ready fixture did not expose the stale prompt: {stale_prompt}"
    );
    assert_eq!(start_count(&counter), 0, "Ready catalog read started upstream");

    SqliteCapabilityCatalog::new(database.pool.clone())
        .invalidate_server(server_id, "full catalog recovery regression")
        .await
        .expect("invalidate full catalog");
    database.capability_cache.invalidate_server(server_id).await;

    let recovered_tools = call_managed_mcp_list(&proxy, SurfaceKind::Tools, context.clone()).await;
    assert!(
        recovered_tools.to_string().contains("current_fixture_tool"),
        "tool recovery did not return the current live payload: {recovered_tools}"
    );
    assert_eq!(
        start_count(&counter),
        1,
        "tool recovery did not start upstream exactly once"
    );

    let recovered_prompts = call_managed_mcp_list(&proxy, SurfaceKind::Prompts, context.clone()).await;
    assert_eq!(
        start_count(&counter),
        2,
        "prompt read reused the invalidated Complete state instead of starting upstream"
    );
    assert!(
        recovered_prompts.to_string().contains("current_fixture_prompt"),
        "prompt recovery did not return the current live payload: {recovered_prompts}"
    );
    assert!(
        !recovered_prompts.to_string().contains("Stale invalidated prompt"),
        "prompt recovery resurrected the invalidated payload: {recovered_prompts}"
    );

    drop((client_service_guard, server_service_guard));
}

#[tokio::test]
#[serial_test::serial]
async fn supported_empty_rest_lists_preserve_sqlite_then_memory_metadata_for_all_capability_kinds() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("rest-empty.db")).await;
    let fixtures = [
        (SurfaceKind::Tools, "server-empty-tools", "empty_tools"),
        (SurfaceKind::Prompts, "server-empty-prompts", "empty_prompts"),
        (SurfaceKind::Resources, "server-empty-resources", "empty_resources"),
        (
            SurfaceKind::ResourceTemplates,
            "server-empty-resource-templates",
            "empty_resource_templates",
        ),
    ];
    for (_, server_id, server_name) in fixtures {
        insert_inert_server(&database, server_id, server_name).await;
        commit_ready_catalog(&database, server_id, server_name, false).await;
    }

    let app = Router::new()
        .route("/tools", get(server_handlers::server_tools))
        .route("/prompts", get(server_handlers::server_prompts))
        .route("/resources", get(server_handlers::server_resources))
        .route("/resource-templates", get(server_handlers::server_resource_templates))
        .with_state(build_app_state(database));

    for (kind, server_id, _) in fixtures {
        for expected_source in ["sqlite_catalog", "memory_cache"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("{}?id={server_id}", kind.rest_path()))
                        .body(axum::body::Body::empty())
                        .expect("build REST capability request"),
                )
                .await
                .expect("call REST capability route");
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{} REST request failed",
                kind.label()
            );
            let body = read_json(response).await;
            assert_eq!(
                body.pointer("/data/items"),
                Some(&json!([])),
                "{} must remain empty",
                kind.label()
            );
            assert_eq!(
                body.pointer("/data/state"),
                Some(&json!("ok")),
                "{} must remain successful",
                kind.label()
            );
            assert_eq!(
                body.pointer("/data/meta/cache_hit"),
                Some(&json!(true)),
                "{} must preserve cache_hit",
                kind.label()
            );
            assert_eq!(
                body.pointer("/data/meta/source"),
                Some(&json!(expected_source)),
                "{} returned the wrong cache source: {body}",
                kind.label()
            );
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn isolated_restart_reset_parity_preserves_catalog_and_target_only_recovery() {
    let temp_dir = TempDir::new().expect("create isolated UAT directory");
    let database_path = temp_dir.path().join("restart-reset-parity.db");
    let target_counter = temp_dir.path().join("restart-reset-target-starts.log");
    let unrelated_counter = temp_dir.path().join("restart-reset-unrelated-starts.log");
    let script = write_counted_stdio_fixture(&temp_dir);
    let target_id = "server-restart-reset-target";
    let target_name = "restart_reset_target";
    let unrelated_id = "server-restart-reset-unrelated";
    let unrelated_name = "restart_reset_unrelated";

    let first_database = open_database(database_path.clone()).await;
    insert_stdio_server(&first_database, &script, &target_counter, target_id, target_name).await;
    insert_stdio_server(
        &first_database,
        &script,
        &unrelated_counter,
        unrelated_id,
        unrelated_name,
    )
    .await;
    commit_ready_catalog(&first_database, unrelated_id, unrelated_name, true).await;
    let first_app = Router::new()
        .route("/tools", get(server_handlers::server_tools))
        .route("/prompts", get(server_handlers::server_prompts))
        .route("/resources", get(server_handlers::server_resources))
        .route("/resource-templates", get(server_handlers::server_resource_templates))
        .route("/cache/reset", post(server_handlers::server_cache_reset))
        .with_state(build_app_state(first_database.clone()));

    for (kind_index, kind) in SurfaceKind::ALL.into_iter().enumerate() {
        let live = call_rest_list(&first_app, kind, target_id).await;
        assert_eq!(
            live.pointer("/data/meta/source"),
            Some(&json!("live")),
            "{} initial discovery did not report live source: {live}",
            kind.label()
        );
        assert_eq!(
            live.pointer("/data/meta/cache_hit"),
            Some(&json!(false)),
            "{} live discovery was incorrectly reported as a cache hit",
            kind.label()
        );
        assert!(
            live.pointer("/data/items")
                .and_then(Value::as_array)
                .is_some_and(|items| items.len() == 1),
            "{} live discovery returned the wrong payload: {live}",
            kind.label()
        );
        let memory = call_rest_list(&first_app, kind, target_id).await;
        assert_eq!(
            memory.pointer("/data/meta/source"),
            Some(&json!("memory_cache")),
            "{} immediate second read did not use the process-local LRU: {memory}",
            kind.label()
        );
        assert_eq!(
            start_count(&target_counter),
            kind_index + 1,
            "{} LRU read unexpectedly restarted upstream",
            kind.label()
        );
    }
    assert_eq!(start_count(&target_counter), SurfaceKind::ALL.len());
    assert_eq!(start_count(&unrelated_counter), 0);

    let catalog = SqliteCapabilityCatalog::new(first_database.pool.clone());
    let live_snapshot = catalog
        .load_snapshot(target_id)
        .await
        .expect("load live snapshot")
        .expect("live snapshot exists");
    assert_eq!(live_snapshot.state, SnapshotState::Ready);
    assert_eq!(live_snapshot.revision, SurfaceKind::ALL.len() as i64);
    assert_eq!(live_snapshot.kind_states.len(), SurfaceKind::ALL.len());
    assert!(live_snapshot.kind_states.iter().all(|state| {
        state.declaration == mcpmate_capability_store::DeclarationState::Supported
            && state.inventory == InventoryState::Complete
    }));
    assert_eq!(live_snapshot.records.len(), SurfaceKind::ALL.len());
    let snapshot_revision: i64 =
        sqlx::query_scalar("SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?")
            .bind(target_id)
            .fetch_one(&first_database.pool)
            .await
            .expect("load snapshot revision");
    let kind_revisions: Vec<i64> =
        sqlx::query_scalar("SELECT catalog_revision FROM capability_kind_states WHERE server_id = ? ORDER BY position")
            .bind(target_id)
            .fetch_all(&first_database.pool)
            .await
            .expect("load kind revisions");
    let record_revisions: Vec<i64> =
        sqlx::query_scalar("SELECT catalog_revision FROM capability_records WHERE server_id = ? ORDER BY position")
            .bind(target_id)
            .fetch_all(&first_database.pool)
            .await
            .expect("load record revisions");
    assert_eq!(snapshot_revision, live_snapshot.revision);
    assert_eq!(kind_revisions, vec![snapshot_revision; SurfaceKind::ALL.len()]);
    assert_eq!(record_revisions, vec![snapshot_revision; SurfaceKind::ALL.len()]);
    for table in [
        "server_tools",
        "server_prompts",
        "server_resources",
        "server_resource_templates",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE server_id = ?"))
            .bind(target_id)
            .fetch_one(&first_database.pool)
            .await
            .unwrap_or_else(|error| panic!("load {table} shadow index: {error}"));
        assert_eq!(count, 1, "{table} shadow index is out of sync with the live catalog");
    }

    drop(first_app);
    first_database.pool.close().await;
    drop(first_database);

    let restarted_database = open_database(database_path).await;
    let restarted_app = Router::new()
        .route("/tools", get(server_handlers::server_tools))
        .route("/prompts", get(server_handlers::server_prompts))
        .route("/resources", get(server_handlers::server_resources))
        .route("/resource-templates", get(server_handlers::server_resource_templates))
        .route("/cache/reset", post(server_handlers::server_cache_reset))
        .with_state(build_app_state(restarted_database.clone()));
    for kind in SurfaceKind::ALL {
        let restarted = call_rest_list(&restarted_app, kind, target_id).await;
        assert_eq!(
            restarted.pointer("/data/meta/source"),
            Some(&json!("sqlite_catalog")),
            "{} restart read returned the wrong source: {restarted}",
            kind.label()
        );
        assert!(
            restarted
                .pointer("/data/items")
                .and_then(Value::as_array)
                .is_some_and(|items| items.len() == 1),
            "{} restart read lost protocol payload: {restarted}",
            kind.label()
        );
        let restarted_memory = call_rest_list(&restarted_app, kind, target_id).await;
        assert_eq!(
            restarted_memory.pointer("/data/meta/source"),
            Some(&json!("memory_cache")),
            "{} repeated restart read did not use the process-local LRU: {restarted_memory}",
            kind.label()
        );
    }
    assert_eq!(start_count(&target_counter), SurfaceKind::ALL.len());
    assert_eq!(start_count(&unrelated_counter), 0);

    let before_reset = SqliteCapabilityCatalog::new(restarted_database.pool.clone())
        .load_snapshot(target_id)
        .await
        .expect("load pre-reset snapshot")
        .expect("pre-reset snapshot exists");
    let reset_response = restarted_app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/cache/reset")
                .body(axum::body::Body::empty())
                .expect("build cache reset request"),
        )
        .await
        .expect("call cache reset route");
    assert_eq!(reset_response.status(), StatusCode::OK);
    let reset_body = read_json(reset_response).await;
    assert_eq!(reset_body.pointer("/data/success"), Some(&json!(true)));

    let catalog = SqliteCapabilityCatalog::new(restarted_database.pool.clone());
    let invalidated = catalog
        .load_snapshot(target_id)
        .await
        .expect("load invalidated target")
        .expect("invalidated target exists");
    assert_eq!(invalidated.state, SnapshotState::Invalidated);
    assert_eq!(invalidated.revision, before_reset.revision + 1);
    let unrelated_after_reset = catalog
        .load_snapshot(unrelated_id)
        .await
        .expect("load reset unrelated server")
        .expect("reset unrelated snapshot exists");
    assert_eq!(unrelated_after_reset.state, SnapshotState::Invalidated);

    let recovered = call_rest_list(&restarted_app, SurfaceKind::Tools, target_id).await;
    assert_eq!(recovered.pointer("/data/meta/source"), Some(&json!("live")));
    assert!(
        recovered
            .pointer("/data/items")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() == 1)
    );
    assert_eq!(start_count(&target_counter), SurfaceKind::ALL.len() + 1);
    assert_eq!(start_count(&unrelated_counter), 0);

    let recovered_snapshot = catalog
        .load_snapshot(target_id)
        .await
        .expect("load recovered target")
        .expect("recovered target exists");
    assert_eq!(recovered_snapshot.state, SnapshotState::Ready);
    assert_eq!(recovered_snapshot.revision, invalidated.revision + 1);
    // Scoped Tools-only recovery from an Invalidated baseline retains the prior
    // prompt/resource/template records (marked Unknown below) instead of wiping them, so the
    // shadow index and Profile associations for those kinds survive the reconcile.
    assert_eq!(recovered_snapshot.records.len(), SurfaceKind::ALL.len());
    assert_eq!(
        recovered_snapshot
            .records
            .iter()
            .filter(|record| record.kind() == CapabilityKind::Tools)
            .count(),
        1
    );
    assert_eq!(
        recovered_snapshot
            .kind_states
            .iter()
            .find(|state| state.kind == CapabilityKind::Tools)
            .map(|state| state.inventory),
        Some(InventoryState::Complete)
    );
    assert!(
        recovered_snapshot
            .kind_states
            .iter()
            .all(|state| { state.kind == CapabilityKind::Tools || state.inventory == InventoryState::Unknown })
    );
    let unrelated_final = catalog
        .load_snapshot(unrelated_id)
        .await
        .expect("reload unrelated server")
        .expect("unrelated snapshot remains");
    assert_eq!(unrelated_final.revision, unrelated_after_reset.revision);
    assert_eq!(unrelated_final.state, SnapshotState::Invalidated);

    let recovered_revision: i64 =
        sqlx::query_scalar("SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?")
            .bind(target_id)
            .fetch_one(&restarted_database.pool)
            .await
            .expect("load recovered revision");
    let distinct_kind_revisions: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT catalog_revision) FROM capability_kind_states WHERE server_id = ?")
            .bind(target_id)
            .fetch_one(&restarted_database.pool)
            .await
            .expect("load recovered kind revision count");
    let distinct_record_revisions: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT catalog_revision) FROM capability_records WHERE server_id = ?")
            .bind(target_id)
            .fetch_one(&restarted_database.pool)
            .await
            .expect("load recovered record revision count");
    assert_eq!(recovered_revision, recovered_snapshot.revision);
    assert_eq!(distinct_kind_revisions, 1);
    assert_eq!(distinct_record_revisions, 1);
    let mut recovered_shadow_counts = Vec::new();
    for table in [
        "server_tools",
        "server_prompts",
        "server_resources",
        "server_resource_templates",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE server_id = ?"))
            .bind(target_id)
            .fetch_one(&restarted_database.pool)
            .await
            .unwrap_or_else(|error| panic!("load recovered {table} index: {error}"));
        recovered_shadow_counts.push((table, count));
    }
    // Retained history keeps the prompt/resource/template shadow rows in place even though
    // only Tools was rediscovered this round; they are not resurrected as Ready inventory
    // (see the `Unknown` assertion above), but their index rows must survive the reconcile.
    assert_eq!(
        recovered_shadow_counts,
        vec![
            ("server_tools", 1),
            ("server_prompts", 1),
            ("server_resources", 1),
            ("server_resource_templates", 1),
        ]
    );
}
