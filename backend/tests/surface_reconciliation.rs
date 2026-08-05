use std::sync::Arc;

use async_trait::async_trait;
use mcpmate::core::capability::{
    management::{ProfileRelationshipAction, ProfileSurfaceManagement},
    reconciliation::{
        CatalogSurfaceReconciler, ReconciliationFault, SurfaceOutboxDelivery, SurfaceReconciliationWorker,
    },
};
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogError, CatalogRecord,
    DeclarationState, InventoryState, KindObservation, SqliteCapabilityCatalog, SqliteSurfaceStore, SurfaceManifest,
    SurfaceManifestEntryInput, SurfaceOutboxEvent, SurfacePublication,
};
use rmcp::model::{InitializeResult, Tool};
use serde_json::json;
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};

fn initialize() -> InitializeResult {
    serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap()
}

fn record(description: &str) -> CatalogRecord {
    let tool: Tool = serde_json::from_value(json!({
        "name": "analyze",
        "description": description,
        "inputSchema": {"type": "object"}
    }))
    .unwrap();
    CatalogRecord::materialize("server-a", "analyze", "fixture__analyze", CapabilityPayload::Tool(tool)).unwrap()
}

fn observation(record: CatalogRecord) -> CapabilityObservation {
    CapabilityObservation::new(
        "server-a",
        "fixture",
        "config-v1",
        initialize(),
        vec![KindObservation::new(
            CapabilityKind::Tools,
            DeclarationState::Supported,
            InventoryState::Complete,
        )],
        vec![record],
    )
}

async fn initialized_surface_pool() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
    mcpmate::config::server::init::initialize_server_tables(&pool)
        .await
        .unwrap();
    mcpmate::config::client::init::initialize_client_table(&pool)
        .await
        .unwrap();
    mcpmate::config::profile::init::initialize_profile_tables(&pool)
        .await
        .unwrap();
    SqliteCapabilityCatalog::new(pool.clone())
        .ensure_schema()
        .await
        .unwrap();
    pool
}

struct FailingOutboxDelivery;

#[async_trait]
impl SurfaceOutboxDelivery for FailingOutboxDelivery {
    async fn deliver(
        &self,
        event: &SurfaceOutboxEvent,
    ) -> mcpmate_capability_store::Result<()> {
        Err(CatalogError::InvalidSurfaceValue {
            field: "test delivery",
            value: event.event_id.clone(),
        })
    }
}

#[tokio::test]
async fn failed_outbox_delivery_remains_pending_for_retry() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
    SqliteCapabilityCatalog::new(pool.clone())
        .ensure_schema()
        .await
        .unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    let event = SurfaceOutboxEvent::new(
        "event-a",
        "surface_publication_changed",
        "consumer-a",
        json!({"generation": 1}),
    );
    let mut transaction = pool.begin().await.unwrap();
    store
        .enqueue_outbox_event_in_transaction(&mut transaction, &event)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let worker = SurfaceReconciliationWorker::new(pool.clone(), "worker-a")
        .with_outbox_delivery(Some(Arc::new(FailingOutboxDelivery)));
    assert!(worker.dispatch_outbox_once().await.is_err());
    let delivered_at: Option<String> =
        sqlx::query_scalar("SELECT delivered_at FROM surface_outbox_events WHERE event_id = 'event-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(delivered_at.is_none());
}

#[tokio::test]
async fn identical_catalog_observation_does_not_touch_surface_governance() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    let first = record("stable definition");
    let first_commit = catalog.commit_observation(observation(first.clone())).await.unwrap();
    assert!(first_commit.changed);

    let store = SqliteSurfaceStore::new(pool.clone());
    let manifest = SurfaceManifest::compile(
        "consumer-a",
        vec![SurfaceManifestEntryInput::new(
            first.ref_id.clone(),
            first.capability_id.clone(),
            first.kind(),
            first.external_key.clone(),
        )],
    )
    .unwrap();
    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_manifest_in_transaction(&mut transaction, &manifest)
        .await
        .unwrap();
    store
        .publish_and_bind_in_transaction(
            &mut transaction,
            &SurfacePublication::new(
                "publication-stable",
                "consumer-a",
                manifest.manifest_id,
                None,
                "initial",
                "system",
                None,
            ),
            None,
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let governance_counts = || async {
        sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM surface_reconciliation_jobs),
                (SELECT COUNT(*) FROM surface_proposals),
                (SELECT COUNT(*) FROM surface_review_items),
                (SELECT COUNT(*) FROM surface_publications),
                (SELECT COUNT(*) FROM capability_change_events),
                (SELECT COUNT(*) FROM surface_outbox_events)
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let before = governance_counts().await;
    let generation_before = store.load_binding("consumer-a").await.unwrap().unwrap().generation;

    let reconciliation = CatalogSurfaceReconciler::new(pool.clone())
        .reconcile(observation(first))
        .await
        .unwrap();

    assert!(!reconciliation.commit.changed);
    assert_eq!(reconciliation.commit.revision, first_commit.revision);
    assert_eq!(governance_counts().await, before);
    assert_eq!(
        store.load_binding("consumer-a").await.unwrap().unwrap().generation,
        generation_before
    );
}

#[tokio::test]
async fn catalog_and_all_consumer_safe_contractions_commit_or_roll_back_together() {
    let pool = initialized_surface_pool().await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    let first = record("version one");
    catalog.commit_observation(observation(first.clone())).await.unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    for consumer_id in ["consumer-a", "consumer-b"] {
        let manifest = SurfaceManifest::compile(
            consumer_id,
            vec![SurfaceManifestEntryInput::new(
                first.ref_id.clone(),
                first.capability_id.clone(),
                first.kind(),
                first.external_key.clone(),
            )],
        )
        .unwrap();
        let mut transaction = pool.begin().await.unwrap();
        store
            .insert_manifest_in_transaction(&mut transaction, &manifest)
            .await
            .unwrap();
        store
            .publish_and_bind_in_transaction(
                &mut transaction,
                &SurfacePublication::new(
                    format!("publication-{consumer_id}-1"),
                    consumer_id,
                    manifest.manifest_id,
                    None,
                    "initial",
                    "system",
                    None,
                ),
                None,
            )
            .await
            .unwrap();
        transaction.commit().await.unwrap();
    }

    let second = record("version two");
    let reconciler = CatalogSurfaceReconciler::new(pool.clone());
    assert!(
        reconciler
            .reconcile_with_fault(
                observation(second.clone()),
                ReconciliationFault::AfterFirstSafePublication,
            )
            .await
            .is_err()
    );
    assert_eq!(catalog.load_snapshot("server-a").await.unwrap().unwrap().revision, 1);
    for consumer_id in ["consumer-a", "consumer-b"] {
        assert_eq!(store.load_binding(consumer_id).await.unwrap().unwrap().generation, 1);
    }

    let result = reconciler.reconcile(observation(second)).await.unwrap();
    assert_eq!(result.commit.revision, 2);
    for consumer_id in ["consumer-a", "consumer-b"] {
        let binding = store.load_binding(consumer_id).await.unwrap().unwrap();
        assert_eq!(binding.generation, 2);
    }
    let safe_entries: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM consumer_surface_bindings b
        JOIN surface_publications p ON p.publication_id = b.active_publication_id
        JOIN surface_manifest_entries e ON e.manifest_id = p.manifest_id
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(safe_entries, 0);
    let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_reconciliation_jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    let outbox_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_outbox_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(job_count, 2);
    assert_eq!(outbox_count, 2);
}

#[tokio::test]
async fn retired_server_reconciliation_converges_after_safe_contraction() {
    let pool = initialized_surface_pool().await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    let first = record("version one");
    catalog.commit_observation(observation(first.clone())).await.unwrap();
    sqlx::query(
        r#"
        INSERT INTO client (
            id, identifier, name, config_mode, approval_status,
            capability_source, selected_profile_ids
        ) VALUES (
            'consumer-a', 'client-a', 'Client A', 'hosted', 'approved',
            'activated', '[]'
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let store = SqliteSurfaceStore::new(pool.clone());
    let manifest = SurfaceManifest::compile(
        "client-a",
        vec![SurfaceManifestEntryInput::new(
            first.ref_id.clone(),
            first.capability_id.clone(),
            first.kind(),
            first.external_key.clone(),
        )],
    )
    .unwrap();
    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_manifest_in_transaction(&mut transaction, &manifest)
        .await
        .unwrap();
    store
        .publish_and_bind_in_transaction(
            &mut transaction,
            &SurfacePublication::new(
                "publication-client-a-1",
                "client-a",
                manifest.manifest_id,
                None,
                "initial",
                "system",
                None,
            ),
            None,
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let reconciler = CatalogSurfaceReconciler::new(pool.clone());
    let mut transaction = pool.begin().await.unwrap();
    reconciler
        .retire_server_in_transaction(&mut transaction, "server-a")
        .await
        .unwrap()
        .expect("server retirement changes the catalog");
    transaction.commit().await.unwrap();

    let worker = SurfaceReconciliationWorker::new(pool.clone(), "worker-a");
    assert!(worker.run_once().await.unwrap());
    let status: String =
        sqlx::query_scalar("SELECT status FROM surface_reconciliation_jobs WHERE consumer_id = 'client-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "succeeded");
    let (generation, entry_count): (i64, i64) = sqlx::query_as(
        r#"
        SELECT binding.generation, COUNT(entry.ref_id)
        FROM consumer_surface_bindings binding
        JOIN surface_publications publication
          ON publication.publication_id = binding.active_publication_id
        LEFT JOIN surface_manifest_entries entry
          ON entry.manifest_id = publication.manifest_id
        WHERE binding.consumer_id = 'client-a'
        GROUP BY binding.generation
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(generation, 2);
    assert_eq!(entry_count, 0);
    assert!(
        !worker.run_once().await.unwrap(),
        "the retired-server job must not retry"
    );
}

#[tokio::test]
async fn durable_worker_materializes_and_records_a_success_receipt() {
    let pool = initialized_surface_pool().await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, command, enabled) VALUES ('server-a', 'fixture', 'stdio', '', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO profile (id, name, description, type, role, is_active) VALUES ('profile-a', 'Profile A', '', 'shared', 'user', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO client (
            id, identifier, name, config_mode, approval_status,
            capability_source, selected_profile_ids
        ) VALUES (
            'consumer-a', 'client-a', 'Client A', 'hosted', 'approved',
            'profiles', '["profile-a"]'
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let first = record("version one");
    catalog.commit_observation(observation(first.clone())).await.unwrap();
    sqlx::query("INSERT INTO profile_capability_refs (profile_id, ref_id, enabled) VALUES ('profile-a', ?, 1)")
        .bind(first.ref_id.as_str())
        .execute(&pool)
        .await
        .unwrap();
    ProfileSurfaceManagement::mutate_capabilities(
        &pool,
        "profile-a",
        &[first.ref_id.to_string()],
        ProfileRelationshipAction::Enable,
        std::collections::HashMap::from([("server-a".to_string(), 1)]),
        "test",
    )
    .await
    .unwrap();

    CatalogSurfaceReconciler::new(pool.clone())
        .reconcile(observation(record("version two")))
        .await
        .unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    let safe_binding = store.load_binding("client-a").await.unwrap().unwrap();
    let safe_manifest_id: String =
        sqlx::query_scalar("SELECT manifest_id FROM surface_publications WHERE publication_id = ?")
            .bind(&safe_binding.active_publication_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let mut race_transaction = pool.begin().await.unwrap();
    store
        .publish_and_bind_in_transaction(
            &mut race_transaction,
            &SurfacePublication::new(
                "publication-management-race",
                "client-a",
                safe_manifest_id.parse().unwrap(),
                None,
                "management_race",
                "test",
                Some(safe_binding.active_publication_id),
            ),
            Some(safe_binding.generation),
        )
        .await
        .unwrap();
    race_transaction.commit().await.unwrap();

    assert!(
        SurfaceReconciliationWorker::new(pool.clone(), "worker-a")
            .run_once()
            .await
            .unwrap()
    );
    let superseded_receipt: String = sqlx::query_scalar(
        r#"
        SELECT success_receipt
        FROM surface_reconciliation_jobs
        WHERE consumer_id = 'client-a' AND success_receipt LIKE '%"outcome":"superseded"%'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(superseded_receipt.contains("successorIdempotencyKey"));
    assert!(
        SurfaceReconciliationWorker::new(pool.clone(), "worker-a")
            .run_once()
            .await
            .unwrap()
    );

    let status: String =
        sqlx::query_scalar(
            "SELECT status FROM surface_reconciliation_jobs WHERE consumer_id = 'client-a' ORDER BY created_at DESC LIMIT 1",
        )
            .fetch_one(&pool)
            .await
            .unwrap();
    let receipt: Option<String> =
        sqlx::query_scalar(
            "SELECT success_receipt FROM surface_reconciliation_jobs WHERE consumer_id = 'client-a' ORDER BY created_at DESC LIMIT 1",
        )
            .fetch_one(&pool)
            .await
            .unwrap();
    let review_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM surface_review_items WHERE consumer_id = 'client-a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "succeeded");
    assert!(receipt.is_some());
    assert_eq!(review_count, 1);
    let (before_capability_id, target_capability_id): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT before_capability_id, target_capability_id FROM surface_review_items WHERE consumer_id = 'client-a'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before_capability_id.as_deref(), Some(first.capability_id.as_str()));
    assert!(target_capability_id.is_some());
    assert_ne!(before_capability_id, target_capability_id);
    let (change_before, change_target, change_class, policy_action, actor): (
        Option<String>,
        Option<String>,
        String,
        String,
        String,
    ) = sqlx::query_as(
        r#"
        SELECT before_capability_id, target_capability_id, change_class, policy_action, actor
        FROM capability_change_events
        WHERE consumer_id = 'client-a'
        ORDER BY occurred_at
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(change_before.as_deref(), Some(first.capability_id.as_str()));
    assert_eq!(change_target, target_capability_id);
    assert_eq!(change_class, "model_visible");
    assert_eq!(policy_action, "review");
    assert_eq!(actor, "worker-a");

    let missing_observation = CapabilityObservation::new(
        "server-a",
        "fixture",
        "config-v1",
        initialize(),
        vec![KindObservation::new(
            CapabilityKind::Tools,
            DeclarationState::Supported,
            InventoryState::Complete,
        )],
        Vec::new(),
    );
    CatalogSurfaceReconciler::new(pool.clone())
        .reconcile(missing_observation)
        .await
        .unwrap();
    assert!(
        SurfaceReconciliationWorker::new(pool.clone(), "worker-a")
            .run_once()
            .await
            .unwrap()
    );
    let pending_missing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM surface_review_items WHERE consumer_id = 'client-a' AND lifecycle = 'pending' AND target_key LIKE 'missing:%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let obsolete_versions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM surface_review_items WHERE consumer_id = 'client-a' AND lifecycle = 'obsolete' AND target_key LIKE 'capability:%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pending_missing, 1);
    assert_eq!(obsolete_versions, 1);
}
#[path = "support/database.rs"]
mod database_support;
