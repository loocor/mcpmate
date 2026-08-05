use anyhow::Result;
use sqlx::{Pool, Sqlite};

pub async fn initialize_llm_tables(pool: &Pool<Sqlite>) -> Result<()> {
    mcpmate_migrations::migrate_config(pool).await?;
    Ok(())
}
