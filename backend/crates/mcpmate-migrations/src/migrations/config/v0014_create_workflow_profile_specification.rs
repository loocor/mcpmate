use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};

use super::super::{Migration, MigrationStep};

const SCHEMA: &str = include_str!("v0014_create_workflow_profile_specification.sql");

pub(super) fn migration() -> Migration {
    Migration::rust(
        14,
        "create workflow profile specification",
        &[include_str!("v0014_create_workflow_profile_specification.rs"), SCHEMA],
        CreateWorkflowProfileSpecification,
    )
}

struct CreateWorkflowProfileSpecification;

#[async_trait]
impl MigrationStep for CreateWorkflowProfileSpecification {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let profile_columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('profile')")
            .fetch_all(&mut **transaction)
            .await
            .context("inspect Profile schema before Workflow specification migration")?;
        if profile_columns.iter().any(|column| column == "profile_mode") {
            verify_existing_storage(transaction).await?;
            return Ok(());
        }
        for statement in SCHEMA.split(";\n").filter(|statement| !statement.trim().is_empty()) {
            sqlx::query(statement)
                .execute(&mut **transaction)
                .await
                .context("create Workflow Profile specification storage")?;
        }
        Ok(())
    }
}

async fn verify_existing_storage(transaction: &mut Transaction<'_, Sqlite>) -> Result<()> {
    for (table, required_columns) in [
        ("profile", &["profile_mode"][..]),
        (
            "workflow_profile_specifications",
            &["profile_id", "specification_revision", "created_at", "updated_at"],
        ),
        (
            "workflow_profile_steps",
            &["profile_id", "step_index", "title", "description"],
        ),
        (
            "workflow_profile_step_bindings",
            &["profile_id", "step_index", "binding_index", "ref_id", "binding_policy"],
        ),
    ] {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)")
                .bind(table)
                .fetch_one(&mut **transaction)
                .await
                .with_context(|| format!("verify Workflow specification table '{table}'"))?;
        if !exists {
            bail!("Workflow Profile mode exists but Workflow specification table '{table}' is missing");
        }
        let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info(?)")
            .bind(table)
            .fetch_all(&mut **transaction)
            .await
            .with_context(|| format!("inspect Workflow specification table '{table}'"))?;
        for column in required_columns {
            if !columns.iter().any(|existing| existing == column) {
                bail!("Workflow specification table '{table}' is missing required column '{column}'");
            }
        }
    }
    Ok(())
}
