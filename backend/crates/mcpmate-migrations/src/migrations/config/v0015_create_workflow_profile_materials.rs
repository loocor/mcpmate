use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};

use super::super::{Migration, MigrationStep};

const SCHEMA: &str = include_str!("v0015_create_workflow_profile_materials.sql");
const STEP_ID_INSERT_TRIGGER: &str = "CREATE TRIGGER validate_workflow_profile_step_id_insert
BEFORE INSERT ON workflow_profile_steps
WHEN NEW.step_id IS NULL OR NEW.step_id = ''
BEGIN
    SELECT RAISE(ABORT, 'workflow step_id is required');
END";
const STEP_ID_UPDATE_TRIGGER: &str = "CREATE TRIGGER validate_workflow_profile_step_id_update
BEFORE UPDATE OF step_id ON workflow_profile_steps
WHEN NEW.step_id IS NULL OR NEW.step_id = ''
BEGIN
    SELECT RAISE(ABORT, 'workflow step_id is required');
END";

pub(super) fn migration() -> Migration {
    Migration::rust(
        15,
        "create workflow profile materials",
        &[include_str!("v0015_create_workflow_profile_materials.rs"), SCHEMA],
        CreateWorkflowProfileMaterials,
    )
}

struct CreateWorkflowProfileMaterials;

#[async_trait]
impl MigrationStep for CreateWorkflowProfileMaterials {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'workflow_profile_materials')",
        )
        .fetch_one(&mut **transaction)
        .await
        .context("inspect Workflow Profile Materials storage")?;
        if exists {
            return verify_existing_storage(transaction).await;
        }
        for statement in SCHEMA.split(";\n").filter(|statement| !statement.trim().is_empty()) {
            sqlx::query(statement)
                .execute(&mut **transaction)
                .await
                .context("create Workflow Profile Materials storage")?;
        }
        for trigger in [STEP_ID_INSERT_TRIGGER, STEP_ID_UPDATE_TRIGGER] {
            sqlx::query(trigger)
                .execute(&mut **transaction)
                .await
                .context("create Workflow Profile Materials Step ID guard")?;
        }
        verify_existing_storage(transaction).await
    }
}

async fn verify_existing_storage(transaction: &mut Transaction<'_, Sqlite>) -> Result<()> {
    for (table, required_columns) in [
        ("workflow_profile_steps", &["profile_id", "step_index", "step_id"][..]),
        ("workflow_profile_skills", &["profile_id", "skill_name"][..]),
        (
            "workflow_profile_material_libraries",
            &["profile_id", "materials_revision"][..],
        ),
        (
            "workflow_profile_materials",
            &[
                "material_id",
                "profile_id",
                "material_revision",
                "title",
                "kind",
                "relative_path",
            ][..],
        ),
        (
            "workflow_profile_step_materials",
            &["profile_id", "step_id", "material_id", "ordinal"][..],
        ),
    ] {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)")
                .bind(table)
                .fetch_one(&mut **transaction)
                .await?;
        if !exists {
            bail!("Workflow Profile Materials table '{table}' is missing");
        }
        let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info(?)")
            .bind(table)
            .fetch_all(&mut **transaction)
            .await?;
        for column in required_columns {
            if !columns.iter().any(|existing| existing == column) {
                bail!("Workflow Profile Materials table '{table}' is missing required column '{column}'");
            }
        }
    }
    Ok(())
}
