// Runtime database initialization
// Contains functions for initializing runtime-related database tables

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

/// Initialize all runtime-related database tables
pub async fn initialize_runtime_tables(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing runtime-related database tables");

    create_runtime_config_table(pool).await?;
    create_runtime_config_indexes(pool).await?;

    verify_runtime_tables(pool).await?;

    tracing::debug!("Runtime-related database tables initialized successfully");
    Ok(())
}

/// Create runtime_config table if it doesn't exist
async fn create_runtime_config_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating runtime_config table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS runtime_config (
            id TEXT PRIMARY KEY,
            runtime_type TEXT NOT NULL UNIQUE,
            version TEXT NOT NULL,
            relative_bin_path TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create runtime_config table: {}", e);
        anyhow::anyhow!("Failed to create runtime_config table: {}", e)
    })?;

    tracing::debug!("runtime_config table created or already exists");
    Ok(())
}

/// Create indexes on runtime_config table
async fn create_runtime_config_indexes(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating indexes on runtime_config table");

    // Create index on runtime_type and version
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_runtime_config_type_version
        ON runtime_config(runtime_type, version)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on runtime_config: {}", e);
        anyhow::anyhow!("Failed to create index on runtime_config: {}", e)
    })?;

    tracing::debug!("Index on runtime_config(runtime_type, version) created or already exists");

    // Note: The second index in the original code was identical to the first one,
    // so we only create one index here to avoid duplication

    Ok(())
}

/// Verify that all runtime tables were created successfully
async fn verify_runtime_tables(pool: &Pool<Sqlite>) -> Result<()> {
    let tables = vec!["runtime_config"];

    for table in tables {
        sqlx::query(&format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"
        ))
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify {} table: {}", table, e);
            anyhow::anyhow!("Failed to verify {} table: {}", table, e)
        })?
        .ok_or_else(|| {
            let err = format!("{table} table not found after creation");
            tracing::error!("{}", err);
            anyhow::anyhow!(err)
        })?;

        tracing::debug!("Verified {} table exists", table);
    }

    Ok(())
}
