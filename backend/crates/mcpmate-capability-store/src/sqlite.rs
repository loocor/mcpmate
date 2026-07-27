use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Pool, Sqlite, Transaction};

use crate::{
    CapabilityFailureObservation, CapabilityId, CapabilityKind, CapabilityObservation, CapabilityRefId,
    CapabilityRefRecord, CapabilityRefState, CapabilityVersionChange, CapabilityVersionRecord, CatalogCommit,
    CatalogDelta, CatalogError, CatalogInvalidation, CatalogReconciliation, CatalogRecord, CatalogSnapshot,
    CatalogStats, DeclarationState, EFFECTIVE_CAPABILITY_FORMAT_V1, EffectiveCapabilityRecordV1, InventoryState,
    KindCompleteness, KindObservation, RECORD_FORMAT_VERSION, Result, SnapshotState, schema,
};

#[async_trait]
pub trait CapabilityCatalog: Send + Sync {
    async fn load_snapshot(
        &self,
        server_id: &str,
    ) -> Result<Option<CatalogSnapshot>>;
    async fn commit_observation(
        &self,
        observation: CapabilityObservation,
    ) -> Result<CatalogCommit>;
    async fn record_failure(
        &self,
        server_id: &str,
        kind: Option<CapabilityKind>,
        reason: &str,
    ) -> Result<CatalogCommit>;
    async fn invalidate_server(
        &self,
        server_id: &str,
        reason: &str,
    ) -> Result<CatalogCommit>;
    async fn remove_server(
        &self,
        server_id: &str,
    ) -> Result<()>;
    async fn stats(&self) -> Result<CatalogStats>;
}

#[derive(Clone, Debug)]
pub struct SqliteCapabilityCatalog {
    pool: Pool<Sqlite>,
}

impl SqliteCapabilityCatalog {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn ensure_schema(&self) -> Result<()> {
        schema::ensure_schema(&self.pool).await
    }

    pub async fn commit_observation_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        observation: CapabilityObservation,
    ) -> Result<CatalogCommit> {
        Ok(reconcile_observation_on_connection(transaction, observation, None)
            .await?
            .commit)
    }

    pub async fn reconcile_observation_in(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        observation: CapabilityObservation,
    ) -> Result<CatalogReconciliation> {
        reconcile_observation_on_connection(transaction, observation, None).await
    }

    pub async fn reconcile_observation_after_revision_in(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        observation: CapabilityObservation,
        previous_revision: i64,
    ) -> Result<CatalogReconciliation> {
        reconcile_observation_on_connection(transaction, observation, Some(previous_revision)).await
    }

    pub async fn commit_observation_after_revision_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        observation: CapabilityObservation,
        previous_revision: i64,
    ) -> Result<CatalogCommit> {
        Ok(
            reconcile_observation_on_connection(transaction, observation, Some(previous_revision))
                .await?
                .commit,
        )
    }

    pub async fn load_revision_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        server_id: &str,
    ) -> Result<Option<i64>> {
        Ok(
            sqlx::query_scalar::<_, i64>(
                "SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?",
            )
            .bind(server_id)
            .fetch_optional(&mut **transaction)
            .await?,
        )
    }

    pub async fn load_snapshot_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        server_id: &str,
    ) -> Result<Option<CatalogSnapshot>> {
        load_snapshot_on_connection(transaction, server_id).await
    }

    pub async fn load_ref(
        &self,
        ref_id: &CapabilityRefId,
    ) -> Result<Option<CapabilityRefRecord>> {
        load_ref_on_pool(&self.pool, ref_id).await
    }

    pub async fn load_version_history(
        &self,
        ref_id: &CapabilityRefId,
    ) -> Result<Vec<CapabilityVersionRecord>> {
        load_version_history_on_pool(&self.pool, ref_id).await
    }

    pub async fn retire_server_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        server_id: &str,
    ) -> Result<Option<CatalogReconciliation>> {
        let Some(previous_revision) = sqlx::query_scalar::<_, i64>(
            "SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?",
        )
        .bind(server_id)
        .fetch_optional(&mut **transaction)
        .await?
        else {
            return Ok(None);
        };
        let revision = previous_revision
            .checked_add(1)
            .ok_or_else(|| CatalogError::InvalidValue {
                field: "catalog_revision",
                value: "overflow".to_string(),
            })?;
        let retired_refs = sqlx::query_scalar::<_, String>(
            "SELECT ref_id FROM capability_refs WHERE server_id = ? AND state <> 'retired' ORDER BY ref_id",
        )
        .bind(server_id)
        .fetch_all(&mut **transaction)
        .await?
        .into_iter()
        .map(|value| value.parse::<CapabilityRefId>())
        .collect::<Result<Vec<_>>>()?;
        sqlx::query(
            "DELETE FROM capability_ref_current WHERE ref_id IN (SELECT ref_id FROM capability_refs WHERE server_id = ?)",
        )
        .bind(server_id)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE capability_refs
            SET state = 'retired',
                state_generation = CASE WHEN state = 'retired' THEN state_generation ELSE state_generation + 1 END,
                last_observed_revision = ?
            WHERE server_id = ?
            "#,
        )
        .bind(revision)
        .bind(server_id)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE capability_kind_states
            SET inventory_state = 'failed', error = 'source server retired',
                catalog_revision = ?, observed_at = ?
            WHERE server_id = ?
            "#,
        )
        .bind(revision)
        .bind(Utc::now().to_rfc3339())
        .bind(server_id)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE capability_server_snapshots
            SET catalog_revision = ?, snapshot_state = 'unavailable',
                committed_at = ?, last_error = 'source server retired'
            WHERE server_id = ?
            "#,
        )
        .bind(revision)
        .bind(Utc::now().to_rfc3339())
        .bind(server_id)
        .execute(&mut **transaction)
        .await?;
        Ok(Some(CatalogReconciliation {
            commit: CatalogCommit {
                server_id: server_id.to_string(),
                revision,
                changed: true,
            },
            delta: CatalogDelta {
                unresolved_refs: retired_refs,
                ..CatalogDelta::default()
            },
        }))
    }

    pub async fn remove_server_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        server_id: &str,
    ) -> Result<()> {
        let _ = self.retire_server_in_transaction(transaction, server_id).await?;
        Ok(())
    }

    pub async fn record_failure_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        observation: CapabilityFailureObservation,
    ) -> Result<CatalogCommit> {
        record_failure_on_connection(transaction, observation).await
    }

    pub async fn invalidate_all(
        &self,
        reason: &str,
    ) -> Result<Vec<CatalogInvalidation>> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let rows = sqlx::query_as::<_, CatalogInvalidationRow>(
            r#"
            SELECT server_id, server_name, catalog_revision + 1 AS revision
            FROM capability_server_snapshots
            WHERE snapshot_state <> ?
            ORDER BY server_id
            "#,
        )
        .bind(SnapshotState::Invalidated.as_str())
        .fetch_all(&mut *transaction)
        .await?;
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE capability_server_snapshots
            SET catalog_revision = catalog_revision + 1,
                snapshot_state = ?,
                committed_at = ?,
                last_error = ?
            WHERE snapshot_state <> ?
            "#,
        )
        .bind(SnapshotState::Invalidated.as_str())
        .bind(&now)
        .bind(reason)
        .bind(SnapshotState::Invalidated.as_str())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE capability_kind_states
            SET catalog_revision = (
                SELECT catalog_revision
                FROM capability_server_snapshots
                WHERE capability_server_snapshots.server_id = capability_kind_states.server_id
            )
            WHERE catalog_revision <> (
                SELECT catalog_revision
                FROM capability_server_snapshots
                WHERE capability_server_snapshots.server_id = capability_kind_states.server_id
            )
            "#,
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            UPDATE capability_ref_current
            SET catalog_revision = (
                SELECT catalog_revision
                FROM capability_server_snapshots s
                JOIN capability_refs r ON r.server_id = s.server_id
                WHERE r.ref_id = capability_ref_current.ref_id
            )
            WHERE catalog_revision <> (
                SELECT catalog_revision
                FROM capability_server_snapshots s
                JOIN capability_refs r ON r.server_id = s.server_id
                WHERE r.ref_id = capability_ref_current.ref_id
            )
            "#,
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(rows.into_iter().map(CatalogInvalidation::from).collect())
    }
}

#[derive(FromRow)]
struct SnapshotRow {
    server_id: String,
    server_name: String,
    config_fingerprint: String,
    record_format_version: i64,
    catalog_revision: i64,
    snapshot_state: String,
    initialize_payload: String,
    observed_at: String,
    committed_at: String,
    last_error: Option<String>,
}

#[derive(FromRow)]
struct KindStateRow {
    kind: String,
    declaration_state: String,
    inventory_state: String,
    error: Option<String>,
}

#[derive(FromRow)]
struct CurrentRecordRow {
    ref_id: String,
    server_id: String,
    kind: String,
    origin_key: String,
    capability_id: String,
    canonical_record: Vec<u8>,
    record_format: String,
}

#[derive(FromRow)]
struct CapabilityRefRow {
    ref_id: String,
    server_id: String,
    kind: String,
    origin_key: String,
    state: String,
    state_generation: i64,
    first_observed_revision: i64,
    last_observed_revision: i64,
}

#[derive(FromRow)]
struct CapabilityVersionRow {
    capability_id: String,
    ref_id: String,
    canonical_record: Vec<u8>,
    record_format: String,
    first_observed_revision: i64,
}

#[derive(FromRow)]
struct CatalogInvalidationRow {
    server_id: String,
    server_name: String,
    revision: i64,
}

#[async_trait]
impl CapabilityCatalog for SqliteCapabilityCatalog {
    async fn load_snapshot(
        &self,
        server_id: &str,
    ) -> Result<Option<CatalogSnapshot>> {
        let mut transaction = self.pool.begin().await?;
        let snapshot = load_snapshot_on_connection(&mut transaction, server_id).await?;
        transaction.commit().await?;
        Ok(snapshot)
    }

    async fn commit_observation(
        &self,
        observation: CapabilityObservation,
    ) -> Result<CatalogCommit> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let commit = self
            .commit_observation_in_transaction(&mut transaction, observation)
            .await?;
        transaction.commit().await?;
        Ok(commit)
    }

    async fn record_failure(
        &self,
        server_id: &str,
        kind: Option<CapabilityKind>,
        reason: &str,
    ) -> Result<CatalogCommit> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let commit =
            update_snapshot_state(&mut transaction, server_id, SnapshotState::Unavailable, reason, kind).await?;
        transaction.commit().await?;
        Ok(commit)
    }

    async fn invalidate_server(
        &self,
        server_id: &str,
        reason: &str,
    ) -> Result<CatalogCommit> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let commit =
            update_snapshot_state(&mut transaction, server_id, SnapshotState::Invalidated, reason, None).await?;
        transaction.commit().await?;
        Ok(commit)
    }

    async fn remove_server(
        &self,
        server_id: &str,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        self.remove_server_in_transaction(&mut transaction, server_id).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn stats(&self) -> Result<CatalogStats> {
        let row = sqlx::query_as::<_, StatsRow>(
            r#"
            SELECT
                COUNT(*) AS snapshots,
                COALESCE(SUM(snapshot_state = 'ready'), 0) AS ready_snapshots,
                COALESCE(SUM(snapshot_state = 'invalidated'), 0) AS invalidated_snapshots,
                COALESCE(SUM(snapshot_state = 'unavailable'), 0) AS unavailable_snapshots,
                (SELECT COUNT(*) FROM capability_refs WHERE state = 'active') AS records,
                (SELECT COUNT(*) FROM capability_refs WHERE state = 'active' AND kind = 'tools') AS tools,
                (SELECT COUNT(*) FROM capability_refs WHERE state = 'active' AND kind = 'prompts') AS prompts,
                (SELECT COUNT(*) FROM capability_refs WHERE state = 'active' AND kind = 'resources') AS resources,
                (SELECT COUNT(*) FROM capability_refs WHERE state = 'active' AND kind = 'resource_templates') AS resource_templates
            FROM capability_server_snapshots
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(CatalogStats {
            snapshots: row.snapshots,
            ready_snapshots: row.ready_snapshots,
            invalidated_snapshots: row.invalidated_snapshots,
            unavailable_snapshots: row.unavailable_snapshots,
            records: row.records,
            tools: row.tools,
            prompts: row.prompts,
            resources: row.resources,
            resource_templates: row.resource_templates,
        })
    }
}

async fn load_snapshot_on_connection(
    transaction: &mut Transaction<'_, Sqlite>,
    server_id: &str,
) -> Result<Option<CatalogSnapshot>> {
    let Some(row) = sqlx::query_as::<_, SnapshotRow>(
        r#"
        SELECT server_id, server_name, config_fingerprint, record_format_version, catalog_revision,
               snapshot_state, initialize_payload, observed_at, committed_at, last_error
        FROM capability_server_snapshots
        WHERE server_id = ?
        "#,
    )
    .bind(server_id)
    .fetch_optional(&mut **transaction)
    .await?
    else {
        return Ok(None);
    };
    validate_version(row.record_format_version)?;
    let kind_rows = sqlx::query_as::<_, KindStateRow>(
        "SELECT kind, declaration_state, inventory_state, error FROM capability_kind_states WHERE server_id = ? ORDER BY position",
    )
    .bind(server_id)
    .fetch_all(&mut **transaction)
    .await?;
    let record_rows = sqlx::query_as::<_, CurrentRecordRow>(
        r#"
        SELECT r.ref_id, r.server_id, r.kind, r.origin_key,
               c.capability_id, v.canonical_record, v.record_format
        FROM capability_refs r
        JOIN capability_ref_current c ON c.ref_id = r.ref_id
        JOIN capability_versions v ON v.capability_id = c.capability_id
        WHERE r.server_id = ? AND r.state = 'active'
        ORDER BY
            CASE r.kind
                WHEN 'tools' THEN 0
                WHEN 'prompts' THEN 1
                WHEN 'resources' THEN 2
                WHEN 'resource_templates' THEN 3
                ELSE 4
            END,
            r.origin_key,
            r.ref_id
        "#,
    )
    .bind(server_id)
    .fetch_all(&mut **transaction)
    .await?;
    let state = parse_snapshot_state(&row.snapshot_state)?;
    let initialize: Option<rmcp::model::InitializeResult> = serde_json::from_str(&row.initialize_payload)?;
    if state == SnapshotState::Ready && initialize.is_none() {
        return Err(CatalogError::InvalidValue {
            field: "initialize_payload",
            value: "null for ready snapshot".to_string(),
        });
    }
    Ok(Some(CatalogSnapshot {
        server_id: row.server_id,
        server_name: row.server_name,
        config_fingerprint: row.config_fingerprint,
        revision: row.catalog_revision,
        state,
        initialize,
        kind_states: kind_rows
            .into_iter()
            .map(KindObservation::try_from)
            .collect::<Result<Vec<_>>>()?,
        records: record_rows
            .into_iter()
            .map(CatalogRecord::try_from)
            .collect::<Result<Vec<_>>>()?,
        observed_at: parse_timestamp("observed_at", &row.observed_at)?,
        committed_at: parse_timestamp("committed_at", &row.committed_at)?,
        last_error: row.last_error,
    }))
}

async fn record_failure_on_connection(
    transaction: &mut Transaction<'_, Sqlite>,
    observation: CapabilityFailureObservation,
) -> Result<CatalogCommit> {
    let current = sqlx::query_as::<_, (i64, String, String, String)>(
        r#"
        SELECT catalog_revision, server_name, config_fingerprint, snapshot_state
        FROM capability_server_snapshots
        WHERE server_id = ?
        "#,
    )
    .bind(&observation.server_id)
    .fetch_optional(&mut **transaction)
    .await?;
    let current_revision = current.as_ref().map(|(revision, _, _, _)| *revision);
    let current_kind_inventory = sqlx::query_scalar::<_, String>(
        "SELECT inventory_state FROM capability_kind_states WHERE server_id = ? AND kind = ?",
    )
    .bind(&observation.server_id)
    .bind(observation.kind.as_str())
    .fetch_optional(&mut **transaction)
    .await?;
    let changed = match current.as_ref() {
        Some((_, server_name, config_fingerprint, snapshot_state)) => {
            server_name != &observation.server_name
                || config_fingerprint != &observation.config_fingerprint
                || parse_snapshot_state(snapshot_state)? != SnapshotState::Unavailable
                || current_kind_inventory.as_deref().and_then(InventoryState::parse) != Some(InventoryState::Failed)
        }
        None => true,
    };
    if !changed {
        let observed_at = observation.observed_at.to_rfc3339();
        sqlx::query("UPDATE capability_server_snapshots SET observed_at = ?, last_error = ? WHERE server_id = ?")
            .bind(&observed_at)
            .bind(&observation.reason)
            .bind(&observation.server_id)
            .execute(&mut **transaction)
            .await?;
        sqlx::query("UPDATE capability_kind_states SET error = ?, observed_at = ? WHERE server_id = ? AND kind = ?")
            .bind(&observation.reason)
            .bind(&observed_at)
            .bind(&observation.server_id)
            .bind(observation.kind.as_str())
            .execute(&mut **transaction)
            .await?;
        return Ok(CatalogCommit {
            server_id: observation.server_id,
            revision: current_revision.expect("an unchanged failure has a persisted snapshot"),
            changed: false,
        });
    }
    let revision = current_revision.unwrap_or(0) + 1;
    let observed_at = observation.observed_at.to_rfc3339();
    let committed_at = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO capability_server_snapshots (
            server_id, server_name, config_fingerprint, record_format_version, catalog_revision,
            snapshot_state, initialize_payload, observed_at, committed_at, last_error
        ) VALUES (?, ?, ?, ?, ?, ?, 'null', ?, ?, ?)
        ON CONFLICT(server_id) DO UPDATE SET
            server_name = excluded.server_name,
            config_fingerprint = excluded.config_fingerprint,
            record_format_version = excluded.record_format_version,
            catalog_revision = excluded.catalog_revision,
            snapshot_state = excluded.snapshot_state,
            observed_at = excluded.observed_at,
            committed_at = excluded.committed_at,
            last_error = excluded.last_error
        "#,
    )
    .bind(&observation.server_id)
    .bind(&observation.server_name)
    .bind(&observation.config_fingerprint)
    .bind(RECORD_FORMAT_VERSION)
    .bind(revision)
    .bind(SnapshotState::Unavailable.as_str())
    .bind(&observed_at)
    .bind(&committed_at)
    .bind(&observation.reason)
    .execute(&mut **transaction)
    .await?;
    sync_child_revisions(transaction, &observation.server_id, revision).await?;
    let position = CapabilityKind::ALL
        .iter()
        .position(|kind| *kind == observation.kind)
        .unwrap_or_default() as i64;
    sqlx::query(
        r#"
        INSERT INTO capability_kind_states (
            server_id, position, kind, declaration_state, inventory_state, error, catalog_revision, observed_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id, kind) DO UPDATE SET
            inventory_state = excluded.inventory_state,
            error = excluded.error,
            catalog_revision = excluded.catalog_revision,
            observed_at = excluded.observed_at
        "#,
    )
    .bind(&observation.server_id)
    .bind(position)
    .bind(observation.kind.as_str())
    .bind(DeclarationState::Unknown.as_str())
    .bind(InventoryState::Failed.as_str())
    .bind(&observation.reason)
    .bind(revision)
    .bind(&observed_at)
    .execute(&mut **transaction)
    .await?;
    Ok(CatalogCommit {
        server_id: observation.server_id,
        revision,
        changed: true,
    })
}

#[derive(FromRow)]
struct StatsRow {
    snapshots: i64,
    ready_snapshots: i64,
    invalidated_snapshots: i64,
    unavailable_snapshots: i64,
    records: i64,
    tools: i64,
    prompts: i64,
    resources: i64,
    resource_templates: i64,
}

impl From<CatalogInvalidationRow> for CatalogInvalidation {
    fn from(row: CatalogInvalidationRow) -> Self {
        Self {
            server_id: row.server_id,
            server_name: row.server_name,
            revision: row.revision,
        }
    }
}

async fn reconcile_observation_on_connection(
    transaction: &mut Transaction<'_, Sqlite>,
    observation: CapabilityObservation,
    previous_revision: Option<i64>,
) -> Result<CatalogReconciliation> {
    let committed_at = Utc::now();
    let initialize_payload = serde_json::to_string(&observation.initialize)?;
    let current_snapshot_semantics = sqlx::query_as::<_, (String, String)>(
        "SELECT config_fingerprint, snapshot_state FROM capability_server_snapshots WHERE server_id = ?",
    )
    .bind(&observation.server_id)
    .fetch_optional(&mut **transaction)
    .await?;
    let existing_kind_semantics = sqlx::query_as::<_, (String, String, String, String)>(
        r#"
        SELECT kind, declaration_state, inventory_state, observed_at
        FROM capability_kind_states
        WHERE server_id = ?
        ORDER BY kind
        "#,
    )
    .bind(&observation.server_id)
    .fetch_all(&mut **transaction)
    .await?;
    let current_revision: Option<i64> =
        sqlx::query_scalar("SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?")
            .bind(&observation.server_id)
            .fetch_optional(&mut **transaction)
            .await?;
    let revision = current_revision.unwrap_or(0).max(previous_revision.unwrap_or(0)) + 1;
    let existing_refs = sqlx::query_as::<_, CapabilityRefRow>(
        r#"
        SELECT ref_id, server_id, kind, origin_key, state, state_generation,
               first_observed_revision, last_observed_revision
        FROM capability_refs
        WHERE server_id = ?
        "#,
    )
    .bind(&observation.server_id)
    .fetch_all(&mut **transaction)
    .await?
    .into_iter()
    .map(CapabilityRefRecord::try_from)
    .collect::<Result<Vec<_>>>()?;
    let existing_by_ref = existing_refs
        .iter()
        .map(|record| (record.ref_id.clone(), record.clone()))
        .collect::<HashMap<_, _>>();
    let current_by_ref = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT c.ref_id, c.capability_id
        FROM capability_ref_current c
        JOIN capability_refs r ON r.ref_id = c.ref_id
        WHERE r.server_id = ?
        "#,
    )
    .bind(&observation.server_id)
    .fetch_all(&mut **transaction)
    .await?
    .into_iter()
    .map(|(ref_id, capability_id)| {
        Ok((
            CapabilityRefId::from_str(&ref_id)?,
            CapabilityId::from_str(&capability_id)?,
        ))
    })
    .collect::<Result<HashMap<_, _>>>()?;
    let existing_kind_states = existing_kind_semantics
        .iter()
        .map(|(kind, declaration, inventory, _)| {
            Ok((
                parse_kind(kind)?,
                (parse_declaration_state(declaration)?, parse_inventory_state(inventory)?),
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    let observed_kind_states = observation
        .kind_states
        .iter()
        .map(|state| (state.kind, (state.declaration, state.inventory)))
        .collect::<HashMap<_, _>>();
    let snapshot_semantics_changed = match current_snapshot_semantics.as_ref() {
        Some((config_fingerprint, snapshot_state)) => {
            config_fingerprint != &observation.config_fingerprint
                || parse_snapshot_state(snapshot_state)? != observation.state
                || existing_kind_states != observed_kind_states
        }
        None => true,
    };
    let previous_kind_observed_at = existing_kind_semantics
        .into_iter()
        .map(|(kind, _, _, observed_at)| Ok((parse_kind(&kind)?, observed_at)))
        .collect::<Result<HashMap<_, _>>>()?;

    sqlx::query("SAVEPOINT capability_catalog_observation")
        .execute(&mut **transaction)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO capability_server_snapshots (
            server_id, server_name, config_fingerprint, record_format_version, catalog_revision,
            snapshot_state, initialize_payload, observed_at, committed_at, last_error
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id) DO UPDATE SET
            server_name = excluded.server_name,
            config_fingerprint = excluded.config_fingerprint,
            record_format_version = excluded.record_format_version,
            catalog_revision = excluded.catalog_revision,
            snapshot_state = excluded.snapshot_state,
            initialize_payload = excluded.initialize_payload,
            observed_at = excluded.observed_at,
            committed_at = excluded.committed_at,
            last_error = excluded.last_error
        "#,
    )
    .bind(&observation.server_id)
    .bind(&observation.server_name)
    .bind(&observation.config_fingerprint)
    .bind(RECORD_FORMAT_VERSION)
    .bind(revision)
    .bind(observation.state.as_str())
    .bind(&initialize_payload)
    .bind(observation.observed_at.to_rfc3339())
    .bind(committed_at.to_rfc3339())
    .bind(&observation.last_error)
    .execute(&mut **transaction)
    .await?;

    sqlx::query("DELETE FROM capability_kind_states WHERE server_id = ?")
        .bind(&observation.server_id)
        .execute(&mut **transaction)
        .await?;

    for (position, state) in observation.kind_states.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO capability_kind_states (
                server_id, position, kind, declaration_state, inventory_state, error, catalog_revision, observed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&observation.server_id)
        .bind(position as i64)
        .bind(state.kind.as_str())
        .bind(state.declaration.as_str())
        .bind(state.inventory.as_str())
        .bind(&state.error)
        .bind(revision)
        .bind(if observation.observed_kinds.contains(&state.kind) {
            observation.observed_at.to_rfc3339()
        } else {
            previous_kind_observed_at
                .get(&state.kind)
                .cloned()
                .unwrap_or_else(|| observation.observed_at.to_rfc3339())
        })
        .execute(&mut **transaction)
        .await?;
    }

    let complete_kinds = observation
        .kind_states
        .iter()
        .filter(|state| state.inventory == InventoryState::Complete)
        .map(|state| state.kind)
        .collect::<HashSet<_>>();
    let mut observed_refs = HashSet::new();
    let mut delta = CatalogDelta {
        kind_completeness: observation
            .kind_states
            .iter()
            .map(|state| KindCompleteness {
                kind: state.kind,
                inventory: state.inventory,
            })
            .collect(),
        ..CatalogDelta::default()
    };

    for record in observation
        .records
        .iter()
        .filter(|record| complete_kinds.contains(&record.kind()))
    {
        let effective_record: EffectiveCapabilityRecordV1 = serde_json::from_slice(&record.canonical_record)?;
        effective_record.validate()?;
        if effective_record.source.server_id != observation.server_id
            || effective_record.source.kind != record.kind()
            || effective_record.source.origin_key != record.upstream_key
            || effective_record.ref_id != record.ref_id
        {
            return Err(CatalogError::IntegrityMismatch {
                identity: record.capability_id.to_string(),
            });
        }
        record
            .capability_id
            .verify_canonical_content(&record.canonical_record, &record.canonical_record)?;
        if !observed_refs.insert(record.ref_id.clone()) {
            return Err(CatalogError::DuplicateOrigin {
                server_id: observation.server_id.clone(),
                kind: record.kind(),
                origin_key: record.upstream_key.clone(),
            });
        }

        let previous = existing_by_ref.get(&record.ref_id);
        let state_generation = match previous.map(|value| value.state) {
            Some(CapabilityRefState::Unresolved) => previous
                .expect("previous ref exists")
                .state_generation
                .checked_add(1)
                .ok_or_else(|| CatalogError::InvalidValue {
                    field: "state_generation",
                    value: "overflow".to_string(),
                })?,
            Some(_) => previous.expect("previous ref exists").state_generation,
            None => 0,
        };
        let next_state = if previous.is_some_and(|value| value.state == CapabilityRefState::Retired) {
            CapabilityRefState::Retired
        } else {
            CapabilityRefState::Active
        };

        sqlx::query(
            r#"
            INSERT INTO capability_refs (
                ref_id, server_id, kind, origin_key, state, state_generation,
                first_observed_revision, last_observed_revision
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(ref_id) DO UPDATE SET
                state = excluded.state,
                state_generation = excluded.state_generation,
                last_observed_revision = excluded.last_observed_revision
            "#,
        )
        .bind(record.ref_id.as_str())
        .bind(&observation.server_id)
        .bind(record.kind().as_str())
        .bind(&record.upstream_key)
        .bind(next_state.as_str())
        .bind(state_generation)
        .bind(previous.map_or(revision, |value| value.first_observed_revision))
        .bind(revision)
        .execute(&mut **transaction)
        .await?;

        let inserted = sqlx::query(
            r#"
            INSERT OR IGNORE INTO capability_versions (
                capability_id, ref_id, canonical_record, record_format, first_observed_revision
            ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.capability_id.as_str())
        .bind(record.ref_id.as_str())
        .bind(&record.canonical_record)
        .bind(EFFECTIVE_CAPABILITY_FORMAT_V1)
        .bind(revision)
        .execute(&mut **transaction)
        .await?;
        if inserted.rows_affected() == 0 {
            let saved: Vec<u8> =
                sqlx::query_scalar("SELECT canonical_record FROM capability_versions WHERE capability_id = ?")
                    .bind(record.capability_id.as_str())
                    .fetch_one(&mut **transaction)
                    .await?;
            record
                .capability_id
                .verify_canonical_content(&saved, &record.canonical_record)?;
        }

        sqlx::query(
            r#"
            INSERT INTO capability_ref_current (ref_id, capability_id, catalog_revision)
            VALUES (?, ?, ?)
            ON CONFLICT(ref_id) DO UPDATE SET
                capability_id = excluded.capability_id,
                catalog_revision = excluded.catalog_revision
            "#,
        )
        .bind(record.ref_id.as_str())
        .bind(record.capability_id.as_str())
        .bind(revision)
        .execute(&mut **transaction)
        .await?;

        match (previous, current_by_ref.get(&record.ref_id)) {
            (None, _) => delta.added_refs.push(record.ref_id.clone()),
            (Some(previous_ref), Some(previous_capability_id))
                if previous_ref.state == CapabilityRefState::Unresolved =>
            {
                delta.reappeared_refs.push(record.ref_id.clone());
                if previous_capability_id != &record.capability_id {
                    delta.changed_versions.push(CapabilityVersionChange {
                        ref_id: record.ref_id.clone(),
                        before_capability_id: previous_capability_id.clone(),
                        target_capability_id: record.capability_id.clone(),
                    });
                }
            }
            (Some(_), Some(previous_capability_id)) if previous_capability_id != &record.capability_id => {
                delta.changed_versions.push(CapabilityVersionChange {
                    ref_id: record.ref_id.clone(),
                    before_capability_id: previous_capability_id.clone(),
                    target_capability_id: record.capability_id.clone(),
                });
            }
            (Some(_), _) => delta.unchanged_refs.push(record.ref_id.clone()),
        }
    }

    for existing in &existing_refs {
        if existing.state == CapabilityRefState::Active
            && complete_kinds.contains(&existing.kind)
            && !observed_refs.contains(&existing.ref_id)
        {
            let next_generation =
                existing
                    .state_generation
                    .checked_add(1)
                    .ok_or_else(|| CatalogError::InvalidValue {
                        field: "state_generation",
                        value: "overflow".to_string(),
                    })?;
            sqlx::query(
                r#"
                UPDATE capability_refs
                SET state = ?, state_generation = ?, last_observed_revision = ?
                WHERE ref_id = ?
                "#,
            )
            .bind(CapabilityRefState::Unresolved.as_str())
            .bind(next_generation)
            .bind(revision)
            .bind(existing.ref_id.as_str())
            .execute(&mut **transaction)
            .await?;
            delta.unresolved_refs.push(existing.ref_id.clone());
        }
    }

    sort_delta(&mut delta);
    let current_ref_semantics = existing_refs
        .iter()
        .map(|record| {
            (
                record.ref_id.clone(),
                (record.kind, record.state, current_by_ref.get(&record.ref_id).cloned()),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut projected_ref_semantics = current_ref_semantics.clone();
    for record in observation
        .records
        .iter()
        .filter(|record| complete_kinds.contains(&record.kind()))
    {
        let existing = existing_by_ref.get(&record.ref_id);
        if existing.is_some_and(|value| value.state == CapabilityRefState::Retired) {
            continue;
        }
        projected_ref_semantics.insert(
            record.ref_id.clone(),
            (
                record.kind(),
                CapabilityRefState::Active,
                Some(record.capability_id.clone()),
            ),
        );
    }
    for existing in &existing_refs {
        if existing.state == CapabilityRefState::Active
            && complete_kinds.contains(&existing.kind)
            && !observed_refs.contains(&existing.ref_id)
        {
            projected_ref_semantics.insert(
                existing.ref_id.clone(),
                (
                    existing.kind,
                    CapabilityRefState::Unresolved,
                    current_by_ref.get(&existing.ref_id).cloned(),
                ),
            );
        }
    }
    let changed =
        previous_revision.is_some() || snapshot_semantics_changed || projected_ref_semantics != current_ref_semantics;
    if !changed {
        sqlx::query("ROLLBACK TO SAVEPOINT capability_catalog_observation")
            .execute(&mut **transaction)
            .await?;
        sqlx::query("RELEASE SAVEPOINT capability_catalog_observation")
            .execute(&mut **transaction)
            .await?;
        sqlx::query(
            r#"
            UPDATE capability_server_snapshots
            SET observed_at = ?, last_error = ?
            WHERE server_id = ?
            "#,
        )
        .bind(observation.observed_at.to_rfc3339())
        .bind(&observation.last_error)
        .bind(&observation.server_id)
        .execute(&mut **transaction)
        .await?;
        for state in observation
            .kind_states
            .iter()
            .filter(|state| observation.observed_kinds.contains(&state.kind))
        {
            sqlx::query(
                r#"
                UPDATE capability_kind_states
                SET error = ?, observed_at = ?
                WHERE server_id = ? AND kind = ?
                "#,
            )
            .bind(&state.error)
            .bind(observation.observed_at.to_rfc3339())
            .bind(&observation.server_id)
            .bind(state.kind.as_str())
            .execute(&mut **transaction)
            .await?;
        }
        return Ok(CatalogReconciliation {
            commit: CatalogCommit {
                server_id: observation.server_id,
                revision: current_revision.expect("an unchanged observation has a persisted snapshot"),
                changed: false,
            },
            delta,
        });
    }
    sqlx::query("RELEASE SAVEPOINT capability_catalog_observation")
        .execute(&mut **transaction)
        .await?;
    Ok(CatalogReconciliation {
        commit: CatalogCommit {
            server_id: observation.server_id,
            revision,
            changed: true,
        },
        delta,
    })
}

async fn update_snapshot_state(
    transaction: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    state: SnapshotState,
    reason: &str,
    failed_kind: Option<CapabilityKind>,
) -> Result<CatalogCommit> {
    let current = sqlx::query_as::<_, (i64, String)>(
        "SELECT catalog_revision, snapshot_state FROM capability_server_snapshots WHERE server_id = ?",
    )
    .bind(server_id)
    .fetch_optional(&mut **transaction)
    .await?;
    let (current_revision, current_state) = current.ok_or_else(|| CatalogError::SnapshotNotFound {
        server_id: server_id.to_owned(),
    })?;
    let now = Utc::now().to_rfc3339();
    let failed_kind_already_recorded = match failed_kind {
        Some(kind) => {
            sqlx::query_scalar::<_, String>(
                "SELECT inventory_state FROM capability_kind_states WHERE server_id = ? AND kind = ?",
            )
            .bind(server_id)
            .bind(kind.as_str())
            .fetch_optional(&mut **transaction)
            .await?
            .as_deref()
            .and_then(InventoryState::parse)
                == Some(InventoryState::Failed)
        }
        None => true,
    };
    if parse_snapshot_state(&current_state)? == state && failed_kind_already_recorded {
        sqlx::query("UPDATE capability_server_snapshots SET observed_at = ?, last_error = ? WHERE server_id = ?")
            .bind(&now)
            .bind(reason)
            .bind(server_id)
            .execute(&mut **transaction)
            .await?;
        if let Some(kind) = failed_kind {
            sqlx::query(
                "UPDATE capability_kind_states SET error = ?, observed_at = ? WHERE server_id = ? AND kind = ?",
            )
            .bind(reason)
            .bind(&now)
            .bind(server_id)
            .bind(kind.as_str())
            .execute(&mut **transaction)
            .await?;
        }
        return Ok(CatalogCommit {
            server_id: server_id.to_owned(),
            revision: current_revision,
            changed: false,
        });
    }
    let revision = current_revision + 1;
    sqlx::query(
        "UPDATE capability_server_snapshots SET catalog_revision = ?, snapshot_state = ?, observed_at = ?, committed_at = ?, last_error = ? WHERE server_id = ?",
    )
    .bind(revision)
    .bind(state.as_str())
    .bind(&now)
    .bind(&now)
    .bind(reason)
    .bind(server_id)
    .execute(&mut **transaction)
    .await?;
    sync_child_revisions(transaction, server_id, revision).await?;
    if let Some(kind) = failed_kind {
        // A plain `UPDATE ... WHERE server_id = ? AND kind = ?` silently affects zero rows if
        // this server never had a `capability_kind_states` row for `kind` (e.g. a kind that
        // was never part of the committed `kind_states` list). That would let a real usage
        // failure disappear without marking anything as failed. Upsert instead so the failure
        // is always recorded, defaulting a freshly-created row's declaration to `Unknown`
        // since we don't know whether the kind was ever declared/discovered (Codex review
        // follow-up, PR #160).
        let position = CapabilityKind::ALL
            .iter()
            .position(|candidate| *candidate == kind)
            .unwrap_or(0) as i64;
        sqlx::query(
            r#"
            INSERT INTO capability_kind_states
                (server_id, position, kind, declaration_state, inventory_state, error, catalog_revision, observed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(server_id, kind) DO UPDATE SET
                inventory_state = excluded.inventory_state,
                error = excluded.error,
                observed_at = excluded.observed_at,
                catalog_revision = excluded.catalog_revision
            "#,
        )
        .bind(server_id)
        .bind(position)
        .bind(kind.as_str())
        .bind(DeclarationState::Unknown.as_str())
        .bind(InventoryState::Failed.as_str())
        .bind(reason)
        .bind(revision)
        .bind(&now)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(CatalogCommit {
        server_id: server_id.to_owned(),
        revision,
        changed: true,
    })
}

fn validate_version(actual: i64) -> Result<()> {
    if actual == RECORD_FORMAT_VERSION {
        Ok(())
    } else {
        Err(CatalogError::UnsupportedRecordVersion {
            actual,
            expected: RECORD_FORMAT_VERSION,
        })
    }
}

fn parse_labeled<T>(
    field: &'static str,
    value: &str,
    parse: fn(&str) -> Option<T>,
) -> Result<T> {
    parse(value).ok_or_else(|| CatalogError::InvalidValue {
        field,
        value: value.to_owned(),
    })
}

fn parse_kind(value: &str) -> Result<CapabilityKind> {
    parse_labeled("kind", value, CapabilityKind::parse)
}

fn parse_snapshot_state(value: &str) -> Result<SnapshotState> {
    parse_labeled("snapshot_state", value, SnapshotState::parse)
}

fn parse_declaration_state(value: &str) -> Result<DeclarationState> {
    parse_labeled("declaration_state", value, DeclarationState::parse)
}

fn parse_inventory_state(value: &str) -> Result<InventoryState> {
    parse_labeled("inventory_state", value, InventoryState::parse)
}

fn parse_ref_state(value: &str) -> Result<CapabilityRefState> {
    parse_labeled("ref_state", value, CapabilityRefState::parse)
}

async fn load_ref_on_pool(
    pool: &Pool<Sqlite>,
    ref_id: &CapabilityRefId,
) -> Result<Option<CapabilityRefRecord>> {
    sqlx::query_as::<_, CapabilityRefRow>(
        r#"
        SELECT ref_id, server_id, kind, origin_key, state, state_generation,
               first_observed_revision, last_observed_revision
        FROM capability_refs
        WHERE ref_id = ?
        "#,
    )
    .bind(ref_id.as_str())
    .fetch_optional(pool)
    .await?
    .map(CapabilityRefRecord::try_from)
    .transpose()
}

async fn load_version_history_on_pool(
    pool: &Pool<Sqlite>,
    ref_id: &CapabilityRefId,
) -> Result<Vec<CapabilityVersionRecord>> {
    sqlx::query_as::<_, CapabilityVersionRow>(
        r#"
        SELECT capability_id, ref_id, canonical_record, record_format, first_observed_revision
        FROM capability_versions
        WHERE ref_id = ?
        ORDER BY first_observed_revision, capability_id
        "#,
    )
    .bind(ref_id.as_str())
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(CapabilityVersionRecord::try_from)
    .collect()
}

async fn sync_child_revisions(
    transaction: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    revision: i64,
) -> Result<()> {
    sqlx::query("UPDATE capability_kind_states SET catalog_revision = ? WHERE server_id = ?")
        .bind(revision)
        .bind(server_id)
        .execute(&mut **transaction)
        .await?;
    sqlx::query(
        "UPDATE capability_ref_current SET catalog_revision = ? WHERE ref_id IN (SELECT ref_id FROM capability_refs WHERE server_id = ?)",
    )
        .bind(revision)
        .bind(server_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

fn parse_timestamp(
    field: &'static str,
    value: &str,
) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|_| CatalogError::InvalidTimestamp {
            field,
            value: value.to_owned(),
        })
}

fn sort_delta(delta: &mut CatalogDelta) {
    delta.added_refs.sort();
    delta
        .changed_versions
        .sort_by(|left, right| left.ref_id.cmp(&right.ref_id));
    delta.unresolved_refs.sort();
    delta.reappeared_refs.sort();
    delta.unchanged_refs.sort();
}

impl TryFrom<KindStateRow> for KindObservation {
    type Error = CatalogError;

    fn try_from(row: KindStateRow) -> Result<Self> {
        Ok(Self {
            kind: parse_kind(&row.kind)?,
            declaration: parse_declaration_state(&row.declaration_state)?,
            inventory: parse_inventory_state(&row.inventory_state)?,
            error: row.error,
        })
    }
}

impl TryFrom<CurrentRecordRow> for CatalogRecord {
    type Error = CatalogError;

    fn try_from(row: CurrentRecordRow) -> Result<Self> {
        validate_effective_format(&row.record_format)?;
        let ref_id = CapabilityRefId::from_str(&row.ref_id)?;
        let capability_id = CapabilityId::from_str(&row.capability_id)?;
        let effective_record: EffectiveCapabilityRecordV1 = serde_json::from_slice(&row.canonical_record)?;
        if effective_record.ref_id != ref_id
            || effective_record.source.server_id != row.server_id
            || effective_record.source.kind != parse_kind(&row.kind)?
            || effective_record.source.origin_key != row.origin_key
        {
            return Err(CatalogError::IntegrityMismatch {
                identity: capability_id.to_string(),
            });
        }
        CatalogRecord::from_effective_record(capability_id, row.canonical_record, effective_record)
    }
}

impl TryFrom<CapabilityRefRow> for CapabilityRefRecord {
    type Error = CatalogError;

    fn try_from(row: CapabilityRefRow) -> Result<Self> {
        let ref_id = CapabilityRefId::from_str(&row.ref_id)?;
        let kind = parse_kind(&row.kind)?;
        ref_id.verify_source(&crate::CapabilitySourceIdentity::new(
            &row.server_id,
            kind,
            &row.origin_key,
        ))?;
        Ok(Self {
            ref_id,
            server_id: row.server_id,
            kind,
            origin_key: row.origin_key,
            state: parse_ref_state(&row.state)?,
            state_generation: row.state_generation,
            first_observed_revision: row.first_observed_revision,
            last_observed_revision: row.last_observed_revision,
        })
    }
}

impl TryFrom<CapabilityVersionRow> for CapabilityVersionRecord {
    type Error = CatalogError;

    fn try_from(row: CapabilityVersionRow) -> Result<Self> {
        validate_effective_format(&row.record_format)?;
        let capability_id = CapabilityId::from_str(&row.capability_id)?;
        let ref_id = CapabilityRefId::from_str(&row.ref_id)?;
        let effective_record: EffectiveCapabilityRecordV1 = serde_json::from_slice(&row.canonical_record)?;
        effective_record.validate()?;
        if effective_record.ref_id != ref_id {
            return Err(CatalogError::IntegrityMismatch {
                identity: capability_id.to_string(),
            });
        }
        capability_id.verify_canonical_content(&row.canonical_record, &row.canonical_record)?;
        Ok(Self {
            capability_id,
            ref_id,
            canonical_record: row.canonical_record,
            record_format: row.record_format,
            first_observed_revision: row.first_observed_revision,
        })
    }
}

fn validate_effective_format(actual: &str) -> Result<()> {
    if actual == EFFECTIVE_CAPABILITY_FORMAT_V1 {
        Ok(())
    } else {
        Err(CatalogError::UnsupportedEffectiveCapabilityFormat {
            actual: actual.to_string(),
            expected: EFFECTIVE_CAPABILITY_FORMAT_V1,
        })
    }
}
