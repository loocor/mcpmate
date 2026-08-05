use mcpmate::core::capability::change_policy::{
    ChangeClass, NewRefPolicy, PolicyAction, RelationshipLevel, policy_action,
};
use mcpmate::core::capability::materializer::{
    AuthoringRelationship, CatalogTarget, MaterializationCoordinator, MaterializationInput, MaterializationTrigger,
    ReviewDecisionState, SurfaceAuthoringLoader, SurfaceMaterializer,
};
use mcpmate::core::capability::mode_policy::DirectExposurePolicy;
use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogRecord, DeclarationState,
    InventoryState, KindObservation, ReviewOwnerType, ReviewResolutionAction, SqliteCapabilityCatalog,
    SurfaceReviewOwner,
};
use rmcp::model::{InitializeResult, Tool};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;

fn record(
    name: &str,
    description: &str,
) -> CatalogRecord {
    let tool: Tool = serde_json::from_value(json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object"}
    }))
    .unwrap();
    CatalogRecord::materialize(
        "server-a",
        name,
        format!("fixture__{name}"),
        CapabilityPayload::Tool(tool),
    )
    .unwrap()
}

#[test]
fn policy_table_matches_the_approved_hybrid_update_contract() {
    let cases = [
        (
            ChangeClass::Unchanged,
            RelationshipLevel::Capability,
            NewRefPolicy::Review,
            PolicyAction::Record,
        ),
        (
            ChangeClass::ObservationMetadata,
            RelationshipLevel::Capability,
            NewRefPolicy::Review,
            PolicyAction::Record,
        ),
        (
            ChangeClass::ModelVisible,
            RelationshipLevel::Capability,
            NewRefPolicy::Follow,
            PolicyAction::Review,
        ),
        (
            ChangeClass::InvocationContract,
            RelationshipLevel::Server,
            NewRefPolicy::Follow,
            PolicyAction::Review,
        ),
        (
            ChangeClass::SecurityExecution,
            RelationshipLevel::Server,
            NewRefPolicy::Follow,
            PolicyAction::Review,
        ),
        (
            ChangeClass::BuiltinDefinition,
            RelationshipLevel::Builtin,
            NewRefPolicy::Follow,
            PolicyAction::Follow,
        ),
        (
            ChangeClass::OriginKey,
            RelationshipLevel::Server,
            NewRefPolicy::Follow,
            PolicyAction::ManualRebind,
        ),
        (
            ChangeClass::Missing,
            RelationshipLevel::Server,
            NewRefPolicy::Follow,
            PolicyAction::Review,
        ),
        (
            ChangeClass::Reappeared,
            RelationshipLevel::Server,
            NewRefPolicy::Follow,
            PolicyAction::Follow,
        ),
        (
            ChangeClass::Reappeared,
            RelationshipLevel::Capability,
            NewRefPolicy::Follow,
            PolicyAction::Review,
        ),
        (
            ChangeClass::NewRef,
            RelationshipLevel::Capability,
            NewRefPolicy::Follow,
            PolicyAction::Record,
        ),
        (
            ChangeClass::NewRef,
            RelationshipLevel::Server,
            NewRefPolicy::Follow,
            PolicyAction::Follow,
        ),
        (
            ChangeClass::BackendEvidence,
            RelationshipLevel::Server,
            NewRefPolicy::Follow,
            PolicyAction::Record,
        ),
        (
            ChangeClass::Authoring,
            RelationshipLevel::Capability,
            NewRefPolicy::Review,
            PolicyAction::Review,
        ),
    ];

    for (change, level, new_ref_policy, expected) in cases {
        assert_eq!(
            policy_action(change, level, new_ref_policy),
            expected,
            "{change:?}/{level:?}/{new_ref_policy:?}"
        );
    }
}

#[tokio::test]
async fn materializer_combines_owners_with_the_strictest_policy_and_restores_items_independently() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    let first = record("first", "version one");
    let second = record("second", "version one");
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-a",
            "fixture",
            "config-v1",
            initialize,
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            vec![first.clone(), second.clone()],
        ))
        .await
        .unwrap();

    let profile_owner = SurfaceReviewOwner::new(ReviewOwnerType::StandardProfile, "profile-a");
    let direct_owner = SurfaceReviewOwner::new(ReviewOwnerType::ConsumerDirectExposure, "consumer-a");
    let relationships = vec![
        AuthoringRelationship::new(
            profile_owner.clone(),
            first.ref_id.clone(),
            CapabilityKind::Tools,
            first.external_key.clone(),
            RelationshipLevel::Server,
            NewRefPolicy::Follow,
        ),
        AuthoringRelationship::new(
            direct_owner,
            first.ref_id.clone(),
            CapabilityKind::Tools,
            first.external_key.clone(),
            RelationshipLevel::Capability,
            NewRefPolicy::Review,
        ),
        AuthoringRelationship::new(
            profile_owner,
            second.ref_id.clone(),
            CapabilityKind::Tools,
            second.external_key.clone(),
            RelationshipLevel::Capability,
            NewRefPolicy::Review,
        ),
    ];
    let targets = vec![
        CatalogTarget::active(
            first.ref_id.clone(),
            first.capability_id.clone(),
            0,
            ChangeClass::ModelVisible,
        ),
        CatalogTarget::active(
            second.ref_id.clone(),
            second.capability_id.clone(),
            0,
            ChangeClass::ModelVisible,
        ),
    ];
    let approved_first = ReviewDecisionState::new(
        first.ref_id.clone(),
        first.capability_id.clone(),
        0,
        ReviewResolutionAction::ApproveTarget,
    );

    let output = SurfaceMaterializer::compile(MaterializationInput::new(
        "consumer-a",
        relationships,
        targets,
        vec![approved_first],
    ))
    .unwrap();

    assert_eq!(output.proposed_manifest.entries.len(), 2);
    assert_eq!(output.publishable_manifest.entries.len(), 1);
    assert_eq!(output.publishable_manifest.entries[0].ref_id, first.ref_id);
    assert_eq!(output.review_candidates.len(), 1);
    assert_eq!(output.review_candidates[0].ref_id, second.ref_id);
    assert_eq!(output.review_candidates[0].owners.len(), 1);

    let coordinator = MaterializationCoordinator::new(pool.clone());
    let trigger = MaterializationTrigger::new(
        "catalog_delta",
        "revision-1",
        HashMap::from([("server-a".to_string(), 1)]),
        "system",
    );
    let initial = coordinator.persist(&output, &trigger).await.unwrap();
    assert!(initial.effective_surface_changed);
    assert_eq!(initial.binding.as_ref().unwrap().generation, 1);
    assert_eq!(initial.review_item_ids.len(), 1);

    let repeated = coordinator.persist(&output, &trigger).await.unwrap();
    assert!(!repeated.effective_surface_changed);
    assert_eq!(repeated.binding.as_ref().unwrap().generation, 1);
    assert_eq!(repeated.review_item_ids, initial.review_item_ids);
    let manifest_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_manifests")
        .fetch_one(&pool)
        .await
        .unwrap();
    let publication_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_publications")
        .fetch_one(&pool)
        .await
        .unwrap();
    let review_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_review_items")
        .fetch_one(&pool)
        .await
        .unwrap();
    let proposal_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_proposals")
        .fetch_one(&pool)
        .await
        .unwrap();
    let change_event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM capability_change_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(manifest_count, 2);
    assert_eq!(publication_count, 1);
    assert_eq!(review_count, 1);
    assert_eq!(proposal_count, 1);
    assert_eq!(change_event_count, 2);

    let stale_trigger = MaterializationTrigger::new(
        "management_save",
        "stale-save",
        HashMap::from([("server-a".to_string(), 0)]),
        "admin",
    );
    assert!(matches!(
        coordinator.persist(&output, &stale_trigger).await,
        Err(mcpmate_capability_store::CatalogError::ConcurrencyConflict { .. })
    ));
    let incomplete_trigger = MaterializationTrigger::new("management_save", "incomplete-save", HashMap::new(), "admin");
    assert!(matches!(
        coordinator.persist(&output, &incomplete_trigger).await,
        Err(mcpmate_capability_store::CatalogError::ConcurrencyConflict { .. })
    ));
}

#[tokio::test]
async fn resolved_proposal_is_idempotent_for_repeated_complete_trigger() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    let capability = record("builtin", "version one");
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-a",
            "fixture",
            "config-v1",
            initialize,
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            vec![capability.clone()],
        ))
        .await
        .unwrap();
    let output = SurfaceMaterializer::compile(MaterializationInput::new(
        "consumer-a",
        vec![AuthoringRelationship::new(
            SurfaceReviewOwner::new(ReviewOwnerType::ModeRule, "builtin"),
            capability.ref_id.clone(),
            CapabilityKind::Tools,
            capability.external_key.clone(),
            RelationshipLevel::Builtin,
            NewRefPolicy::Follow,
        )],
        vec![CatalogTarget::active(
            capability.ref_id,
            capability.capability_id,
            0,
            ChangeClass::BuiltinDefinition,
        )],
        Vec::new(),
    ))
    .unwrap();
    assert!(output.review_candidates.is_empty());

    let coordinator = MaterializationCoordinator::new(pool.clone());
    let trigger = MaterializationTrigger::new(
        "catalog_delta",
        "revision-1",
        HashMap::from([("server-a".to_string(), 1)]),
        "system",
    );
    coordinator.persist(&output, &trigger).await.unwrap();
    coordinator.persist(&output, &trigger).await.unwrap();

    let proposal_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_proposals")
        .fetch_one(&pool)
        .await
        .unwrap();
    let change_event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM capability_change_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    let publication_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_publications")
        .fetch_one(&pool)
        .await
        .unwrap();
    let lifecycle: String = sqlx::query_scalar("SELECT lifecycle FROM surface_proposals")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(proposal_count, 1);
    assert_eq!(change_event_count, 1);
    assert_eq!(publication_count, 1);
    assert_eq!(lifecycle, "resolved");
}

#[tokio::test]
async fn authoring_loader_combines_direct_exposure_and_builtin_records_without_session_input() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database_support::prepare_config(&pool).await;
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    mcpmate::config::server::init::initialize_server_tables(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO server_config (id, name, server_type, enabled, unify_direct_exposure_eligible) \
         VALUES ('server-a', 'Fixture', 'stdio', 1, 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let upstream = record("upstream", "version one");
    let builtin = CatalogRecord::materialize(
        mcpmate_capability_store::BUILTIN_CAPABILITY_SOURCE_ID,
        "mcpmate_builtin",
        "mcpmate_builtin",
        CapabilityPayload::Tool(
            serde_json::from_value(json!({
                "name": "mcpmate_builtin",
                "description": "built in",
                "inputSchema": {"type": "object"}
            }))
            .unwrap(),
        ),
    )
    .unwrap();
    let initialize: InitializeResult = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap();
    catalog
        .commit_observation(CapabilityObservation::new(
            "server-a",
            "fixture",
            "config-v1",
            initialize,
            vec![KindObservation::new(
                CapabilityKind::Tools,
                DeclarationState::Supported,
                InventoryState::Complete,
            )],
            vec![upstream.clone()],
        ))
        .await
        .unwrap();
    sqlx::query("INSERT INTO client (id, name, identifier) VALUES ('client-a', 'Consumer A', 'consumer-a')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO direct_exposure_refs VALUES ('consumer-a', ?, 1)")
        .bind(upstream.ref_id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let mut transaction = pool.begin().await.unwrap();
    let relationships = SurfaceAuthoringLoader::load_relationships_in_transaction(
        &mut transaction,
        "consumer-a",
        &[],
        DirectExposurePolicy::CapabilityLevel,
        std::slice::from_ref(&builtin),
        Some("unify"),
    )
    .await
    .unwrap();
    assert_eq!(relationships.len(), 2);
    assert!(relationships.iter().any(|relationship| {
        relationship.ref_id == builtin.ref_id && relationship.level == RelationshipLevel::Builtin
    }));
    let targets = SurfaceAuthoringLoader::load_catalog_targets_in_transaction(
        &mut transaction,
        &relationships
            .iter()
            .filter(|relationship| relationship.ref_id == upstream.ref_id)
            .cloned()
            .collect::<Vec<_>>(),
        &HashMap::new(),
    )
    .await
    .unwrap();
    assert_eq!(targets.len(), 1);
    transaction.rollback().await.unwrap();
}
#[path = "support/database.rs"]
mod database_support;
