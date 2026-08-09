use std::{collections::HashMap, path::Path, str::FromStr, sync::Arc};

use axum::{Json, extract::State};
use mcpmate::{
    api::{
        handlers::server::capability::refresh_server_capabilities,
        models::server::ServerCapabilityRefreshReq,
        routes::{AppState, unavailable_secret_store_readiness},
    },
    clients::models::CapabilitySource,
    common::{constants::protocol, profile::ProfileType},
    config::{
        database::Database,
        initialization::run_initialization,
        models::{Profile, Server, ServerTransportDraft},
        server,
    },
    core::{
        capability::{AffinityKey, ConnectionSelection, naming},
        models::Config,
        pool::UpstreamConnectionPool,
        profile::ConfigApplicationStateManager,
        proxy::server::{
            ClientContext, ClientIdentitySource, ClientTransport, ManagedClientContextResolver, ProxyServer,
        },
    },
    inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager},
    mcper::{MCPMATE_UCAN_CALL_TOOL, MCPMATE_UCAN_CATALOG_TOOL, MCPMATE_UCAN_DETAILS_TOOL},
    system::metrics::MetricsCollector,
};
use mcpmate_capability_store::DerivedCapabilityCache;
use rmcp::{
    ServerHandler, ServiceExt as _,
    model::{CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, RequestId},
    service::{RequestContext, RoleClient, RoleServer, RunningService},
};
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tempfile::TempDir;
use tokio::{
    io::duplex,
    sync::{Mutex, RwLock},
};

#[path = "support/database.rs"]
mod database_support;

const CLIENT_ID: &str = "ucan-partial-client";
const SESSION_ID: &str = "ucan-partial-session";

const UPSTREAM_FIXTURE: &str = r#"
import json
import pathlib
import sys

state_path = pathlib.Path(sys.argv[1])
label = sys.argv[2]
protocol_version = sys.argv[3]

def reply(request_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()

def fail(request_id):
    sys.stdout.write(json.dumps({
        "jsonrpc": "2.0",
        "id": request_id,
        "error": {"code": -32000, "message": "fixture upstream inventory failure"}
    }) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    request = json.loads(line)
    request_id = request.get("id")
    method = request.get("method")
    if request_id is None:
        continue
    if method == "initialize":
        reply(request_id, {
            "protocolVersion": protocol_version,
            "capabilities": {"tools": {}, "prompts": {}, "resources": {}},
            "serverInfo": {"name": label, "version": "1.0.0"}
        })
    elif method in ("tools/list", "prompts/list", "resources/list", "resources/templates/list"):
        if state_path.read_text(encoding="utf-8").strip() == "fail":
            fail(request_id)
        elif method == "tools/list":
            reply(request_id, {"tools": [{
                "name": label + "_tool",
                "description": "UCan partial-catalog fixture",
                "inputSchema": {"type": "object"}
            }]})
        elif method == "prompts/list":
            reply(request_id, {"prompts": []})
        elif method == "resources/list":
            reply(request_id, {"resources": []})
        else:
            reply(request_id, {"resourceTemplates": []})
    elif method == "tools/call" and state_path.read_text(encoding="utf-8").strip() == "tool_error":
        reply(request_id, {
            "content": [{"type": "text", "text": "fixture upstream tool failure"}],
            "isError": True
        })
    else:
        fail(request_id)
"#;

#[derive(Clone)]
struct DownstreamContextServer;

impl ServerHandler for DownstreamContextServer {}

async fn open_database(temp_dir: &TempDir) -> Arc<Database> {
    let path = temp_dir.path().join("ucan-partial-catalog.db");
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
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
    naming::initialize(pool.clone());

    Arc::new(Database {
        pool,
        path,
        capability_cache: Arc::new(DerivedCapabilityCache::default()),
    })
}

fn write_upstream_fixture(temp_dir: &TempDir) -> std::path::PathBuf {
    let path = temp_dir.path().join("ucan_partial_catalog_fixture.py");
    std::fs::write(&path, UPSTREAM_FIXTURE).expect("write stdio fixture");
    path
}

async fn insert_stdio_server(
    database: &Database,
    script: &Path,
    state_path: &Path,
    server_id: &str,
    server_name: &str,
) {
    let python = which::which("python3").expect("python3 is required for stdio fixture");
    let mut server_config = Server::new_stdio(server_name.to_string(), Some(python.to_string_lossy().into_owned()));
    server_config.id = Some(server_id.to_string());
    server::upsert_server_definition(
        &database.pool,
        &server_config,
        &ServerTransportDraft::Stdio {
            command: server_config.command.clone(),
            args: vec![
                script.to_string_lossy().into_owned(),
                state_path.to_string_lossy().into_owned(),
                server_name.to_string(),
                protocol::CURRENT_VERSION.to_string(),
            ],
            env: Default::default(),
        },
    )
    .await
    .expect("insert typed stdio server");
}

async fn insert_profiles_client(
    database: &Database,
    server_ids: &[&str],
) -> String {
    let mut profile = Profile::new("Selected profile".to_string(), ProfileType::Shared);
    profile.is_active = true;
    let profile_id = database_support::insert_profile(&database.pool, &profile).await;
    for server_id in server_ids {
        database_support::insert_profile_server_relationship(&database.pool, &profile_id, server_id, true).await;
    }
    sqlx::query(
        "INSERT INTO client (id, name, identifier, config_mode, approval_status) VALUES (?, ?, ?, 'hosted', 'approved')",
    )
    .bind(CLIENT_ID)
    .bind(CLIENT_ID)
    .bind(CLIENT_ID)
    .execute(&database.pool)
    .await
    .expect("insert managed client");
    profile_id
}

async fn materialize_ucan_surface(
    proxy: &ProxyServer,
    profile_id: &str,
) {
    let service = proxy
        .client_config_service
        .as_ref()
        .expect("client configuration service is initialized");
    let revisions = service.catalog_revision_set().await.expect("load catalog revisions");
    service
        .update_capability_config_state_and_invalidate(
            CLIENT_ID,
            Some("hosted".to_string()),
            CapabilitySource::Profiles,
            vec![profile_id.to_string()],
            None,
            revisions,
        )
        .await
        .expect("materialize managed UCan surface");
}

async fn bind_client(
    proxy: &ProxyServer,
    profile_id: String,
) {
    proxy
        .client_context_resolver
        .bind_session(
            SESSION_ID,
            &ClientContext {
                client_id: CLIENT_ID.to_string(),
                session_id: Some(SESSION_ID.to_string()),
                profile_id: Some(profile_id),
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

async fn downstream_request_context() -> (
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
        RequestId::String("ucan-partial-catalog".into()),
        server_service.peer().clone(),
    );
    let request = hyper::Request::builder()
        .uri(format!("/mcp?client_id={CLIENT_ID}"))
        .header("mcp-session-id", SESSION_ID)
        .header(protocol::MCP_PROTOCOL_VERSION_HEADER, protocol::CURRENT_VERSION)
        .body(())
        .expect("build downstream request parts");
    context.extensions.insert(request.into_parts().0);
    (context, client_service, server_service)
}

async fn call_ucan(
    proxy: &ProxyServer,
    context: &RequestContext<RoleServer>,
    tool_name: &str,
    arguments: Value,
) -> Result<CallToolResponse, rmcp::ErrorData> {
    ServerHandler::call_tool(
        proxy,
        CallToolRequestParams::new(tool_name.to_string())
            .with_arguments(arguments.as_object().expect("UCan arguments must be an object").clone()),
        context.clone(),
    )
    .await
}

fn complete_result(response: CallToolResponse) -> CallToolResult {
    let CallToolResponse::Complete(result) = response else {
        panic!("UCan request must complete with a tool result");
    };
    result
}

fn text_json(result: &CallToolResult) -> Value {
    let ContentBlock::Text(text) = result.content.first().expect("UCan result has text content") else {
        panic!("UCan result content must be text");
    };
    serde_json::from_str(&text.text).expect("UCan text content is JSON")
}

fn assert_catalog_incomplete(response: CallToolResponse) {
    let result = complete_result(response);
    assert_eq!(result.is_error, Some(true));
    let body = text_json(&result);
    assert_eq!(body["error_code"], "catalog_incomplete");
    assert_eq!(body["retry_eligible"], true);
    assert!(
        !result.content.iter().any(|content| {
            matches!(content, ContentBlock::Text(text) if text.text.contains("fixture upstream inventory failure"))
        }),
        "downstream error must not expose the upstream fixture reason: {:?}",
        result.content
    );
}

fn build_app_state(
    database: Arc<Database>,
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
) -> Arc<AppState> {
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
        inspector_calls: Arc::new(InspectorCallRegistry::new()),
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: RwLock::new(None),
        secret_store: RwLock::new(None),
        secret_store_readiness: RwLock::new(unavailable_secret_store_readiness("test_unavailable")),
    })
}

async fn initialize_proxy(database: &Database) -> ProxyServer {
    let mut proxy = ProxyServer::new(Arc::new(Config::default()));
    proxy
        .set_database(database.clone())
        .await
        .expect("initialize proxy with builtin UCan services");
    proxy
}

async fn prepare_ucan_context(
    proxy: &ProxyServer,
    database: &Database,
    server_ids: &[&str],
) -> (
    RequestContext<RoleServer>,
    RunningService<RoleClient, ()>,
    RunningService<RoleServer, DownstreamContextServer>,
) {
    let profile_id = insert_profiles_client(database, server_ids).await;
    materialize_ucan_surface(proxy, &profile_id).await;
    bind_client(proxy, profile_id).await;
    downstream_request_context().await
}

async fn refresh_catalog(
    database: Arc<Database>,
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    server_id: &str,
) {
    let _ = refresh_server_capabilities(
        State(build_app_state(database, connection_pool)),
        Json(ServerCapabilityRefreshReq {
            id: server_id.to_string(),
        }),
    )
    .await
    .expect("forced capability sync must complete");
}

#[tokio::test]
#[serial_test::serial]
async fn ucan_partial_catalog_preserves_healthy_entries_returns_structured_errors_and_recovers_after_forced_sync() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(&temp_dir).await;
    let proxy = initialize_proxy(&database).await;

    let script = write_upstream_fixture(&temp_dir);
    let healthy_state = temp_dir.path().join("healthy.state");
    let failed_state = temp_dir.path().join("failed.state");
    std::fs::write(&healthy_state, "ready").expect("write healthy fixture state");
    std::fs::write(&failed_state, "ready").expect("write failed fixture initial state");
    insert_stdio_server(&database, &script, &healthy_state, "server-healthy", "healthy").await;
    insert_stdio_server(&database, &script, &failed_state, "server-failed", "failed").await;
    refresh_catalog(database.clone(), proxy.connection_pool.clone(), "server-healthy").await;
    refresh_catalog(database.clone(), proxy.connection_pool.clone(), "server-failed").await;
    std::fs::write(&failed_state, "fail").expect("simulate failed fixture state");
    let failed_refresh = refresh_server_capabilities(
        State(build_app_state(database.clone(), proxy.connection_pool.clone())),
        Json(ServerCapabilityRefreshReq {
            id: "server-failed".to_string(),
        }),
    )
    .await;
    assert!(
        failed_refresh.is_err(),
        "fixture failure must be recorded through the normal refresh path"
    );
    let (context, client_service, server_service) =
        prepare_ucan_context(&proxy, &database, &["server-healthy", "server-failed"]).await;

    let healthy_catalog = complete_result(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_CATALOG_TOOL,
            json!({"kind_filter": ["tool"]}),
        )
        .await
        .expect("healthy tool result must survive a partial upstream failure"),
    );
    assert_ne!(healthy_catalog.is_error, Some(true));
    assert!(
        text_json(&healthy_catalog)["total_items"]
            .as_u64()
            .is_some_and(|count| count >= 1),
        "healthy upstream tool must remain visible: {:?}",
        healthy_catalog.content
    );

    assert_catalog_incomplete(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_CATALOG_TOOL,
            json!({"kind_filter": ["resource"]}),
        )
        .await
        .expect("incomplete catalog must return a structured UCan result"),
    );
    assert_catalog_incomplete(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_DETAILS_TOOL,
            json!({"capability_kind": "tool", "capability_name": "missing_tool"}),
        )
        .await
        .expect("incomplete details must return a structured UCan result"),
    );
    assert_catalog_incomplete(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_CALL_TOOL,
            json!({"capability_kind": "tool", "capability_name": "missing_tool", "arguments": {}}),
        )
        .await
        .expect("incomplete call must return a structured UCan result"),
    );
    assert_catalog_incomplete(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_CALL_TOOL,
            json!({
                "capability_kind": "resource",
                "capability_name": "missing_resource",
                "arguments": {"unexpected": true}
            }),
        )
        .await
        .expect("partial catalog unknown resource call must remain authoritative"),
    );

    std::fs::write(&failed_state, "ready").expect("restore upstream fixture behavior");
    let app_state = build_app_state(database.clone(), proxy.connection_pool.clone());
    let _ = refresh_server_capabilities(
        State(app_state),
        Json(ServerCapabilityRefreshReq {
            id: "server-failed".to_string(),
        }),
    )
    .await
    .expect("existing forced capability sync must recover the upstream catalog");

    let recovered_catalog = complete_result(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_CATALOG_TOOL,
            json!({"kind_filter": ["tool"]}),
        )
        .await
        .expect("recovered upstream catalog must be visible through UCan"),
    );
    assert!(
        text_json(&recovered_catalog)["total_items"]
            .as_u64()
            .is_some_and(|count| count >= 2),
        "forced synchronization did not expose recovered upstream tool: {:?}",
        recovered_catalog.content
    );

    drop((client_service, server_service));
}

#[tokio::test]
#[serial_test::serial]
async fn ucan_details_keeps_capability_not_found_for_an_unknown_item_in_a_complete_catalog() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(&temp_dir).await;
    let proxy = initialize_proxy(&database).await;

    let script = write_upstream_fixture(&temp_dir);
    let state = temp_dir.path().join("complete.state");
    std::fs::write(&state, "ready").expect("write complete fixture state");
    insert_stdio_server(&database, &script, &state, "server-complete", "complete").await;
    refresh_catalog(database.clone(), proxy.connection_pool.clone(), "server-complete").await;
    let (context, client_service, server_service) = prepare_ucan_context(&proxy, &database, &["server-complete"]).await;

    let result = complete_result(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_DETAILS_TOOL,
            json!({"capability_kind": "tool", "capability_name": "missing_tool"}),
        )
        .await
        .expect("complete catalog unknown must be a structured UCan result"),
    );
    assert_eq!(result.is_error, Some(true));
    assert_eq!(text_json(&result)["error_code"], "capability_not_found");

    let result = complete_result(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_CALL_TOOL,
            json!({"capability_kind": "tool", "capability_name": "missing_tool", "arguments": {}}),
        )
        .await
        .expect("complete catalog unknown call must be a structured UCan result"),
    );
    assert_eq!(result.is_error, Some(true));
    assert_eq!(text_json(&result)["error_code"], "capability_not_found");

    let result = complete_result(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_CALL_TOOL,
            json!({
                "capability_kind": "resource",
                "capability_name": "missing_resource",
                "arguments": {"unexpected": true}
            }),
        )
        .await
        .expect("complete catalog unknown resource call must be a structured UCan result"),
    );
    assert_eq!(result.is_error, Some(true));
    assert_eq!(text_json(&result)["error_code"], "capability_not_found");

    drop((client_service, server_service));
}

#[tokio::test]
#[serial_test::serial]
async fn ucan_call_converts_upstream_error_results_to_a_generic_error() {
    let temp_dir = TempDir::new().expect("create test directory");
    let database = open_database(&temp_dir).await;
    let script = write_upstream_fixture(&temp_dir);
    let state = temp_dir.path().join("tool-error.state");
    std::fs::write(&state, "ready").expect("write initial fixture state");
    insert_stdio_server(&database, &script, &state, "server-tool-error", "tool_error").await;
    let (_, upstream_config) =
        mcpmate::core::foundation::loader::load_server_config_strict(&database, "server-tool-error", None)
            .await
            .expect("load upstream config for the broker fixture");
    let mut proxy = ProxyServer::new(Arc::new(Config {
        mcp_servers: HashMap::from([("server-tool-error".to_string(), upstream_config)]),
        ..Default::default()
    }));
    proxy
        .set_database((*database).clone())
        .await
        .expect("initialize proxy with builtin UCan services");
    refresh_catalog(database.clone(), proxy.connection_pool.clone(), "server-tool-error").await;
    let profile_id = insert_profiles_client(&database, &["server-tool-error"]).await;
    materialize_ucan_surface(&proxy, &profile_id).await;
    bind_client(&proxy, profile_id).await;
    UpstreamConnectionPool::ensure_connected_coordinated(
        &proxy.connection_pool,
        &ConnectionSelection {
            server_id: "server-tool-error".to_string(),
            affinity_key: AffinityKey::Default,
        },
    )
    .await
    .expect("connect upstream tool fixture");
    std::fs::write(&state, "tool_error").expect("configure upstream tool error response");
    let (context, client_service, server_service) = downstream_request_context().await;

    let result = complete_result(
        call_ucan(
            &proxy,
            &context,
            MCPMATE_UCAN_CALL_TOOL,
            json!({"capability_kind": "tool", "capability_name": "tool_error_tool", "arguments": {}}),
        )
        .await
        .expect("upstream error result must be converted to a UCan result"),
    );
    assert_eq!(result.is_error, Some(true));
    assert_eq!(text_json(&result)["error_code"], "upstream_error");
    assert!(
        !result.content.iter().any(|content| {
            matches!(content, ContentBlock::Text(text) if text.text.contains("fixture upstream tool failure"))
        }),
        "downstream error must not expose the upstream error content: {:?}",
        result.content
    );

    drop((client_service, server_service));
}
