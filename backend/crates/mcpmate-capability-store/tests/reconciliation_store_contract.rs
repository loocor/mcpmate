use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use mcpmate_capability_store::{
    ReconciliationJobStatus, SqliteCapabilityCatalog, SqliteSurfaceStore, SurfaceOutboxEvent, SurfaceReconciliationJob,
};
use mcpmate_migrations::{DatabaseSource, prepare_config_database};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn jobs_are_idempotent_leased_recoverable_and_receipted() {
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect("sqlite::memory:?cache=shared")
        .await
        .unwrap();
    prepare_config_database(&pool, DatabaseSource::InMemory)
        .await
        .expect("prepare config schema");
    SqliteCapabilityCatalog::new(pool.clone())
        .ensure_schema()
        .await
        .unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    let job =
        SurfaceReconciliationJob::new("catalog_delta", "server-a:2", "consumer-a", json!({"server-a": 2}), 3).unwrap();

    let mut transaction = pool.begin().await.unwrap();
    store
        .enqueue_reconciliation_job_in_transaction(&mut transaction, &job)
        .await
        .unwrap();
    store
        .enqueue_reconciliation_job_in_transaction(&mut transaction, &job)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let first = store
        .lease_next_reconciliation_job("worker-a", Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.idempotency_key, job.idempotency_key);
    assert_eq!(first.status, ReconciliationJobStatus::Leased);
    assert_eq!(first.attempt_count, 1);
    assert!(
        store
            .lease_next_reconciliation_job("worker-b", Duration::from_secs(30))
            .await
            .unwrap()
            .is_none()
    );

    sqlx::query("UPDATE surface_reconciliation_jobs SET lease_expires_at = ?")
        .bind((Utc::now() - ChronoDuration::seconds(1)).to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    let recovered = store
        .lease_next_reconciliation_job("worker-b", Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.attempt_count, 2);

    store
        .record_reconciliation_failure(&job.idempotency_key, "worker-b", "transient", Utc::now())
        .await
        .unwrap();
    assert!(
        store
            .lease_next_reconciliation_job("worker-c", Duration::from_secs(30))
            .await
            .unwrap()
            .is_none()
    );
    sqlx::query("UPDATE surface_reconciliation_jobs SET next_attempt_at = ?")
        .bind((Utc::now() - ChronoDuration::seconds(1)).to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    let retry = store
        .lease_next_reconciliation_job("worker-c", Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    store
        .record_reconciliation_success(
            &retry.idempotency_key,
            "worker-c",
            json!({"publicationId": "publication-2"}),
        )
        .await
        .unwrap();
    let completed = store
        .load_reconciliation_job(&job.idempotency_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.status, ReconciliationJobStatus::Succeeded);
    assert_eq!(completed.attempt_count, 3);
    assert!(completed.success_receipt.is_some());
}

#[tokio::test]
async fn outbox_events_are_insert_or_verified_and_delivered_once() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    prepare_config_database(&pool, DatabaseSource::InMemory)
        .await
        .expect("prepare config schema");
    SqliteCapabilityCatalog::new(pool.clone())
        .ensure_schema()
        .await
        .unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    let event = SurfaceOutboxEvent::new(
        "outbox-a",
        "surface_publication_changed",
        "consumer-a",
        json!({"generation": 2}),
    );
    let mut transaction = pool.begin().await.unwrap();
    store
        .enqueue_outbox_event_in_transaction(&mut transaction, &event)
        .await
        .unwrap();
    store
        .enqueue_outbox_event_in_transaction(&mut transaction, &event)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let pending = store.load_pending_outbox_events(10).await.unwrap();
    assert_eq!(pending, vec![event.clone()]);
    store.mark_outbox_event_delivered("outbox-a").await.unwrap();
    assert!(store.load_pending_outbox_events(10).await.unwrap().is_empty());
    assert!(store.mark_outbox_event_delivered("outbox-a").await.is_err());
}
