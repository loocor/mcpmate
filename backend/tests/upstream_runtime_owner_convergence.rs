#[path = "support/runtime_database.rs"]
mod runtime_database;
#[path = "support/upstream_preview.rs"]
mod upstream_preview;

use std::time::Duration;
use upstream_preview::PreviewUpstreamFixture;

#[tokio::test]
async fn repeated_preview_reuses_one_owner() {
    let fixture = PreviewUpstreamFixture::new().await;

    let first = fixture.preview("everything", &[]).await;
    let second = fixture.preview("everything", &[]).await;

    assert_eq!(
        first.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        second.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fixture.startup_count(),
        1,
        "matching previews must share one upstream owner"
    );
}

#[tokio::test]
async fn changed_preview_config_starts_a_new_owner() {
    let fixture = PreviewUpstreamFixture::new().await;

    let first = fixture.preview("everything", &[]).await;
    let changed = fixture.preview("everything", &["--changed"]).await;

    assert_eq!(
        first.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        changed.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fixture.startup_count(),
        2,
        "a changed runtime config must not reuse the previous preview owner"
    );
}

#[tokio::test]
async fn concurrent_previews_join_one_owner_acquisition() {
    let fixture = PreviewUpstreamFixture::new_with_delay(Duration::from_millis(300)).await;

    let (first, second) = tokio::join!(fixture.preview("everything", &[]), fixture.preview("everything", &[]));

    assert_eq!(
        first.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        second.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fixture.startup_count(),
        1,
        "concurrent previews must join one acquisition"
    );
}

#[tokio::test]
async fn joined_preview_uses_its_own_operation_timeout() {
    let fixture = PreviewUpstreamFixture::new_with_delays(Duration::from_millis(200), Duration::from_millis(80)).await;

    let (first, second) = tokio::join!(
        fixture.preview_with_timeout("everything", &[], Some(500)),
        fixture.preview_with_timeout("everything", &[], Some(10)),
    );

    assert_eq!(
        first.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        second.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(false),
        "the joined caller must apply its own tools/list deadline"
    );
    assert_eq!(
        fixture.startup_count(),
        1,
        "operation timeout must not restart the owner"
    );
}

#[tokio::test]
async fn cancelling_the_first_preview_does_not_strand_the_acquisition() {
    let fixture = PreviewUpstreamFixture::new_with_delay(Duration::from_millis(500)).await;
    let mut first = Box::pin(fixture.preview("everything", &[]));
    tokio::select! {
        _ = fixture.wait_until_started() => {}
        result = &mut first => panic!("first preview completed before cancellation: {result}"),
    }
    drop(first);

    let second = tokio::time::timeout(Duration::from_secs(5), fixture.preview("everything", &[]))
        .await
        .expect("next preview must not wait on a stranded acquisition");

    assert_eq!(
        second.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fixture.startup_count(),
        1,
        "caller cancellation must not restart the physical acquisition"
    );
}

#[tokio::test]
async fn one_shot_preview_discovery_closes_its_owner() {
    let fixture = PreviewUpstreamFixture::new().await;

    fixture.discover_once().await;
    fixture.wait_until_exited().await;

    assert_eq!(fixture.startup_count(), 1);
}

#[tokio::test]
async fn ready_preview_owner_is_promoted_to_production() {
    let fixture = PreviewUpstreamFixture::new().await;
    let server_id = "server-preview-production";

    let preview = fixture.preview("everything", &[]).await;
    fixture.persist_server(server_id, "everything").await;
    let instance_id = fixture.enable_server(server_id).await;
    fixture.wait_until_catalog_ready(server_id).await;

    assert_eq!(
        preview.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fixture.startup_count(),
        1,
        "production must promote the matching ready preview owner"
    );
    fixture
        .pool
        .lock()
        .await
        .disconnect(server_id, &instance_id)
        .await
        .expect("disconnect promoted preview owner");
    fixture.wait_until_exited().await;
}

#[tokio::test]
async fn production_joins_starting_preview_owner() {
    let fixture = PreviewUpstreamFixture::new_with_delay(Duration::from_millis(300)).await;
    let server_id = "server-starting-preview-production";

    let preview = fixture.preview("everything", &[]);
    let enable = async {
        fixture.wait_until_started().await;
        fixture.persist_server(server_id, "everything").await;
        fixture.enable_server(server_id).await
    };
    let (preview, _) = tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(preview, enable) })
        .await
        .expect("preview promotion should complete");
    fixture.wait_until_catalog_ready(server_id).await;

    assert_eq!(
        preview.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fixture.startup_count(),
        1,
        "production must join the matching in-flight preview owner"
    );
}

#[tokio::test]
async fn management_discovery_joins_starting_production_owner() {
    let fixture = PreviewUpstreamFixture::new_with_delay(Duration::from_millis(300)).await;
    let server_id = "server-production-discovery";
    fixture.persist_server(server_id, "everything").await;

    let enable = fixture.enable_server(server_id);
    let refresh = async {
        fixture.wait_until_started().await;
        fixture.refresh_capabilities(server_id).await;
    };
    let (_, ()) = tokio::join!(enable, refresh);

    assert_eq!(
        fixture.startup_count(),
        1,
        "management discovery must join the in-flight production owner"
    );
}

#[tokio::test]
async fn preview_joins_starting_production_owner() {
    let fixture = PreviewUpstreamFixture::new_with_delay(Duration::from_millis(300)).await;
    let server_id = "server-starting-production-preview";
    fixture.persist_server(server_id, "everything").await;

    let enable = fixture.enable_server(server_id);
    let preview = async {
        fixture.wait_until_started().await;
        fixture.preview("everything", &[]).await
    };
    let (_, preview) = tokio::join!(enable, preview);

    assert_eq!(
        preview.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fixture.startup_count(),
        1,
        "preview must join the matching in-flight production owner"
    );
}

#[tokio::test]
async fn preview_does_not_reuse_production_with_a_different_launch_identity() {
    let fixture = PreviewUpstreamFixture::new().await;
    let server_id = "server-production-runtime-mismatch";
    fixture.persist_server(server_id, "everything").await;
    let instance_id = fixture.enable_server(server_id).await;

    fixture
        .pool
        .lock()
        .await
        .get_instance_mut(server_id, &instance_id)
        .expect("production instance")
        .runtime_fingerprint = Some("sha256:stale-launch".to_string());

    let preview = fixture.preview("everything", &[]).await;

    assert_eq!(
        preview.pointer("/data/items/0/ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        fixture.startup_count(),
        2,
        "preview must not reuse a production owner launched from different runtime materialization"
    );

    let promoted_instance_id = fixture.enable_server(server_id).await;
    let pool = fixture.pool.lock().await;
    let promoted = pool
        .get_instance(server_id, &promoted_instance_id)
        .expect("promoted production instance");
    assert_ne!(
        promoted.runtime_fingerprint.as_deref(),
        Some("sha256:stale-launch"),
        "production acquisition must not return the stale launch identity"
    );
    assert_eq!(
        fixture.startup_count(),
        2,
        "production must promote the matching preview instead of starting a third owner"
    );
}
