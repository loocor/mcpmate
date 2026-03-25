use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

use crate::common::constants::database::tables;

const DEFAULT_BACKUP_POLICY: &str = "keep_n";
const DEFAULT_CAPABILITY_SOURCE: &str = "activated";

/// Initialize client management table (identifier-managed/policy metadata)
pub async fn initialize_client_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing client management table");

    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {table} (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            identifier TEXT NOT NULL UNIQUE,
            managed INTEGER NOT NULL DEFAULT 1 CHECK (managed IN (0, 1)),
            -- Management mode: hosted|transparent
            config_mode TEXT NOT NULL DEFAULT 'hosted' CHECK (config_mode IN ('hosted','transparent')),
            -- Transport protocol: auto|stdio|streamable_http (default: auto)
            transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                transport IN ('auto', 'stdio', 'streamable_http')
            ),
            -- Client version string (optional)
            client_version TEXT,
            backup_policy TEXT NOT NULL DEFAULT '{default_policy}' CHECK (
                backup_policy IN ('keep_last', 'keep_n', 'off')
            ),
            backup_limit INTEGER DEFAULT 30,
            capability_source TEXT NOT NULL DEFAULT '{default_capability_source}' CHECK (
                capability_source IN ('activated', 'profiles', 'custom')
            ),
            selected_profile_ids TEXT,
            custom_profile_id TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        table = tables::CLIENT,
        default_policy = DEFAULT_BACKUP_POLICY,
        default_capability_source = DEFAULT_CAPABILITY_SOURCE,
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create {} table: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to create {} table: {}", tables::CLIENT, e)
    })?;

    ensure_column(
        pool,
        tables::CLIENT,
        "capability_source",
        "TEXT NOT NULL DEFAULT 'activated' CHECK (capability_source IN ('activated', 'profiles', 'custom'))",
    )
    .await?;
    ensure_column(pool, tables::CLIENT, "selected_profile_ids", "TEXT").await?;
    ensure_column(pool, tables::CLIENT, "custom_profile_id", "TEXT").await?;

    sqlx::query(&format!(
        "UPDATE {table} SET capability_source = ? WHERE capability_source IS NULL OR capability_source = ''",
        table = tables::CLIENT
    ))
    .bind(DEFAULT_CAPABILITY_SOURCE)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to backfill {} capability_source: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to backfill {} capability_source: {}", tables::CLIENT, e)
    })?;

    tracing::debug!("{} table initialized", tables::CLIENT);
    Ok(())
}

async fn ensure_column(
    pool: &Pool<Sqlite>,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let stmt = format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition}",
        table = table,
        column = column,
        definition = definition
    );

    match sqlx::query(&stmt).execute(pool).await {
        Ok(_) => {
            tracing::debug!("Added column {}.{}", table, column);
            Ok(())
        }
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column name") => {
            tracing::trace!("Column {}.{} already exists", table, column);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to add column {}.{}: {}", table, column, e);
            Err(anyhow::anyhow!("Failed to add column {}.{}: {}", table, column, e))
        }
    }
}
