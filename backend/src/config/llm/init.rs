use anyhow::Result;
use sqlx::{Pool, Sqlite};

pub async fn initialize_llm_tables(pool: &Pool<Sqlite>) -> Result<()> {
    mcpmate_migrations::verify_config_database(pool).await?;
    Ok(())
}
