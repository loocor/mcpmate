mod support;

use mcpmate::clients::ConfigError;
use mcpmate::clients::models::MergeStrategy;
use mcpmate::clients::service::settings::ActiveClientSettingsUpdate;
use support::client_writeback::{CLIENT_ID, ClientWritebackFixture};

#[test]
fn settings_request_accepts_an_explicit_merge_strategy_override() {
    let request = serde_json::from_value::<mcpmate::api::models::client::ClientSettingsUpdateReq>(serde_json::json!({
        "identifier": CLIENT_ID,
        "merge_strategy_override": "deep_merge"
    }))
    .expect("parse settings request");

    assert_eq!(request.merge_strategy_override, Some(MergeStrategy::DeepMerge));
    assert!(!request.clear_merge_strategy_override);
}

#[test]
fn settings_request_accepts_clearing_the_merge_strategy_override() {
    let request = serde_json::from_value::<mcpmate::api::models::client::ClientSettingsUpdateReq>(serde_json::json!({
        "identifier": CLIENT_ID,
        "clear_merge_strategy_override": true
    }))
    .expect("parse settings request");

    assert_eq!(request.merge_strategy_override, None);
    assert!(request.clear_merge_strategy_override);
}

#[tokio::test]
#[serial_test::serial]
async fn active_client_settings_persist_and_clear_the_override() {
    let fixture = ClientWritebackFixture::new().await;
    fixture
        .service
        .set_active_client_settings(
            CLIENT_ID,
            ActiveClientSettingsUpdate {
                merge_strategy_override: Some(MergeStrategy::DeepMerge),
                ..ActiveClientSettingsUpdate::default()
            },
        )
        .await
        .expect("persist merge strategy through client settings");

    let state = fixture
        .service
        .fetch_state(CLIENT_ID)
        .await
        .expect("load client state")
        .expect("client state exists");
    assert_eq!(state.merge_strategy_override(), Some("deep_merge"));
    assert_eq!(
        state.effective_merge_strategy_value(None).expect("resolve override"),
        MergeStrategy::DeepMerge
    );

    fixture
        .service
        .set_active_client_settings(
            CLIENT_ID,
            ActiveClientSettingsUpdate {
                clear_merge_strategy_override: true,
                ..ActiveClientSettingsUpdate::default()
            },
        )
        .await
        .expect("clear merge strategy through client settings");

    let state = fixture
        .service
        .fetch_state(CLIENT_ID)
        .await
        .expect("reload client state")
        .expect("client state exists");
    assert_eq!(state.merge_strategy_override(), None);
    assert_eq!(
        state
            .effective_merge_strategy_value(None)
            .expect("resolve template strategy"),
        MergeStrategy::Replace
    );
}

#[tokio::test]
#[serial_test::serial]
async fn active_client_settings_reject_conflicting_override_operations() {
    let fixture = ClientWritebackFixture::new().await;

    let error = fixture
        .service
        .set_active_client_settings(
            CLIENT_ID,
            ActiveClientSettingsUpdate {
                display_name: Some("Must Not Persist".to_string()),
                merge_strategy_override: Some(MergeStrategy::DeepMerge),
                clear_merge_strategy_override: true,
                ..ActiveClientSettingsUpdate::default()
            },
        )
        .await
        .expect_err("conflicting override operations must be rejected");

    let ConfigError::DataAccessError(message) = error else {
        panic!("expected data access error, got {error}");
    };
    assert!(message.contains("merge strategy override"));
    let state = fixture
        .service
        .fetch_state(CLIENT_ID)
        .await
        .expect("reload client state")
        .expect("client state exists");
    assert_eq!(state.display_name(), "Writeback Client");
    assert_eq!(state.merge_strategy_override(), None);
}

#[tokio::test]
#[serial_test::serial]
async fn deep_merge_override_preserves_external_entries_in_preview_and_apply() {
    let fixture = ClientWritebackFixture::new().await;
    fixture
        .service
        .set_merge_strategy_override(CLIENT_ID, Some(MergeStrategy::DeepMerge))
        .await
        .expect("set deep merge override");

    let preview = fixture.apply_managed(true).await;
    let preview_after = preview.preview.after.expect("preview after content");
    let preview_config: serde_json::Value = serde_json::from_str(&preview_after).expect("parse preview");
    assert_eq!(
        preview_config["mcpServers"]["plugin-owned"]["command"],
        "plugin-command"
    );
    assert!(preview_config["mcpServers"].get("MCPMate").is_some());

    fixture.apply_managed(false).await;
    let applied = fixture.config().await;
    assert_eq!(applied["mcpServers"]["plugin-owned"]["command"], "plugin-command");
    assert!(applied["mcpServers"].get("MCPMate").is_some());

    let mut reapplied_input = applied;
    reapplied_input["mcpServers"]["added-after-apply"] = serde_json::json!({"command": "later-command"});
    tokio::fs::write(
        &fixture.config_path,
        serde_json::to_vec(&reapplied_input).expect("serialize re-apply input"),
    )
    .await
    .expect("write re-apply input");

    fixture.apply_managed(false).await;
    let reapplied = fixture.config().await;
    assert_eq!(reapplied["mcpServers"]["added-after-apply"]["command"], "later-command");
    assert!(reapplied["mcpServers"].get("MCPMate").is_some());
}

#[tokio::test]
#[serial_test::serial]
async fn rendering_uses_the_system_default_when_template_strategy_is_missing() {
    let fixture = ClientWritebackFixture::new().await;
    sqlx::query("UPDATE client SET merge_strategy = NULL WHERE identifier = ?")
        .bind(CLIENT_ID)
        .execute(&fixture.pool)
        .await
        .expect("remove template merge strategy");

    let preview = fixture.apply_managed(true).await;
    let after = preview.preview.after.expect("preview after content");
    let rendered: serde_json::Value = serde_json::from_str(&after).expect("parse preview");
    assert!(rendered["mcpServers"].get("plugin-owned").is_none());
    assert!(rendered["mcpServers"].get("MCPMate").is_some());
}

#[tokio::test]
#[serial_test::serial]
async fn rendering_rejects_an_unknown_template_merge_strategy() {
    let fixture = ClientWritebackFixture::new().await;
    sqlx::query("UPDATE client SET merge_strategy = 'unknown' WHERE identifier = ?")
        .bind(CLIENT_ID)
        .execute(&fixture.pool)
        .await
        .expect("corrupt template merge strategy");

    let error = fixture
        .apply_managed_result(true)
        .await
        .expect_err("unknown merge strategy must fail");
    assert!(
        error
            .to_string()
            .contains("unsupported persisted merge_strategy 'unknown'")
    );
}

#[tokio::test]
#[serial_test::serial]
async fn system_override_applies_when_the_client_has_no_override() {
    let fixture = ClientWritebackFixture::new().await;
    mcpmate::config::client::runtime_settings::set_default_merge_strategy_override(
        &fixture.pool,
        Some(MergeStrategy::DeepMerge),
    )
    .await
    .expect("set system merge strategy override");

    fixture.apply_managed(false).await;
    let applied = fixture.config().await;
    assert_eq!(applied["mcpServers"]["plugin-owned"]["command"], "plugin-command");
    assert!(applied["mcpServers"].get("MCPMate").is_some());
}

#[tokio::test]
#[serial_test::serial]
async fn template_strategy_applies_when_no_client_or_system_override_exists() {
    let fixture = ClientWritebackFixture::new_with_template_strategy(MergeStrategy::DeepMerge).await;

    fixture.apply_managed(false).await;
    let applied = fixture.config().await;
    assert_eq!(applied["mcpServers"]["plugin-owned"]["command"], "plugin-command");
    assert!(applied["mcpServers"].get("MCPMate").is_some());
}

#[tokio::test]
#[serial_test::serial]
async fn client_override_takes_precedence_over_the_system_override() {
    let fixture = ClientWritebackFixture::new().await;
    mcpmate::config::client::runtime_settings::set_default_merge_strategy_override(
        &fixture.pool,
        Some(MergeStrategy::DeepMerge),
    )
    .await
    .expect("set system merge strategy override");
    fixture
        .service
        .set_merge_strategy_override(CLIENT_ID, Some(MergeStrategy::Replace))
        .await
        .expect("set client merge strategy override");

    fixture.apply_managed(false).await;
    let applied = fixture.config().await;
    assert!(applied["mcpServers"].get("plugin-owned").is_none());
    assert!(applied["mcpServers"].get("MCPMate").is_some());
}

#[tokio::test]
#[serial_test::serial]
async fn missing_client_defaults_row_uses_no_system_override() {
    let fixture = ClientWritebackFixture::new().await;
    let defaults = mcpmate::config::client::runtime_settings::get_client_runtime_defaults(&fixture.pool)
        .await
        .expect("load absent client defaults");

    assert_eq!(defaults.default_merge_strategy_override, None);
}

#[tokio::test]
#[serial_test::serial]
async fn system_override_roundtrips_in_structured_client_defaults_json() {
    let fixture = ClientWritebackFixture::new().await;
    sqlx::query("INSERT INTO client_runtime_settings (key, value) VALUES ('client_defaults', ?)")
        .bind(r#"{"future_option":{"enabled":true}}"#)
        .execute(&fixture.pool)
        .await
        .expect("seed an unknown client default");
    mcpmate::config::client::runtime_settings::set_default_merge_strategy_override(
        &fixture.pool,
        Some(MergeStrategy::Replace),
    )
    .await
    .expect("persist system override");

    let stored: String = sqlx::query_scalar("SELECT value FROM client_runtime_settings WHERE key = 'client_defaults'")
        .fetch_one(&fixture.pool)
        .await
        .expect("load client defaults JSON");
    let stored: serde_json::Value = serde_json::from_str(&stored).expect("parse client defaults JSON");
    assert_eq!(stored["default_merge_strategy_override"], "replace");
    assert_eq!(stored["future_option"]["enabled"], true);

    let defaults = mcpmate::config::client::runtime_settings::set_default_merge_strategy_override(&fixture.pool, None)
        .await
        .expect("clear system override");
    assert_eq!(defaults.default_merge_strategy_override, None);
}

#[tokio::test]
#[serial_test::serial]
async fn replace_override_discards_external_entries_before_detach() {
    let fixture = ClientWritebackFixture::new_with_template_strategy(MergeStrategy::DeepMerge).await;
    fixture
        .service
        .set_merge_strategy_override(CLIENT_ID, Some(MergeStrategy::Replace))
        .await
        .expect("set replace override");

    fixture.apply_managed(false).await;
    let applied = fixture.config().await;
    assert!(applied["mcpServers"].get("plugin-owned").is_none());
    assert!(applied["mcpServers"].get("MCPMate").is_some());

    fixture.service.detach_client(CLIENT_ID).await.expect("detach client");
    let detached = fixture.config().await;
    assert_eq!(detached["mcpServers"], serde_json::json!({}));
}

#[tokio::test]
#[serial_test::serial]
async fn clearing_override_restores_template_replace_behavior() {
    let fixture = ClientWritebackFixture::new().await;
    fixture
        .service
        .set_merge_strategy_override(CLIENT_ID, Some(MergeStrategy::DeepMerge))
        .await
        .expect("set deep merge override");
    fixture
        .service
        .set_merge_strategy_override(CLIENT_ID, None)
        .await
        .expect("clear merge override");

    fixture.apply_managed(false).await;
    let applied = fixture.config().await;
    assert!(applied["mcpServers"].get("plugin-owned").is_none());
    assert!(applied["mcpServers"].get("MCPMate").is_some());
}

#[tokio::test]
#[serial_test::serial]
async fn detach_after_deep_merge_removes_only_the_mcpmate_entry() {
    let fixture = ClientWritebackFixture::new().await;
    fixture
        .service
        .set_merge_strategy_override(CLIENT_ID, Some(MergeStrategy::DeepMerge))
        .await
        .expect("set deep merge override");
    fixture.apply_managed(false).await;

    let changed = fixture.service.detach_client(CLIENT_ID).await.expect("detach client");

    assert!(changed);
    let detached = fixture.config().await;
    assert_eq!(detached["mcpServers"]["plugin-owned"]["command"], "plugin-command");
    assert!(detached["mcpServers"].get("MCPMate").is_none());
}
