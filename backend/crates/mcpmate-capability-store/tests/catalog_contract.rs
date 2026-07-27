use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityId, CapabilityKind, CapabilityObservation, CapabilityPayload, CapabilityRefId,
    CapabilityRefState, CatalogRecord, DeclarationState, DerivedCapabilityCache, EffectiveCapabilityRecordV1,
    InventoryState, KindObservation, ProjectionKey, ProjectionNameDomain, ProjectionPayload, SnapshotState,
    SqliteCapabilityCatalog,
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

fn full_records(server_id: &str) -> Vec<CatalogRecord> {
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
        CatalogRecord::materialize(
            server_id,
            "analyze",
            "fixture-server__analyze",
            CapabilityPayload::Tool(tool),
        )
        .unwrap(),
        CatalogRecord::materialize(
            server_id,
            "summarize",
            "fixture-server__summarize",
            CapabilityPayload::Prompt(prompt),
        )
        .unwrap(),
        CatalogRecord::materialize(
            server_id,
            "file:///fixture/report.md",
            "mcpmate://fixture-server/resources/resource-1",
            CapabilityPayload::Resource(resource),
        )
        .unwrap(),
        CatalogRecord::materialize(
            server_id,
            "file:///fixture/{name}.md",
            "mcpmate://fixture-server/resource-templates/template-1",
            CapabilityPayload::ResourceTemplate(template),
        )
        .unwrap(),
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
        CatalogRecord::materialize(
            server_id,
            marker.clone(),
            format!("{marker}-external-tool"),
            CapabilityPayload::Tool(tool),
        )
        .unwrap(),
        CatalogRecord::materialize(
            server_id,
            marker.clone(),
            format!("{marker}-external-prompt"),
            CapabilityPayload::Prompt(prompt),
        )
        .unwrap(),
        CatalogRecord::materialize(
            server_id,
            format!("fixture://{marker}/item"),
            format!("mcpmate://{marker}/resource"),
            CapabilityPayload::Resource(resource),
        )
        .unwrap(),
        CatalogRecord::materialize(
            server_id,
            format!("fixture://{marker}/{{item}}"),
            format!("mcpmate://{marker}/template"),
            CapabilityPayload::ResourceTemplate(template),
        )
        .unwrap(),
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
            .all(|record| record.external_key.contains(&snapshot.config_fingerprint)),
        "snapshot mixed records from different observations: {snapshot:?}"
    );
}

async fn catalog() -> SqliteCapabilityCatalog {
    SqliteCapabilityCatalog::new(test_pool().await)
}

fn materialized_tool_record(description: &str) -> CatalogRecord {
    CatalogRecord::materialize(
        "server-versioned",
        "analyze",
        "server_versioned__analyze",
        CapabilityPayload::Tool(decode(json!({
            "name": "analyze",
            "description": description,
            "inputSchema": {"type": "object"}
        }))),
    )
    .expect("materialize tool record")
}

fn tools_observation(
    inventory: InventoryState,
    records: Vec<CatalogRecord>,
) -> CapabilityObservation {
    CapabilityObservation::new(
        "server-versioned",
        "Versioned Server",
        "config-v1",
        initialize_result(),
        vec![KindObservation::new(
            CapabilityKind::Tools,
            DeclarationState::Supported,
            inventory,
        )],
        records,
    )
}

#[tokio::test]
async fn same_ref_changed_content_appends_version_and_advances_only_current_pointer() {
    let catalog = catalog().await;
    let first_record = materialized_tool_record("Version one");
    let second_record = materialized_tool_record("Version two");
    assert_eq!(first_record.ref_id, second_record.ref_id);
    assert_ne!(first_record.capability_id, second_record.capability_id);

    catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![first_record.clone()]))
        .await
        .expect("commit first version");

    let mut transaction = catalog.pool().begin_with("BEGIN IMMEDIATE").await.unwrap();
    let reconciliation = catalog
        .reconcile_observation_in(
            &mut transaction,
            tools_observation(InventoryState::Complete, vec![second_record.clone()]),
        )
        .await
        .expect("reconcile second version");
    transaction.commit().await.unwrap();

    assert!(reconciliation.delta.added_refs.is_empty());
    assert_eq!(reconciliation.delta.changed_versions.len(), 1);
    assert_eq!(
        reconciliation.delta.changed_versions[0].before_capability_id,
        first_record.capability_id
    );
    assert_eq!(
        reconciliation.delta.changed_versions[0].target_capability_id,
        second_record.capability_id
    );
    assert_eq!(reconciliation.delta.kind_completeness.len(), 1);
    assert_eq!(reconciliation.delta.kind_completeness[0].kind, CapabilityKind::Tools);
    assert_eq!(
        reconciliation.delta.kind_completeness[0].inventory,
        InventoryState::Complete
    );

    let history = catalog
        .load_version_history(&first_record.ref_id)
        .await
        .expect("load version history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].capability_id, first_record.capability_id);
    assert_eq!(history[1].capability_id, second_record.capability_id);

    let current = catalog.load_snapshot("server-versioned").await.unwrap().unwrap();
    assert_eq!(current.records, vec![second_record]);
}

#[tokio::test]
async fn same_ref_same_content_does_not_append_version() {
    let catalog = catalog().await;
    let record = materialized_tool_record("Stable content");

    catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![record.clone()]))
        .await
        .unwrap();
    let mut transaction = catalog.pool().begin_with("BEGIN IMMEDIATE").await.unwrap();
    let reconciliation = catalog
        .reconcile_observation_in(
            &mut transaction,
            tools_observation(InventoryState::Complete, vec![record.clone()]),
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    assert_eq!(reconciliation.delta.unchanged_refs, vec![record.ref_id.clone()]);
    assert!(reconciliation.delta.changed_versions.is_empty());
    assert_eq!(catalog.load_version_history(&record.ref_id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn identical_observation_is_catalog_noop_even_when_record_order_changes() {
    let catalog = catalog().await;
    let records = full_records("server-stable");
    let first = CapabilityObservation::new(
        "server-stable",
        "Stable Server",
        "config-v1",
        initialize_result(),
        complete_states(),
        records.clone(),
    );
    let first_commit = catalog.commit_observation(first).await.unwrap();
    assert_eq!(first_commit.revision, 1);
    assert!(first_commit.changed);

    let second = CapabilityObservation::new(
        "server-stable",
        "Stable Server",
        "config-v1",
        initialize_result(),
        complete_states(),
        records.into_iter().rev().collect(),
    );
    let second_commit = catalog.commit_observation(second).await.unwrap();

    assert_eq!(
        second_commit.revision, first_commit.revision,
        "an identical logical observation must not advance the catalog revision"
    );
    assert!(!second_commit.changed);
    assert_eq!(
        catalog.load_snapshot("server-stable").await.unwrap().unwrap().revision,
        first_commit.revision
    );
}

#[tokio::test]
async fn identical_scoped_observations_only_refresh_the_probed_kind_evidence() {
    let catalog = catalog().await;
    let baseline_at = chrono::DateTime::parse_from_rfc3339("2026-07-26T08:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let mut baseline = CapabilityObservation::new(
        "server-scoped",
        "Scoped Server",
        "config-v1",
        initialize_result(),
        complete_states(),
        full_records("server-scoped"),
    );
    baseline.observed_at = baseline_at;
    let initial = catalog.commit_observation(baseline).await.unwrap();
    assert_eq!(initial.revision, 1);
    assert!(initial.changed);
    let committed_at: String =
        sqlx::query_scalar("SELECT committed_at FROM capability_server_snapshots WHERE server_id = ?")
            .bind("server-scoped")
            .fetch_one(catalog.pool())
            .await
            .unwrap();

    for (offset, kind) in CapabilityKind::ALL.into_iter().enumerate() {
        let mut scoped = CapabilityObservation::new(
            "server-scoped",
            "Scoped Server",
            "config-v1",
            initialize_result(),
            complete_states(),
            full_records("server-scoped"),
        )
        .with_observed_kinds([kind]);
        scoped.observed_at = baseline_at + chrono::Duration::seconds((offset + 1) as i64);
        let commit = catalog.commit_observation(scoped).await.unwrap();
        assert_eq!(commit.revision, initial.revision);
        assert!(!commit.changed);
    }

    let observed_at = sqlx::query_as::<_, (String, String)>(
        "SELECT kind, observed_at FROM capability_kind_states WHERE server_id = ? ORDER BY kind",
    )
    .bind("server-scoped")
    .fetch_all(catalog.pool())
    .await
    .unwrap()
    .into_iter()
    .collect::<std::collections::HashMap<_, _>>();
    for (offset, kind) in CapabilityKind::ALL.into_iter().enumerate() {
        assert_eq!(
            observed_at.get(kind.as_str()).unwrap(),
            &(baseline_at + chrono::Duration::seconds((offset + 1) as i64)).to_rfc3339()
        );
    }
    let committed_at_after: String =
        sqlx::query_scalar("SELECT committed_at FROM capability_server_snapshots WHERE server_id = ?")
            .bind("server-scoped")
            .fetch_one(catalog.pool())
            .await
            .unwrap();
    assert_eq!(committed_at_after, committed_at);
}

#[tokio::test]
async fn ordinary_initialize_metadata_does_not_change_catalog_semantics() {
    let catalog = catalog().await;
    let initial_initialize = initialize_result();
    let first = CapabilityObservation::new(
        "server-metadata",
        "Metadata Server",
        "config-v1",
        initial_initialize.clone(),
        complete_states(),
        full_records("server-metadata"),
    );
    let first_commit = catalog.commit_observation(first).await.unwrap();

    let mut refreshed_initialize = initial_initialize.clone();
    refreshed_initialize.server_info.title = Some("Updated display title".to_string());
    refreshed_initialize.server_info.version = "9.9.9".to_string();
    refreshed_initialize.instructions = Some("Updated operational instructions".to_string());
    let second = CapabilityObservation::new(
        "server-metadata",
        "Metadata Server",
        "config-v1",
        refreshed_initialize,
        complete_states(),
        full_records("server-metadata"),
    );
    let second_commit = catalog.commit_observation(second).await.unwrap();

    assert_eq!(second_commit.revision, first_commit.revision);
    assert!(!second_commit.changed);
    assert_eq!(
        catalog
            .load_snapshot("server-metadata")
            .await
            .unwrap()
            .unwrap()
            .initialize,
        Some(initial_initialize)
    );
}

#[tokio::test]
async fn origin_key_change_creates_new_ref_and_unresolves_old_ref_only_when_complete() {
    let catalog = catalog().await;
    let old_record = materialized_tool_record("Stable content");
    let mut renamed_payload = old_record.payload.clone();
    if let CapabilityPayload::Tool(tool) = &mut renamed_payload {
        tool.name = "analyze-v2".to_string().into();
    }
    let new_record = CatalogRecord::materialize(
        "server-versioned",
        "analyze-v2",
        "server_versioned__analyze",
        renamed_payload,
    )
    .unwrap();
    assert_ne!(old_record.ref_id, new_record.ref_id);

    catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![old_record.clone()]))
        .await
        .unwrap();
    catalog
        .commit_observation(tools_observation(InventoryState::Failed, vec![new_record.clone()]))
        .await
        .unwrap();
    assert_eq!(
        catalog.load_ref(&old_record.ref_id).await.unwrap().unwrap().state,
        CapabilityRefState::Active
    );
    assert!(catalog.load_ref(&new_record.ref_id).await.unwrap().is_none());

    let reconciliation = catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![new_record.clone()]))
        .await
        .unwrap();
    assert_eq!(reconciliation.revision, 3);
    assert_eq!(
        catalog.load_ref(&old_record.ref_id).await.unwrap().unwrap().state,
        CapabilityRefState::Unresolved
    );
    assert_eq!(
        catalog.load_ref(&new_record.ref_id).await.unwrap().unwrap().state,
        CapabilityRefState::Active
    );
}

#[tokio::test]
async fn duplicate_origin_tuple_rolls_back_the_observation() {
    let catalog = catalog().await;
    let record = materialized_tool_record("Stable content");
    let error = catalog
        .commit_observation(tools_observation(
            InventoryState::Complete,
            vec![record.clone(), record],
        ))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("duplicate capability origin"));
    assert!(catalog.load_snapshot("server-versioned").await.unwrap().is_none());
}

#[tokio::test]
async fn invalid_effective_record_format_rolls_back_the_observation() {
    let catalog = catalog().await;
    let mut record = materialized_tool_record("Stable content");
    let mut wire: Value = serde_json::from_slice(&record.canonical_record).unwrap();
    wire["format"] = Value::String("mcpmate.effective-capability.v2".to_string());
    let invalid_record: EffectiveCapabilityRecordV1 = serde_json::from_value(wire).unwrap();
    record.capability_id = CapabilityId::derive(&invalid_record).unwrap();
    record.canonical_record = invalid_record.canonical_bytes().unwrap();

    let error = catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![record]))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("effective capability format"));
    assert!(catalog.load_snapshot("server-versioned").await.unwrap().is_none());
}

#[tokio::test]
async fn complete_removal_and_reappearance_advance_ref_state_generation_once_each() {
    let catalog = catalog().await;
    let record = materialized_tool_record("Stable content");
    catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![record.clone()]))
        .await
        .unwrap();

    catalog
        .commit_observation(tools_observation(InventoryState::Complete, Vec::new()))
        .await
        .unwrap();
    let missing = catalog.load_ref(&record.ref_id).await.unwrap().unwrap();
    assert_eq!(missing.state, CapabilityRefState::Unresolved);
    assert_eq!(missing.state_generation, 1);

    catalog
        .commit_observation(tools_observation(InventoryState::Complete, Vec::new()))
        .await
        .unwrap();
    let repeated_missing = catalog.load_ref(&record.ref_id).await.unwrap().unwrap();
    assert_eq!(repeated_missing.state_generation, 1);

    catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![record.clone()]))
        .await
        .unwrap();
    let reappeared = catalog.load_ref(&record.ref_id).await.unwrap().unwrap();
    assert_eq!(reappeared.state, CapabilityRefState::Active);
    assert_eq!(reappeared.state_generation, 2);

    catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![record.clone()]))
        .await
        .unwrap();
    let repeated_active = catalog.load_ref(&record.ref_id).await.unwrap().unwrap();
    assert_eq!(repeated_active.state_generation, 2);
}

#[tokio::test]
async fn failed_inventory_never_marks_missing_refs_unresolved() {
    let catalog = catalog().await;
    let record = materialized_tool_record("Stable content");
    catalog
        .commit_observation(tools_observation(InventoryState::Complete, vec![record.clone()]))
        .await
        .unwrap();

    catalog
        .commit_observation(tools_observation(InventoryState::Failed, Vec::new()))
        .await
        .unwrap();

    let preserved = catalog.load_ref(&record.ref_id).await.unwrap().unwrap();
    assert_eq!(preserved.state, CapabilityRefState::Active);
    assert_eq!(preserved.state_generation, 0);
    assert_eq!(
        catalog
            .load_snapshot("server-versioned")
            .await
            .unwrap()
            .unwrap()
            .records,
        vec![record]
    );
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
        full_records("server-1"),
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
        CatalogRecord::materialize(
            "server-a",
            "status",
            "server_a_status",
            CapabilityPayload::Tool(test_tool("status")),
        )
        .unwrap(),
        CatalogRecord::materialize(
            "server-a",
            "status",
            "server_a_status",
            CapabilityPayload::Prompt(test_prompt("status")),
        )
        .unwrap(),
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
        full_records("server-version"),
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
async fn rejects_unknown_effective_record_format_metadata() {
    let catalog = catalog().await;
    let observation = CapabilityObservation::new(
        "server-version",
        "fixture-server",
        "config-v1",
        initialize_result(),
        complete_states(),
        full_records("server-version"),
    );
    catalog.commit_observation(observation).await.unwrap();
    sqlx::query("UPDATE capability_versions SET record_format = 'mcpmate.effective-capability.v2'")
        .execute(catalog.pool())
        .await
        .unwrap();

    let error = catalog.load_snapshot("server-version").await.unwrap_err();

    assert!(error.to_string().contains("unsupported effective capability format"));
}

#[tokio::test]
async fn rolls_back_snapshot_when_record_insert_fails() {
    let catalog = catalog().await;
    sqlx::query(
        "CREATE TRIGGER fail_catalog_record BEFORE INSERT ON capability_versions BEGIN SELECT RAISE(ABORT, 'fixture failure'); END",
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
        full_records("server-rollback"),
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
            full_records("server-concurrent"),
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
async fn concurrent_identical_writers_produce_one_change_and_one_noop() {
    let directory = tempfile::tempdir().unwrap();
    let database_url = format!("sqlite://{}", directory.path().join("catalog-noop.db").display());
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
    let observation = || {
        CapabilityObservation::new(
            "server-concurrent-noop",
            "fixture-server",
            "config-v1",
            initialize_result(),
            complete_states(),
            full_records("server-concurrent-noop"),
        )
    };

    let (left, right) = tokio::join!(
        first.commit_observation(observation()),
        second.commit_observation(observation()),
    );
    let commits = [left.unwrap(), right.unwrap()];

    assert_eq!(commits.iter().filter(|commit| commit.changed).count(), 1);
    assert_eq!(commits.iter().filter(|commit| !commit.changed).count(), 1);
    assert!(commits.iter().all(|commit| commit.revision == 1));
    assert_eq!(
        first
            .load_snapshot("server-concurrent-noop")
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn rejects_ref_row_source_mismatch() {
    let catalog = catalog().await;
    let observation = CapabilityObservation::new(
        "server-source",
        "fixture-server",
        "config-v1",
        initialize_result(),
        complete_states(),
        full_records("server-source"),
    );
    catalog.commit_observation(observation).await.unwrap();
    sqlx::query("UPDATE capability_refs SET origin_key = 'tampered' WHERE server_id = 'server-source'")
        .execute(catalog.pool())
        .await
        .unwrap();

    let error = catalog.load_snapshot("server-source").await.unwrap_err();

    assert!(error.to_string().contains("integrity mismatch"));
}

#[tokio::test]
async fn remove_server_retires_refs_and_preserves_history_without_foreign_keys() {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let foreign_keys_enabled: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        foreign_keys_enabled, 0,
        "this test only proves anything with cascade disabled"
    );

    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-no-cascade",
            "fixture-server",
            "config-v1",
            initialize_result(),
            complete_states(),
            full_records("server-no-cascade"),
        ))
        .await
        .unwrap();

    catalog.remove_server("server-no-cascade").await.unwrap();

    let retired = catalog.load_snapshot("server-no-cascade").await.unwrap().unwrap();
    assert_eq!(retired.state, SnapshotState::Unavailable);
    assert!(retired.records.is_empty());
    let stats = catalog.stats().await.unwrap();
    assert_eq!(stats.records, 0, "retired refs must not count as active");
    let retired_refs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM capability_refs WHERE state = 'retired'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(retired_refs, 4);
    let retained_versions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM capability_versions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(retained_versions, 4);
}

#[tokio::test]
async fn record_failure_creates_the_kind_state_row_when_it_never_existed() {
    // A server whose committed `kind_states` never included a given kind (e.g. it was never
    // discovered or its declaration was omitted) has no `capability_kind_states` row for that
    // kind. `record_failure` must still surface the failure for that kind instead of silently
    // affecting zero rows (Codex review follow-up, PR #160).
    let catalog = catalog().await;
    let partial_states = vec![
        KindObservation::new(
            CapabilityKind::Tools,
            DeclarationState::Supported,
            InventoryState::Complete,
        ),
        KindObservation::new(
            CapabilityKind::Prompts,
            DeclarationState::Supported,
            InventoryState::Complete,
        ),
    ];
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-partial-kinds",
            "fixture-server",
            "config-v1",
            initialize_result(),
            partial_states,
            Vec::new(),
        ))
        .await
        .unwrap();
    let before = catalog.load_snapshot("server-partial-kinds").await.unwrap().unwrap();
    assert!(
        !before
            .kind_states
            .iter()
            .any(|state| state.kind == CapabilityKind::Resources),
        "fixture must start without a Resources kind_states row"
    );

    catalog
        .record_failure(
            "server-partial-kinds",
            Some(CapabilityKind::Resources),
            "session closed",
        )
        .await
        .unwrap();

    let after = catalog.load_snapshot("server-partial-kinds").await.unwrap().unwrap();
    let resources_state = after
        .kind_states
        .iter()
        .find(|state| state.kind == CapabilityKind::Resources)
        .expect("record_failure must create the missing Resources kind_states row");
    assert_eq!(resources_state.inventory, InventoryState::Failed);
    assert_eq!(resources_state.error.as_deref(), Some("session closed"));
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
            full_records("server-lifecycle"),
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
    assert_eq!(snapshot.records, full_records("server-lifecycle"));

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
    assert_eq!(snapshot.records, full_records("server-lifecycle"));

    let stats = catalog.stats().await.unwrap();
    assert_eq!(stats.snapshots, 1);
    assert_eq!(stats.unavailable_snapshots, 1);
    assert_eq!(stats.records, 4);

    catalog.remove_server("server-lifecycle").await.unwrap();
    let retired = catalog.load_snapshot("server-lifecycle").await.unwrap().unwrap();
    assert_eq!(retired.state, SnapshotState::Unavailable);
    assert!(retired.records.is_empty());
    assert_eq!(catalog.stats().await.unwrap().records, 0);
}

#[tokio::test]
async fn repeated_failure_with_the_same_semantic_state_is_catalog_noop() {
    let catalog = catalog().await;
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-repeat-failure",
            "fixture-server",
            "config-v1",
            initialize_result(),
            complete_states(),
            full_records("server-repeat-failure"),
        ))
        .await
        .unwrap();

    let first_failure = catalog
        .record_failure(
            "server-repeat-failure",
            Some(CapabilityKind::Tools),
            "connection attempt one failed",
        )
        .await
        .unwrap();
    assert!(first_failure.changed);

    let repeated_failure = catalog
        .record_failure(
            "server-repeat-failure",
            Some(CapabilityKind::Tools),
            "connection attempt two failed",
        )
        .await
        .unwrap();

    assert_eq!(repeated_failure.revision, first_failure.revision);
    assert!(!repeated_failure.changed);
    let snapshot = catalog.load_snapshot("server-repeat-failure").await.unwrap().unwrap();
    assert_eq!(
        snapshot.last_error.as_deref(),
        Some("connection attempt two failed"),
        "diagnostic evidence should refresh without changing catalog semantics"
    );
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
                full_records("server-atomic"),
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
        let records = full_records(server_id);
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
        let expected_records = full_records(server_id);
        assert_eq!(snapshot.records.len(), expected_records.len());
        assert!(snapshot.records.iter().all(|record| {
            record.ref_id
                == CapabilityRefId::derive(&mcpmate_capability_store::CapabilitySourceIdentity::new(
                    server_id,
                    record.kind(),
                    &record.upstream_key,
                ))
                .unwrap()
        }));
        assert_eq!(
            snapshot
                .records
                .iter()
                .map(|record| &record.payload)
                .collect::<Vec<_>>(),
            expected_records
                .iter()
                .map(|record| &record.payload)
                .collect::<Vec<_>>()
        );
        assert_eq!(snapshot.last_error.as_deref(), Some("explicit reset"));
    }

    let repeated = catalog.invalidate_all("repeated reset").await.unwrap();
    assert!(repeated.is_empty());
    for server_id in ["server-alpha", "server-beta"] {
        let snapshot = catalog.load_snapshot(server_id).await.unwrap().unwrap();
        assert_eq!(snapshot.revision, 2);
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

#[tokio::test]
async fn schema_initialization_rejects_legacy_capability_tables() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect legacy schema fixture");
    sqlx::query("CREATE TABLE capability_records (id TEXT PRIMARY KEY)")
        .execute(&pool)
        .await
        .expect("create legacy capability table");

    let error = SqliteCapabilityCatalog::new(pool)
        .ensure_schema()
        .await
        .expect_err("legacy schemas must require a clean rebuild");

    assert!(matches!(
        error,
        mcpmate_capability_store::CatalogError::IncompatibleSchema { .. }
    ));
}

#[tokio::test]
async fn schema_initialization_rejects_an_unknown_capability_epoch() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect epoch fixture");
    sqlx::query(
        r#"
        CREATE TABLE capability_schema_metadata (
            singleton INTEGER PRIMARY KEY,
            schema_epoch INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("create schema metadata");
    sqlx::query("INSERT INTO capability_schema_metadata (singleton, schema_epoch) VALUES (1, 999)")
        .execute(&pool)
        .await
        .expect("seed unsupported epoch");

    let error = SqliteCapabilityCatalog::new(pool)
        .ensure_schema()
        .await
        .expect_err("unknown schema epochs must require a clean rebuild");

    assert!(matches!(
        error,
        mcpmate_capability_store::CatalogError::IncompatibleSchema { .. }
    ));
}
