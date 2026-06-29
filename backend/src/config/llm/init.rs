use anyhow::Result;
use sqlx::{Pool, Sqlite};

use crate::config::server::init::ensure_column;

pub async fn initialize_llm_tables(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing LLM provider database tables");
    create_llm_provider_table(pool).await?;
    tracing::debug!("LLM provider database tables initialized successfully");
    Ok(())
}

async fn create_llm_provider_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating llm_provider table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS llm_provider (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            provider_type TEXT NOT NULL,
            base_url TEXT NOT NULL,
            model_id TEXT NOT NULL,
            secret_alias TEXT,
            default_params_json TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create llm_provider table: {}", e);
        anyhow::anyhow!("Failed to create llm_provider table: {}", e)
    })?;

    tracing::debug!("llm_provider table created or already exists");
    ensure_column(pool, "llm_provider", "is_default", "BOOLEAN NOT NULL DEFAULT 0").await?;
    Ok(())
}
