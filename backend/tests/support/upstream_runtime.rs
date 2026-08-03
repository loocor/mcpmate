use std::{path::Path, sync::Arc, time::Duration};

use axum::Router;
use mcpmate::{
    api::routes::{AppState, unavailable_secret_store_readiness},
    common::constants::protocol,
    config::{
        database::Database,
        models::Server,
        server::{upsert_server, upsert_server_args},
    },
    core::{
        foundation::{load_server_config_strict, types::ConnectionStatus},
        models::Config,
        pool::UpstreamConnectionPool,
        profile::ConfigApplicationStateManager,
    },
    inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager},
    system::metrics::MetricsCollector,
};
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};

use crate::runtime_database::open_database;

const SLOW_STDIO_SERVER: &str = r#"
import json
import pathlib
import sys
import time

marker = pathlib.Path(sys.argv[1])
counter = pathlib.Path(sys.argv[2])
protocol_version = sys.argv[3]
delay = float(sys.argv[4])
behavior = sys.argv[5]

for line in sys.stdin:
    request = json.loads(line)
    request_id = request.get("id")
    if request_id is None:
        continue
    if request.get("method") == "initialize":
        marker.write_text("initializing", encoding="utf-8")
        with counter.open("a", encoding="utf-8") as starts:
            starts.write("start\n")
        time.sleep(delay)
        if behavior == "fail":
            error = {"code": -32000, "message": "slow initialize failure"}
            sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "error": error}) + "\n")
            sys.stdout.flush()
            break
        result = {
            "protocolVersion": protocol_version,
            "capabilities": {},
            "serverInfo": {"name": "slow-runtime-fixture", "version": "1.0.0"},
        }
    elif request.get("method") == "tools/list":
        result = {"tools": []}
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "result": result}) + "\n")
    sys.stdout.flush()
"#;

#[derive(Clone, Copy)]
pub enum StartupBehavior {
    Ready,
    Fail,
}

impl StartupBehavior {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Fail => "fail",
        }
    }
}

pub struct RuntimeServerFixture {
    pub server_id: &'static str,
    pub marker: std::path::PathBuf,
    pub counter: std::path::PathBuf,
}

async fn wait_until_initializing(marker: &Path) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("stdio fixture must enter initialize");
}

fn startup_count(counter: &Path) -> usize {
    std::fs::read_to_string(counter).unwrap_or_default().lines().count()
}

pub struct SlowUpstreamFixture {
    _temp_dir: TempDir,
    pub database: Arc<Database>,
    pub pool: Arc<Mutex<UpstreamConnectionPool>>,
    pub marker: std::path::PathBuf,
    pub counter: std::path::PathBuf,
    pub server_id: &'static str,
    pub server_name: &'static str,
}

impl SlowUpstreamFixture {
    pub async fn new(
        server_id: &'static str,
        server_name: &'static str,
        delay: Duration,
    ) -> Self {
        Self::new_with_behavior(server_id, server_name, delay, StartupBehavior::Ready).await
    }

    pub async fn new_with_behavior(
        server_id: &'static str,
        server_name: &'static str,
        delay: Duration,
        behavior: StartupBehavior,
    ) -> Self {
        let temp_dir = TempDir::new().expect("create temp directory");
        let database = open_database(&temp_dir).await;
        let runtime_server = Self::insert_server(&temp_dir, &database, server_id, server_name, delay, behavior).await;

        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(
            Arc::new(Config::default()),
            Some(database.clone()),
        )));

        Self {
            _temp_dir: temp_dir,
            database,
            pool,
            marker: runtime_server.marker,
            counter: runtime_server.counter,
            server_id,
            server_name,
        }
    }

    pub async fn add_server(
        &self,
        server_id: &'static str,
        server_name: &'static str,
        delay: Duration,
        behavior: StartupBehavior,
    ) -> RuntimeServerFixture {
        Self::insert_server(&self._temp_dir, &self.database, server_id, server_name, delay, behavior).await
    }

    async fn insert_server(
        temp_dir: &TempDir,
        database: &Database,
        server_id: &'static str,
        server_name: &'static str,
        delay: Duration,
        behavior: StartupBehavior,
    ) -> RuntimeServerFixture {
        let script = temp_dir.path().join(format!("{server_id}.py"));
        std::fs::write(&script, SLOW_STDIO_SERVER).expect("write stdio fixture");
        let marker = temp_dir.path().join(format!("{server_id}.marker"));
        let counter = temp_dir.path().join(format!("{server_id}.starts.log"));
        let python = which::which("python3").expect("python3 is required for the stdio fixture");
        let mut server = Server::new_stdio(server_name.to_string(), Some(python.to_string_lossy().into_owned()));
        server.id = Some(server_id.to_string());
        upsert_server(&database.pool, &server)
            .await
            .expect("insert stdio server");
        upsert_server_args(
            &database.pool,
            server_id,
            &[
                script.to_string_lossy().into_owned(),
                marker.to_string_lossy().into_owned(),
                counter.to_string_lossy().into_owned(),
                protocol::CURRENT_VERSION.to_string(),
                delay.as_secs_f64().to_string(),
                behavior.as_arg().to_string(),
            ],
        )
        .await
        .expect("insert stdio server arguments");
        RuntimeServerFixture {
            server_id,
            marker,
            counter,
        }
    }

    pub async fn register_runtime_config_without_instance(&self) {
        let (_, server_config) = load_server_config_strict(&self.database, self.server_id, None)
            .await
            .expect("load runtime fixture config");
        let mut config = Config::default();
        config.mcp_servers.insert(self.server_id.to_string(), server_config);
        let mut pool = self.pool.lock().await;
        pool.set_config(Arc::new(config))
            .expect("register runtime fixture config");
        pool.connections.remove(self.server_id);
    }

    pub fn server_management_app(&self) -> Router {
        let state = Arc::new(AppState {
            connection_pool: self.pool.clone(),
            metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(1))),
            http_proxy: None,
            profile_merge_service: None,
            database: Some(self.database.clone()),
            audit_database: None,
            audit_service: None,
            config_application_state: Arc::new(ConfigApplicationStateManager::new()),
            client_service: None,
            inspector_calls: Arc::new(InspectorCallRegistry::new()),
            inspector_sessions: Arc::new(InspectorSessionManager::new()),
            oauth_manager: RwLock::new(None),
            secret_store: RwLock::new(None),
            secret_store_readiness: RwLock::new(unavailable_secret_store_readiness("test_unavailable")),
        });
        Router::new().merge(mcpmate::api::routes::server::routes(state))
    }

    pub async fn wait_until_initializing(&self) {
        wait_until_initializing(&self.marker).await;
    }

    pub async fn wait_until_ready(&self) -> String {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let ready_instance = self
                    .pool
                    .lock()
                    .await
                    .get_snapshot()
                    .get(self.server_id)
                    .and_then(|instances| {
                        instances.iter().find_map(|(instance_id, status, _, _, _)| {
                            matches!(status, ConnectionStatus::Ready).then(|| instance_id.clone())
                        })
                    });
                if let Some(instance_id) = ready_instance {
                    return instance_id;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("coordinated upstream must finish startup")
    }

    pub fn startup_count(&self) -> usize {
        startup_count(&self.counter)
    }
}
