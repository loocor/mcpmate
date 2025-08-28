// Server database initialization
// Contains functions for initializing server-related database tables

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

use crate::common::constants::database::tables;

/// Initialize all server-related database tables
pub async fn initialize_server_tables(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing server-related database tables");

    create_server_config_table(pool).await?;
    create_server_args_table(pool).await?;
    create_server_env_table(pool).await?;
    create_server_meta_table(pool).await?;

    verify_server_tables(pool).await?;

    tracing::debug!("Server-related database tables initialized successfully");
    Ok(())
}

/// Create server_config table if it doesn't exist
async fn create_server_config_table(pool: &Pool<Sqlite>) -> Result<()> {
    use crate::common::constants::transport;

    tracing::debug!("Creating server_config table if it doesn't exist");

    let create_sql = format!(
        r#"
        CREATE TABLE IF NOT EXISTS server_config (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            server_type TEXT NOT NULL CHECK (
                server_type IN ('{}', '{}', '{}')
            ),
            command TEXT,
            url TEXT,
            transport_type TEXT CHECK (
                transport_type IN ('{}', '{}', '{}') OR transport_type IS NULL
            ),
            capabilities TEXT,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        transport::STDIO,
        transport::SSE,
        transport::STREAMABLE_HTTP,
        transport::STDIO,
        transport::SSE,
        transport::STREAMABLE_HTTP
    );

    sqlx::query(&create_sql).execute(pool).await.map_err(|e| {
        tracing::error!("Failed to create server_config table: {}", e);
        anyhow::anyhow!("Failed to create server_config table: {}", e)
    })?;

    tracing::debug!("server_config table created or already exists");
    Ok(())
}

/// Create server_args table if it doesn't exist
async fn create_server_args_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_args table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_args (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            arg_index INTEGER NOT NULL,
            arg_value TEXT NOT NULL,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, arg_index)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_args table: {}", e);
        anyhow::anyhow!("Failed to create server_args table: {}", e)
    })?;

    tracing::debug!("server_args table created or already exists");
    Ok(())
}

/// Create server_env table if it doesn't exist
async fn create_server_env_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_env table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_env (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            env_key TEXT NOT NULL,
            env_value TEXT NOT NULL,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, env_key)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_env table: {}", e);
        anyhow::anyhow!("Failed to create server_env table: {}", e)
    })?;

    tracing::debug!("server_env table created or already exists");
    Ok(())
}

/// Create server_meta table if it doesn't exist
async fn create_server_meta_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_meta table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_meta (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            description TEXT,
            author TEXT,
            website TEXT,
            repository TEXT,
            category TEXT,
            recommended_scenario TEXT,
            rating INTEGER,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_meta table: {}", e);
        anyhow::anyhow!("Failed to create server_meta table: {}", e)
    })?;

    tracing::debug!("server_meta table created or already exists");
    Ok(())
}

/// Verify that all server tables were created successfully
async fn verify_server_tables(pool: &Pool<Sqlite>) -> Result<()> {
    for table in [
        tables::SERVER_CONFIG,
        tables::SERVER_ARGS,
        tables::SERVER_ENV,
        tables::SERVER_META,
    ] {
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
