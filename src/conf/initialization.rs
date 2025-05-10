// Database initialization for MCPMate
// Contains functions for initializing the database schema

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

/// Run database initialization
pub async fn run_initialization(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::info!("Running database initialization");

    // Create server_config table if it doesn't exist
    tracing::debug!("Creating server_config table if it doesn't exist");
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_config (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            server_type TEXT NOT NULL,
            command TEXT,
            url TEXT,
            transport_type TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => tracing::debug!("server_config table created or already exists"),
        Err(e) => {
            tracing::error!("Failed to create server_config table: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to create server_config table: {}",
                e
            ));
        }
    };

    // Create server_args table if it doesn't exist
    tracing::debug!("Creating server_args table if it doesn't exist");
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_args (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            arg_index INTEGER NOT NULL,
            arg_value TEXT NOT NULL,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, arg_index)
        )
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => tracing::debug!("server_args table created or already exists"),
        Err(e) => {
            tracing::error!("Failed to create server_args table: {}", e);
            return Err(anyhow::anyhow!("Failed to create server_args table: {}", e));
        }
    };

    // Create server_env table if it doesn't exist
    tracing::debug!("Creating server_env table if it doesn't exist");
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_env (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            env_key TEXT NOT NULL,
            env_value TEXT NOT NULL,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, env_key)
        )
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => tracing::debug!("server_env table created or already exists"),
        Err(e) => {
            tracing::error!("Failed to create server_env table: {}", e);
            return Err(anyhow::anyhow!("Failed to create server_env table: {}", e));
        }
    };

    // Create server_meta table if it doesn't exist
    tracing::debug!("Creating server_meta table if it doesn't exist");
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_meta (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
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
    {
        Ok(_) => tracing::debug!("server_meta table created or already exists"),
        Err(e) => {
            tracing::error!("Failed to create server_meta table: {}", e);
            return Err(anyhow::anyhow!("Failed to create server_meta table: {}", e));
        }
    };

    // Create config_suit table if it doesn't exist
    tracing::debug!("Creating config_suit table if it doesn't exist");
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_suit (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            type TEXT NOT NULL,
            multi_select BOOLEAN NOT NULL DEFAULT 0,
            priority INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => tracing::debug!("config_suit table created or already exists"),
        Err(e) => {
            tracing::error!("Failed to create config_suit table: {}", e);
            return Err(anyhow::anyhow!("Failed to create config_suit table: {}", e));
        }
    };

    // Create config_suit_server table if it doesn't exist
    tracing::debug!("Creating config_suit_server table if it doesn't exist");
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_suit_server (
            id TEXT PRIMARY KEY,
            config_suit_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
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
    {
        Ok(_) => tracing::debug!("config_suit_server table created or already exists"),
        Err(e) => {
            tracing::error!("Failed to create config_suit_server table: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to create config_suit_server table: {}",
                e
            ));
        }
    };

    // Create config_suit_tool table if it doesn't exist
    tracing::debug!("Creating config_suit_tool table if it doesn't exist");
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_suit_tool (
            id TEXT PRIMARY KEY,
            config_suit_id INTEGER NOT NULL,
            server_id INTEGER NOT NULL,
            tool_name TEXT NOT NULL,
            prefixed_name TEXT,
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
    {
        Ok(_) => tracing::debug!("config_suit_tool table created or already exists"),
        Err(e) => {
            tracing::error!("Failed to create config_suit_tool table: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to create config_suit_tool table: {}",
                e
            ));
        }
    };

    // Verify that all tables were created successfully
    let tables = vec![
        "server_config",
        "server_args",
        "server_env",
        "server_meta",
        "config_suit",
        "config_suit_server",
        "config_suit_tool",
    ];

    for table in tables {
        match sqlx::query(&format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
            table
        ))
        .fetch_optional(pool)
        .await
        {
            Ok(Some(_)) => tracing::info!("Verified {} table exists", table),
            Ok(None) => {
                let err = format!("{} table not found after creation", table);
                tracing::error!("{}", err);
                return Err(anyhow::anyhow!(err));
            }
            Err(e) => {
                tracing::error!("Failed to verify {} table: {}", table, e);
                return Err(anyhow::anyhow!("Failed to verify {} table: {}", table, e));
            }
        };
    }

    tracing::info!("Database initialization completed successfully");
    Ok(())
}
