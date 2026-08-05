use std::sync::Arc;

use mcpmate::clients::models::{
    CapabilitySource, UnifyDirectCapabilityRefs, UnifyDirectExposureIntent, UnifyRouteMode,
};
use mcpmate::clients::{ClientConfigService, DbTemplateSource};
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogRecord, DeclarationState,
    InventoryState, KindObservation, SqliteCapabilityCatalog,
};
use rmcp::model::{InitializeResult, Tool};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

async fn fixture() -> (sqlx::SqlitePool, ClientConfigService, CatalogRecord) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create database");
    database_support::prepare_config(&pool).await;
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
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, command, enabled, unify_direct_exposure_eligible) \
         VALUES ('server-a', 'Server A', 'stdio', '', 1, 1)",
    )
    .execute(&pool)
    .await
    .expect("insert server");
    sqlx::query(
        r#"
        INSERT INTO client (
            id, identifier, name, config_mode, approval_status,
            capability_source, selected_profile_ids, unify_route_mode
        )
        VALUES (
            'consumer-a', 'client-a', 'Client A', 'unify', 'approved',
            'activated', '[]', 'broker_only'
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert consumer");
    let record = CatalogRecord::materialize(
        "server-a",
        "analyze",
        "server_a__analyze",
        CapabilityPayload::Tool(Tool::new(
            "analyze",
            "Analyze input",
            Arc::new(json!({"type": "object"}).as_object().expect("object schema").clone()),
        )),
    )
    .expect("materialize tool");
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .expect("initialize payload");
    SqliteCapabilityCatalog::new(pool.clone())
        .commit_observation(CapabilityObservation::new(
            "server-a",
            "Server A",
            "server-v1",
            initialize,
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            vec![record.clone()],
        ))
        .await
        .expect("commit catalog");
    let pool = Arc::new(pool);
    let source = Arc::new(DbTemplateSource::new(pool.clone()).expect("create template source"));
    let service = ClientConfigService::with_source(pool.clone(), source)
        .await
        .expect("create client service");
    (pool.as_ref().clone(), service, record)
}

#[tokio::test]
async fn capability_level_typed_ref_mismatch_fails_without_replacing_existing_intent() {
    let (pool, service, tool) = fixture().await;
    let requested = UnifyDirectExposureIntent {
        route_mode: UnifyRouteMode::CapabilityLevel,
        server_ids: Vec::new(),
        capability_refs: UnifyDirectCapabilityRefs {
            prompt_refs: vec![tool.ref_id.to_string()],
            ..UnifyDirectCapabilityRefs::default()
        },
    };

    let error = service
        .update_capability_config_state_and_invalidate(
            "client-a",
            Some("unify".to_string()),
            CapabilitySource::Activated,
            Vec::new(),
            Some(requested),
            service.catalog_revision_set().await.expect("load catalog revisions"),
        )
        .await
        .expect_err("a Tool Ref must not be accepted in prompt_refs");

    assert!(
        error.to_string().contains("expected prompts but catalog Ref is tools"),
        "unexpected error: {error}"
    );
    let route_mode: String = sqlx::query_scalar("SELECT unify_route_mode FROM client WHERE identifier = 'client-a'")
        .fetch_one(&pool)
        .await
        .expect("load route mode");
    assert_eq!(route_mode, "broker_only");
    let persisted_refs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM direct_exposure_refs WHERE consumer_id = 'client-a'")
            .fetch_one(&pool)
            .await
            .expect("count direct refs");
    assert_eq!(persisted_refs, 0);
}

#[tokio::test]
async fn retired_same_kind_refs_remain_as_intent_but_are_excluded_from_the_active_publication() {
    let (pool, service, tool) = fixture().await;
    SqliteCapabilityCatalog::new(pool.clone())
        .remove_server("server-a")
        .await
        .expect("retire server catalog");
    let requested = UnifyDirectExposureIntent {
        route_mode: UnifyRouteMode::CapabilityLevel,
        server_ids: Vec::new(),
        capability_refs: UnifyDirectCapabilityRefs {
            tool_refs: vec![tool.ref_id.to_string()],
            ..UnifyDirectCapabilityRefs::default()
        },
    };

    let (state, _, _) = service
        .update_capability_config_state_and_invalidate(
            "client-a",
            Some("unify".to_string()),
            CapabilitySource::Activated,
            Vec::new(),
            Some(requested),
            service.catalog_revision_set().await.expect("load catalog revisions"),
        )
        .await
        .expect("persist retired same-kind intent");

    assert_eq!(
        state.unify_direct_exposure_intent.capability_refs.tool_refs,
        vec![tool.ref_id.to_string()]
    );
    assert!(state.unify_direct_exposure.selected_tool_surfaces.is_empty());
    assert_eq!(
        state.unify_direct_exposure_diagnostics.invalid_capability_refs,
        vec![tool.ref_id.to_string()]
    );
    let persisted_refs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM direct_exposure_refs WHERE consumer_id = 'client-a' AND ref_id = ?")
            .bind(tool.ref_id.as_str())
            .fetch_one(&pool)
            .await
            .expect("count retained direct refs");
    assert_eq!(persisted_refs, 1);
    let published_refs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM consumer_surface_bindings binding
        JOIN surface_publications publication
          ON publication.publication_id = binding.active_publication_id
        JOIN surface_manifest_entries entry ON entry.manifest_id = publication.manifest_id
        WHERE binding.consumer_id = 'client-a' AND entry.ref_id = ?
        "#,
    )
    .bind(tool.ref_id.as_str())
    .fetch_one(&pool)
    .await
    .expect("count published retired refs");
    assert_eq!(published_refs, 0);
}
#[path = "support/database.rs"]
mod database_support;
