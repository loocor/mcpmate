use anyhow::Result;
use sqlx::{Pool, Sqlite};

use crate::common::constants::database::tables;

/// Verify Profile authoring storage after the config migration stream has run.
pub async fn initialize_profile_tables(pool: &Pool<Sqlite>) -> Result<()> {
    mcpmate_migrations::verify_config_database(pool).await?;
    verify_profile_tables(pool).await
}

async fn verify_profile_tables(pool: &Pool<Sqlite>) -> Result<()> {
    for table in [
        tables::PROFILE,
        "profile_server_relationships",
        tables::SERVER_TOOLS,
        tables::SERVER_PROMPTS,
        tables::SERVER_RESOURCES,
        tables::SERVER_RESOURCE_TEMPLATES,
        "server_issued_resources",
        "profile_capability_refs",
        "direct_exposure_refs",
        "direct_exposure_servers",
        "workflow_profile_specifications",
        "workflow_profile_steps",
        "workflow_profile_step_bindings",
    ] {
        sqlx::query(&format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"
        ))
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("{table} table not found after migration"))?;
    }
    Ok(())
}
