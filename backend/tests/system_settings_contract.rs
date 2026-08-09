use mcpmate::api::models::system::SystemSettingsUpdateReq;
use mcpmate::clients::models::FirstContactBehavior;
use mcpmate::common::MCPMatePaths;
use mcpmate::system::settings::{
    SystemSettings, apply_settings_with_effects_for_paths, get_settings_sync_for_paths,
    set_client_discovery_snapshot_last_success_at_for_paths, set_onboarding_completed_for_paths,
    set_settings_sync_for_paths,
};
use std::sync::Arc;
use tokio::sync::Barrier;

#[test]
fn rejects_updates_that_cross_file_and_database_setting_stores() {
    let request = serde_json::from_value::<SystemSettingsUpdateReq>(serde_json::json!({
        "default_config_mode": "hosted",
        "default_merge_strategy_override": "deep_merge"
    }))
    .expect("parse mixed settings request");

    assert!(request.validate_storage_boundary().is_err());
}

#[test]
fn accepts_updates_scoped_to_one_setting_store() {
    let file_request = serde_json::from_value::<SystemSettingsUpdateReq>(serde_json::json!({
        "inspector_timeout_ms": 12000
    }))
    .expect("parse file settings request");
    let database_request = serde_json::from_value::<SystemSettingsUpdateReq>(serde_json::json!({
        "clear_default_merge_strategy_override": true
    }))
    .expect("parse database settings request");

    assert!(file_request.validate_storage_boundary().is_ok());
    assert!(database_request.validate_storage_boundary().is_ok());
}

#[test]
fn snapshot_clock_fields_are_backward_compatible_and_round_trip() {
    let mut legacy_settings = serde_json::to_value(SystemSettings::default()).expect("serialize legacy settings");
    let legacy_object = legacy_settings
        .as_object_mut()
        .expect("system settings serialize to an object");
    legacy_object.remove("client_discovery_snapshot_ttl_seconds");
    legacy_object.remove("client_discovery_snapshot_last_success_at");

    let legacy_settings = serde_json::from_value::<SystemSettings>(legacy_settings).expect("parse legacy settings");
    assert_eq!(legacy_settings.client_discovery_snapshot_ttl_seconds, 21_600);
    assert_eq!(legacy_settings.client_discovery_snapshot_last_success_at, None);

    let settings = SystemSettings {
        client_discovery_snapshot_ttl_seconds: 9_000,
        client_discovery_snapshot_last_success_at: Some("2026-08-08T12:34:56Z".to_string()),
        ..SystemSettings::default()
    };

    let round_tripped = serde_json::from_value::<SystemSettings>(
        serde_json::to_value(&settings).expect("serialize snapshot clock settings"),
    )
    .expect("deserialize snapshot clock settings");
    assert_eq!(round_tripped, settings);
}

#[tokio::test]
async fn snapshot_success_update_preserves_other_system_settings() {
    let temp_dir = tempfile::tempdir().expect("create temporary settings directory");
    let paths = MCPMatePaths::from_base_dir(temp_dir.path()).expect("create test paths");
    let settings = SystemSettings {
        api_port: 18_080,
        mcp_port: 18_000,
        default_config_mode: "hosted".to_string(),
        client_discovery_snapshot_ttl_seconds: 7_200,
        ..SystemSettings::default()
    };
    set_settings_sync_for_paths(&paths, &settings).expect("write settings");

    set_client_discovery_snapshot_last_success_at_for_paths(&paths, "2026-08-08T12:34:56Z".to_string())
        .await
        .expect("record snapshot success");

    let updated = get_settings_sync_for_paths(&paths).expect("read updated settings");
    assert_eq!(
        updated.client_discovery_snapshot_last_success_at.as_deref(),
        Some("2026-08-08T12:34:56Z")
    );
    assert_eq!(updated.api_port, settings.api_port);
    assert_eq!(updated.mcp_port, settings.mcp_port);
    assert_eq!(updated.default_config_mode, settings.default_config_mode);
    assert_eq!(
        updated.client_discovery_snapshot_ttl_seconds,
        settings.client_discovery_snapshot_ttl_seconds
    );
    assert_eq!(updated.first_contact_behavior, settings.first_contact_behavior);
    assert_eq!(updated.inspector_timeout_ms, settings.inspector_timeout_ms);
    assert_eq!(updated.onboarding_completed, settings.onboarding_completed);
}

#[tokio::test]
async fn concurrent_snapshot_and_settings_mutations_preserve_both_changes() {
    let temp_dir = tempfile::tempdir().expect("create temporary settings directory");
    let paths = MCPMatePaths::from_base_dir(temp_dir.path()).expect("create test paths");
    set_settings_sync_for_paths(&paths, &SystemSettings::default()).expect("write settings");

    let previous = get_settings_sync_for_paths(&paths).expect("read initial settings");
    let mut next = previous.clone();
    next.first_contact_behavior = FirstContactBehavior::Review;

    let paths = Arc::new(paths);
    let barrier = Arc::new(Barrier::new(3));

    let timestamp_paths = Arc::clone(&paths);
    let timestamp_barrier = Arc::clone(&barrier);
    let timestamp = tokio::spawn(async move {
        timestamp_barrier.wait().await;
        set_client_discovery_snapshot_last_success_at_for_paths(&timestamp_paths, "2026-08-08T12:34:56Z".to_string())
            .await
    });

    let settings_paths = Arc::clone(&paths);
    let settings_barrier = Arc::clone(&barrier);
    let settings = tokio::spawn(async move {
        settings_barrier.wait().await;
        apply_settings_with_effects_for_paths(&settings_paths, &previous, &next, None).await
    });

    barrier.wait().await;
    timestamp.await.expect("join timestamp task").expect("write timestamp");
    settings.await.expect("join settings task").expect("write settings");

    let updated = get_settings_sync_for_paths(&paths).expect("read updated settings");
    assert_eq!(
        updated.client_discovery_snapshot_last_success_at.as_deref(),
        Some("2026-08-08T12:34:56Z")
    );
    assert_eq!(updated.first_contact_behavior, FirstContactBehavior::Review);
}

#[tokio::test]
async fn stale_effects_apply_preserves_snapshot_timestamp_and_requested_field() {
    let temp_dir = tempfile::tempdir().expect("create temporary settings directory");
    let paths = MCPMatePaths::from_base_dir(temp_dir.path()).expect("create test paths");
    set_settings_sync_for_paths(&paths, &SystemSettings::default()).expect("write settings");

    let previous = get_settings_sync_for_paths(&paths).expect("read stale settings");
    let mut next = previous.clone();
    next.inspector_timeout_ms = 9_500;

    set_client_discovery_snapshot_last_success_at_for_paths(&paths, "2026-08-08T12:34:56Z".to_string())
        .await
        .expect("write snapshot timestamp");

    apply_settings_with_effects_for_paths(&paths, &previous, &next, None)
        .await
        .expect("apply stale settings update");

    let updated = get_settings_sync_for_paths(&paths).expect("read updated settings");
    assert_eq!(
        updated.client_discovery_snapshot_last_success_at.as_deref(),
        Some("2026-08-08T12:34:56Z")
    );
    assert_eq!(updated.inspector_timeout_ms, 9_500);
}

#[tokio::test]
async fn onboarding_complete_and_reset_preserve_snapshot_timestamp() {
    let temp_dir = tempfile::tempdir().expect("create temporary settings directory");
    let paths = MCPMatePaths::from_base_dir(temp_dir.path()).expect("create test paths");
    set_settings_sync_for_paths(&paths, &SystemSettings::default()).expect("write settings");

    set_client_discovery_snapshot_last_success_at_for_paths(&paths, "2026-08-08T12:34:56Z".to_string())
        .await
        .expect("write snapshot timestamp");
    set_onboarding_completed_for_paths(&paths, true)
        .await
        .expect("complete onboarding");

    let completed = get_settings_sync_for_paths(&paths).expect("read completed onboarding");
    assert!(completed.onboarding_completed);
    assert_eq!(
        completed.client_discovery_snapshot_last_success_at.as_deref(),
        Some("2026-08-08T12:34:56Z")
    );

    set_onboarding_completed_for_paths(&paths, false)
        .await
        .expect("reset onboarding");

    let reset = get_settings_sync_for_paths(&paths).expect("read reset onboarding");
    assert!(!reset.onboarding_completed);
    assert_eq!(
        reset.client_discovery_snapshot_last_success_at.as_deref(),
        Some("2026-08-08T12:34:56Z")
    );
}
