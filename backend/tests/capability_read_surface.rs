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
        server as server_config,
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
    SnapshotState, SqliteCapabilityCatalog, SqliteSurfaceStore, SurfaceManifest, SurfaceManifestEntryInput,
    SurfacePublication,
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
    database_support::prepare_config(&pool).await;
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
    build_app_state_with_pool(database, connection_pool)
}

fn build_app_state_with_pool(
    database: Arc<Database>,
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
) -> Arc<AppState> {
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

async fn build_capability_refresh_app_state(
    database: Arc<Database>,
    server_id: &str,
) -> Arc<AppState> {
    let (_, server_config) = load_server_config_strict(&database, server_id, None)
        .await
        .expect("load capability refresh server config");
    let config = Arc::new(Config {
        mcp_servers: HashMap::from([(server_id.to_string(), server_config)]),
        ..Default::default()
    });
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());

    Arc::new(AppState {
        connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(config, Some(database.clone())))),
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database: Some(database),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
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
import time

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
        "undeclared_count_methods",
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
            "undeclared_count_methods",
            "method_not_found_templates_count_methods",
            "paginated_tools_method_not_found_templates_count_methods",
        ):
            with open(counter_path, "a", encoding="utf-8") as counter:
                counter.write("start\n")
                counter.flush()
        capabilities = {} if mode == "undeclared_count_methods" else {
            "tools": {}, "prompts": {}, "resources": {}
        }
        reply(request_id, {
            "protocolVersion": protocol_version,
            "capabilities": capabilities,
            "serverInfo": {"name": label, "version": "1.0.0"}
        })
    elif method == "tools/list":
        if mode == "slow_tools":
            time.sleep(0.25)
        elif mode == "batch_slow_tools":
            time.sleep(2.0)
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
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
        Arc::new(config),
        Some(database),
    )));
    let selection = mcpmate::core::capability::ConnectionSelection {
        server_id: server_id.to_string(),
        affinity_key: mcpmate::core::capability::AffinityKey::Default,
    };
    let instance_id = UpstreamConnectionPool::ensure_connected_coordinated(&pool, &selection)
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
    pool.lock()
        .await
        .disconnect(server_id, &instance_id)
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
    let profile_id = database_support::insert_profile(&database.pool, &profile).await;
    for server_id in server_ids {
        database_support::insert_profile_server_relationship(&database.pool, &profile_id, server_id, true).await;
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
                source: ClientIdentitySource::ManagedQuery,
                observed_client_info: None,
            },
        )
        .await
        .expect("bind managed client session");
}

async fn publish_current_surface(
    database: &Database,
    consumer_id: &str,
    server_ids: &[&str],
) {
    sqlx::query(
        "INSERT INTO client (id, name, identifier, config_mode, approval_status) VALUES (?, ?, ?, 'hosted', 'approved')",
    )
        .bind(consumer_id)
        .bind(consumer_id)
        .bind(consumer_id)
        .execute(&database.pool)
        .await
        .expect("insert managed Consumer");
    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());
    let mut entries = Vec::new();
    for server_id in server_ids {
        let snapshot = catalog
            .load_snapshot(server_id)
            .await
            .expect("load current catalog snapshot")
            .expect("current catalog snapshot exists");
        entries.extend(snapshot.records.into_iter().map(|record| {
            let kind = record.kind();
            SurfaceManifestEntryInput::new(record.ref_id, record.capability_id, kind, record.external_key)
        }));
    }
    let manifest = SurfaceManifest::compile(consumer_id, entries).expect("compile current Surface manifest");
    let store = SqliteSurfaceStore::new(database.pool.clone());
    let mut transaction = database.pool.begin().await.expect("begin Surface publication");
    store
        .insert_manifest_in_transaction(&mut transaction, &manifest)
        .await
        .expect("insert current Surface manifest");
    store
        .publish_and_bind_in_transaction(
            &mut transaction,
            &SurfacePublication::new(
                format!("publication-{consumer_id}"),
                consumer_id,
                manifest.manifest_id,
                None,
                "test_fixture",
                "test",
                None,
            ),
            None,
        )
        .await
        .expect("publish current Surface");
    transaction.commit().await.expect("commit current Surface publication");
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
fn public_capability_surfaces_use_their_authoritative_readers() {
    for path in [
        "src/core/proxy/server/tools.rs",
        "src/core/proxy/server/prompts.rs",
        "src/core/proxy/server/resources.rs",
    ] {
        let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .unwrap_or_else(|error| panic!("read {path}: {error}"));
        assert!(
            source.contains("load_active_surface"),
            "{path} does not list from the active Surface publication"
        );
        assert!(
            !source.contains("CapabilityReadService::from_runtime"),
            "{path} recomputes a managed Consumer surface at request time"
        );
    }

    for path in ["src/mcper/builtin/broker.rs", "src/api/handlers/server/capability.rs"] {
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
    ] {
        let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .unwrap_or_else(|error| panic!("read {path}: {error}"));
        assert!(
            !source.contains("runtime::list("),
            "{path} bypasses CapabilityReadService"
        );
        assert!(
            source.contains("list_server_capability"),
            "{path} does not route through the shared capability list reader"
        );
        assert!(
            !source.contains("CapabilityReadService::from_runtime"),
            "{path} should not construct CapabilityReadService directly"
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
    assert_eq!(after.pointer("/data/meta/source"), Some(&json!("memory_cache")));
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
    assert_eq!(failed.state, SnapshotState::Ready);
    let resources_state = failed
        .kind_states
        .iter()
        .find(|state| state.kind == CapabilityKind::Resources)
        .expect("resources state exists");
    assert_eq!(resources_state.inventory, InventoryState::Failed);
    let reason = resources_state
        .error
        .as_deref()
        .expect("scoped failure reason is persisted");
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
    let state = build_capability_refresh_app_state(database.clone(), server_id).await;
    let app = Router::new()
        .route("/", post(server_handlers::refresh_server_capabilities))
        .with_state(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(json!({"id": server_id}).to_string()))
                .expect("build validation refresh request"),
        )
        .await
        .expect("call validation refresh");
    assert_eq!(response.status(), StatusCode::OK);

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
async fn management_discovery_skips_capability_kinds_not_declared_at_initialize() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("undeclared-capabilities.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let operations = temp_dir.path().join("undeclared-capabilities.log");
    let server_id = "server-undeclared-capabilities";
    let server_name = "undeclared_capabilities";
    insert_stdio_server_with_mode(
        &database,
        &script,
        &operations,
        server_id,
        server_name,
        "undeclared_count_methods",
    )
    .await;
    let state = build_capability_refresh_app_state(database.clone(), server_id).await;
    let app = Router::new()
        .route("/", post(server_handlers::refresh_server_capabilities))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(json!({"id": server_id}).to_string()))
                .expect("build validation refresh request"),
        )
        .await
        .expect("call validation refresh");
    assert_eq!(response.status(), StatusCode::OK);
    for operation in [
        "tools/list",
        "prompts/list",
        "resources/list",
        "resources/templates/list",
    ] {
        assert_eq!(
            operation_count(&operations, operation),
            0,
            "undeclared capability must not be requested: {operation}"
        );
    }
    let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(server_id)
        .await
        .expect("load capability snapshot")
        .expect("capability snapshot exists");
    assert!(snapshot.kind_states.iter().all(|state| {
        state.declaration == DeclarationState::Unsupported && state.inventory == InventoryState::Complete
    }));
}

#[tokio::test]
#[serial_test::serial]
async fn server_capability_refresh_commits_one_complete_catalog_observation() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("server-capability-refresh.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let operations = temp_dir.path().join("server-capability-refresh.log");
    let server_id = "server-capability-refresh";
    let server_name = "capability_refresh_fixture";
    insert_stdio_server_with_mode(&database, &script, &operations, server_id, server_name, "count_methods").await;
    let state = build_capability_refresh_app_state(database.clone(), server_id).await;
    let app = Router::new().merge(mcpmate::api::routes::server::routes(state));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers/capabilities/refresh")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_vec(&json!({ "id": server_id })).expect("encode refresh request"),
                ))
                .expect("build capability refresh request"),
        )
        .await
        .expect("call capability refresh route");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read capability refresh response");
    assert_eq!(
        status,
        StatusCode::OK,
        "capability refresh failed: {}",
        String::from_utf8_lossy(&bytes)
    );
    let body: Value = serde_json::from_slice(&bytes).expect("decode capability refresh response");

    assert_eq!(body.pointer("/data/server_id"), Some(&json!(server_id)));
    assert_eq!(body.pointer("/data/catalog_revision"), Some(&json!(1)));
    assert_eq!(body.pointer("/data/catalog_changed"), Some(&json!(true)));
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
            "server refresh must execute {operation} exactly once"
        );
    }

    let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(server_id)
        .await
        .expect("load refreshed catalog")
        .expect("refreshed catalog exists");
    assert_eq!(snapshot.revision, 1);
    assert_eq!(snapshot.records.len(), 4);
    assert_eq!(snapshot.kind_states.len(), 4);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers/capabilities/refresh")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_vec(&json!({ "id": server_id })).expect("encode second refresh request"),
                ))
                .expect("build second capability refresh request"),
        )
        .await
        .expect("call capability refresh route a second time");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read second capability refresh response");
    assert_eq!(
        status,
        StatusCode::OK,
        "second capability refresh failed: {}",
        String::from_utf8_lossy(&bytes)
    );
    let body: Value = serde_json::from_slice(&bytes).expect("decode second capability refresh response");
    assert_eq!(body.pointer("/data/catalog_revision"), Some(&json!(1)));
    assert_eq!(body.pointer("/data/catalog_changed"), Some(&json!(false)));
    assert_eq!(
        operation_count(&operations, "initialize"),
        1,
        "production owner must remain initialized across refreshes"
    );
    for operation in [
        "tools/list",
        "prompts/list",
        "resources/list",
        "resources/templates/list",
    ] {
        assert_eq!(
            operation_count(&operations, operation),
            2,
            "each server refresh must execute {operation} exactly once"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn validation_sync_kind_failure_records_scoped_evidence() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("validation-terminal-failure.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("validation-terminal-failure.log");
    let server_id = "server-validation-terminal-failure";
    let server_name = "validation_failure_fixture";
    insert_stdio_server_with_mode(&database, &script, &counter, server_id, server_name, "fail_resources").await;
    let mut receiver = EventBus::global().subscribe_async();

    let state = build_capability_refresh_app_state(database.clone(), server_id).await;
    let app = Router::new().merge(mcpmate::api::routes::server::routes(state));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers/capabilities/refresh")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(json!({"id": server_id}).to_string()))
                .expect("build failing validation refresh request"),
        )
        .await
        .expect("call failing validation refresh");
    assert!(
        response.status().is_server_error(),
        "inventory failure must remain visible to refresh callers"
    );

    let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(server_id)
        .await
        .expect("load validation failure")
        .expect("validation failure evidence exists");
    assert_eq!(snapshot.state, SnapshotState::Ready);
    assert_eq!(snapshot.server_name, server_name);
    let resources = snapshot
        .kind_states
        .iter()
        .find(|state| state.kind == CapabilityKind::Resources)
        .expect("resource failure state exists");
    assert_eq!(resources.inventory, InventoryState::Failed);
    let reason = resources.error.as_deref().expect("resource failure reason exists");
    assert!(reason.contains(&format!(
        "server_id={server_id} server_name={server_name} kinds=[resources]"
    )));
    assert!(
        reason.contains("instance=Some(\"UPSV") && reason.contains("generation=None"),
        "owner evidence mismatch: {reason}"
    );
    assert!(
        reason.contains("resource inventory failed"),
        "upstream cause missing: {reason}"
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/mcp/servers/capabilities/lists?id={server_id}&refresh=auto"))
                .body(axum::body::Body::empty())
                .expect("build batch list request"),
        )
        .await
        .expect("call batch list route");
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read batch list response"),
    )
    .expect("decode batch list response");
    assert_eq!(body.pointer("/data/resources/state"), Some(&json!("failed")));
    assert_eq!(body.pointer("/data/resources/items"), Some(&json!([])));
    assert!(
        body.pointer("/data/resources/degraded_reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("resource inventory failed"))
    );
    assert_eq!(body.pointer("/data/tools/state"), Some(&json!("ok")));
    assert_eq!(
        body.pointer("/data/tools/items/0/name"),
        Some(&json!("validation_failure_fixture_tool"))
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
async fn import_kind_failure_records_one_scoped_observation() {
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
        database.clone(),
        Some(&pool),
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
        server_config::ImportOptions::dashboard_import(false),
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
    assert_eq!(snapshot.state, SnapshotState::Ready);
    assert_eq!(
        snapshot.revision, 1,
        "one import discovery must commit one catalog observation"
    );
    let resources = snapshot
        .kind_states
        .iter()
        .find(|state| state.kind == CapabilityKind::Resources)
        .expect("resource failure state exists");
    assert_eq!(resources.inventory, InventoryState::Failed);
    let reason = resources
        .error
        .as_deref()
        .expect("import resource failure reason exists");
    assert!(reason.contains(&format!(
        "server_id={server_id} server_name={server_name} kinds=[resources]"
    )));
    assert!(reason.contains("generation=None"));
    assert!(reason.contains("resource inventory failed"));
    assert_eq!(start_count(&counter), 1, "one import must create one discovery owner");

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
async fn import_and_management_read_share_one_discovery_owner() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("import-management-single-flight.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("import-management-single-flight.log");
    let server_name = "import_management_single_flight";
    let python = which::which("python3").expect("python3 is required for the import fixture");
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
        Arc::new(Config::default()),
        Some(database.clone()),
    )));

    let outcome = server_config::import_batch(
        database.clone(),
        Some(&pool),
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
                    "slow_tools".to_string(),
                ]),
                url: None,
                env: None,
                headers: None,
                source: None,
                meta: None,
            },
        )]),
        server_config::ImportOptions::dashboard_import(false),
    )
    .await
    .expect("schedule imported capability discovery");
    assert!(outcome.scheduled);

    let server = server_config::get_server(&database.pool, server_name)
        .await
        .expect("load imported server")
        .expect("imported server exists");
    let server_id = server.id.expect("imported server has stable id");
    assert!(
        pool.lock().await.config.mcp_servers.contains_key(&server_id),
        "completed import must synchronize the production pool configuration"
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while start_count(&counter) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("import discovery owner must start");

    let app = Router::new()
        .route("/", get(server_handlers::server_capability_lists))
        .with_state(build_app_state_with_pool(database.clone(), pool.clone()));
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/?id={server_id}"))
                .body(axum::body::Body::empty())
                .expect("build management capability request"),
        )
        .await
        .expect("call management capability list");
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        start_count(&counter),
        1,
        "import and management reads must join the same per-server discovery"
    );
    let snapshot = SqliteCapabilityCatalog::new(database.pool.clone())
        .load_snapshot(&server_id)
        .await
        .expect("load imported capability catalog")
        .expect("imported capability catalog exists");
    assert_eq!(snapshot.state, SnapshotState::Ready);
}

#[tokio::test]
#[serial_test::serial]
async fn refresh_and_management_read_share_one_discovery_owner() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("refresh-management-single-flight.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let counter = temp_dir.path().join("refresh-management-single-flight.log");
    let server_id = "server-refresh-management-single-flight";
    let server_name = "refresh_management_single_flight";
    insert_stdio_server_with_mode(&database, &script, &counter, server_id, server_name, "slow_tools").await;
    let state = build_capability_refresh_app_state(database, server_id).await;
    let app = Router::new()
        .route("/refresh", post(server_handlers::refresh_server_capabilities))
        .route("/lists", get(server_handlers::server_capability_lists))
        .with_state(state);

    let refresh_app = app.clone();
    let refresh = tokio::spawn(async move {
        refresh_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/refresh")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(json!({"id": server_id}).to_string()))
                    .expect("build capability refresh request"),
            )
            .await
            .expect("call capability refresh")
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while start_count(&counter) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("refresh discovery owner must start");

    let list_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/lists?id={server_id}"))
                .body(axum::body::Body::empty())
                .expect("build management capability request"),
        )
        .await
        .expect("call management capability list");
    let refresh_response = refresh.await.expect("join capability refresh");
    assert_eq!(refresh_response.status(), StatusCode::OK);
    assert_eq!(list_response.status(), StatusCode::OK);
    assert_eq!(
        start_count(&counter),
        1,
        "refresh and management reads must join the same per-server discovery"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn batch_import_starts_two_servers_without_serializing_discovery() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(temp_dir.path().join("batch-import-discovery.db")).await;
    let script = write_counted_stdio_fixture(&temp_dir);
    let python = which::which("python3").expect("python3 is required for the import fixture");
    let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
        Arc::new(Config::default()),
        Some(database.clone()),
    )));
    let fixtures = ["batch_discovery_one", "batch_discovery_two", "batch_discovery_three"]
        .into_iter()
        .map(|server_name| {
            let counter = temp_dir.path().join(format!("{server_name}.log"));
            (server_name.to_string(), counter)
        })
        .collect::<Vec<_>>();
    let items = fixtures
        .iter()
        .map(|(server_name, counter)| {
            let config = ServersImportConfig {
                kind: "stdio".to_string(),
                command: Some(python.to_string_lossy().into_owned()),
                args: Some(vec![
                    script.to_string_lossy().into_owned(),
                    counter.to_string_lossy().into_owned(),
                    server_name.clone(),
                    protocol::CURRENT_VERSION.to_string(),
                    "batch_slow_tools".to_string(),
                ]),
                url: None,
                env: None,
                headers: None,
                source: None,
                meta: None,
            };
            (server_name.clone(), config)
        })
        .collect();

    let outcome = server_config::import_batch(
        database.clone(),
        Some(&pool),
        items,
        server_config::ImportOptions::dashboard_import(false),
    )
    .await
    .expect("schedule batch capability discovery");
    assert_eq!(outcome.imported.len(), 3);

    tokio::time::timeout(Duration::from_millis(1500), async {
        loop {
            let starts = fixtures.iter().map(|(_, counter)| start_count(counter)).sum::<usize>();
            if starts >= 2 {
                assert_eq!(starts, 2, "batch discovery concurrency must remain bounded");
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("two imported servers must begin discovery concurrently");

    let catalog = SqliteCapabilityCatalog::new(database.pool.clone());
    tokio::time::timeout(Duration::from_secs(7), async {
        loop {
            let mut ready = 0;
            for (server_name, _) in &fixtures {
                let server = server_config::get_server(&database.pool, server_name)
                    .await
                    .expect("load imported server")
                    .expect("imported server exists");
                let server_id = server.id.expect("imported server has stable id");
                if catalog
                    .load_snapshot(&server_id)
                    .await
                    .expect("load imported catalog")
                    .is_some_and(|snapshot| snapshot.state == SnapshotState::Ready)
                {
                    ready += 1;
                }
            }
            if ready == fixtures.len() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("all imported capability catalogs must become ready");
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

    let selection = mcpmate::core::capability::ConnectionSelection {
        server_id: server_id.to_string(),
        affinity_key: mcpmate::core::capability::AffinityKey::Default,
    };
    let instance_id = UpstreamConnectionPool::ensure_connected_coordinated(&pool, &selection)
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
async fn active_publication_never_recovers_or_recomputes_during_mcp_list_requests() {
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
    commit_ready_catalog(&database, "server-target", "target_fixture", true).await;
    commit_ready_catalog(&database, "server-unrelated", "unrelated_fixture", false).await;

    let client_id = "capability-surface-client";
    publish_current_surface(&database, client_id, &["server-target"]).await;
    let proxy = build_proxy(database.clone());
    let session_id = "capability-surface-session";
    bind_client(&proxy, client_id, session_id, &profile_id).await;
    let (context, client_service_guard, server_service_guard) = downstream_request_context(client_id, session_id).await;

    let mut published = Vec::new();
    for kind in SurfaceKind::ALL {
        let payload = call_managed_mcp_list(&proxy, kind, context.clone()).await;
        assert!(
            payload
                .pointer(kind.protocol_items_pointer())
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty()),
            "{} active publication returned no protocol items: {payload}",
            kind.label()
        );
        published.push((kind, payload));
    }
    SqliteCapabilityCatalog::new(database.pool.clone())
        .invalidate_server("server-target", "public surface invalidation test")
        .await
        .expect("invalidate target catalog");
    database.capability_cache.invalidate_server("server-target").await;

    for (kind, expected) in published {
        let after_invalidation = call_managed_mcp_list(&proxy, kind, context.clone()).await;
        assert_eq!(
            expected,
            after_invalidation,
            "{} request-time read changed the pinned publication",
            kind.label()
        );
    }
    assert_eq!(
        start_count(&target_counter),
        0,
        "managed MCP list requests must not start the target upstream"
    );
    assert_eq!(
        start_count(&unrelated_counter),
        0,
        "managed MCP list requests must not start an unrelated upstream"
    );

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
    let client_id = "restart-client";
    publish_current_surface(&first_database, client_id, &["server-restart"]).await;
    first_database.pool.close().await;

    let restarted_database = open_database(database_path).await;
    let proxy = build_proxy(restarted_database);
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
async fn invalidated_catalog_cannot_mutate_a_pinned_surface_without_materialization() {
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

    let client_id = "invalidated-full-client";
    publish_current_surface(&database, client_id, &[server_id]).await;
    let proxy = build_proxy(database.clone());
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

    let pinned_tools = call_managed_mcp_list(&proxy, SurfaceKind::Tools, context.clone()).await;
    assert!(
        pinned_tools.to_string().contains("current_fixture_tool"),
        "pinned Surface did not retain the published tool: {pinned_tools}"
    );
    assert_eq!(
        start_count(&counter),
        0,
        "tool list recomputed an invalidated catalog at request time"
    );

    let pinned_prompts = call_managed_mcp_list(&proxy, SurfaceKind::Prompts, context.clone()).await;
    assert_eq!(
        start_count(&counter),
        0,
        "prompt list recomputed an invalidated catalog at request time"
    );
    assert!(
        pinned_prompts.to_string().contains("Stale invalidated prompt"),
        "catalog invalidation mutated the active publication: {pinned_prompts}"
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
        let response = call_rest_list(&first_app, kind, target_id).await;
        if kind_index == 0 {
            assert_eq!(
                response.pointer("/data/meta/source"),
                Some(&json!("live")),
                "{} initial discovery did not report live source: {response}",
                kind.label()
            );
            assert_eq!(
                response.pointer("/data/meta/cache_hit"),
                Some(&json!(false)),
                "{} live discovery was incorrectly reported as a cache hit",
                kind.label()
            );
        } else {
            assert_eq!(
                response.pointer("/data/meta/cache_hit"),
                Some(&json!(true)),
                "{} follow-up kind read should come from the warmed catalog: {response}",
                kind.label()
            );
        }
        assert!(
            response
                .pointer("/data/items")
                .and_then(Value::as_array)
                .is_some_and(|items| items.len() == 1),
            "{} discovery returned the wrong payload: {response}",
            kind.label()
        );
        assert_eq!(
            start_count(&target_counter),
            1,
            "{} discovery unexpectedly restarted upstream",
            kind.label()
        );
    }
    let memory = call_rest_list(&first_app, SurfaceKind::Tools, target_id).await;
    assert_eq!(
        memory.pointer("/data/meta/source"),
        Some(&json!("memory_cache")),
        "immediate second read did not use the process-local LRU: {memory}"
    );
    assert_eq!(start_count(&target_counter), 1);
    assert_eq!(start_count(&unrelated_counter), 0);

    let catalog = SqliteCapabilityCatalog::new(first_database.pool.clone());
    let live_snapshot = catalog
        .load_snapshot(target_id)
        .await
        .expect("load live snapshot")
        .expect("live snapshot exists");
    assert_eq!(live_snapshot.state, SnapshotState::Ready);
    assert_eq!(live_snapshot.revision, 1);
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
    let record_revisions: Vec<i64> = sqlx::query_scalar(
        r#"
            SELECT c.catalog_revision
            FROM capability_ref_current c
            JOIN capability_refs r ON r.ref_id = c.ref_id
            WHERE r.server_id = ? AND r.state = 'active'
            ORDER BY r.kind, r.origin_key
            "#,
    )
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
    assert_eq!(start_count(&target_counter), 1);
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
    assert_eq!(start_count(&target_counter), 2);
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
            .all(|state| state.inventory == InventoryState::Complete)
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
    let current_ref_revisions: Vec<(String, i64)> = sqlx::query_as(
        r#"
            SELECT r.kind, c.catalog_revision
            FROM capability_ref_current c
            JOIN capability_refs r ON r.ref_id = c.ref_id
            WHERE r.server_id = ? AND r.state = 'active'
            ORDER BY r.kind
            "#,
    )
    .bind(target_id)
    .fetch_all(&restarted_database.pool)
    .await
    .expect("load recovered current ref revisions");
    assert_eq!(recovered_revision, recovered_snapshot.revision);
    assert_eq!(distinct_kind_revisions, 1);
    assert_eq!(
        current_ref_revisions,
        vec![
            ("prompts".to_string(), recovered_snapshot.revision),
            ("resource_templates".to_string(), recovered_snapshot.revision),
            ("resources".to_string(), recovered_snapshot.revision),
            ("tools".to_string(), recovered_snapshot.revision),
        ]
    );
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
    // Full management warm rediscovers every kind in one observation, so all shadow rows
    // remain present and move to the recovered catalog revision together.
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
#[path = "support/database.rs"]
mod database_support;
