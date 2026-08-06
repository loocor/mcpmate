pub(crate) mod audit;
pub(crate) mod config;

use std::borrow::Cow;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, Transaction};

#[async_trait]
pub(crate) trait MigrationStep: Send + Sync {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()>;
}

pub(crate) struct SqlMigration {
    sql: &'static str,
}

impl SqlMigration {
    const fn new(sql: &'static str) -> Self {
        Self { sql }
    }
}

#[async_trait]
impl MigrationStep for SqlMigration {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        for statement in self.sql.split(";\n").filter(|statement| !statement.trim().is_empty()) {
            sqlx::query(statement)
                .execute(&mut **transaction)
                .await
                .context("execute SQL migration statement")?;
        }
        Ok(())
    }
}

pub(crate) struct Migration {
    pub(crate) version: i64,
    pub(crate) name: &'static str,
    checksum_sources: Vec<&'static str>,
    pub(crate) step: Box<dyn MigrationStep>,
}

impl Migration {
    pub(crate) fn sql(
        version: i64,
        name: &'static str,
        sql: &'static str,
    ) -> Self {
        Self {
            version,
            name,
            checksum_sources: vec![sql],
            step: Box::new(SqlMigration::new(sql)),
        }
    }

    pub(crate) fn rust(
        version: i64,
        name: &'static str,
        checksum_sources: &'static [&'static str],
        step: impl MigrationStep + 'static,
    ) -> Self {
        Self {
            version,
            name,
            checksum_sources: checksum_sources.to_vec(),
            step: Box::new(step),
        }
    }

    pub(crate) fn checksum(&self) -> String {
        let mut digest = Sha256::new();
        for source in &self.checksum_sources {
            let source = normalize_line_endings(source);
            digest.update((source.len() as u64).to_be_bytes());
            digest.update(source.as_bytes());
        }
        format!("{:x}", digest.finalize())
    }
}

fn normalize_line_endings(source: &str) -> Cow<'_, str> {
    if source.contains('\r') {
        Cow::Owned(source.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(source)
    }
}

#[cfg(test)]
mod tests {
    use super::Migration;

    #[test]
    fn checksum_is_stable_across_line_endings() {
        let lf = Migration::sql(1, "line endings", "CREATE TABLE example (id TEXT);\n");
        let crlf = Migration::sql(1, "line endings", "CREATE TABLE example (id TEXT);\r\n");

        assert_eq!(lf.checksum(), crlf.checksum());
    }
}
