use std::{collections::HashMap, sync::Arc};

use mcpmate::clients::models::{
    ClientTemplate, ConfigMapping, FormatRule, MergeStrategy, StorageConfig, StorageKind, TemplateFormat,
};
use mcpmate::clients::{ClientConfigService, ClientRenderOptions, ConfigMode, DbTemplateSource};
use mcpmate::common::MCPMatePaths;
use mcpmate::core::capability::materializer::{MaterializationCoordinator, MaterializationTrigger};
use mcpmate::system::settings::{
    SystemSettings, apply_settings_with_effects_for_paths_and_pool, get_settings_sync_for_paths,
    resume_pending_configuration_mode_transitions, set_settings_sync_for_paths,
};
use mcpmate_capability_store::{
    BUILTIN_CAPABILITY_SOURCE_ID, CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload,
    CatalogRecord, DeclarationState, InventoryState, KindObservation, SqliteCapabilityCatalog,
};
use rmcp::model::{InitializeResult, Tool};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;

struct Fixture {
    _temp_dir: TempDir,
    paths: MCPMatePaths,
    pool: sqlx::SqlitePool,
    service: Arc<ClientConfigService>,
    config_path: std::path::PathBuf,
}

async fn fixture(explicit_mode: Option<&str>) -> Fixture {
    let temp_dir = TempDir::new().expect("create temp dir");
    let paths = MCPMatePaths::from_base_dir(temp_dir.path()).expect("create test paths");
    paths.ensure_directories().expect("create test directories");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create database");
    mcpmate::config::server::init::initialize_server_tables(&pool)
        .await
        .expect("initialize servers");
    mcpmate::config::client::init::initialize_client_table(&pool)
        .await
        .expect("initialize clients");
    SqliteCapabilityCatalog::new(pool.clone())
        .ensure_schema()
        .await
        .expect("initialize capability schema");
    mcpmate::config::profile::init::initialize_profile_tables(&pool)
        .await
        .expect("initialize profiles");
    let config_path = temp_dir.path().join("client-a.json");
    tokio::fs::write(&config_path, r#"{"mcpServers":{}}"#)
        .await
        .expect("write client config");
    let transports = HashMap::from([(
        "streamable_http".to_string(),
        FormatRule {
            template: json!({"url": "{{{url}}}"}),
            selected: Some(true),
            ..FormatRule::default()
        },
    )]);
    let template = ClientTemplate {
        identifier: "client-a".to_string(),
        display_name: Some("Client A".to_string()),
        format: TemplateFormat::Json,
        storage: StorageConfig {
            kind: StorageKind::File,
            path_strategy: Some("config_path".to_string()),
            adapter: None,
        },
        config_mapping: ConfigMapping {
            container_keys: vec!["mcpServers".to_string()],
            container_type: mcpmate::clients::ContainerType::ObjectMap,
            merge_strategy: MergeStrategy::Replace,
            keep_original_config: false,
            managed_endpoint: None,
            managed_source: Some("profile".to_string()),
            parse: None,
            format_rules: transports.clone(),
        },
        ..ClientTemplate::default()
    };
    sqlx::query(
        "INSERT INTO client_template_runtime (identifier, payload_json, updated_at) \
         VALUES (?, ?, CURRENT_TIMESTAMP)",
    )
    .bind(&template.identifier)
    .bind(serde_json::to_string(&template).expect("serialize client template"))
    .execute(&pool)
    .await
    .expect("insert client template");

    let records = mcpmate::mcper::HOSTED_BUILTIN_TOOL_NAMES
        .iter()
        .map(|name| {
            CatalogRecord::materialize(
                BUILTIN_CAPABILITY_SOURCE_ID,
                *name,
                *name,
                CapabilityPayload::Tool(Tool::new(
                    *name,
                    format!("{name} description"),
                    Arc::new(json!({"type": "object"}).as_object().expect("object schema").clone()),
                )),
            )
            .expect("materialize builtin")
        })
        .collect::<Vec<_>>();
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "builtin-fixture", "version": "1.0.0"}
    }))
    .expect("initialize payload");
    SqliteCapabilityCatalog::new(pool.clone())
        .commit_observation(CapabilityObservation::new(
            BUILTIN_CAPABILITY_SOURCE_ID,
            "MCPMate Builtin Services",
            "builtin-v1",
            initialize,
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            records,
        ))
        .await
        .expect("commit builtin catalog");

    sqlx::query(
        r#"
        INSERT INTO client (
            id, identifier, name, config_mode, approval_status,
            capability_source, selected_profile_ids, config_path, connection_mode,
            attachment_state, template_identifier, config_format, container_type,
            container_keys, storage_kind, storage_path_strategy, merge_strategy,
            keep_original_config, managed_source, transports
        )
        VALUES (
            'consumer-a', 'client-a', 'Client A', ?, 'approved',
            'profiles', '[]', ?, 'local_config_detected',
            'attached', 'client-a', 'json', 'object',
            '["mcpServers"]', 'file', 'config_path', 'replace',
            0, 'profile', ?
        )
        "#,
    )
    .bind(explicit_mode)
    .bind(config_path.to_string_lossy().to_string())
    .bind(serde_json::to_string(&transports).expect("serialize transports"))
    .execute(&pool)
    .await
    .expect("insert consumer");

    let pool = Arc::new(pool);
    let source = Arc::new(DbTemplateSource::new(pool.clone()).expect("create template source"));
    let service = Arc::new(
        ClientConfigService::with_source(pool.clone(), source)
            .await
            .expect("create client service"),
    );
    Fixture {
        _temp_dir: temp_dir,
        paths,
        pool: pool.as_ref().clone(),
        service,
        config_path,
    }
}

async fn publish_for_mode(
    pool: &sqlx::SqlitePool,
    mode: &str,
) {
    let revisions = sqlx::query_as::<_, (String, i64)>(
        "SELECT server_id, catalog_revision FROM capability_server_snapshots ORDER BY server_id",
    )
    .fetch_all(pool)
    .await
    .expect("load revisions")
    .into_iter()
    .collect::<HashMap<_, _>>();
    let trigger = MaterializationTrigger::new("test_setup", mode, revisions, "test");
    let mut transaction = pool.begin().await.expect("begin setup publication");
    MaterializationCoordinator::new(pool.clone())
        .compile_consumer_in_transaction_with_default(&mut transaction, "client-a", mode, &trigger)
        .await
        .expect("compile setup publication");
    transaction.commit().await.expect("commit setup publication");
}

async fn published_tool_count(pool: &sqlx::SqlitePool) -> Option<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM consumer_surface_bindings binding
        JOIN surface_publications publication
          ON publication.publication_id = binding.active_publication_id
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        JOIN capability_refs ref ON ref.ref_id = entry.ref_id
        WHERE binding.consumer_id = 'client-a' AND ref.kind = 'tools'
        GROUP BY binding.consumer_id
        "#,
    )
    .fetch_optional(pool)
    .await
    .expect("load active tool count")
}

fn expected_tool_count(mode: &str) -> Option<i64> {
    match mode {
        "unify" => Some(mcpmate::mcper::UNIFY_BUILTIN_TOOL_NAMES.len() as i64),
        "hosted" => Some(mcpmate::mcper::HOSTED_BUILTIN_TOOL_NAMES.len() as i64),
        "transparent" => None,
        other => panic!("unexpected mode: {other}"),
    }
}

async fn client_server_count(config_path: &std::path::Path) -> usize {
    let content = tokio::fs::read_to_string(config_path)
        .await
        .expect("read client config");
    serde_json::from_str::<serde_json::Value>(&content)
        .expect("parse client config")
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .expect("mcpServers object")
        .len()
}

async fn apply_client_mode(
    fixture: &Fixture,
    mode: ConfigMode,
) {
    let outcome = fixture
        .service
        .apply_with_deferred(ClientRenderOptions {
            client_id: "client-a".to_string(),
            mode,
            profile_id: None,
            server_ids: None,
            dry_run: false,
        })
        .await
        .expect("apply client mode");
    assert!(outcome.applied || outcome.scheduled);
}

async fn apply_default_transition(
    fixture: &Fixture,
    previous: &SystemSettings,
    next: &SystemSettings,
) -> mcpmate::clients::error::ConfigResult<mcpmate::system::settings::SystemSettingsApplyResult> {
    apply_settings_with_effects_for_paths_and_pool(
        &fixture.paths,
        &fixture.pool,
        previous,
        next,
        Some(fixture.service.clone()),
    )
    .await
}

#[tokio::test]
async fn inherited_managed_client_file_is_rewritten_to_native_on_default_transition() {
    let fixture = fixture(None).await;
    let previous = SystemSettings {
        default_config_mode: "unify".to_string(),
        ..SystemSettings::default()
    };
    set_settings_sync_for_paths(&fixture.paths, &previous).expect("write previous settings");
    publish_for_mode(&fixture.pool, "unify").await;
    apply_client_mode(&fixture, ConfigMode::Managed).await;
    assert_eq!(client_server_count(&fixture.config_path).await, 1);
    let next = SystemSettings {
        default_config_mode: "transparent".to_string(),
        ..previous.clone()
    };

    apply_default_transition(&fixture, &previous, &next)
        .await
        .expect("apply default mode transition");

    assert_eq!(client_server_count(&fixture.config_path).await, 0);
}

#[tokio::test]
async fn inherited_native_client_file_is_rewritten_to_managed_on_default_transition() {
    let fixture = fixture(None).await;
    let previous = SystemSettings {
        default_config_mode: "transparent".to_string(),
        ..SystemSettings::default()
    };
    set_settings_sync_for_paths(&fixture.paths, &previous).expect("write previous settings");
    apply_client_mode(&fixture, ConfigMode::Native).await;
    assert_eq!(client_server_count(&fixture.config_path).await, 0);
    let next = SystemSettings {
        default_config_mode: "hosted".to_string(),
        ..previous.clone()
    };

    apply_default_transition(&fixture, &previous, &next)
        .await
        .expect("apply default mode transition");

    assert_eq!(client_server_count(&fixture.config_path).await, 1);
}

#[tokio::test]
async fn transparent_profiles_with_no_selection_write_no_enabled_servers() {
    let fixture = fixture(None).await;
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, url, enabled) \
         VALUES ('server-a', 'Server A', 'streamable_http', 'http://127.0.0.1:9000/mcp', 1)",
    )
    .execute(&fixture.pool)
    .await
    .expect("insert enabled server");

    apply_client_mode(&fixture, ConfigMode::Native).await;

    assert_eq!(client_server_count(&fixture.config_path).await, 0);
}

#[tokio::test]
async fn inherited_consumers_converge_synchronously_across_all_default_mode_transitions() {
    for (previous_mode, next_mode) in [
        ("unify", "hosted"),
        ("unify", "transparent"),
        ("hosted", "unify"),
        ("hosted", "transparent"),
        ("transparent", "unify"),
        ("transparent", "hosted"),
    ] {
        let fixture = fixture(None).await;
        let previous = SystemSettings {
            default_config_mode: previous_mode.to_string(),
            ..SystemSettings::default()
        };
        set_settings_sync_for_paths(&fixture.paths, &previous).expect("write previous settings");
        if previous_mode != "transparent" {
            publish_for_mode(&fixture.pool, previous_mode).await;
        }
        let next = SystemSettings {
            default_config_mode: next_mode.to_string(),
            ..previous.clone()
        };

        let applied = apply_default_transition(&fixture, &previous, &next)
            .await
            .expect("apply default mode transition");

        assert_eq!(applied.settings.default_config_mode, next_mode);
        assert_eq!(
            published_tool_count(&fixture.pool).await,
            expected_tool_count(next_mode),
            "{previous_mode} -> {next_mode}"
        );
        let pending: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration_mode_transitions WHERE status <> 'completed'")
                .fetch_one(&fixture.pool)
                .await
                .expect("count pending transitions");
        assert_eq!(pending, 0, "{previous_mode} -> {next_mode}");
    }
}

#[tokio::test]
async fn explicit_mode_consumers_ignore_default_mode_transitions() {
    let fixture = fixture(Some("unify")).await;
    let original_config = r#"{"mcpServers":{"explicit-sentinel":{"command":"keep"}}}"#;
    tokio::fs::write(&fixture.config_path, original_config)
        .await
        .expect("write explicit client config");
    let previous = SystemSettings {
        default_config_mode: "unify".to_string(),
        ..SystemSettings::default()
    };
    set_settings_sync_for_paths(&fixture.paths, &previous).expect("write previous settings");
    publish_for_mode(&fixture.pool, "unify").await;
    let next = SystemSettings {
        default_config_mode: "hosted".to_string(),
        ..previous.clone()
    };

    apply_default_transition(&fixture, &previous, &next)
        .await
        .expect("apply default mode transition");

    assert_eq!(published_tool_count(&fixture.pool).await, expected_tool_count("unify"));
    assert_eq!(
        tokio::fs::read_to_string(&fixture.config_path)
            .await
            .expect("read explicit client config"),
        original_config
    );
}

#[tokio::test]
async fn pending_default_mode_transition_is_completed_before_runtime_bootstrap() {
    let fixture = fixture(None).await;
    let previous = SystemSettings {
        default_config_mode: "transparent".to_string(),
        ..SystemSettings::default()
    };
    set_settings_sync_for_paths(&fixture.paths, &previous).expect("write previous settings");
    sqlx::query(
        r#"
        INSERT INTO configuration_mode_transitions (
            transition_id, previous_mode, target_mode, status, created_at
        )
        VALUES ('transition-recovery', 'transparent', 'unify', 'pending', CURRENT_TIMESTAMP)
        "#,
    )
    .execute(&fixture.pool)
    .await
    .expect("insert pending transition");

    let resumed =
        resume_pending_configuration_mode_transitions(&fixture.paths, &fixture.pool, Some(fixture.service.clone()))
            .await
            .expect("resume pending transition");

    assert_eq!(resumed, 1);
    assert_eq!(
        get_settings_sync_for_paths(&fixture.paths)
            .expect("read recovered settings")
            .default_config_mode,
        "unify"
    );
    assert_eq!(published_tool_count(&fixture.pool).await, expected_tool_count("unify"));
    assert_eq!(client_server_count(&fixture.config_path).await, 1);
    let status: String = sqlx::query_scalar(
        "SELECT status FROM configuration_mode_transitions WHERE transition_id = 'transition-recovery'",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("load transition status");
    assert_eq!(status, "completed");
}

#[tokio::test]
async fn failed_client_file_convergence_keeps_the_transition_pending_for_recovery() {
    let fixture = fixture(None).await;
    let previous = SystemSettings {
        default_config_mode: "unify".to_string(),
        ..SystemSettings::default()
    };
    set_settings_sync_for_paths(&fixture.paths, &previous).expect("write previous settings");
    publish_for_mode(&fixture.pool, "unify").await;
    apply_client_mode(&fixture, ConfigMode::Managed).await;
    tokio::fs::remove_file(&fixture.config_path)
        .await
        .expect("remove client config target");
    let next = SystemSettings {
        default_config_mode: "transparent".to_string(),
        ..previous.clone()
    };

    let error = apply_default_transition(&fixture, &previous, &next).await;
    assert!(
        error.is_err(),
        "missing client config target must leave the transition recoverable"
    );

    let status: String =
        sqlx::query_scalar("SELECT status FROM configuration_mode_transitions ORDER BY created_at DESC LIMIT 1")
            .fetch_one(&fixture.pool)
            .await
            .expect("load pending transition");
    assert_eq!(status, "pending");

    tokio::fs::write(&fixture.config_path, r#"{"mcpServers":{"stale":{}}}"#)
        .await
        .expect("restore client config target");
    let resumed =
        resume_pending_configuration_mode_transitions(&fixture.paths, &fixture.pool, Some(fixture.service.clone()))
            .await
            .expect("resume client file convergence");

    assert_eq!(resumed, 1);
    assert_eq!(client_server_count(&fixture.config_path).await, 0);
    let status: String =
        sqlx::query_scalar("SELECT status FROM configuration_mode_transitions ORDER BY created_at DESC LIMIT 1")
            .fetch_one(&fixture.pool)
            .await
            .expect("load completed transition");
    assert_eq!(status, "completed");
}

#[tokio::test]
async fn only_one_default_mode_transition_can_remain_pending() {
    let fixture = fixture(None).await;
    sqlx::query(
        r#"
        INSERT INTO configuration_mode_transitions (
            transition_id, previous_mode, target_mode, status, created_at
        )
        VALUES ('transition-first', 'unify', 'hosted', 'pending', CURRENT_TIMESTAMP)
        "#,
    )
    .execute(&fixture.pool)
    .await
    .expect("insert first pending transition");

    let error = sqlx::query(
        r#"
        INSERT INTO configuration_mode_transitions (
            transition_id, previous_mode, target_mode, status, created_at
        )
        VALUES ('transition-second', 'unify', 'transparent', 'pending', CURRENT_TIMESTAMP)
        "#,
    )
    .execute(&fixture.pool)
    .await
    .expect_err("a second pending default mode transition must be rejected");

    assert!(error.as_database_error().is_some());
}
