use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogRecord, DeclarationState,
    DerivedCapabilityCache, InventoryState, KindObservation, ProjectionKey, ProjectionNameDomain, ProjectionPayload,
    SnapshotState, SqliteCapabilityCatalog,
};
use rmcp::model::{InitializeResult, Prompt, Resource, ResourceTemplate, Tool};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::{
    Pool, Row, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::sync::{Barrier, oneshot};

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("fixture must match RMCP 2.2")
}

fn initialize_result() -> InitializeResult {
    decode(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {
            "experimental": {"mcpmate.dev/full-fidelity": {"enabled": true}},
            "extensions": {"io.modelcontextprotocol/apps": {}},
            "logging": {},
            "completions": {},
            "prompts": {"listChanged": true},
            "resources": {"subscribe": true, "listChanged": true},
            "tools": {"listChanged": true},
            "tasks": {"list": {}, "cancel": {}, "requests": {"tools": {"call": {}}}}
        },
        "serverInfo": {"name": "fixture-server", "title": "Fixture Server", "version": "2.2.0"},
        "instructions": "Preserve this initialize result exactly.",
        "_meta": {"fixture": "initialize"}
    }))
}

fn full_records() -> Vec<CatalogRecord> {
    let tool: Tool = decode(json!({
        "name": "analyze",
        "title": "Analyze",
        "description": "Analyze a payload",
        "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}, "required": ["query"]},
        "outputSchema": {"type": "object", "properties": {"result": {"type": "string"}}},
        "annotations": {
            "title": "Safe analyzer",
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        },
        "execution": {"taskSupport": "optional"},
        "icons": [{"src": "https://icons.example/tool.svg", "mimeType": "image/svg+xml", "sizes": ["any"]}],
        "_meta": {"fixture": "tool"}
    }));
    let prompt: Prompt = decode(json!({
        "name": "summarize",
        "title": "Summarize",
        "description": "Summarize a document",
        "arguments": [{"name": "document", "title": "Document", "description": "Input text", "required": true}],
        "icons": [{"src": "https://icons.example/prompt.png", "mimeType": "image/png"}],
        "_meta": {"fixture": "prompt"}
    }));
    let resource: Resource = decode(json!({
        "uri": "file:///fixture/report.md",
        "name": "report",
        "title": "Fixture Report",
        "description": "A complete resource fixture",
        "mimeType": "text/markdown",
        "size": 4096,
        "icons": [{"src": "https://icons.example/resource.svg", "mimeType": "image/svg+xml"}],
        "_meta": {"fixture": "resource"},
        "annotations": {"audience": ["user", "assistant"], "priority": 0.75, "lastModified": "2026-07-20T00:00:00Z"}
    }));
    let template: ResourceTemplate = decode(json!({
        "uriTemplate": "file:///fixture/{name}.md",
        "name": "fixture-template",
        "title": "Fixture Template",
        "description": "A complete template fixture",
        "mimeType": "text/markdown",
        "icons": [{"src": "https://icons.example/template.svg", "mimeType": "image/svg+xml"}],
        "_meta": {"fixture": "template"},
        "annotations": {"audience": ["assistant"], "priority": 0.5}
    }));

    vec![
        CatalogRecord::new(
            "tool-1",
            "analyze",
            "fixture-server__analyze",
            CapabilityPayload::Tool(tool),
        ),
        CatalogRecord::new(
            "prompt-1",
            "summarize",
            "fixture-server__summarize",
            CapabilityPayload::Prompt(prompt),
        ),
        CatalogRecord::new(
            "resource-1",
            "file:///fixture/report.md",
            "mcpmate://fixture-server/resources/resource-1",
            CapabilityPayload::Resource(resource),
        ),
        CatalogRecord::new(
            "template-1",
            "file:///fixture/{name}.md",
            "mcpmate://fixture-server/resource-templates/template-1",
            CapabilityPayload::ResourceTemplate(template),
        ),
    ]
}

fn complete_states() -> Vec<KindObservation> {
    CapabilityKind::ALL
        .into_iter()
        .map(|kind| KindObservation::new(kind, DeclarationState::Supported, InventoryState::Complete))
        .collect()
}

async fn test_pool() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    SqliteCapabilityCatalog::new(pool.clone())
        .ensure_schema()
        .await
        .unwrap();
    pool
}

fn test_tool(name: &str) -> Tool {
    decode(json!({
        "name": name,
        "description": "Fixture tool",
        "inputSchema": {"type": "object"}
    }))
}

fn test_prompt(name: &str) -> Prompt {
    decode(json!({
        "name": name,
        "description": "Fixture prompt"
    }))
}

fn complete_observation(server_id: &str) -> CapabilityObservation {
    CapabilityObservation::new(
        server_id,
        format!("{server_id}-name"),
        "config-v1",
        initialize_result(),
        complete_states(),
        Vec::new(),
    )
}

fn versioned_records(
    server_id: &str,
    version: usize,
) -> Vec<CatalogRecord> {
    let marker = format!("{server_id}-v{version}");
    let tool: Tool = decode(json!({
        "name": marker,
        "description": marker,
        "inputSchema": {"type": "object"}
    }));
    let prompt: Prompt = decode(json!({"name": marker, "description": marker}));
    let resource: Resource = decode(json!({
        "uri": format!("fixture://{marker}/item"),
        "name": marker
    }));
    let template: ResourceTemplate = decode(json!({
        "uriTemplate": format!("fixture://{marker}/{{item}}"),
        "name": marker
    }));

    vec![
        CatalogRecord::new(
            format!("{marker}-tool"),
            marker.clone(),
            format!("{marker}-external-tool"),
            CapabilityPayload::Tool(tool),
        ),
        CatalogRecord::new(
            format!("{marker}-prompt"),
            marker.clone(),
            format!("{marker}-external-prompt"),
            CapabilityPayload::Prompt(prompt),
        ),
        CatalogRecord::new(
            format!("{marker}-resource"),
            format!("fixture://{marker}/item"),
            format!("mcpmate://{marker}/resource"),
            CapabilityPayload::Resource(resource),
        ),
        CatalogRecord::new(
            format!("{marker}-template"),
            format!("fixture://{marker}/{{item}}"),
            format!("mcpmate://{marker}/template"),
            CapabilityPayload::ResourceTemplate(template),
        ),
    ]
}

fn versioned_observation(
    server_id: &str,
    version: usize,
) -> CapabilityObservation {
    CapabilityObservation::new(
        server_id,
        format!("{server_id}-name"),
        format!("{server_id}-v{version}"),
        initialize_result(),
        complete_states(),
        versioned_records(server_id, version),
    )
}

fn assert_complete_version(snapshot: &mcpmate_capability_store::CatalogSnapshot) {
    assert_eq!(snapshot.state, SnapshotState::Ready);
    assert_eq!(snapshot.kind_states.len(), CapabilityKind::ALL.len());
    assert!(snapshot.kind_states.iter().all(|state| {
        state.declaration == DeclarationState::Supported && state.inventory == InventoryState::Complete
    }));
    assert_eq!(snapshot.records.len(), CapabilityKind::ALL.len());
    assert!(
        snapshot
            .records
            .iter()
            .all(|record| record.stable_id.starts_with(&snapshot.config_fingerprint)),
        "snapshot mixed records from different observations: {snapshot:?}"
    );
}

async fn catalog() -> SqliteCapabilityCatalog {
    SqliteCapabilityCatalog::new(test_pool().await)
}

#[tokio::test]
async fn round_trips_full_rmcp_payload_and_initialize_result() {
    let catalog = catalog().await;
    let observation = CapabilityObservation::new(
        "server-1",
        "fixture-server",
        "config-v1",
        initialize_result(),
        complete_states(),
        full_records(),
    );

    let committed = catalog.commit_observation(observation.clone()).await.unwrap();
    let loaded = catalog.load_snapshot("server-1").await.unwrap().unwrap();

    assert_eq!(committed.revision, 1);
    assert_eq!(loaded.state, SnapshotState::Ready);
    assert_eq!(loaded.revision, 1);
    assert_eq!(
        serde_json::to_value(loaded.initialize.as_ref().expect("ready snapshot initialize")).unwrap(),
        serde_json::to_value(&observation.initialize).unwrap()
    );
    assert_eq!(loaded.records, observation.records);
    assert_eq!(loaded.kind_states, observation.kind_states);

    let kind_observed_at: String =
        sqlx::query_scalar("SELECT observed_at FROM capability_kind_states WHERE server_id = ? AND kind = 'tools'")
            .bind("server-1")
            .fetch_one(catalog.pool())
            .await
            .unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(&kind_observed_at).is_ok());
}

#[tokio::test]
async fn keeps_supported_empty_distinct_from_unsupported_and_failed() {
    let catalog = catalog().await;
    let states = vec![
        KindObservation::new(
            CapabilityKind::Tools,
            DeclarationState::Supported,
            InventoryState::Complete,
        ),
        KindObservation::new(
            CapabilityKind::Prompts,
            DeclarationState::Unsupported,
            InventoryState::Complete,
        ),
        KindObservation::new(
            CapabilityKind::Resources,
            DeclarationState::Supported,
            InventoryState::Failed,
        )
        .with_error("resources/list timed out"),
        KindObservation::new(
            CapabilityKind::ResourceTemplates,
            DeclarationState::Unknown,
            InventoryState::Unknown,
        ),
    ];
    let observation = CapabilityObservation::new(
        "server-empty",
        "empty-server",
        "config-v1",
        initialize_result(),
        states.clone(),
        Vec::new(),
    );

    catalog.commit_observation(observation).await.unwrap();
    let loaded = catalog.load_snapshot("server-empty").await.unwrap().unwrap();

    assert_eq!(loaded.kind_states, states);
}

#[tokio::test]
async fn permits_the_same_external_key_in_distinct_capability_kinds() {
    let pool = test_pool().await;
    let catalog = SqliteCapabilityCatalog::new(pool);
    let mut observation = complete_observation("server-a");
    observation.records = vec![
        CatalogRecord::new(
            "tool-stable",
            "status",
            "server_a_status",
            CapabilityPayload::Tool(test_tool("status")),
        ),
        CatalogRecord::new(
            "prompt-stable",
            "status",
            "server_a_status",
            CapabilityPayload::Prompt(test_prompt("status")),
        ),
    ];

    catalog
        .commit_observation(observation)
        .await
        .expect("commit observation");
    let snapshot = catalog
        .load_snapshot("server-a")
        .await
        .expect("load snapshot")
        .expect("snapshot");
    assert_eq!(snapshot.records.len(), 2);
}

#[tokio::test]
async fn rejects_unknown_record_format_version() {
    let catalog = catalog().await;
    let observation = CapabilityObservation::new(
        "server-version",
        "fixture-server",
        "config-v1",
        initialize_result(),
        complete_states(),
        full_records(),
    );
    catalog.commit_observation(observation).await.unwrap();
    sqlx::query("UPDATE capability_server_snapshots SET record_format_version = 99 WHERE server_id = ?")
        .bind("server-version")
        .execute(catalog.pool())
        .await
        .unwrap();

    let error = catalog.load_snapshot("server-version").await.unwrap_err();

    assert!(error.to_string().contains("unsupported record format version 99"));
}

#[tokio::test]
async fn rolls_back_snapshot_when_record_insert_fails() {
    let catalog = catalog().await;
    sqlx::query(
        "CREATE TRIGGER fail_catalog_record BEFORE INSERT ON capability_records BEGIN SELECT RAISE(ABORT, 'fixture failure'); END",
    )
    .execute(catalog.pool())
    .await
    .unwrap();
    let observation = CapabilityObservation::new(
        "server-rollback",
        "fixture-server",
        "config-v1",
        initialize_result(),
        complete_states(),
        full_records(),
    );

    assert!(catalog.commit_observation(observation).await.is_err());
    assert!(catalog.load_snapshot("server-rollback").await.unwrap().is_none());

    let kind_count: i64 = sqlx::query("SELECT COUNT(*) AS count FROM capability_kind_states WHERE server_id = ?")
        .bind("server-rollback")
        .fetch_one(catalog.pool())
        .await
        .unwrap()
        .get("count");
    assert_eq!(kind_count, 0);
}

#[tokio::test]
async fn concurrent_writers_from_independent_pools_commit_consecutive_revisions() {
    let directory = tempfile::tempdir().unwrap();
    let database_url = format!("sqlite://{}", directory.path().join("catalog.db").display());
    let options = || {
        SqliteConnectOptions::from_str(&database_url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true)
    };
    let first = SqliteCapabilityCatalog::new(
        SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options())
            .await
            .unwrap(),
    );
    first.ensure_schema().await.unwrap();
    let second = SqliteCapabilityCatalog::new(
        SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options())
            .await
            .unwrap(),
    );
    let observation = |fingerprint: &str| {
        CapabilityObservation::new(
            "server-concurrent",
            "fixture-server",
            fingerprint,
            initialize_result(),
            complete_states(),
            full_records(),
        )
    };

    let (left, right) = tokio::join!(
        first.commit_observation(observation("config-a")),
        second.commit_observation(observation("config-b")),
    );

    let mut revisions = vec![left.unwrap().revision, right.unwrap().revision];
    revisions.sort_unstable();
    assert_eq!(revisions, vec![1, 2]);
    assert_eq!(
        first
            .load_snapshot("server-concurrent")
            .await
            .unwrap()
            .unwrap()
            .revision,
        2
    );
}

#[tokio::test]
async fn lifecycle_updates_preserve_payload_and_advance_revision() {
    let catalog = catalog().await;
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-lifecycle",
            "fixture-server",
            "config-v1",
            initialize_result(),
            complete_states(),
            full_records(),
        ))
        .await
        .unwrap();

    let invalidated = catalog
        .invalidate_server("server-lifecycle", "configuration changed")
        .await
        .unwrap();
    assert_eq!(invalidated.revision, 2);
    let snapshot = catalog.load_snapshot("server-lifecycle").await.unwrap().unwrap();
    assert_eq!(snapshot.state, SnapshotState::Invalidated);
    assert_eq!(snapshot.records, full_records());

    let unavailable = catalog
        .record_failure("server-lifecycle", Some(CapabilityKind::Tools), "transport closed")
        .await
        .unwrap();
    assert_eq!(unavailable.revision, 3);
    let snapshot = catalog.load_snapshot("server-lifecycle").await.unwrap().unwrap();
    assert_eq!(snapshot.state, SnapshotState::Unavailable);
    assert_eq!(snapshot.last_error.as_deref(), Some("transport closed"));
    assert_eq!(
        snapshot
            .kind_states
            .iter()
            .find(|state| state.kind == CapabilityKind::Tools)
            .unwrap()
            .inventory,
        InventoryState::Failed
    );
    assert_eq!(snapshot.records, full_records());

    let stats = catalog.stats().await.unwrap();
    assert_eq!(stats.snapshots, 1);
    assert_eq!(stats.unavailable_snapshots, 1);
    assert_eq!(stats.records, 4);

    catalog.remove_server("server-lifecycle").await.unwrap();
    assert!(catalog.load_snapshot("server-lifecycle").await.unwrap().is_none());
    assert_eq!(catalog.stats().await.unwrap().records, 0);
}

#[tokio::test]
async fn caller_owned_transaction_rolls_back_catalog_with_other_sqlite_state() {
    let catalog = catalog().await;
    sqlx::query("CREATE TABLE projection_guard (server_id TEXT PRIMARY KEY, revision INTEGER NOT NULL)")
        .execute(catalog.pool())
        .await
        .unwrap();
    let mut transaction = catalog.pool().begin_with("BEGIN IMMEDIATE").await.unwrap();
    let commit = catalog
        .commit_observation_in_transaction(
            &mut transaction,
            CapabilityObservation::new(
                "server-atomic",
                "fixture-server",
                "config-v1",
                initialize_result(),
                complete_states(),
                full_records(),
            ),
        )
        .await
        .unwrap();
    sqlx::query("INSERT INTO projection_guard (server_id, revision) VALUES (?, ?)")
        .bind(&commit.server_id)
        .bind(commit.revision)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.rollback().await.unwrap();

    assert!(catalog.load_snapshot("server-atomic").await.unwrap().is_none());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projection_guard")
        .fetch_one(catalog.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn invalidate_all_preserves_payload_and_advances_each_revision_atomically() {
    let catalog = catalog().await;
    for server_id in ["server-alpha", "server-beta"] {
        let records = full_records()
            .into_iter()
            .map(|mut record| {
                record.stable_id = format!("{server_id}:{}", record.stable_id);
                record.external_key = format!("{server_id}:{}", record.external_key);
                record
            })
            .collect();
        catalog
            .commit_observation(CapabilityObservation::new(
                server_id,
                format!("{server_id}-name"),
                "config-v1",
                initialize_result(),
                complete_states(),
                records,
            ))
            .await
            .unwrap();
    }

    let invalidated = catalog.invalidate_all("explicit reset").await.unwrap();

    assert_eq!(invalidated.len(), 2);
    for server_id in ["server-alpha", "server-beta"] {
        let snapshot = catalog.load_snapshot(server_id).await.unwrap().unwrap();
        assert_eq!(snapshot.state, SnapshotState::Invalidated);
        assert_eq!(snapshot.revision, 2);
        assert_eq!(snapshot.records.len(), full_records().len());
        assert!(
            snapshot
                .records
                .iter()
                .all(|record| record.stable_id.starts_with(server_id))
        );
        assert_eq!(
            snapshot
                .records
                .iter()
                .map(|record| &record.payload)
                .collect::<Vec<_>>(),
            full_records().iter().map(|record| &record.payload).collect::<Vec<_>>()
        );
        assert_eq!(snapshot.last_error.as_deref(), Some("explicit reset"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_readers_observe_atomic_server_local_revisions() {
    const SERVER_COUNT: usize = 4;
    const READER_COUNT: usize = 32;
    const WRITES_PER_SERVER: usize = 4;

    let directory = tempfile::tempdir().expect("create catalog directory");
    let database_url = format!("sqlite://{}", directory.path().join("concurrent-catalog.db").display());
    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("parse catalog URL")
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(10))
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(16)
        .connect_with(options)
        .await
        .expect("open concurrent catalog");
    let catalog = Arc::new(SqliteCapabilityCatalog::new(pool));
    catalog.ensure_schema().await.expect("initialize catalog schema");
    let cache = Arc::new(DerivedCapabilityCache::new(32, 32));
    let server_ids = (0..SERVER_COUNT)
        .map(|index| format!("server-concurrency-{index}"))
        .collect::<Vec<_>>();
    for server_id in &server_ids {
        catalog
            .commit_observation(versioned_observation(server_id, 0))
            .await
            .expect("seed server catalog");
    }

    let start = Arc::new(Barrier::new(READER_COUNT + SERVER_COUNT + 1));
    let mut readers = Vec::with_capacity(READER_COUNT);
    for reader_index in 0..READER_COUNT {
        let catalog = catalog.clone();
        let cache = cache.clone();
        let start = start.clone();
        let server_id = server_ids[reader_index % SERVER_COUNT].clone();
        readers.push(tokio::spawn(async move {
            start.wait().await;
            for _ in 0..24 {
                let loader_catalog = catalog.clone();
                let loader_server_id = server_id.clone();
                let snapshot = cache
                    .get_or_load_current_snapshot(&server_id, || async move {
                        loader_catalog.load_snapshot(&loader_server_id).await
                    })
                    .await
                    .expect("load concurrent snapshot")
                    .expect("concurrent snapshot exists");
                assert_complete_version(&snapshot);
                tokio::task::yield_now().await;
            }
        }));
    }

    let mut writers = Vec::with_capacity(SERVER_COUNT);
    for server_id in &server_ids {
        let catalog = catalog.clone();
        let cache = cache.clone();
        let start = start.clone();
        let server_id = server_id.clone();
        writers.push(tokio::spawn(async move {
            start.wait().await;
            for version in 1..=WRITES_PER_SERVER {
                let commit = catalog
                    .commit_observation(versioned_observation(&server_id, version))
                    .await
                    .expect("commit sequential server observation");
                assert_eq!(commit.revision, version as i64 + 1);
                cache.invalidate_server(&server_id).await;
                tokio::task::yield_now().await;
            }
        }));
    }

    start.wait().await;
    for writer in writers {
        writer.await.expect("join sequential writer");
    }
    for reader in readers {
        reader.await.expect("join concurrent reader");
    }

    for server_id in &server_ids {
        let snapshot = catalog
            .load_snapshot(server_id)
            .await
            .expect("load final snapshot")
            .expect("final snapshot exists");
        assert_eq!(snapshot.revision, WRITES_PER_SERVER as i64 + 1);
        assert_complete_version(&snapshot);
    }

    let unaffected_revisions = server_ids[1..]
        .iter()
        .map(|server_id| async {
            (
                server_id.clone(),
                catalog
                    .load_snapshot(server_id)
                    .await
                    .expect("load unaffected baseline")
                    .expect("unaffected baseline exists")
                    .revision,
            )
        })
        .collect::<Vec<_>>();
    let mut unaffected_baselines = Vec::with_capacity(unaffected_revisions.len());
    for revision in unaffected_revisions {
        unaffected_baselines.push(revision.await);
    }
    catalog
        .record_failure(
            &server_ids[0],
            Some(CapabilityKind::Tools),
            "isolated concurrent failure",
        )
        .await
        .expect("record isolated failure");
    cache.invalidate_server(&server_ids[0]).await;
    let failed = catalog
        .load_snapshot(&server_ids[0])
        .await
        .expect("load failed server")
        .expect("failed snapshot exists");
    assert_eq!(failed.state, SnapshotState::Unavailable);
    assert_eq!(failed.revision, WRITES_PER_SERVER as i64 + 2);
    for (server_id, revision) in unaffected_baselines {
        let unaffected = catalog
            .load_snapshot(&server_id)
            .await
            .expect("reload unaffected server")
            .expect("unaffected snapshot remains");
        assert_eq!(unaffected.state, SnapshotState::Ready);
        assert_eq!(unaffected.revision, revision);
        assert_complete_version(&unaffected);
    }

    let projection_key = ProjectionKey::new(
        "concurrent-surface",
        "concurrent-fingerprint",
        CapabilityKind::Tools,
        ProjectionNameDomain::External,
        "concurrent-revision-set",
    );
    let stale_projection = ProjectionPayload::Tools(vec![test_tool("stale")]);
    let stale_projection_for_loader = stale_projection.clone();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let stale_cache = cache.clone();
    let stale_key = projection_key.clone();
    let stale_task = tokio::spawn(async move {
        stale_cache
            .get_or_project(stale_key, || async move {
                started_tx.send(()).expect("signal stale projection load");
                release_rx.await.expect("release stale projection load");
                Ok::<_, &'static str>(stale_projection_for_loader)
            })
            .await
            .expect("load stale projection")
    });
    started_rx.await.expect("stale projection load started");
    cache.invalidate_server(&server_ids[0]).await;
    release_tx.send(()).expect("release stale projection loader");
    assert_eq!(
        stale_task.await.expect("join stale projection loader").as_ref(),
        &stale_projection
    );

    let fresh_projection = ProjectionPayload::Tools(vec![test_tool("fresh")]);
    let fresh_result = cache
        .get_or_project(projection_key, || {
            let fresh_projection = fresh_projection.clone();
            async move { Ok::<_, &'static str>(fresh_projection) }
        })
        .await
        .expect("load fresh projection");
    assert_eq!(fresh_result.as_ref(), &fresh_projection);
}
