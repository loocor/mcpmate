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
    create_server_tools_table(pool).await?;
    create_server_tools_index(pool).await?;
    create_server_prompts_table(pool).await?;
    create_server_prompts_index(pool).await?;
    create_server_resources_table(pool).await?;
    create_server_resources_index(pool).await?;
    create_server_resource_templates_table(pool).await?;
    create_server_resource_templates_index(pool).await?;
    create_config_suit_tool_table(pool).await?;
    create_config_suit_tool_index(pool).await?;
    create_config_suit_resource_table(pool).await?;
    create_config_suit_resource_index(pool).await?;
    create_config_suit_prompt_table(pool).await?;
    create_config_suit_prompt_index(pool).await?;

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

/// Create server_tools table if it doesn't exist
async fn create_server_tools_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_tools table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_tools (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            tool_name TEXT NOT NULL,
            unique_name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, tool_name),
            UNIQUE(unique_name)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_tools table: {}", e);
        anyhow::anyhow!("Failed to create server_tools table: {}", e)
    })?;

    tracing::debug!("server_tools table created or already exists");
    Ok(())
}

/// Create indexes on server_tools table for performance
async fn create_server_tools_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating indexes on server_tools table for performance");

    // Index for lookup by server_id and tool_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_tools_lookup
        ON server_tools(server_id, tool_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_tools lookup: {}", e);
        anyhow::anyhow!("Failed to create index on server_tools lookup: {}", e)
    })?;

    // Index for lookup by unique_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_tools_unique_name
        ON server_tools(unique_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_tools unique_name: {}", e);
        anyhow::anyhow!("Failed to create index on server_tools unique_name: {}", e)
    })?;

    // Index for lookup by server_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_tools_server_name
        ON server_tools(server_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_tools server_name: {}", e);
        anyhow::anyhow!("Failed to create index on server_tools server_name: {}", e)
    })?;

    tracing::debug!("Indexes on server_tools table created or already exists");
    Ok(())
}

/// Create server_prompts table if it doesn't exist (shadow table for indexing)
async fn create_server_prompts_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_prompts table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_prompts (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            prompt_name TEXT NOT NULL,
            unique_name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, prompt_name),
            UNIQUE(unique_name)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_prompts table: {}", e);
        anyhow::anyhow!("Failed to create server_prompts table: {}", e)
    })?;

    tracing::debug!("server_prompts table created or already exists");
    Ok(())
}

/// Create indexes on server_prompts table for performance
async fn create_server_prompts_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating indexes on server_prompts table for performance");

    // Index for lookup by server_id and prompt_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_prompts_lookup
        ON server_prompts(server_id, prompt_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_prompts lookup: {}", e);
        anyhow::anyhow!("Failed to create index on server_prompts lookup: {}", e)
    })?;

    // Index for lookup by unique_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_prompts_unique_name
        ON server_prompts(unique_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_prompts unique_name: {}", e);
        anyhow::anyhow!("Failed to create index on server_prompts unique_name: {}", e)
    })?;

    // Index for lookup by server_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_prompts_server_name
        ON server_prompts(server_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_prompts server_name: {}", e);
        anyhow::anyhow!("Failed to create index on server_prompts server_name: {}", e)
    })?;

    tracing::debug!("Indexes on server_prompts table created or already exists");
    Ok(())
}

/// Create server_resources table if it doesn't exist (shadow table for indexing)
async fn create_server_resources_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_resources table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_resources (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            resource_uri TEXT NOT NULL,
            unique_uri TEXT NOT NULL,
            name TEXT,
            description TEXT,
            mime_type TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, resource_uri),
            UNIQUE(unique_uri)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_resources table: {}", e);
        anyhow::anyhow!("Failed to create server_resources table: {}", e)
    })?;

    tracing::debug!("server_resources table created or already exists");
    Ok(())
}

/// Create indexes on server_resources table for performance
async fn create_server_resources_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating indexes on server_resources table for performance");

    // Index for lookup by server_id and resource_uri
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_resources_lookup
        ON server_resources(server_id, resource_uri)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_resources lookup: {}", e);
        anyhow::anyhow!("Failed to create index on server_resources lookup: {}", e)
    })?;

    // Index for lookup by unique_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_resources_unique_uri
        ON server_resources(unique_uri)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_resources unique_uri: {}", e);
        anyhow::anyhow!("Failed to create index on server_resources unique_uri: {}", e)
    })?;

    // Index for lookup by server_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_resources_server_name
        ON server_resources(server_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_resources server_name: {}", e);
        anyhow::anyhow!("Failed to create index on server_resources server_name: {}", e)
    })?;

    tracing::debug!("Indexes on server_resources table created or already exists");
    Ok(())
}

/// Create server_resource_templates table if it doesn't exist (shadow table for indexing)
async fn create_server_resource_templates_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_resource_templates table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_resource_templates (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            uri_template TEXT NOT NULL,
            unique_name TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, uri_template),
            UNIQUE(unique_name)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_resource_templates table: {}", e);
        anyhow::anyhow!("Failed to create server_resource_templates table: {}", e)
    })?;

    tracing::debug!("server_resource_templates table created or already exists");
    Ok(())
}

/// Create indexes on server_resource_templates table for performance
async fn create_server_resource_templates_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating indexes on server_resource_templates table for performance");

    // Index for lookup by server_id and uri_template
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_resource_templates_lookup
        ON server_resource_templates(server_id, uri_template)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_resource_templates lookup: {}", e);
        anyhow::anyhow!("Failed to create index on server_resource_templates lookup: {}", e)
    })?;

    // Index for lookup by unique_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_resource_templates_unique_name
        ON server_resource_templates(unique_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_resource_templates unique_name: {}", e);
        anyhow::anyhow!("Failed to create index on server_resource_templates unique_name: {}", e)
    })?;

    // Index for lookup by server_name
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_resource_templates_server_name
        ON server_resource_templates(server_name)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_resource_templates server_name: {}", e);
        anyhow::anyhow!("Failed to create index on server_resource_templates server_name: {}", e)
    })?;

    tracing::debug!("Indexes on server_resource_templates table created or already exists");
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
            server_tool_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (config_suit_id) REFERENCES config_suit (id) ON DELETE CASCADE,
            FOREIGN KEY (server_tool_id) REFERENCES server_tools (id) ON DELETE CASCADE,
            UNIQUE(config_suit_id, server_tool_id)
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

/// Create indexes on config_suit_tool table for performance
async fn create_config_suit_tool_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating indexes on config_suit_tool table for performance");

    // Index for lookup by config_suit_id and enabled status
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_config_suit_tool_lookup
        ON config_suit_tool(config_suit_id, enabled)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on config_suit_tool lookup: {}", e);
        anyhow::anyhow!("Failed to create index on config_suit_tool lookup: {}", e)
    })?;

    // Index for lookup by server_tool_id
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_config_suit_tool_server_tool
        ON config_suit_tool(server_tool_id)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on config_suit_tool server_tool: {}", e);
        anyhow::anyhow!("Failed to create index on config_suit_tool server_tool: {}", e)
    })?;

    tracing::debug!("Indexes on config_suit_tool table created or already exists");
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

/// Create config_suit_prompt table if it doesn't exist
async fn create_config_suit_prompt_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating config_suit_prompt table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_suit_prompt (
            id TEXT PRIMARY KEY,
            config_suit_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            prompt_name TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (config_suit_id) REFERENCES config_suit (id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(config_suit_id, server_id, prompt_name)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create config_suit_prompt table: {}", e);
        anyhow::anyhow!("Failed to create config_suit_prompt table: {}", e)
    })?;

    tracing::debug!("config_suit_prompt table created or already exists");
    Ok(())
}

/// Create index on config_suit_prompt for performance
async fn create_config_suit_prompt_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating index on config_suit_prompt for performance");

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_config_suit_prompt_lookup
        ON config_suit_prompt(config_suit_id, enabled)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on config_suit_prompt: {}", e);
        anyhow::anyhow!("Failed to create index on config_suit_prompt: {}", e)
    })?;

    tracing::debug!("Index on config_suit_prompt created or already exists");
    Ok(())
}

/// Verify that all config suit tables were created successfully
async fn verify_suit_tables(pool: &Pool<Sqlite>) -> Result<()> {
    let tables = vec![
        "config_suit",
        "config_suit_server",
        "server_tools",
        "server_prompts",
        "server_resources",
        "server_resource_templates",
        "config_suit_tool",
        "config_suit_resource",
        "config_suit_prompt",
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
