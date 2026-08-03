use std::{path::PathBuf, sync::Arc, time::Duration};

use axum::{Router, body::to_bytes, routing::post};
use mcpmate::{
    api::{handlers::server, models::server::ServerCapabilityRefreshReq, routes::AppState},
    common::constants::protocol,
    config::{
        models::Server,
        server::{upsert_server, upsert_server_args},
    },
    core::{models::Config, pool::UpstreamConnectionPool, profile::ConfigApplicationStateManager},
    inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager},
    system::metrics::MetricsCollector,
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};
use tower::ServiceExt;

use crate::runtime_database::open_database;

const COUNTED_PREVIEW_SERVER: &str = r#"
import json
import pathlib
import sys
import time

counter = pathlib.Path(sys.argv[1])
protocol_version = sys.argv[2]
startup_delay = float(sys.argv[3])
exit_marker = pathlib.Path(sys.argv[4])
list_delay = float(sys.argv[5])
with counter.open("a", encoding="utf-8") as starts:
    starts.write("start\n")

for line in sys.stdin:
    request = json.loads(line)
    request_id = request.get("id")
    if request_id is None:
        continue
    method = request.get("method")
    if method == "initialize":
        time.sleep(startup_delay)
        result = {
            "protocolVersion": protocol_version,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "preview-runtime-fixture", "version": "1.0.0"},
        }
    elif method == "tools/list":
        time.sleep(list_delay)
        result = {"tools": []}
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()

exit_marker.write_text("stopped\n", encoding="utf-8")
"#;

fn app_state(
    database: Arc<mcpmate::config::database::Database>,
    connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
) -> Arc<AppState> {
    Arc::new(AppState {
        connection_pool,
        metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(1))),
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
        secret_store_readiness: RwLock::new(mcpmate::api::routes::unavailable_secret_store_readiness(
            "test_unavailable",
        )),
    })
}

pub struct PreviewUpstreamFixture {
    _temp_dir: TempDir,
    app: Router,
    state: Arc<AppState>,
    database: Arc<mcpmate::config::database::Database>,
    pub pool: Arc<Mutex<UpstreamConnectionPool>>,
    command: String,
    script: PathBuf,
    counter: PathBuf,
    exit_marker: PathBuf,
    startup_delay: Duration,
    list_delay: Duration,
}

impl PreviewUpstreamFixture {
    pub async fn new() -> Self {
        Self::new_with_delay(Duration::ZERO).await
    }

    pub async fn new_with_delay(startup_delay: Duration) -> Self {
        Self::new_with_delays(startup_delay, Duration::ZERO).await
    }

    pub async fn new_with_delays(
        startup_delay: Duration,
        list_delay: Duration,
    ) -> Self {
        let temp_dir = TempDir::new().expect("create temp directory");
        let database = open_database(&temp_dir).await;
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
            Arc::new(Config::default()),
            Some(database.clone()),
        )));
        let state = app_state(database.clone(), pool.clone());
        let script = temp_dir.path().join("counted_preview.py");
        std::fs::write(&script, COUNTED_PREVIEW_SERVER).expect("write counted preview fixture");
        let counter = temp_dir.path().join("preview-starts.log");
        let exit_marker = temp_dir.path().join("preview-stopped.log");
        let command = which::which("python3")
            .expect("python3 is required for the preview fixture")
            .to_string_lossy()
            .into_owned();
        let app = Router::new()
            .route("/", post(server::preview_servers))
            .with_state(state.clone());

        Self {
            _temp_dir: temp_dir,
            app,
            state,
            database,
            pool,
            command,
            script,
            counter,
            exit_marker,
            startup_delay,
            list_delay,
        }
    }

    fn server_args(&self) -> Vec<String> {
        vec![
            self.script.to_string_lossy().into_owned(),
            self.counter.to_string_lossy().into_owned(),
            protocol::CURRENT_VERSION.to_string(),
            self.startup_delay.as_secs_f64().to_string(),
            self.exit_marker.to_string_lossy().into_owned(),
            self.list_delay.as_secs_f64().to_string(),
        ]
    }

    pub async fn persist_server(
        &self,
        server_id: &str,
        namespace: &str,
    ) {
        let mut server = Server::new_stdio(namespace.to_string(), Some(self.command.clone()));
        server.id = Some(server_id.to_string());
        upsert_server(&self.database.pool, &server)
            .await
            .expect("persist preview fixture server");
        upsert_server_args(&self.database.pool, server_id, &self.server_args())
            .await
            .expect("persist preview fixture arguments");
    }

    pub async fn enable_server(
        &self,
        server_id: &str,
    ) -> String {
        UpstreamConnectionPool::enable_server_coordinated(&self.pool, server_id)
            .await
            .expect("enable persisted preview fixture")
    }

    pub async fn refresh_capabilities(
        &self,
        server_id: &str,
    ) {
        let _ = server::refresh_server_capabilities(
            axum::extract::State(self.state.clone()),
            axum::Json(ServerCapabilityRefreshReq {
                id: server_id.to_string(),
            }),
        )
        .await
        .expect("refresh persisted fixture capabilities");
    }

    pub async fn wait_until_catalog_ready(
        &self,
        server_id: &str,
    ) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let (snapshot, _) = self
                    .database
                    .load_capability_snapshot(server_id)
                    .await
                    .expect("load promoted owner catalog");
                if snapshot.is_some_and(|snapshot| snapshot.state == mcpmate_capability_store::SnapshotState::Ready) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("promoted owner must publish a ready capability catalog");
    }

    pub async fn preview(
        &self,
        namespace: &str,
        extra_args: &[&str],
    ) -> Value {
        self.preview_with_timeout(namespace, extra_args, None).await
    }

    pub async fn preview_with_timeout(
        &self,
        namespace: &str,
        extra_args: &[&str],
        timeout_ms: Option<u64>,
    ) -> Value {
        let mut args = self.server_args();
        args.extend(extra_args.iter().map(|value| (*value).to_string()));
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&json!({
                    "servers": [{
                        "name": namespace,
                        "kind": "stdio",
                        "command": self.command,
                        "args": args
                    }],
                    "timeout_ms": timeout_ms
                }))
                .expect("serialize preview request"),
            ))
            .expect("build preview request");
        let response = self.app.clone().oneshot(request).await.expect("call preview handler");
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("read preview response");
        let body: Value = serde_json::from_slice(&body).expect("decode preview response");
        assert!(status.is_success(), "preview request failed: {body}");
        body
    }

    pub fn startup_count(&self) -> usize {
        std::fs::read_to_string(&self.counter)
            .unwrap_or_default()
            .lines()
            .count()
    }

    pub async fn wait_until_started(&self) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while self.startup_count() == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("preview owner must start");
    }

    pub async fn discover_once(&self) {
        let config = mcpmate::core::models::MCPServerConfig {
            source_fingerprint: None,
            kind: mcpmate::common::server::ServerType::Stdio,
            command: Some(self.command.clone()),
            url: None,
            args: Some(self.server_args()),
            env: None,
            headers: None,
        };
        mcpmate::config::server::capabilities::discover_from_config_preview(
            "everything",
            &config,
            mcpmate::common::server::ServerType::Stdio,
            None,
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("discover one preview");
    }

    pub async fn wait_until_exited(&self) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while !self.exit_marker.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("preview process must exit after discovery");
    }
}
