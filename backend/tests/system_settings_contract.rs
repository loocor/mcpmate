use mcpmate::api::models::system::SystemSettingsUpdateReq;

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
