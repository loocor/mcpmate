use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

use crate::common::constants::database::tables;

const DEFAULT_BACKUP_POLICY: &str = "keep_n";

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
            -- Transport protocol: auto|sse|stdio|streamable_http (default: auto)
            transport TEXT NOT NULL DEFAULT 'auto' CHECK (
                transport IN ('auto', 'sse', 'stdio', 'streamable_http')
            ),
            -- Client version string (optional)
            client_version TEXT,
            backup_policy TEXT NOT NULL DEFAULT '{default_policy}' CHECK (
                backup_policy IN ('keep_last', 'keep_n', 'off')
            ),
            backup_limit INTEGER DEFAULT 30,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        table = tables::CLIENT,
        default_policy = DEFAULT_BACKUP_POLICY,
    ))
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create {} table: {}", tables::CLIENT, e);
        anyhow::anyhow!("Failed to create {} table: {}", tables::CLIENT, e)
    })?;

    tracing::debug!("{} table initialized", tables::CLIENT);
    Ok(())
}
