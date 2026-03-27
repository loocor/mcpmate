use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use serde_json::Value;
use sqlx::{FromRow, Pool, QueryBuilder, Row, Sqlite};

use crate::{audit::types::{AuditCursor, AuditCursorScope, AuditEventDto, AuditFilter, AuditListPage, AuditSortCursor}, config::audit_database::AuditDatabase};

use super::policy::{AuditRetentionPolicy, AuditRetentionPolicySetting};

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 200;
const POLICY_ROW_ID: i64 = 1;

#[derive(Debug, Clone)]
pub struct AuditStore {
    pool: Pool<Sqlite>,
}

impl AuditStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub fn from_database(database: &AuditDatabase) -> Self {
        Self::new(database.pool.clone())
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn initialize(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category TEXT NOT NULL,
                action TEXT NOT NULL,
                status TEXT NOT NULL,
                occurred_at_ms INTEGER NOT NULL,
                actor TEXT,
                request_id TEXT,
                client_id TEXT,
                profile_id TEXT,
                server_id TEXT,
                session_id TEXT,
                protocol_version TEXT,
                http_method TEXT,
                route TEXT,
                mcp_method TEXT,
                target TEXT,
                direction TEXT,
                error_code TEXT,
                error_message TEXT,
                detail TEXT,
                duration_ms INTEGER,
                data_json TEXT,
                task_id TEXT,
                related_task_id TEXT,
                progress_token TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create audit_events table")?;

        for statement in [
            "CREATE INDEX IF NOT EXISTS idx_audit_events_occurred_at ON audit_events (occurred_at_ms DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_audit_events_category_action ON audit_events (category, action, occurred_at_ms DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_audit_events_status ON audit_events (status, occurred_at_ms DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_audit_events_server_id ON audit_events (server_id, occurred_at_ms DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_audit_events_profile_id ON audit_events (profile_id, occurred_at_ms DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_audit_events_client_id ON audit_events (client_id, occurred_at_ms DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_audit_events_session_id ON audit_events (session_id, occurred_at_ms DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_audit_events_task_id ON audit_events (task_id, occurred_at_ms DESC, id DESC)",
        ] {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .with_context(|| format!("Failed to execute audit index statement: {statement}"))?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_policy (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                policy TEXT NOT NULL,
                sweep_interval_secs INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create audit_policy table")?;

        Ok(())
    }

    pub async fn insert(
        &self,
        event: &AuditEventDto,
    ) -> Result<AuditEventDto> {
        let data_json = event
            .data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize audit data to JSON")?;

        let result = sqlx::query(
            r#"
            INSERT INTO audit_events (
                category, action, status, occurred_at_ms, actor, request_id, client_id, profile_id, server_id, session_id,
                protocol_version, http_method, route, mcp_method, target, direction, error_code, error_message, detail,
                duration_ms, data_json, task_id, related_task_id, progress_token
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(to_enum_string(event.category)?)
        .bind(to_enum_string(event.action)?)
        .bind(to_enum_string(event.status)?)
        .bind(event.occurred_at_ms)
        .bind(event.actor.as_deref())
        .bind(event.request_id.as_deref())
        .bind(event.client_id.as_deref())
        .bind(event.profile_id.as_deref())
        .bind(event.server_id.as_deref())
        .bind(event.session_id.as_deref())
        .bind(event.protocol_version.as_deref())
        .bind(event.http_method.as_deref())
        .bind(event.route.as_deref())
        .bind(event.mcp_method.as_deref())
        .bind(event.target.as_deref())
        .bind(event.direction.as_deref())
        .bind(event.error_code.as_deref())
        .bind(event.error_message.as_deref())
        .bind(event.detail.as_deref())
        .bind(event.duration_ms.map(|value| value as i64))
        .bind(data_json.as_deref())
        .bind(event.task_id.as_deref())
        .bind(event.related_task_id.as_deref())
        .bind(event.progress_token.as_deref())
        .execute(&self.pool)
        .await
        .context("Failed to insert audit event")?;

        let mut stored = event.clone();
        stored.id = Some(result.last_insert_rowid());
        Ok(stored)
    }

    pub async fn list(
        &self,
        filter: &AuditFilter,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<AuditListPage> {
        let normalized = filter.normalized();
        let decoded_cursor = cursor.map(decode_cursor).transpose()?;
        if let Some(cursor) = decoded_cursor.as_ref() {
            if cursor.scope.filters != normalized.scope_map() {
                return Err(anyhow!("Audit cursor does not match current filter scope"));
            }
        }

        let page_limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as i64;
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT id, category, action, status, occurred_at_ms, actor, request_id, client_id, profile_id, server_id, session_id, protocol_version, http_method, route, mcp_method, target, direction, error_code, error_message, detail, duration_ms, data_json, task_id, related_task_id, progress_token FROM audit_events WHERE 1 = 1",
        );

        apply_filter(&mut query, &normalized);
        if let Some(cursor) = decoded_cursor.as_ref() {
            query.push(" AND (occurred_at_ms < ");
            query.push_bind(cursor.sort.occurred_at_ms);
            query.push(" OR (occurred_at_ms = ");
            query.push_bind(cursor.sort.occurred_at_ms);
            query.push(" AND id < ");
            query.push_bind(cursor.sort.id);
            query.push("))");
        }

        query.push(" ORDER BY occurred_at_ms DESC, id DESC LIMIT ");
        query.push_bind(page_limit + 1);

        let rows = query
            .build_query_as::<AuditEventRow>()
            .fetch_all(&self.pool)
            .await
            .context("Failed to list audit events")?;

        let has_more = rows.len() as i64 > page_limit;
        let mut events: Vec<AuditEventDto> = rows
            .into_iter()
            .take(page_limit as usize)
            .map(AuditEventDto::try_from)
            .collect::<Result<Vec<_>>>()?;

        let next_cursor = if has_more {
            events.last().cloned().map(|event| {
                encode_cursor(&AuditCursor {
                    sort: AuditSortCursor {
                        occurred_at_ms: event.occurred_at_ms,
                        id: event.id.expect("stored audit event must have id"),
                    },
                    scope: AuditCursorScope {
                        filters: normalized.scope_map(),
                    },
                })
            }).transpose()?
        } else {
            None
        };

        Ok(AuditListPage { events: std::mem::take(&mut events), next_cursor })
    }

    pub async fn purge_older_than(
        &self,
        min_occurred_at_ms: i64,
    ) -> Result<u64> {
        let ids = self.select_ids_for_purge(
            "SELECT id FROM audit_events WHERE occurred_at_ms < ? ORDER BY occurred_at_ms ASC, id ASC",
            Some(min_occurred_at_ms),
        )
        .await?;
        self.delete_ids(&ids).await
    }

    pub async fn enforce_capacity(
        &self,
        max_rows: i64,
    ) -> Result<u64> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count audit events")?;

        if total <= max_rows {
            return Ok(0);
        }

        let excess = total - max_rows;
        let ids = self
            .select_ids_for_purge(
                "SELECT id FROM audit_events ORDER BY occurred_at_ms ASC, id ASC LIMIT ?",
                Some(excess),
            )
            .await?;
        self.delete_ids(&ids).await
    }

    async fn select_ids_for_purge(
        &self,
        sql: &str,
        bind: Option<i64>,
    ) -> Result<Vec<i64>> {
        let query = sqlx::query(sql);
        let query = if let Some(bind) = bind { query.bind(bind) } else { query };
        let rows = query.fetch_all(&self.pool).await.context("Failed to select audit ids for purge")?;
        rows.into_iter().map(|row| row.try_get::<i64, _>("id").context("Missing purge id")).collect()
    }

    async fn delete_ids(
        &self,
        ids: &[i64],
    ) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await.context("Failed to open audit purge transaction")?;
        let mut deleted = 0_u64;
        for id in ids {
            deleted += sqlx::query("DELETE FROM audit_events WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .context("Failed to delete audit event")?
                .rows_affected();
        }
        tx.commit().await.context("Failed to commit audit purge transaction")?;
        Ok(deleted)
    }

    pub async fn get_policy(&self) -> Result<AuditRetentionPolicySetting> {
        let row = sqlx::query_as::<_, PolicyRow>(
            "SELECT id, policy, sweep_interval_secs, updated_at_ms FROM audit_policy WHERE id = ?",
        )
        .bind(POLICY_ROW_ID)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query audit policy")?;

        match row {
            Some(r) => {
                let policy = parse_policy(&r.policy)?;
                Ok(AuditRetentionPolicySetting {
                    policy,
                    sweep_interval_secs: r.sweep_interval_secs as u64,
                })
            }
            None => Ok(AuditRetentionPolicySetting::default()),
        }
    }

    pub async fn set_policy(&self, setting: &AuditRetentionPolicySetting) -> Result<()> {
        let policy_str = policy_to_string(&setting.policy);
        let updated_at_ms = chrono::Utc::now().timestamp_millis();

        sqlx::query(
            r#"
            INSERT INTO audit_policy (id, policy, sweep_interval_secs, updated_at_ms)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                policy = excluded.policy,
                sweep_interval_secs = excluded.sweep_interval_secs,
                updated_at_ms = excluded.updated_at_ms
            "#,
        )
        .bind(POLICY_ROW_ID)
        .bind(&policy_str)
        .bind(setting.sweep_interval_secs as i64)
        .bind(updated_at_ms)
        .execute(&self.pool)
        .await
        .context("Failed to upsert audit policy")?;

        Ok(())
    }
}

#[derive(Debug, Clone, FromRow)]
struct AuditEventRow {
    id: i64,
    category: String,
    action: String,
    status: String,
    occurred_at_ms: i64,
    actor: Option<String>,
    request_id: Option<String>,
    client_id: Option<String>,
    profile_id: Option<String>,
    server_id: Option<String>,
    session_id: Option<String>,
    protocol_version: Option<String>,
    http_method: Option<String>,
    route: Option<String>,
    mcp_method: Option<String>,
    target: Option<String>,
    direction: Option<String>,
    error_code: Option<String>,
    error_message: Option<String>,
    detail: Option<String>,
    duration_ms: Option<i64>,
    data_json: Option<String>,
    task_id: Option<String>,
    related_task_id: Option<String>,
    progress_token: Option<String>,
}

impl TryFrom<AuditEventRow> for AuditEventDto {
    type Error = anyhow::Error;

    fn try_from(value: AuditEventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Some(value.id),
            category: from_enum_string(&value.category)?,
            action: from_enum_string(&value.action)?,
            status: from_enum_string(&value.status)?,
            occurred_at_ms: value.occurred_at_ms,
            actor: value.actor,
            request_id: value.request_id,
            client_id: value.client_id,
            profile_id: value.profile_id,
            server_id: value.server_id,
            session_id: value.session_id,
            protocol_version: value.protocol_version,
            http_method: value.http_method,
            route: value.route,
            mcp_method: value.mcp_method,
            target: value.target,
            direction: value.direction,
            error_code: value.error_code,
            error_message: value.error_message,
            detail: value.detail,
            duration_ms: value.duration_ms.map(|value| value as u64),
            data: value.data_json.map(|value| serde_json::from_str(&value)).transpose().context("Failed to decode audit data JSON")?,
            task_id: value.task_id,
            related_task_id: value.related_task_id,
            progress_token: value.progress_token,
        })
    }
}

fn apply_filter(
    query: &mut QueryBuilder<'_, Sqlite>,
    filter: &AuditFilter,
) {
    if let Some(category) = filter.category {
        query.push(" AND category = ").push_bind(to_enum_string(category).expect("serialize category"));
    }
    if let Some(action) = filter.action {
        query.push(" AND action = ").push_bind(to_enum_string(action).expect("serialize action"));
    }
    if let Some(status) = filter.status {
        query.push(" AND status = ").push_bind(to_enum_string(status).expect("serialize status"));
    }
    bind_optional_string(query, "actor", filter.actor.clone());
    bind_optional_string(query, "client_id", filter.client_id.clone());
    bind_optional_string(query, "profile_id", filter.profile_id.clone());
    bind_optional_string(query, "server_id", filter.server_id.clone());
    bind_optional_string(query, "session_id", filter.session_id.clone());
    bind_optional_string(query, "request_id", filter.request_id.clone());
    bind_optional_string(query, "task_id", filter.task_id.clone());
    bind_optional_string(query, "related_task_id", filter.related_task_id.clone());
    bind_optional_string(query, "progress_token", filter.progress_token.clone());
    if let Some(value) = filter.from_occurred_at_ms {
        query.push(" AND occurred_at_ms >= ").push_bind(value);
    }
    if let Some(value) = filter.to_occurred_at_ms {
        query.push(" AND occurred_at_ms <= ").push_bind(value);
    }
}

fn bind_optional_string(
    query: &mut QueryBuilder<'_, Sqlite>,
    column: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        query.push(" AND ").push(column).push(" = ").push_bind(value);
    }
}

fn to_enum_string<T: serde::Serialize>(value: T) -> Result<String> {
    serde_json::to_value(value)
        .context("Failed to serialize audit enum")?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Audit enum serialized to non-string value"))
}

fn from_enum_string<T: for<'de> serde::Deserialize<'de>>(value: &str) -> Result<T> {
    serde_json::from_value(Value::String(value.to_string())).context("Failed to deserialize audit enum")
}

fn encode_cursor(cursor: &AuditCursor) -> Result<String> {
    let payload = serde_json::to_vec(cursor).context("Failed to serialize audit cursor")?;
    Ok(general_purpose::STANDARD.encode(payload))
}

fn decode_cursor(cursor: &str) -> Result<AuditCursor> {
    let payload = general_purpose::STANDARD
        .decode(cursor)
        .context("Failed to decode audit cursor from base64")?;
    serde_json::from_slice(&payload).context("Failed to deserialize audit cursor")
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct PolicyRow {
    id: i64,
    policy: String,
    sweep_interval_secs: i64,
    updated_at_ms: i64,
}

fn policy_to_string(policy: &AuditRetentionPolicy) -> String {
    match policy {
        AuditRetentionPolicy::Off => "off".to_string(),
        AuditRetentionPolicy::KeepDays { days } => format!("keep_days:{}", days),
        AuditRetentionPolicy::KeepCount { count } => format!("keep_count:{}", count),
        AuditRetentionPolicy::Combined { days, count } => format!("combined:{}:{}", days, count),
    }
}

fn parse_policy(s: &str) -> Result<AuditRetentionPolicy> {
    if s == "off" {
        return Ok(AuditRetentionPolicy::Off);
    }
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid policy format: {}", s));
    }
    match parts[0] {
        "keep_days" => {
            let days: u32 = parts[1].parse().context("Invalid days value")?;
            Ok(AuditRetentionPolicy::KeepDays { days })
        }
        "keep_count" => {
            let count: u32 = parts[1].parse().context("Invalid count value")?;
            Ok(AuditRetentionPolicy::KeepCount { count })
        }
        "combined" => {
            let subparts: Vec<&str> = parts[1].splitn(2, ':').collect();
            if subparts.len() != 2 {
                return Err(anyhow!("Invalid combined policy format: {}", s));
            }
            let days: u32 = subparts[0].parse().context("Invalid days value")?;
            let count: u32 = subparts[1].parse().context("Invalid count value")?;
            Ok(AuditRetentionPolicy::Combined { days, count })
        }
        _ => Err(anyhow!("Unknown policy type: {}", parts[0])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    use crate::audit::{AuditAction, AuditCategory, apply_retention_policy};
    use crate::audit::types::{AuditEvent, AuditStatus};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::tempdir;

    async fn setup_store() -> AuditStore {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("audit.db");
        let url = format!("sqlite:{}", path.display());
        let options = SqliteConnectOptions::from_str(&url)
            .expect("options")
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_millis(5_000))
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.expect("connect");
        let store = AuditStore::new(pool);
        store.initialize().await.expect("initialize audit store");
        store
    }

    #[tokio::test]
    async fn inserts_and_filters_records() {
        let store = setup_store().await;
        let first = AuditEvent::new(AuditAction::ToolsCall, AuditStatus::Success)
            .with_client_id("client-a")
            .with_server_id("server-a")
            .with_mcp_method("tools/call")
            .occurred_at_ms(1_000)
            .build();
        let second = AuditEvent::new(AuditAction::ServerEnable, AuditStatus::Success)
            .with_server_id("server-a")
            .occurred_at_ms(2_000)
            .build();
        store.insert(&first).await.expect("insert first");
        store.insert(&second).await.expect("insert second");

        let page = store
            .list(
                &AuditFilter {
                    category: Some(AuditCategory::McpRequest),
                    server_id: Some("server-a".to_string()),
                    ..AuditFilter::default()
                },
                None,
                Some(10),
            )
            .await
            .expect("list events");

        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].action, AuditAction::ToolsCall);
    }

    #[tokio::test]
    async fn paginates_with_filter_bound_cursor() {
        let store = setup_store().await;
        for index in 0..3 {
            let event = AuditEvent::new(AuditAction::ToolsCall, AuditStatus::Success)
                .with_client_id("client-a")
                .occurred_at_ms(1_000 + index)
                .build();
            store.insert(&event).await.expect("insert event");
        }

        let filter = AuditFilter {
            client_id: Some("client-a".to_string()),
            ..AuditFilter::default()
        };
        let first = store.list(&filter, None, Some(2)).await.expect("first page");
        assert_eq!(first.events.len(), 2);
        assert!(first.next_cursor.is_some());

        let second = store
            .list(&filter, first.next_cursor.as_deref(), Some(2))
            .await
            .expect("second page");
        assert_eq!(second.events.len(), 1);

        let mismatch = store
            .list(&AuditFilter::default(), first.next_cursor.as_deref(), Some(2))
            .await;
        assert!(mismatch.is_err());
    }

    #[tokio::test]
    async fn purges_by_age_and_capacity() {
        let store = setup_store().await;
        for index in 0..5 {
            let event = AuditEvent::new(AuditAction::ToolsCall, AuditStatus::Success)
                .occurred_at_ms(1_000 + index)
                .build();
            store.insert(&event).await.expect("insert event");
        }

        let deleted_by_age = store.purge_older_than(1_002).await.expect("purge age");
        assert_eq!(deleted_by_age, 2);
        let deleted_by_capacity = store.enforce_capacity(2).await.expect("purge capacity");
        assert_eq!(deleted_by_capacity, 1);

        let remaining = store.list(&AuditFilter::default(), None, Some(10)).await.expect("remaining rows");
        assert_eq!(remaining.events.len(), 2);
    }

    #[tokio::test]
    async fn policy_defaults_when_not_set() {
        let store = setup_store().await;
        let policy = store.get_policy().await.expect("get default policy");
        assert!(matches!(policy.policy, AuditRetentionPolicy::Combined { .. }));
    }

    #[tokio::test]
    async fn policy_roundtrips_keep_days() {
        let store = setup_store().await;
        let setting = AuditRetentionPolicySetting::new(AuditRetentionPolicy::KeepDays { days: 7 })
            .with_sweep_interval(1800);
        store.set_policy(&setting).await.expect("set policy");

        let loaded = store.get_policy().await.expect("get policy");
        assert_eq!(loaded.policy.max_age_days(), Some(7));
        assert_eq!(loaded.sweep_interval_secs, 1800);
    }

    #[tokio::test]
    async fn policy_roundtrips_combined() {
        let store = setup_store().await;
        let setting = AuditRetentionPolicySetting {
            policy: AuditRetentionPolicy::Combined { days: 30, count: 5000 },
            sweep_interval_secs: 7200,
        };
        store.set_policy(&setting).await.expect("set policy");

        let loaded = store.get_policy().await.expect("get policy");
        assert_eq!(loaded.policy.max_age_days(), Some(30));
        assert_eq!(loaded.policy.max_rows(), Some(5000));
        assert_eq!(loaded.sweep_interval_secs, 7200);
    }

    #[tokio::test]
    async fn apply_retention_policy_off_deletes_nothing() {
        let store = setup_store().await;
        for i in 0..5 {
            let event = AuditEvent::new(AuditAction::ToolsCall, AuditStatus::Success)
                .occurred_at_ms(i)
                .build();
            store.insert(&event).await.expect("insert");
        }

        let deleted = apply_retention_policy(&store, &AuditRetentionPolicy::Off).await.expect("apply");
        assert_eq!(deleted, 0);

        let remaining = store.list(&AuditFilter::default(), None, Some(10)).await.expect("list");
        assert_eq!(remaining.events.len(), 5);
    }
}
