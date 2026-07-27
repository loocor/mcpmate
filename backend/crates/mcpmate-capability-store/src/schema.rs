use sqlx::{Pool, Sqlite};

use crate::Result;

const CAPABILITY_SCHEMA_EPOCH: i64 = 2;
const LEGACY_CAPABILITY_TABLES: &[&str] = &[
    "capability_records",
    "profile_tool",
    "profile_prompt",
    "profile_resource",
    "profile_resource_template",
    "profile_server",
];

pub(crate) async fn ensure_schema(pool: &Pool<Sqlite>) -> Result<()> {
    let mut transaction = pool.begin().await?;
    ensure_compatible_schema(&mut transaction).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_server_snapshots (
            server_id TEXT PRIMARY KEY,
            server_name TEXT NOT NULL,
            config_fingerprint TEXT NOT NULL,
            record_format_version INTEGER NOT NULL,
            catalog_revision INTEGER NOT NULL,
            snapshot_state TEXT NOT NULL,
            initialize_payload TEXT NOT NULL,
            observed_at TEXT NOT NULL,
            committed_at TEXT NOT NULL,
            last_error TEXT
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_kind_states (
            server_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            kind TEXT NOT NULL,
            declaration_state TEXT NOT NULL,
            inventory_state TEXT NOT NULL,
            error TEXT,
            catalog_revision INTEGER NOT NULL,
            observed_at TEXT NOT NULL,
            PRIMARY KEY (server_id, kind),
            FOREIGN KEY (server_id) REFERENCES capability_server_snapshots(server_id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_refs (
            ref_id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            origin_key TEXT NOT NULL,
            state TEXT NOT NULL,
            state_generation INTEGER NOT NULL,
            first_observed_revision INTEGER NOT NULL,
            last_observed_revision INTEGER NOT NULL,
            FOREIGN KEY (server_id) REFERENCES capability_server_snapshots(server_id) ON DELETE CASCADE,
            UNIQUE (server_id, kind, origin_key)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_capability_refs_server_kind ON capability_refs(server_id, kind)")
        .execute(&mut *transaction)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_versions (
            capability_id TEXT PRIMARY KEY,
            ref_id TEXT NOT NULL,
            canonical_record BLOB NOT NULL,
            record_format TEXT NOT NULL,
            first_observed_revision INTEGER NOT NULL,
            FOREIGN KEY (ref_id) REFERENCES capability_refs(ref_id) ON DELETE CASCADE,
            UNIQUE (ref_id, capability_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_capability_versions_ref ON capability_versions(ref_id)")
        .execute(&mut *transaction)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_ref_current (
            ref_id TEXT PRIMARY KEY,
            capability_id TEXT NOT NULL,
            catalog_revision INTEGER NOT NULL,
            FOREIGN KEY (ref_id) REFERENCES capability_refs(ref_id) ON DELETE CASCADE,
            FOREIGN KEY (capability_id) REFERENCES capability_versions(capability_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_capability_ref_current_version ON capability_ref_current(capability_id)",
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_manifests (
            manifest_id TEXT PRIMARY KEY,
            consumer_id TEXT NOT NULL,
            canonical_content BLOB NOT NULL,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_manifest_entries (
            manifest_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            ref_id TEXT NOT NULL,
            capability_id TEXT NOT NULL,
            PRIMARY KEY (manifest_id, position),
            UNIQUE (manifest_id, ref_id),
            FOREIGN KEY (manifest_id) REFERENCES surface_manifests(manifest_id) ON DELETE CASCADE,
            FOREIGN KEY (ref_id, capability_id) REFERENCES capability_versions(ref_id, capability_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_proposals (
            proposal_id TEXT PRIMARY KEY,
            consumer_id TEXT NOT NULL,
            base_publication_id TEXT,
            proposed_manifest_id TEXT NOT NULL,
            trigger_kind TEXT NOT NULL,
            trigger_id TEXT NOT NULL,
            source_revision_set TEXT NOT NULL,
            diff_summary TEXT NOT NULL,
            lifecycle TEXT NOT NULL CHECK (lifecycle IN ('pending', 'resolved', 'superseded')),
            created_at TEXT NOT NULL,
            resolved_at TEXT,
            FOREIGN KEY (proposed_manifest_id) REFERENCES surface_manifests(manifest_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_review_items (
            review_item_id TEXT PRIMARY KEY,
            created_by_proposal_id TEXT NOT NULL,
            consumer_id TEXT NOT NULL,
            ref_id TEXT NOT NULL,
            before_capability_id TEXT,
            target_capability_id TEXT,
            target_key TEXT NOT NULL,
            change_class TEXT NOT NULL,
            policy_action TEXT NOT NULL,
            lifecycle TEXT NOT NULL CHECK (lifecycle IN ('pending', 'resolved', 'obsolete')),
            current_decision_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE (consumer_id, ref_id, target_key),
            FOREIGN KEY (created_by_proposal_id) REFERENCES surface_proposals(proposal_id),
            FOREIGN KEY (ref_id) REFERENCES capability_refs(ref_id),
            FOREIGN KEY (before_capability_id) REFERENCES capability_versions(capability_id),
            FOREIGN KEY (target_capability_id) REFERENCES capability_versions(capability_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_review_decisions (
            decision_id TEXT PRIMARY KEY,
            review_item_id TEXT NOT NULL,
            resolution_action TEXT NOT NULL CHECK (
                resolution_action IN ('approve_target', 'reject_target', 'keep_intent', 'remove_intent', 'rebind_ref')
            ),
            resolution_payload TEXT,
            actor TEXT NOT NULL,
            decided_at TEXT NOT NULL,
            supersedes_decision_id TEXT,
            FOREIGN KEY (review_item_id) REFERENCES surface_review_items(review_item_id),
            FOREIGN KEY (supersedes_decision_id) REFERENCES surface_review_decisions(decision_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_proposal_review_items (
            proposal_id TEXT NOT NULL,
            review_item_id TEXT NOT NULL,
            PRIMARY KEY (proposal_id, review_item_id),
            FOREIGN KEY (proposal_id) REFERENCES surface_proposals(proposal_id) ON DELETE CASCADE,
            FOREIGN KEY (review_item_id) REFERENCES surface_review_items(review_item_id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_review_owners (
            review_item_id TEXT NOT NULL,
            owner_type TEXT NOT NULL CHECK (
                owner_type IN (
                    'standard_profile',
                    'custom_profile',
                    'consumer_direct_exposure',
                    'profile_server_exposure',
                    'consumer_server_exposure',
                    'mode_rule'
                )
            ),
            owner_id TEXT NOT NULL,
            active INTEGER NOT NULL CHECK (active IN (0, 1)),
            first_proposal_id TEXT NOT NULL,
            last_proposal_id TEXT NOT NULL,
            PRIMARY KEY (review_item_id, owner_type, owner_id),
            FOREIGN KEY (review_item_id) REFERENCES surface_review_items(review_item_id) ON DELETE CASCADE,
            FOREIGN KEY (first_proposal_id) REFERENCES surface_proposals(proposal_id),
            FOREIGN KEY (last_proposal_id) REFERENCES surface_proposals(proposal_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_publications (
            publication_id TEXT PRIMARY KEY,
            consumer_id TEXT NOT NULL,
            manifest_id TEXT NOT NULL,
            proposal_id TEXT,
            reason TEXT NOT NULL,
            published_by TEXT NOT NULL,
            published_at TEXT NOT NULL,
            supersedes_publication_id TEXT,
            FOREIGN KEY (manifest_id) REFERENCES surface_manifests(manifest_id),
            FOREIGN KEY (proposal_id) REFERENCES surface_proposals(proposal_id),
            FOREIGN KEY (supersedes_publication_id) REFERENCES surface_publications(publication_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS consumer_surface_bindings (
            consumer_id TEXT PRIMARY KEY,
            active_publication_id TEXT NOT NULL,
            generation INTEGER NOT NULL CHECK (generation > 0),
            FOREIGN KEY (active_publication_id) REFERENCES surface_publications(publication_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS consumer_surface_generations (
            consumer_id TEXT PRIMARY KEY,
            last_generation INTEGER NOT NULL CHECK (last_generation >= 0)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_surface_proposals_consumer ON surface_proposals(consumer_id, created_at)",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_surface_reviews_consumer_state ON surface_review_items(consumer_id, lifecycle)",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_surface_publications_consumer ON surface_publications(consumer_id, published_at)",
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_reconciliation_jobs (
            idempotency_key TEXT PRIMARY KEY,
            cause_kind TEXT NOT NULL,
            cause_id TEXT NOT NULL,
            consumer_id TEXT NOT NULL,
            target_revision_set TEXT NOT NULL,
            expected_binding_generation INTEGER NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('pending', 'leased', 'succeeded', 'failed')),
            attempt_count INTEGER NOT NULL,
            leased_by TEXT,
            lease_expires_at TEXT,
            next_attempt_at TEXT NOT NULL,
            last_error TEXT,
            success_receipt TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_surface_jobs_lease
        ON surface_reconciliation_jobs(status, next_attempt_at, lease_expires_at)
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS surface_outbox_events (
            event_id TEXT PRIMARY KEY,
            event_kind TEXT NOT NULL,
            aggregate_id TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at TEXT NOT NULL,
            delivered_at TEXT
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_surface_outbox_pending
        ON surface_outbox_events(delivered_at, created_at)
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_change_events (
            event_id TEXT PRIMARY KEY,
            consumer_id TEXT NOT NULL,
            proposal_id TEXT NOT NULL,
            ref_id TEXT NOT NULL,
            before_capability_id TEXT,
            target_capability_id TEXT,
            change_class TEXT NOT NULL,
            policy_action TEXT NOT NULL,
            actor TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            FOREIGN KEY (proposal_id) REFERENCES surface_proposals(proposal_id),
            FOREIGN KEY (ref_id) REFERENCES capability_refs(ref_id),
            FOREIGN KEY (before_capability_id) REFERENCES capability_versions(capability_id),
            FOREIGN KEY (target_capability_id) REFERENCES capability_versions(capability_id)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_capability_change_events_consumer_time
        ON capability_change_events(consumer_id, occurred_at)
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}

async fn ensure_compatible_schema(transaction: &mut sqlx::Transaction<'_, Sqlite>) -> Result<()> {
    let metadata_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'capability_schema_metadata')",
    )
    .fetch_one(&mut **transaction)
    .await?;
    if metadata_exists {
        let epoch: i64 = sqlx::query_scalar("SELECT schema_epoch FROM capability_schema_metadata WHERE singleton = 1")
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or_else(|| crate::CatalogError::IncompatibleSchema {
                details: "capability_schema_metadata is missing its singleton epoch row".to_string(),
            })?;
        if epoch != CAPABILITY_SCHEMA_EPOCH {
            return Err(crate::CatalogError::IncompatibleSchema {
                details: format!(
                    "schema epoch {epoch} is not supported; clean rebuild is required for epoch {CAPABILITY_SCHEMA_EPOCH}"
                ),
            });
        }
        return Ok(());
    }

    let existing_tables: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .fetch_all(&mut **transaction)
            .await?;
    let incompatible_tables = existing_tables
        .iter()
        .filter(|table| {
            LEGACY_CAPABILITY_TABLES.contains(&table.as_str())
                || matches!(
                    table.as_str(),
                    "capability_refs"
                        | "capability_versions"
                        | "capability_ref_current"
                        | "surface_manifests"
                        | "surface_publications"
                        | "consumer_surface_generations"
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    if !incompatible_tables.is_empty() {
        return Err(crate::CatalogError::IncompatibleSchema {
            details: format!(
                "database contains unversioned capability tables [{}]; clean rebuild is required",
                incompatible_tables.join(", ")
            ),
        });
    }

    sqlx::query(
        r#"
        CREATE TABLE capability_schema_metadata (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            schema_epoch INTEGER NOT NULL
        )
        "#,
    )
    .execute(&mut **transaction)
    .await?;
    sqlx::query("INSERT INTO capability_schema_metadata (singleton, schema_epoch) VALUES (1, ?)")
        .bind(CAPABILITY_SCHEMA_EPOCH)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
