//! The sole owner of durable SQLite schema evolution in MCPMate.

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite, Transaction};

const LEDGER_TABLE: &str = "mcpmate_schema_migrations";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseTarget {
    Config,
    Audit,
}

impl DatabaseTarget {
    const fn name(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Audit => "audit",
        }
    }
}

#[async_trait]
pub trait MigrationStep: Send + Sync {
    async fn apply(&self, transaction: &mut Transaction<'_, Sqlite>) -> Result<()>;
}

pub struct SqlMigration {
    sql: &'static str,
}

impl SqlMigration {
    pub const fn new(sql: &'static str) -> Self {
        Self { sql }
    }
}

#[async_trait]
impl MigrationStep for SqlMigration {
    async fn apply(&self, transaction: &mut Transaction<'_, Sqlite>) -> Result<()> {
        for statement in self.sql.split(";\n").filter(|statement| !statement.trim().is_empty()) {
            sqlx::query(statement)
                .execute(&mut **transaction)
                .await
                .context("execute SQL migration statement")?;
        }
        Ok(())
    }
}

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub checksum_source: &'static str,
    pub step: Box<dyn MigrationStep>,
}

impl Migration {
    pub fn checksum(&self) -> String {
        format!("{:x}", Sha256::digest(self.checksum_source.as_bytes()))
    }
}

pub async fn run(pool: &Pool<Sqlite>, target: DatabaseTarget, migrations: Vec<Migration>) -> Result<()> {
    let mut transaction = pool.begin().await.context("begin migration transaction")?;
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {LEDGER_TABLE} (target TEXT NOT NULL, version INTEGER NOT NULL, name TEXT NOT NULL, checksum TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY (target, version))"
    ))
    .execute(&mut *transaction)
    .await
    .context("create migration ledger")?;

    let mut previous = 0;
    for migration in migrations {
        if migration.version <= previous {
            bail!("migration versions for {} are not strictly increasing", target.name());
        }
        previous = migration.version;
        let checksum = migration.checksum();
        let applied: Option<(String, String)> = sqlx::query_as(
            &format!("SELECT name, checksum FROM {LEDGER_TABLE} WHERE target = ? AND version = ?"),
        )
        .bind(target.name())
        .bind(migration.version)
        .fetch_optional(&mut *transaction)
        .await
        .context("read migration ledger")?;
        if let Some((name, existing_checksum)) = applied {
            if name != migration.name || existing_checksum != checksum {
                bail!("migration {} for {} was modified after being applied", migration.version, target.name());
            }
            continue;
        }
        migration.step.apply(&mut transaction).await.with_context(|| {
            format!("apply migration {} ({}) for {}", migration.version, migration.name, target.name())
        })?;
        sqlx::query(&format!(
            "INSERT INTO {LEDGER_TABLE} (target, version, name, checksum) VALUES (?, ?, ?, ?)"
        ))
        .bind(target.name())
        .bind(migration.version)
        .bind(migration.name)
        .bind(checksum)
        .execute(&mut *transaction)
        .await
        .context("record applied migration")?;
    }
    transaction.commit().await.context("commit migrations")
}

pub async fn migrate_audit(pool: &Pool<Sqlite>) -> Result<()> {
    run(
        pool,
        DatabaseTarget::Audit,
        vec![Migration {
            version: 1,
            name: "create audit storage",
            checksum_source: AUDIT_INITIAL_SCHEMA,
            step: Box::new(SqlMigration::new(AUDIT_INITIAL_SCHEMA)),
        }],
    )
    .await
}

const AUDIT_INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL, action TEXT NOT NULL, status TEXT NOT NULL,
    occurred_at_ms INTEGER NOT NULL, actor TEXT, request_id TEXT, client_id TEXT,
    profile_id TEXT, server_id TEXT, session_id TEXT, protocol_version TEXT,
    http_method TEXT, route TEXT, mcp_method TEXT, target TEXT, direction TEXT,
    error_code TEXT, error_message TEXT, detail TEXT, duration_ms INTEGER,
    data_json TEXT, task_id TEXT, related_task_id TEXT, progress_token TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_events_occurred_at ON audit_events (occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_category_action ON audit_events (category, action, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_status ON audit_events (status, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_server_id ON audit_events (server_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_profile_id ON audit_events (profile_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_client_id ON audit_events (client_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_session_id ON audit_events (session_id, occurred_at_ms DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_task_id ON audit_events (task_id, occurred_at_ms DESC, id DESC);
CREATE TABLE IF NOT EXISTS audit_policy (
    id INTEGER PRIMARY KEY CHECK (id = 1), policy TEXT NOT NULL,
    sweep_interval_secs INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL
);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn applies_once_and_detects_mutated_history() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        run(
            &pool,
            DatabaseTarget::Config,
            vec![Migration {
                version: 1,
                name: "create example",
                checksum_source: "create example v1",
                step: Box::new(SqlMigration::new("CREATE TABLE example (id INTEGER PRIMARY KEY);\n")),
            }],
        )
        .await
        .unwrap();
        run(
            &pool,
            DatabaseTarget::Config,
            vec![Migration {
                version: 1,
                name: "create example",
                checksum_source: "create example v1",
                step: Box::new(SqlMigration::new("CREATE TABLE example (id INTEGER PRIMARY KEY);\n")),
            }],
        )
        .await
        .unwrap();
        let error = run(
            &pool,
            DatabaseTarget::Config,
            vec![Migration {
                version: 1,
                name: "create example",
                checksum_source: "changed",
                step: Box::new(SqlMigration::new("SELECT 1;\n")),
            }],
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("modified"));
    }

    #[tokio::test]
    async fn creates_audit_schema_through_the_ledger() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        migrate_audit(&pool).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mcpmate_schema_migrations WHERE target = 'audit'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
        sqlx::query("INSERT INTO audit_policy (id, policy, sweep_interval_secs, updated_at_ms) VALUES (1, 'keep', 1, 1)")
            .execute(&pool)
            .await
            .unwrap();
    }
}
