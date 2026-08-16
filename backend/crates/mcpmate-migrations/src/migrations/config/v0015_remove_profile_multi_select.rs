use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};

use super::super::{Migration, MigrationStep};

pub(super) fn migration() -> Migration {
    Migration::rust(
        15,
        "remove Profile multi-select",
        &[include_str!("v0015_remove_profile_multi_select.rs")],
        RemoveProfileMultiSelect,
    )
}

struct RemoveProfileMultiSelect;

#[async_trait]
impl MigrationStep for RemoveProfileMultiSelect {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let has_multi_select: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pragma_table_info('profile') WHERE name = 'multi_select')")
                .fetch_one(&mut **transaction)
                .await
                .context("inspect Profile schema before removing multi-select")?;
        if !has_multi_select {
            return Ok(());
        }
        sqlx::query("ALTER TABLE profile DROP COLUMN multi_select")
            .execute(&mut **transaction)
            .await
            .context("remove retired Profile multi-select column")?;
        Ok(())
    }
}
