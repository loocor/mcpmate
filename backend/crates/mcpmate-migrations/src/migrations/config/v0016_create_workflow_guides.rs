use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use sqlx::{Connection, Sqlite, SqliteConnection, Transaction};

use super::super::{Migration, MigrationStep};

const SCHEMA: &str = include_str!("v0016_create_workflow_guides.sql");
const OWNED_TABLES: &[&str] = &[
    "workflow_profile_guides",
    "workflow_profile_guide_steps",
    "workflow_profile_capability_aliases",
    "workflow_profile_package_files",
    "workflow_profile_external_guides",
    "workflow_profile_skill_projections",
    "workflow_profile_guide_step_package_files",
];

pub(super) fn migration() -> Migration {
    Migration::rust(
        16,
        "create document-first workflow guides",
        &[include_str!("v0016_create_workflow_guides.rs"), SCHEMA],
        CreateWorkflowGuides,
    )
}

struct CreateWorkflowGuides;

#[async_trait]
impl MigrationStep for CreateWorkflowGuides {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let guides_exist: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'workflow_profile_guides')",
        )
        .fetch_one(&mut **transaction)
        .await
        .context("inspect document-first Workflow Guide storage")?;
        if guides_exist {
            return verify_existing_storage(transaction).await;
        }
        apply_schema(transaction).await?;
        verify_existing_storage(transaction).await
    }
}

async fn verify_existing_storage(transaction: &mut Transaction<'_, Sqlite>) -> Result<()> {
    let actual = schema_contract(transaction).await?;
    let mut reference_connection = SqliteConnection::connect("sqlite::memory:").await?;
    let mut reference_transaction = reference_connection.begin().await?;
    sqlx::query(
        "CREATE TABLE profile (id TEXT PRIMARY KEY);
         CREATE TABLE workflow_profile_steps (
            profile_id TEXT NOT NULL,
            step_id TEXT NOT NULL,
            PRIMARY KEY (profile_id, step_id)
         );",
    )
    .execute(&mut *reference_transaction)
    .await?;
    apply_schema(&mut reference_transaction).await?;
    let expected = schema_contract(&mut reference_transaction).await?;
    if actual != expected {
        bail!("incomplete document-first Workflow Guide schema; current storage does not match the versioned contract");
    }
    Ok(())
}

async fn schema_contract(transaction: &mut Transaction<'_, Sqlite>) -> Result<Vec<(String, String, String, String)>> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT type, name, tbl_name, sql
         FROM sqlite_master
         WHERE sql IS NOT NULL
         ORDER BY type, name",
    )
    .fetch_all(&mut **transaction)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, _, table, _)| OWNED_TABLES.contains(&table.as_str()))
        .map(|(kind, name, table, sql)| (kind, name, table, normalize_schema_sql(&sql)))
        .collect())
}

fn normalize_schema_sql(sql: &str) -> String {
    sql.split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

async fn apply_schema(transaction: &mut Transaction<'_, Sqlite>) -> Result<()> {
    for statement in SCHEMA.split(";\n").filter(|statement| !statement.trim().is_empty()) {
        sqlx::query(statement)
            .execute(&mut **transaction)
            .await
            .context("create document-first Workflow Guide storage")?;
    }
    Ok(())
}
