// Config Suit database initialization
// Contains functions for initializing config suit-related database tables

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

/// Initialize all config suit-related database tables
pub async fn initialize_suit_tables(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing config suit-related database tables");

    create_config_suit_table(pool).await?;
    create_config_suit_server_table(pool).await?;
    create_config_suit_tool_table(pool).await?;
    create_config_suit_tool_index(pool).await?;
    create_config_suit_resource_table(pool).await?;
    create_config_suit_resource_index(pool).await?;

    verify_suit_tables(pool).await?;

    tracing::debug!("Config suit-related database tables initialized successfully");
    Ok(())
}

/// Create config_suit table if it doesn't exist
async fn create_config_suit_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating config_suit table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_suit (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            type TEXT NOT NULL,
            multi_select BOOLEAN NOT NULL DEFAULT 0,
            priority INTEGER NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT 0,
            is_default BOOLEAN NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create config_suit table: {}", e);
        anyhow::anyhow!("Failed to create config_suit table: {}", e)
    })?;

    tracing::debug!("config_suit table created or already exists");
    Ok(())
}

/// Create config_suit_server table if it doesn't exist
async fn create_config_suit_server_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating config_suit_server table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_suit_server (
            id TEXT PRIMARY KEY,
            config_suit_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (config_suit_id) REFERENCES config_suit (id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(config_suit_id, server_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create config_suit_server table: {}", e);
        anyhow::anyhow!("Failed to create config_suit_server table: {}", e)
    })?;

    tracing::debug!("config_suit_server table created or already exists");
    Ok(())
}

/// Create config_suit_tool table if it doesn't exist
async fn create_config_suit_tool_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating config_suit_tool table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_suit_tool (
            id TEXT PRIMARY KEY,
            config_suit_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            tool_name TEXT NOT NULL,
            unique_name TEXT,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (config_suit_id) REFERENCES config_suit (id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(config_suit_id, server_id, tool_name)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create config_suit_tool table: {}", e);
        anyhow::anyhow!("Failed to create config_suit_tool table: {}", e)
    })?;

    tracing::debug!("config_suit_tool table created or already exists");
    Ok(())
}

/// Create unique index on config_suit_tool.unique_name if it doesn't exist
async fn create_config_suit_tool_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating unique index on config_suit_tool.unique_name if it doesn't exist");

    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_config_suit_tool_unique_name
        ON config_suit_tool(unique_name)
        WHERE unique_name IS NOT NULL
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to create unique index on config_suit_tool.unique_name: {}",
            e
        );
        anyhow::anyhow!(
            "Failed to create unique index on config_suit_tool.unique_name: {}",
            e
        )
    })?;

    tracing::debug!("Unique index on config_suit_tool.unique_name created or already exists");
    Ok(())
}

/// Create config_suit_resource table if it doesn't exist
async fn create_config_suit_resource_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating config_suit_resource table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_suit_resource (
            id TEXT PRIMARY KEY,
            config_suit_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            resource_uri TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (config_suit_id) REFERENCES config_suit (id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(config_suit_id, server_id, resource_uri)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create config_suit_resource table: {}", e);
        anyhow::anyhow!("Failed to create config_suit_resource table: {}", e)
    })?;

    tracing::debug!("config_suit_resource table created or already exists");
    Ok(())
}

/// Create index on config_suit_resource for performance
async fn create_config_suit_resource_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating index on config_suit_resource for performance");

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_config_suit_resource_lookup
        ON config_suit_resource(config_suit_id, enabled)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on config_suit_resource: {}", e);
        anyhow::anyhow!("Failed to create index on config_suit_resource: {}", e)
    })?;

    tracing::debug!("Index on config_suit_resource created or already exists");
    Ok(())
}

/// Verify that all config suit tables were created successfully
async fn verify_suit_tables(pool: &Pool<Sqlite>) -> Result<()> {
    let tables = vec![
        "config_suit",
        "config_suit_server",
        "config_suit_tool",
        "config_suit_resource",
    ];

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
