use mcpmate_capability_store::{
    CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogError, CatalogRecord,
    DeclarationState, InventoryState, KindObservation, ProposalLifecycle, ReviewLifecycle, ReviewOwnerType,
    ReviewResolutionAction, ReviewTargetKey, SqliteCapabilityCatalog, SqliteSurfaceStore, SurfaceManifest,
    SurfaceManifestEntryInput, SurfaceProposal, SurfacePublication, SurfaceReviewDecisionDraft, SurfaceReviewFilter,
    SurfaceReviewItemDraft, SurfaceReviewOwner,
};
use mcpmate_migrations::{DatabaseSource, prepare_config_database};
use rmcp::model::{InitializeResult, Tool};
use serde_json::json;
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};

fn initialize_result() -> InitializeResult {
    serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {"listChanged": true}},
        "serverInfo": {"name": "fixture", "version": "1.0.0"}
    }))
    .unwrap()
}

fn tool_record(
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

fn observation(records: Vec<CatalogRecord>) -> CapabilityObservation {
    CapabilityObservation::new(
        "server-a",
        "fixture",
        "config-v1",
        initialize_result(),
        vec![KindObservation::new(
            CapabilityKind::Tools,
            DeclarationState::Supported,
            InventoryState::Complete,
        )],
        records,
    )
}

async fn test_store() -> (Pool<Sqlite>, SqliteCapabilityCatalog, SqliteSurfaceStore) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    prepare_config_database(&pool, DatabaseSource::InMemory)
        .await
        .expect("prepare config schema");
    let catalog = SqliteCapabilityCatalog::new(pool.clone());
    catalog.ensure_schema().await.unwrap();
    let store = SqliteSurfaceStore::new(pool.clone());
    (pool, catalog, store)
}

fn entry(record: &CatalogRecord) -> SurfaceManifestEntryInput {
    SurfaceManifestEntryInput::new(
        record.ref_id.clone(),
        record.capability_id.clone(),
        record.kind(),
        record.external_key.clone(),
    )
}

async fn insert_manifest(
    pool: &Pool<Sqlite>,
    store: &SqliteSurfaceStore,
    manifest: &SurfaceManifest,
) {
    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_manifest_in_transaction(&mut transaction, manifest)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
}

#[tokio::test]
async fn owner_scoped_review_decisions_reject_incomplete_payloads() {
    let (pool, _, store) = test_store().await;
    let invalid_decisions = [
        (
            "null payload",
            SurfaceReviewDecisionDraft::new(
                "decision-remove-null",
                "review-a",
                ReviewResolutionAction::RemoveIntent,
                Some(json!(null)),
                "reviewer-a",
            ),
        ),
        (
            "owner without ID",
            SurfaceReviewDecisionDraft::new(
                "decision-remove-owner",
                "review-a",
                ReviewResolutionAction::RemoveIntent,
                Some(json!({"owner": {"owner_type": "standard_profile"}})),
                "reviewer-a",
            ),
        ),
        (
            "rebind without ref ID",
            SurfaceReviewDecisionDraft::new(
                "decision-rebind-ref",
                "review-a",
                ReviewResolutionAction::RebindRef,
                Some(json!({
                    "owner": {
                        "owner_type": "standard_profile",
                        "owner_id": "profile-a"
                    }
                })),
                "reviewer-a",
            ),
        ),
    ];

    for (case, decision) in invalid_decisions {
        let mut transaction = pool.begin().await.unwrap();
        let error = store
            .append_review_decision_in_transaction(&mut transaction, &decision, None)
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                CatalogError::InvalidSurfaceValue {
                    field: "review decision payload",
                    ..
                }
            ),
            "{case}"
        );
        transaction.rollback().await.unwrap();
    }
}

#[tokio::test]
async fn manifest_identity_is_deterministic_consumer_scoped_and_insert_or_verify() {
    let (pool, catalog, store) = test_store().await;
    let first = tool_record("zeta", "first");
    let second = tool_record("alpha", "second");
    catalog
        .commit_observation(observation(vec![first.clone(), second.clone()]))
        .await
        .unwrap();

    let forward = SurfaceManifest::compile("consumer-a", vec![entry(&first), entry(&second)]).unwrap();
    let reverse = SurfaceManifest::compile("consumer-a", vec![entry(&second), entry(&first)]).unwrap();
    let other_consumer = SurfaceManifest::compile("consumer-b", vec![entry(&first), entry(&second)]).unwrap();

    assert_eq!(forward.manifest_id, reverse.manifest_id);
    assert_eq!(forward.canonical_content, reverse.canonical_content);
    assert_ne!(forward.manifest_id, other_consumer.manifest_id);
    assert_eq!(forward.entries[0].ref_id, second.ref_id);

    insert_manifest(&pool, &store, &forward).await;
    insert_manifest(&pool, &store, &reverse).await;
    insert_manifest(&pool, &store, &other_consumer).await;

    let manifest_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM surface_manifests")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(manifest_count, 2);

    sqlx::query("UPDATE surface_manifests SET canonical_content = X'7B7D' WHERE manifest_id = ?")
        .bind(forward.manifest_id.as_str())
        .execute(&pool)
        .await
        .unwrap();
    let mut transaction = pool.begin().await.unwrap();
    let error = store
        .insert_manifest_in_transaction(&mut transaction, &forward)
        .await
        .unwrap_err();
    assert!(matches!(error, CatalogError::IntegrityMismatch { .. }));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn review_items_reuse_target_decisions_and_keep_consumer_isolation() {
    let (pool, catalog, store) = test_store().await;
    let first = tool_record("analyze", "version one");
    catalog
        .commit_observation(observation(vec![first.clone()]))
        .await
        .unwrap();
    let manifest = SurfaceManifest::compile("consumer-a", vec![entry(&first)]).unwrap();
    insert_manifest(&pool, &store, &manifest).await;

    let proposal_a = SurfaceProposal::new(
        "proposal-a",
        "consumer-a",
        None,
        manifest.manifest_id.clone(),
        "catalog_delta",
        "revision-1",
        json!({"server-a": 1}),
        json!({"changed": 1}),
    );
    let proposal_b = SurfaceProposal::new(
        "proposal-b",
        "consumer-a",
        None,
        manifest.manifest_id.clone(),
        "catalog_delta",
        "revision-1-repeat",
        json!({"server-a": 1}),
        json!({"changed": 1}),
    );
    let proposal_other_consumer = SurfaceProposal::new(
        "proposal-consumer-b",
        "consumer-b",
        None,
        SurfaceManifest::compile("consumer-b", vec![entry(&first)])
            .unwrap()
            .manifest_id,
        "catalog_delta",
        "revision-1",
        json!({"server-a": 1}),
        json!({"changed": 1}),
    );

    let target_key = ReviewTargetKey::capability(&first.capability_id);
    let draft_a = SurfaceReviewItemDraft::new(
        "review-a",
        "proposal-a",
        "consumer-a",
        first.ref_id.clone(),
        None,
        Some(first.capability_id.clone()),
        target_key.clone(),
        "model_visible",
        "review",
    );
    let draft_repeated = SurfaceReviewItemDraft::new(
        "review-duplicate",
        "proposal-b",
        "consumer-a",
        first.ref_id.clone(),
        None,
        Some(first.capability_id.clone()),
        target_key.clone(),
        "model_visible",
        "review",
    );
    let draft_other_consumer = SurfaceReviewItemDraft::new(
        "review-consumer-b",
        "proposal-consumer-b",
        "consumer-b",
        first.ref_id.clone(),
        None,
        Some(first.capability_id.clone()),
        target_key,
        "model_visible",
        "review",
    );
    let owner = SurfaceReviewOwner::new(ReviewOwnerType::StandardProfile, "profile-a");

    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_manifest_in_transaction(
            &mut transaction,
            &SurfaceManifest::compile("consumer-b", vec![entry(&first)]).unwrap(),
        )
        .await
        .unwrap();
    store
        .insert_proposal_in_transaction(&mut transaction, &proposal_a)
        .await
        .unwrap();
    store
        .insert_proposal_in_transaction(&mut transaction, &proposal_b)
        .await
        .unwrap();
    store
        .insert_proposal_in_transaction(&mut transaction, &proposal_other_consumer)
        .await
        .unwrap();
    let first_item = store
        .create_or_reuse_review_item_in_transaction(&mut transaction, &draft_a, std::slice::from_ref(&owner))
        .await
        .unwrap();
    let repeated_item = store
        .create_or_reuse_review_item_in_transaction(&mut transaction, &draft_repeated, std::slice::from_ref(&owner))
        .await
        .unwrap();
    let other_item = store
        .create_or_reuse_review_item_in_transaction(
            &mut transaction,
            &draft_other_consumer,
            &[SurfaceReviewOwner::new(
                ReviewOwnerType::ConsumerDirectExposure,
                "consumer-b",
            )],
        )
        .await
        .unwrap();
    assert_eq!(first_item.review_item_id, repeated_item.review_item_id);
    assert_ne!(first_item.review_item_id, other_item.review_item_id);

    let reject = SurfaceReviewDecisionDraft::new(
        "decision-reject",
        first_item.review_item_id.clone(),
        ReviewResolutionAction::RejectTarget,
        None,
        "reviewer-a",
    );
    store
        .append_review_decision_in_transaction(&mut transaction, &reject, None)
        .await
        .unwrap();
    let approve = SurfaceReviewDecisionDraft::new(
        "decision-approve",
        first_item.review_item_id.clone(),
        ReviewResolutionAction::ApproveTarget,
        None,
        "reviewer-a",
    );
    store
        .append_review_decision_in_transaction(&mut transaction, &approve, Some("decision-reject"))
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let current = store
        .load_review_item(&first_item.review_item_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.lifecycle, ReviewLifecycle::Resolved);
    assert_eq!(current.current_decision_id.as_deref(), Some("decision-approve"));
    let other = store
        .load_review_item(&other_item.review_item_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(other.lifecycle, ReviewLifecycle::Pending);
    assert!(other.current_decision_id.is_none());
    let decision_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM surface_review_decisions WHERE review_item_id = ?")
            .bind(&first_item.review_item_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(decision_count, 2);
    let proposal_link_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM surface_proposal_review_items WHERE review_item_id = ?")
            .bind(&first_item.review_item_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(proposal_link_count, 2);
}

#[tokio::test]
async fn review_queries_share_owner_decision_and_lifecycle_facts() {
    let (pool, catalog, store) = test_store().await;
    let record = tool_record("analyze", "version one");
    catalog
        .commit_observation(observation(vec![record.clone()]))
        .await
        .unwrap();
    let manifest = SurfaceManifest::compile("consumer-a", vec![entry(&record)]).unwrap();
    insert_manifest(&pool, &store, &manifest).await;
    let proposal = SurfaceProposal::new(
        "proposal-query",
        "consumer-a",
        None,
        manifest.manifest_id,
        "catalog_delta",
        "revision-query",
        json!({"server-a": 1}),
        json!({"changed": 1}),
    );
    let draft = SurfaceReviewItemDraft::new(
        "review-query",
        "proposal-query",
        "consumer-a",
        record.ref_id,
        None,
        Some(record.capability_id.clone()),
        ReviewTargetKey::capability(&record.capability_id),
        "model_visible",
        "review",
    );
    let owners = [
        SurfaceReviewOwner::new(ReviewOwnerType::StandardProfile, "profile-a"),
        SurfaceReviewOwner::new(ReviewOwnerType::ConsumerDirectExposure, "consumer-a"),
    ];

    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_proposal_in_transaction(&mut transaction, &proposal)
        .await
        .unwrap();
    store
        .create_or_reuse_review_item_in_transaction(&mut transaction, &draft, &owners)
        .await
        .unwrap();
    store
        .append_review_decision_in_transaction(
            &mut transaction,
            &SurfaceReviewDecisionDraft::new(
                "decision-query",
                "review-query",
                ReviewResolutionAction::RemoveIntent,
                Some(json!({
                    "owner": {
                        "owner_type": "standard_profile",
                        "owner_id": "profile-a"
                    }
                })),
                "reviewer-a",
            ),
            None,
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let records = store
        .list_review_items(&SurfaceReviewFilter {
            consumer_id: Some("consumer-a".to_string()),
            owner_type: Some(ReviewOwnerType::StandardProfile),
            owner_id: Some("profile-a".to_string()),
            lifecycle: Some(ReviewLifecycle::Resolved),
        })
        .await
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].item.review_item_id, "review-query");
    assert_eq!(records[0].owners, owners);
    assert_eq!(
        records[0]
            .current_decision
            .as_ref()
            .map(|decision| decision.resolution_action),
        Some(ReviewResolutionAction::RemoveIntent)
    );
    assert_eq!(
        store.load_review_record("review-query").await.unwrap().unwrap(),
        records[0]
    );

    let mut transaction = pool.begin().await.unwrap();
    store
        .deactivate_review_owner_in_transaction(
            &mut transaction,
            "review-query",
            &SurfaceReviewOwner::new(ReviewOwnerType::StandardProfile, "profile-a"),
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let remaining = store.load_review_record("review-query").await.unwrap().unwrap();
    assert_eq!(remaining.item.lifecycle, ReviewLifecycle::Pending);
    assert_eq!(
        remaining.owners,
        vec![SurfaceReviewOwner::new(
            ReviewOwnerType::ConsumerDirectExposure,
            "consumer-a",
        )]
    );
    assert!(remaining.current_decision.is_none());

    let mut transaction = pool.begin().await.unwrap();
    store
        .sync_review_item_owners_in_transaction(
            &mut transaction,
            "review-query",
            "proposal-query",
            &[SurfaceReviewOwner::new(
                ReviewOwnerType::ConsumerDirectExposure,
                "consumer-a",
            )],
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let resynchronized = store.load_review_record("review-query").await.unwrap().unwrap();
    assert_eq!(resynchronized.item.lifecycle, ReviewLifecycle::Pending);
    assert!(resynchronized.current_decision.is_none());
    let decision_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM surface_review_decisions WHERE review_item_id = 'review-query'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(decision_count, 1, "owner-scoped decision history remains immutable");
}

#[tokio::test]
async fn target_generation_and_owner_changes_obsolete_only_the_stale_review_item() {
    let (pool, catalog, store) = test_store().await;
    let record = tool_record("analyze", "version one");
    catalog
        .commit_observation(observation(vec![record.clone()]))
        .await
        .unwrap();
    let manifest = SurfaceManifest::compile("consumer-a", vec![entry(&record)]).unwrap();
    insert_manifest(&pool, &store, &manifest).await;

    let proposal = SurfaceProposal::new(
        "proposal-a",
        "consumer-a",
        None,
        manifest.manifest_id,
        "catalog_delta",
        "missing",
        json!({"server-a": 2}),
        json!({"missing": 1}),
    );
    let missing_one = SurfaceReviewItemDraft::new(
        "review-missing-1",
        "proposal-a",
        "consumer-a",
        record.ref_id.clone(),
        Some(record.capability_id.clone()),
        None,
        ReviewTargetKey::missing(1),
        "missing",
        "review",
    );
    let missing_two = SurfaceReviewItemDraft::new(
        "review-missing-2",
        "proposal-a",
        "consumer-a",
        record.ref_id,
        Some(record.capability_id),
        None,
        ReviewTargetKey::missing(2),
        "missing",
        "review",
    );

    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_proposal_in_transaction(&mut transaction, &proposal)
        .await
        .unwrap();
    let first_item = store
        .create_or_reuse_review_item_in_transaction(
            &mut transaction,
            &missing_one,
            &[SurfaceReviewOwner::new(
                ReviewOwnerType::ConsumerDirectExposure,
                "consumer-a",
            )],
        )
        .await
        .unwrap();
    let second_item = store
        .create_or_reuse_review_item_in_transaction(
            &mut transaction,
            &missing_two,
            &[SurfaceReviewOwner::new(
                ReviewOwnerType::ConsumerDirectExposure,
                "consumer-a",
            )],
        )
        .await
        .unwrap();
    store
        .sync_review_item_owners_in_transaction(&mut transaction, &second_item.review_item_id, "proposal-a", &[])
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    assert_eq!(
        store
            .load_review_item(&first_item.review_item_id)
            .await
            .unwrap()
            .unwrap()
            .lifecycle,
        ReviewLifecycle::Obsolete
    );
    assert_eq!(
        store
            .load_review_item(&second_item.review_item_id)
            .await
            .unwrap()
            .unwrap()
            .lifecycle,
        ReviewLifecycle::Obsolete
    );
}

#[tokio::test]
async fn proposal_lifecycle_is_cas_guarded() {
    let (pool, catalog, store) = test_store().await;
    let record = tool_record("analyze", "version one");
    catalog
        .commit_observation(observation(vec![record.clone()]))
        .await
        .unwrap();
    let manifest = SurfaceManifest::compile("consumer-a", vec![entry(&record)]).unwrap();
    insert_manifest(&pool, &store, &manifest).await;
    let proposal = SurfaceProposal::new(
        "proposal-a",
        "consumer-a",
        None,
        manifest.manifest_id,
        "management_save",
        "save-1",
        json!({"server-a": 1}),
        json!({}),
    );

    let mut transaction = pool.begin().await.unwrap();
    store
        .insert_proposal_in_transaction(&mut transaction, &proposal)
        .await
        .unwrap();
    store
        .transition_proposal_in_transaction(
            &mut transaction,
            "proposal-a",
            ProposalLifecycle::Pending,
            ProposalLifecycle::Resolved,
        )
        .await
        .unwrap();
    let error = store
        .transition_proposal_in_transaction(
            &mut transaction,
            "proposal-a",
            ProposalLifecycle::Pending,
            ProposalLifecycle::Superseded,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, CatalogError::ConcurrencyConflict { .. }));
    transaction.commit().await.unwrap();
}

#[tokio::test]
async fn publication_binding_cas_and_rollback_preserve_append_only_history() {
    let (pool, catalog, store) = test_store().await;
    let first = tool_record("analyze", "version one");
    catalog
        .commit_observation(observation(vec![first.clone()]))
        .await
        .unwrap();
    let manifest_v1 = SurfaceManifest::compile("consumer-a", vec![entry(&first)]).unwrap();
    insert_manifest(&pool, &store, &manifest_v1).await;

    let second = tool_record("analyze", "version two");
    catalog
        .commit_observation(observation(vec![second.clone()]))
        .await
        .unwrap();
    let manifest_v2 = SurfaceManifest::compile("consumer-a", vec![entry(&second)]).unwrap();
    insert_manifest(&pool, &store, &manifest_v2).await;

    let publication_v1 = SurfacePublication::new(
        "publication-v1",
        "consumer-a",
        manifest_v1.manifest_id.clone(),
        None,
        "initial",
        "system",
        None,
    );
    let publication_v2 = SurfacePublication::new(
        "publication-v2",
        "consumer-a",
        manifest_v2.manifest_id.clone(),
        None,
        "catalog_follow",
        "system",
        Some("publication-v1".to_string()),
    );
    let stale_publication = SurfacePublication::new(
        "publication-stale",
        "consumer-a",
        manifest_v1.manifest_id.clone(),
        None,
        "stale",
        "system",
        Some("publication-v1".to_string()),
    );

    let mut transaction = pool.begin().await.unwrap();
    let initial_binding = store
        .publish_and_bind_in_transaction(&mut transaction, &publication_v1, None)
        .await
        .unwrap();
    assert_eq!(initial_binding.generation, 1);
    let current_binding = store
        .publish_and_bind_in_transaction(&mut transaction, &publication_v2, Some(1))
        .await
        .unwrap();
    assert_eq!(current_binding.generation, 2);
    let stale_error = store
        .publish_and_bind_in_transaction(&mut transaction, &stale_publication, Some(1))
        .await
        .unwrap_err();
    assert!(matches!(stale_error, CatalogError::ConcurrencyConflict { .. }));
    assert!(
        store
            .is_publication_rollback_eligible_in_transaction(&mut transaction, "publication-v1")
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        store
            .is_publication_rollback_eligible_in_transaction(&mut transaction, "publication-v2")
            .await
            .unwrap()
            .is_ok()
    );

    let rollback = SurfacePublication::new(
        "publication-rollback",
        "consumer-a",
        manifest_v2.manifest_id,
        None,
        "rollback",
        "reviewer-a",
        Some("publication-v2".to_string()),
    );
    let rollback_binding = store
        .publish_and_bind_in_transaction(&mut transaction, &rollback, Some(2))
        .await
        .unwrap();
    assert_eq!(rollback_binding.generation, 3);
    transaction.commit().await.unwrap();

    let history = store.load_publication_history("consumer-a").await.unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].publication_id, "publication-rollback");
    assert_eq!(
        store.load_publication("publication-v2").await.unwrap().unwrap(),
        publication_v2
    );
    let stale_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM surface_publications WHERE publication_id = 'publication-stale'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stale_count, 0);
    let binding = store.load_binding("consumer-a").await.unwrap().unwrap();
    assert_eq!(binding.active_publication_id, "publication-rollback");
    assert_eq!(binding.generation, 3);
}

#[tokio::test]
async fn binding_generation_remains_monotonic_across_managed_lifecycles() {
    let (pool, catalog, store) = test_store().await;
    let record = tool_record("analyze", "version one");
    catalog
        .commit_observation(observation(vec![record.clone()]))
        .await
        .unwrap();
    let manifest = SurfaceManifest::compile("consumer-a", vec![entry(&record)]).unwrap();
    insert_manifest(&pool, &store, &manifest).await;

    let first_publication = SurfacePublication::new(
        "publication-first-lifecycle",
        "consumer-a",
        manifest.manifest_id.clone(),
        None,
        "initial",
        "system",
        None,
    );
    let mut transaction = pool.begin().await.unwrap();
    let first_binding = store
        .publish_and_bind_in_transaction(&mut transaction, &first_publication, None)
        .await
        .unwrap();
    assert_eq!(first_binding.generation, 1);
    sqlx::query("DELETE FROM consumer_surface_bindings WHERE consumer_id = 'consumer-a'")
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let second_publication = SurfacePublication::new(
        "publication-second-lifecycle",
        "consumer-a",
        manifest.manifest_id,
        None,
        "managed_reentry",
        "system",
        None,
    );
    let mut transaction = pool.begin().await.unwrap();
    let second_binding = store
        .publish_and_bind_in_transaction(&mut transaction, &second_publication, None)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    assert_eq!(second_binding.generation, 2);
    assert_ne!(second_binding.generation, first_binding.generation);
}
