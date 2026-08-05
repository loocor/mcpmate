use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};

use super::super::{Migration, MigrationStep};

pub(super) fn migration() -> Migration {
    Migration::rust(
        4,
        "upgrade server configuration columns",
        &[include_str!("v0004_upgrade_server_columns.rs")],
        UpgradeServerColumns,
    )
}

struct UpgradeServerColumns;

#[async_trait]
impl MigrationStep for UpgradeServerColumns {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        ensure_columns(
            transaction,
            "server_config",
            &[
                ("pending_import", "BOOLEAN NOT NULL DEFAULT 0"),
                ("unify_direct_exposure_eligible", "BOOLEAN NOT NULL DEFAULT 0"),
                ("source", "TEXT"),
            ],
        )
        .await?;
        ensure_columns(
            transaction,
            "server_meta",
            &[
                ("extras_json", "TEXT"),
                ("icons_json", "TEXT"),
                ("protocol_version", "TEXT"),
                ("registry_meta_json", "TEXT"),
                ("registry_version", "TEXT"),
                ("upstream_name", "TEXT"),
                ("upstream_title", "TEXT"),
                ("server_version", "TEXT"),
            ],
        )
        .await
    }
}

async fn ensure_columns(
    transaction: &mut Transaction<'_, Sqlite>,
    table: &str,
    columns: &[(&str, &str)],
) -> Result<()> {
    let existing: Vec<String> = sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .fetch_all(&mut **transaction)
        .await
        .with_context(|| format!("inspect {table} columns"))?;
    for (column, definition) in columns {
        if !existing.iter().any(|existing| existing == column) {
            sqlx::query(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"))
                .execute(&mut **transaction)
                .await
                .with_context(|| format!("add {table}.{column}"))?;
        }
    }
    Ok(())
}
