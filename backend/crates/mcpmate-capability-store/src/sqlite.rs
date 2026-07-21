use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rmcp::model::{Prompt, Resource, ResourceTemplate, Tool};
use sqlx::{FromRow, Pool, Sqlite, Transaction};

use crate::{
    CapabilityFailureObservation, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogCommit,
    CatalogError, CatalogInvalidation, CatalogRecord, CatalogSnapshot, CatalogStats, DeclarationState, InventoryState,
    KindObservation, RECORD_FORMAT_VERSION, Result, SnapshotState, schema,
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
        commit_observation_on_connection(transaction, observation, None).await
    }

    pub async fn commit_observation_after_revision_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        observation: CapabilityObservation,
        previous_revision: i64,
    ) -> Result<CatalogCommit> {
        commit_observation_on_connection(transaction, observation, Some(previous_revision)).await
    }

    pub async fn load_revision_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        server_id: &str,
    ) -> Result<Option<i64>> {
        Ok(
            sqlx::query_scalar("SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?")
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

    pub async fn remove_server_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        server_id: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM capability_server_snapshots WHERE server_id = ?")
            .bind(server_id)
            .execute(&mut **transaction)
            .await?;
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
            "SELECT server_id, server_name, catalog_revision + 1 AS revision FROM capability_server_snapshots ORDER BY server_id",
        )
        .fetch_all(&mut *transaction)
        .await?;
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE capability_server_snapshots SET catalog_revision = catalog_revision + 1, snapshot_state = ?, committed_at = ?, last_error = ?",
        )
        .bind(SnapshotState::Invalidated.as_str())
        .bind(&now)
        .bind(reason)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE capability_kind_states SET catalog_revision = (SELECT catalog_revision FROM capability_server_snapshots WHERE capability_server_snapshots.server_id = capability_kind_states.server_id)",
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE capability_records SET catalog_revision = (SELECT catalog_revision FROM capability_server_snapshots WHERE capability_server_snapshots.server_id = capability_records.server_id)",
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
struct RecordRow {
    stable_id: String,
    kind: String,
    upstream_key: String,
    external_key: String,
    payload_json: String,
    record_format_version: i64,
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
        sqlx::query("DELETE FROM capability_server_snapshots WHERE server_id = ?")
            .bind(server_id)
            .execute(&self.pool)
            .await?;
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
                (SELECT COUNT(*) FROM capability_records) AS records,
                (SELECT COUNT(*) FROM capability_records WHERE kind = 'tools') AS tools,
                (SELECT COUNT(*) FROM capability_records WHERE kind = 'prompts') AS prompts,
                (SELECT COUNT(*) FROM capability_records WHERE kind = 'resources') AS resources,
                (SELECT COUNT(*) FROM capability_records WHERE kind = 'resource_templates') AS resource_templates
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
    let record_rows = sqlx::query_as::<_, RecordRow>(
        "SELECT stable_id, kind, upstream_key, external_key, payload_json, record_format_version FROM capability_records WHERE server_id = ? ORDER BY position",
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
    let current_revision: Option<i64> =
        sqlx::query_scalar("SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?")
            .bind(&observation.server_id)
            .fetch_optional(&mut **transaction)
            .await?;
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

async fn commit_observation_on_connection(
    transaction: &mut Transaction<'_, Sqlite>,
    observation: CapabilityObservation,
    previous_revision: Option<i64>,
) -> Result<CatalogCommit> {
    let committed_at = Utc::now();
    let initialize_payload = serde_json::to_string(&observation.initialize)?;
    let record_payloads = observation
        .records
        .iter()
        .map(|record| encode_payload(&record.payload))
        .collect::<Result<Vec<_>>>()?;
    let current_revision: Option<i64> =
        sqlx::query_scalar("SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?")
            .bind(&observation.server_id)
            .fetch_optional(&mut **transaction)
            .await?;
    let revision = current_revision.unwrap_or(0).max(previous_revision.unwrap_or(0)) + 1;
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
    .bind(initialize_payload)
    .bind(observation.observed_at.to_rfc3339())
    .bind(committed_at.to_rfc3339())
    .bind(&observation.last_error)
    .execute(&mut **transaction)
    .await?;

    sqlx::query("DELETE FROM capability_kind_states WHERE server_id = ?")
        .bind(&observation.server_id)
        .execute(&mut **transaction)
        .await?;
    sqlx::query("DELETE FROM capability_records WHERE server_id = ?")
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
        .bind(observation.observed_at.to_rfc3339())
        .execute(&mut **transaction)
        .await?;
    }

    for (position, (record, payload_json)) in observation.records.iter().zip(record_payloads).enumerate() {
        sqlx::query(
            r#"
            INSERT INTO capability_records (
                stable_id, server_id, position, kind, upstream_key, external_key, payload_json,
                record_format_version, catalog_revision
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.stable_id)
        .bind(&observation.server_id)
        .bind(position as i64)
        .bind(record.kind().as_str())
        .bind(&record.upstream_key)
        .bind(&record.external_key)
        .bind(payload_json)
        .bind(RECORD_FORMAT_VERSION)
        .bind(revision)
        .execute(&mut **transaction)
        .await?;
    }

    Ok(CatalogCommit {
        server_id: observation.server_id,
        revision,
    })
}

async fn update_snapshot_state(
    transaction: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    state: SnapshotState,
    reason: &str,
    failed_kind: Option<CapabilityKind>,
) -> Result<CatalogCommit> {
    let current_revision: Option<i64> =
        sqlx::query_scalar("SELECT catalog_revision FROM capability_server_snapshots WHERE server_id = ?")
            .bind(server_id)
            .fetch_optional(&mut **transaction)
            .await?;
    let revision = current_revision.ok_or_else(|| CatalogError::SnapshotNotFound {
        server_id: server_id.to_owned(),
    })? + 1;
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE capability_server_snapshots SET catalog_revision = ?, snapshot_state = ?, committed_at = ?, last_error = ? WHERE server_id = ?",
    )
    .bind(revision)
    .bind(state.as_str())
    .bind(&now)
    .bind(reason)
    .bind(server_id)
    .execute(&mut **transaction)
    .await?;
    sync_child_revisions(transaction, server_id, revision).await?;
    if let Some(kind) = failed_kind {
        sqlx::query(
            "UPDATE capability_kind_states SET inventory_state = ?, error = ?, observed_at = ? WHERE server_id = ? AND kind = ?",
        )
        .bind(InventoryState::Failed.as_str())
        .bind(reason)
        .bind(&now)
        .bind(server_id)
        .bind(kind.as_str())
        .execute(&mut **transaction)
        .await?;
    }
    Ok(CatalogCommit {
        server_id: server_id.to_owned(),
        revision,
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
    sqlx::query("UPDATE capability_records SET catalog_revision = ? WHERE server_id = ?")
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

fn encode_payload(payload: &CapabilityPayload) -> Result<String> {
    match payload {
        CapabilityPayload::Tool(value) => Ok(serde_json::to_string(value)?),
        CapabilityPayload::Prompt(value) => Ok(serde_json::to_string(value)?),
        CapabilityPayload::Resource(value) => Ok(serde_json::to_string(value)?),
        CapabilityPayload::ResourceTemplate(value) => Ok(serde_json::to_string(value)?),
    }
}

fn decode_payload(
    kind: CapabilityKind,
    payload_json: &str,
) -> Result<CapabilityPayload> {
    match kind {
        CapabilityKind::Tools => Ok(CapabilityPayload::Tool(serde_json::from_str::<Tool>(payload_json)?)),
        CapabilityKind::Prompts => Ok(CapabilityPayload::Prompt(serde_json::from_str::<Prompt>(payload_json)?)),
        CapabilityKind::Resources => Ok(CapabilityPayload::Resource(serde_json::from_str::<Resource>(
            payload_json,
        )?)),
        CapabilityKind::ResourceTemplates => Ok(CapabilityPayload::ResourceTemplate(serde_json::from_str::<
            ResourceTemplate,
        >(payload_json)?)),
    }
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

impl TryFrom<RecordRow> for CatalogRecord {
    type Error = CatalogError;

    fn try_from(row: RecordRow) -> Result<Self> {
        validate_version(row.record_format_version)?;
        let kind = parse_kind(&row.kind)?;
        Ok(Self {
            stable_id: row.stable_id,
            upstream_key: row.upstream_key,
            external_key: row.external_key,
            payload: decode_payload(kind, &row.payload_json)?,
        })
    }
}
