use anyhow::Result;
use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};

use super::super::{Migration, MigrationStep};

pub(super) fn migration() -> Migration {
    Migration::rust(
        2,
        "add llm provider default flag",
        &[include_str!("v0002_add_llm_provider_default.rs")],
        AddLlmProviderDefault,
    )
}

struct AddLlmProviderDefault;

#[async_trait]
impl MigrationStep for AddLlmProviderDefault {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('llm_provider') WHERE name = 'is_default')",
        )
        .fetch_one(&mut **transaction)
        .await?;
        if !exists {
            sqlx::query("ALTER TABLE llm_provider ADD COLUMN is_default BOOLEAN NOT NULL DEFAULT 0")
                .execute(&mut **transaction)
                .await?;
        }
        Ok(())
    }
}
