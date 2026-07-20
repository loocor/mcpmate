use std::time::{Duration, Instant};

use chrono::Utc;
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogError, CatalogRecord,
    CatalogSnapshot, DeclarationState, DerivedCapabilityCache, InventoryState, KindObservation,
    SqliteCapabilityCatalog,
};
use rmcp::model::{Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, Tool};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

fn percentile(
    samples: &mut [Duration],
    percentile: usize,
) -> Duration {
    samples.sort_unstable();
    let index = (samples.len().saturating_sub(1) * percentile) / 100;
    samples[index]
}

fn observation(
    server_index: usize,
    capabilities_per_server: usize,
) -> CapabilityObservation {
    let server_id = format!("server-{server_index:04}");
    let records = (0..capabilities_per_server)
        .map(|capability_index| {
            let upstream_key = format!("tool-{capability_index:03}");
            CatalogRecord::new(
                format!("{server_id}:tool:{capability_index:03}"),
                upstream_key.clone(),
                format!("{server_id}__{upstream_key}"),
                CapabilityPayload::Tool(Tool::new(upstream_key, "Scale evidence tool", serde_json::Map::new())),
            )
        })
        .collect();
    CapabilityObservation::new(
        &server_id,
        format!("Scale Server {server_index}"),
        format!("config-{server_index}"),
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
            .with_server_info(Implementation::new("scale-fixture", "1.0.0")),
        vec![KindObservation::new(
            CapabilityKind::Tools,
            DeclarationState::Supported,
            InventoryState::Complete,
        )],
        records,
    )
}

#[tokio::test]
#[ignore = "manual release-mode scale evidence"]
async fn measures_sqlite_lru_and_serialization_at_configured_scale() {
    let server_count = std::env::var("MCPMATE_SCALE_SERVERS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(100);
    let capabilities_per_server = std::env::var("MCPMATE_SCALE_CAPABILITIES_PER_SERVER")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10);
    assert!(
        server_count <= 1_024,
        "raw LRU evidence must fit its production capacity"
    );

    let directory = tempfile::tempdir().expect("create scale database directory");
    let database_path = directory.path().join("catalog.db");
    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .expect("open scale database");
    let catalog = SqliteCapabilityCatalog::new(pool);
    catalog.ensure_schema().await.expect("create catalog schema");

    for server_index in 0..server_count {
        catalog
            .commit_observation(observation(server_index, capabilities_per_server))
            .await
            .expect("seed server snapshot");
    }

    let mut sqlite_samples = Vec::with_capacity(server_count);
    for server_index in 0..server_count {
        let started = Instant::now();
        let snapshot = catalog
            .load_snapshot(&format!("server-{server_index:04}"))
            .await
            .expect("load SQLite snapshot")
            .expect("snapshot exists");
        sqlite_samples.push(started.elapsed());
        assert_eq!(snapshot.records.len(), capabilities_per_server);
    }

    let cache = DerivedCapabilityCache::new(1_024, 4_096);
    for server_index in 0..server_count {
        let server_id = format!("server-{server_index:04}");
        let loader_catalog = catalog.clone();
        let loader_server_id = server_id.clone();
        cache
            .get_or_load_current_snapshot(&server_id, || async move {
                loader_catalog.load_snapshot(&loader_server_id).await
            })
            .await
            .expect("prime raw LRU")
            .expect("primed snapshot exists");
    }

    let mut lru_samples = Vec::with_capacity(server_count);
    let mut serialization_samples = Vec::with_capacity(server_count);
    for server_index in 0..server_count {
        let server_id = format!("server-{server_index:04}");
        let started = Instant::now();
        let snapshot = cache
            .get_or_load_current_snapshot(&server_id, || async {
                Ok::<Option<CatalogSnapshot>, CatalogError>(None)
            })
            .await
            .expect("read warm LRU")
            .expect("warm snapshot exists");
        lru_samples.push(started.elapsed());

        let started = Instant::now();
        let encoded = serde_json::to_vec(&snapshot).expect("serialize HTTP response payload input");
        serialization_samples.push(started.elapsed());
        assert!(!encoded.is_empty());
    }

    let stats = catalog.stats().await.expect("read catalog stats");
    let expected_capabilities = server_count * capabilities_per_server;
    assert_eq!(stats.snapshots as usize, server_count);
    assert_eq!(stats.records as usize, expected_capabilities);
    assert_eq!(cache.metrics().await.raw_entries, server_count);

    let sqlite_p50 = percentile(&mut sqlite_samples, 50);
    let sqlite_p95 = percentile(&mut sqlite_samples, 95);
    let lru_p50 = percentile(&mut lru_samples, 50);
    let lru_p95 = percentile(&mut lru_samples, 95);
    let serialization_p50 = percentile(&mut serialization_samples, 50);
    let serialization_p95 = percentile(&mut serialization_samples, 95);
    eprintln!(
        "scale evidence at {}: servers={server_count} capabilities={expected_capabilities} sqlite_p50_ns={} sqlite_p95_ns={} lru_p50_ns={} lru_p95_ns={} serialization_p50_ns={} serialization_p95_ns={}",
        Utc::now().to_rfc3339(),
        sqlite_p50.as_nanos(),
        sqlite_p95.as_nanos(),
        lru_p50.as_nanos(),
        lru_p95.as_nanos(),
        serialization_p50.as_nanos(),
        serialization_p95.as_nanos(),
    );
}
