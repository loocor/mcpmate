use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};

use super::super::{Migration, MigrationStep};

const SCHEMA: &str = include_str!("v0016_add_workflow_specification_guidance.sql");

pub(super) fn migration() -> Migration {
    Migration::rust(
        16,
        "add workflow specification guidance",
        &[include_str!("v0016_add_workflow_specification_guidance.rs"), SCHEMA],
        AddWorkflowSpecificationGuidance,
    )
}

struct AddWorkflowSpecificationGuidance;

#[async_trait]
impl MigrationStep for AddWorkflowSpecificationGuidance {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let existing_columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('workflow_profile_specifications')")
                .fetch_all(&mut **transaction)
                .await
                .context("inspect Workflow specification guidance columns")?;
        for (column, statement) in [
            (
                "validation_notes",
                "ALTER TABLE workflow_profile_specifications ADD COLUMN validation_notes TEXT",
            ),
            (
                "avoid_rules",
                "ALTER TABLE workflow_profile_specifications ADD COLUMN avoid_rules TEXT",
            ),
        ] {
            if existing_columns.iter().any(|existing| existing == column) {
                continue;
            }
            sqlx::query(statement)
                .execute(&mut **transaction)
                .await
                .with_context(|| format!("add Workflow specification guidance column '{column}'"))?;
        }
        Ok(())
    }
}
