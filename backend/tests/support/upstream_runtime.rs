use std::{sync::Arc, time::Duration};

use mcpmate::{
    common::constants::protocol,
    config::{
        models::Server,
        server::{upsert_server, upsert_server_args},
    },
    core::{foundation::types::ConnectionStatus, models::Config, pool::UpstreamConnectionPool},
};
use tempfile::TempDir;
use tokio::sync::Mutex;

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

pub struct SlowUpstreamFixture {
    _temp_dir: TempDir,
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
        let temp_dir = TempDir::new().expect("create temp directory");
        let database = open_database(&temp_dir).await;

        let script = temp_dir.path().join("slow_runtime.py");
        std::fs::write(&script, SLOW_STDIO_SERVER).expect("write stdio fixture");
        let marker = temp_dir.path().join("initializing.marker");
        let counter = temp_dir.path().join("starts.log");
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
            ],
        )
        .await
        .expect("insert stdio server arguments");

        Self {
            _temp_dir: temp_dir,
            pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
                Arc::new(Config::default()),
                Some(database),
            ))),
            marker,
            counter,
            server_id,
            server_name,
        }
    }

    pub async fn wait_until_initializing(&self) {
        tokio::time::timeout(Duration::from_secs(10), async {
            while !self.marker.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("stdio fixture must enter initialize");
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
        std::fs::read_to_string(&self.counter)
            .unwrap_or_default()
            .lines()
            .count()
    }
}
