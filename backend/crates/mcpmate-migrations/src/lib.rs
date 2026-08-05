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
}
