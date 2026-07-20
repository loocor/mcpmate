use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use chrono::Utc;
use mcpmate_capability_store::{
    CapabilityKind, CatalogSnapshot, DeclarationState, DerivedCapabilityCache, InventoryState, KindObservation,
    ProjectionKey, ProjectionNameDomain, ProjectionPayload, RawSnapshotKey, SnapshotState,
};
use rmcp::model::{Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, Tool};
use tokio::sync::oneshot;

fn snapshot(
    server_id: &str,
    revision: i64,
) -> CatalogSnapshot {
    CatalogSnapshot {
        server_id: server_id.to_string(),
        server_name: format!("{server_id}-name"),
        config_fingerprint: "config-fingerprint".to_string(),
        revision,
        state: SnapshotState::Ready,
        initialize: Some(
            InitializeResult::new(ServerCapabilities::default())
                .with_protocol_version(ProtocolVersion::V_2025_06_18)
                .with_server_info(Implementation::new("fixture", "1.0.0")),
        ),
        kind_states: vec![KindObservation::new(
            CapabilityKind::Tools,
            DeclarationState::Supported,
            InventoryState::Complete,
        )],
        records: Vec::new(),
        observed_at: Utc::now(),
        committed_at: Utc::now(),
        last_error: None,
    }
}

fn projection_key(
    surface: &str,
    revision: &str,
) -> ProjectionKey {
    ProjectionKey::new(
        "managed:client-alpha",
        surface,
        CapabilityKind::Tools,
        ProjectionNameDomain::External,
        revision,
    )
}

#[tokio::test]
async fn raw_snapshot_miss_is_single_flight_and_then_hits_memory() {
    let cache = Arc::new(DerivedCapabilityCache::new(4, 4));
    let loads = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for _ in 0..16 {
        let cache = cache.clone();
        let loads = loads.clone();
        tasks.push(tokio::spawn(async move {
            cache
                .get_or_load_current_snapshot("server-alpha", || async move {
                    loads.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    Ok::<_, &'static str>(Some(snapshot("server-alpha", 7)))
                })
                .await
                .expect("load snapshot")
                .expect("snapshot exists")
        }));
    }

    for task in tasks {
        assert_eq!(task.await.expect("join loader").revision, 7);
    }
    assert_eq!(loads.load(Ordering::SeqCst), 1);

    let cached = cache
        .get_or_load_current_snapshot("server-alpha", || async {
            Err::<Option<CatalogSnapshot>, _>("memory hit must not reload")
        })
        .await
        .expect("read cached snapshot")
        .expect("cached snapshot exists");
    assert_eq!(cached.revision, 7);

    let metrics = cache.metrics().await;
    assert_eq!(metrics.raw_entries, 1);
    assert_eq!(metrics.raw_loads, 1);
    assert_eq!(metrics.raw_hits, 1);
    assert_eq!(metrics.raw_misses, 16);
}

#[tokio::test]
async fn projection_eviction_only_recomputes_and_does_not_change_results() {
    let cache = DerivedCapabilityCache::new(2, 1);
    let first_key = projection_key("surface-a", "rev-a");
    let second_key = projection_key("surface-b", "rev-b");
    let first_payload = ProjectionPayload::Tools(vec![Tool::new("alpha", "alpha", serde_json::Map::new())]);

    let cached = cache
        .get_or_project(first_key.clone(), || async {
            Ok::<_, &'static str>(first_payload.clone())
        })
        .await
        .expect("project first");
    assert_eq!(*cached, first_payload);

    cache
        .get_or_project(second_key, || async {
            Ok::<_, &'static str>(ProjectionPayload::Tools(Vec::new()))
        })
        .await
        .expect("project second");

    let recomputations = AtomicUsize::new(0);
    let reloaded = cache
        .get_or_project(first_key, || async {
            recomputations.fetch_add(1, Ordering::SeqCst);
            Ok::<_, &'static str>(first_payload.clone())
        })
        .await
        .expect("reproject first");

    assert_eq!(*reloaded, first_payload);
    assert_eq!(recomputations.load(Ordering::SeqCst), 1);
    assert_eq!(cache.metrics().await.projection_evictions, 2);
}

#[tokio::test]
async fn reset_clears_both_lrus_but_preserves_diagnostic_counters() {
    let cache = DerivedCapabilityCache::new(2, 2);
    cache
        .get_or_load_snapshot(RawSnapshotKey::new("server-alpha", 1), || async {
            Ok::<_, &'static str>(Some(snapshot("server-alpha", 1)))
        })
        .await
        .expect("load snapshot");
    cache
        .get_or_project(projection_key("surface-secret", "revision-secret"), || async {
            Ok::<_, &'static str>(ProjectionPayload::Tools(Vec::new()))
        })
        .await
        .expect("load projection");

    cache.clear().await;

    let metrics = cache.metrics().await;
    assert_eq!(metrics.raw_entries, 0);
    assert_eq!(metrics.projection_entries, 0);
    assert_eq!(metrics.invalidations, 1);
    assert!(metrics.total_queries >= 2);
}

#[tokio::test]
async fn diagnostic_keys_are_bounded_and_redacted() {
    let cache = DerivedCapabilityCache::new(2, 2);
    for server_id in ["server-sensitive", "server-other"] {
        cache
            .get_or_load_snapshot(RawSnapshotKey::new(server_id, 42), || async {
                Ok::<_, &'static str>(Some(snapshot(server_id, 42)))
            })
            .await
            .expect("load snapshot");
    }
    cache
        .get_or_project(projection_key("surface-sensitive", "revision-sensitive"), || async {
            Ok::<_, &'static str>(ProjectionPayload::Tools(Vec::new()))
        })
        .await
        .expect("load projection");

    let keys = cache.diagnostic_keys(10).await;
    assert_eq!(keys.len(), 3);
    let encoded = serde_json::to_string(&keys).expect("serialize diagnostics");
    assert!(!encoded.contains("server-sensitive"));
    assert!(!encoded.contains("surface-sensitive"));
    assert!(!encoded.contains("revision-sensitive"));

    let filtered = cache.diagnostic_keys_for_server(10, Some("server-sensitive")).await;
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].cache, "raw_snapshot");
}

#[tokio::test]
async fn reset_during_load_does_not_repopulate_raw_or_projection_cache() {
    let cache = Arc::new(DerivedCapabilityCache::new(2, 2));
    let (raw_started_tx, raw_started_rx) = oneshot::channel();
    let (raw_release_tx, raw_release_rx) = oneshot::channel();
    let raw_cache = cache.clone();
    let raw_task = tokio::spawn(async move {
        raw_cache
            .get_or_load_current_snapshot("server-alpha", || async move {
                raw_started_tx.send(()).expect("signal raw load");
                raw_release_rx.await.expect("release raw load");
                Ok::<_, &'static str>(Some(snapshot("server-alpha", 1)))
            })
            .await
            .expect("load raw snapshot")
            .expect("raw snapshot exists")
    });

    raw_started_rx.await.expect("raw load started");
    cache.clear().await;
    raw_release_tx.send(()).expect("release raw loader");
    assert_eq!(raw_task.await.expect("join raw loader").revision, 1);
    assert_eq!(cache.metrics().await.raw_entries, 0);

    let (projection_started_tx, projection_started_rx) = oneshot::channel();
    let (projection_release_tx, projection_release_rx) = oneshot::channel();
    let projection_cache = cache.clone();
    let projection_task = tokio::spawn(async move {
        projection_cache
            .get_or_project(projection_key("surface-a", "revision-a"), || async move {
                projection_started_tx.send(()).expect("signal projection load");
                projection_release_rx.await.expect("release projection load");
                Ok::<_, &'static str>(ProjectionPayload::Tools(Vec::new()))
            })
            .await
            .expect("load projection")
    });

    projection_started_rx.await.expect("projection load started");
    cache.clear().await;
    projection_release_tx.send(()).expect("release projection loader");
    assert_eq!(
        *projection_task.await.expect("join projection loader"),
        ProjectionPayload::Tools(Vec::new())
    );
    let metrics = cache.metrics().await;
    assert_eq!(metrics.raw_entries, 0);
    assert_eq!(metrics.projection_entries, 0);
}

#[tokio::test]
async fn server_invalidation_during_load_does_not_restore_stale_snapshot() {
    let cache = Arc::new(DerivedCapabilityCache::new(2, 2));
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let task_cache = cache.clone();
    let task = tokio::spawn(async move {
        task_cache
            .get_or_load_current_snapshot("server-alpha", || async move {
                started_tx.send(()).expect("signal load");
                release_rx.await.expect("release load");
                Ok::<_, &'static str>(Some(snapshot("server-alpha", 1)))
            })
            .await
            .expect("load snapshot")
            .expect("snapshot exists")
    });

    started_rx.await.expect("load started");
    cache.invalidate_server("server-alpha").await;
    release_tx.send(()).expect("release loader");
    assert_eq!(task.await.expect("join loader").revision, 1);
    assert_eq!(cache.metrics().await.raw_entries, 0);

    let fresh = cache
        .get_or_load_current_snapshot("server-alpha", || async {
            Ok::<_, &'static str>(Some(snapshot("server-alpha", 2)))
        })
        .await
        .expect("load fresh snapshot")
        .expect("fresh snapshot exists");
    assert_eq!(fresh.revision, 2);
    assert_eq!(cache.metrics().await.raw_entries, 1);
}

#[tokio::test]
async fn server_invalidation_during_projection_does_not_restore_stale_value() {
    let cache = Arc::new(DerivedCapabilityCache::new(2, 2));
    let key = projection_key("surface-a", "revision-a");
    let old_projection = ProjectionPayload::Tools(vec![Tool::new("old", "old", serde_json::Map::new())]);
    let old_projection_for_loader = old_projection.clone();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let task_cache = cache.clone();
    let task_key = key.clone();
    let stale_task = tokio::spawn(async move {
        task_cache
            .get_or_project(task_key, || async move {
                started_tx.send(()).expect("signal projection load");
                release_rx.await.expect("release projection load");
                Ok::<_, &'static str>(old_projection_for_loader)
            })
            .await
            .expect("load stale projection")
    });

    started_rx.await.expect("projection load started");
    cache.invalidate_server("server-a").await;
    release_tx.send(()).expect("release projection loader");
    let stale_result = stale_task.await.expect("join stale projection loader");

    let new_projection = ProjectionPayload::Tools(vec![Tool::new("new", "new", serde_json::Map::new())]);
    let fresh_loader_calls = Arc::new(AtomicUsize::new(0));
    let fresh_result = cache
        .get_or_project(key, || {
            let fresh_loader_calls = fresh_loader_calls.clone();
            let new_projection = new_projection.clone();
            async move {
                fresh_loader_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, &'static str>(new_projection)
            }
        })
        .await
        .expect("load fresh projection");

    assert_eq!(stale_result.as_ref(), &old_projection);
    assert_eq!(fresh_result.as_ref(), &new_projection);
    assert_eq!(fresh_loader_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn projection_epoch_captured_before_invalidation_blocks_late_cache_write() {
    let cache = DerivedCapabilityCache::new(2, 2);
    let key = projection_key("surface-a", "revision-a");
    let expected_epoch = cache.projection_epoch();

    cache.invalidate_server("server-a").await;

    let stale_projection = ProjectionPayload::Tools(vec![Tool::new("stale", "stale", serde_json::Map::new())]);
    let stale_result = cache
        .get_or_project_at_epoch(key.clone(), expected_epoch, || async {
            Ok::<_, &'static str>(stale_projection.clone())
        })
        .await
        .expect("return request-local stale projection");

    assert_eq!(stale_result.as_ref(), &stale_projection);
    assert_eq!(cache.metrics().await.projection_entries, 0);

    let fresh_projection = ProjectionPayload::Tools(vec![Tool::new("fresh", "fresh", serde_json::Map::new())]);
    let fresh_loader_calls = AtomicUsize::new(0);
    let fresh_result = cache
        .get_or_project(key.clone(), || async {
            fresh_loader_calls.fetch_add(1, Ordering::SeqCst);
            Ok::<_, &'static str>(fresh_projection.clone())
        })
        .await
        .expect("cache fresh projection");

    assert_eq!(fresh_result.as_ref(), &fresh_projection);
    assert_eq!(fresh_loader_calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache.metrics().await.projection_entries, 1);

    let cached_result = cache
        .get_or_project(key, || async {
            Err::<ProjectionPayload, _>("fresh projection must remain cached")
        })
        .await
        .expect("read fresh cached projection");
    assert_eq!(cached_result.as_ref(), &fresh_projection);
}
