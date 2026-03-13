// Profile database initialization
// Contains functions for initializing profile-related database tables

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

use crate::common::constants::database::tables;

/// Initialize all profile-related database tables
pub async fn initialize_profile_tables(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing profile-related database tables");

    create_profile_table(pool).await?;
    create_profile_server_table(pool).await?;
    create_server_tools_table(pool).await?;
    create_server_tools_index(pool).await?;
    create_server_prompts_table(pool).await?;
    create_server_prompts_index(pool).await?;
    create_server_resources_table(pool).await?;
    create_server_resources_index(pool).await?;
    create_server_resource_templates_table(pool).await?;
    create_server_resource_templates_index(pool).await?;
    create_profile_tool_table(pool).await?;
    create_profile_tool_index(pool).await?;
    create_profile_resource_table(pool).await?;
    create_profile_resource_index(pool).await?;
    create_profile_resource_template_table(pool).await?;
    create_profile_resource_template_index(pool).await?;
    create_profile_prompt_table(pool).await?;
    create_profile_prompt_index(pool).await?;

    verify_profile_tables(pool).await?;

    tracing::debug!("Profile-related database tables initialized successfully");
    Ok(())
}

/// Create profile table if it doesn't exist
async fn create_profile_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating {} table if it doesn't exist", tables::PROFILE);

    let create_sql = format!(
        r#"
        CREATE TABLE IF NOT EXISTS {} (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            type TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            multi_select BOOLEAN NOT NULL DEFAULT 0,
            priority INTEGER NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT 0,
            is_default BOOLEAN NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        tables::PROFILE
    );

    sqlx::query(&create_sql).execute(pool).await.map_err(|e| {
        tracing::error!("Failed to create {} table: {}", tables::PROFILE, e);
        anyhow::anyhow!("Failed to create {} table: {}", tables::PROFILE, e)
    })?;

    tracing::debug!("{} table created or already exists", tables::PROFILE);
    Ok(())
}

/// Create profile_server table if it doesn't exist
async fn create_profile_server_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating profile_server table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profile_server (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(profile_id, server_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create profile_server table: {}", e);
        anyhow::anyhow!("Failed to create profile_server table: {}", e)
    })?;

    tracing::debug!("profile_server table created or already exists");
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

/// Create profile_tool table if it doesn't exist
async fn create_profile_tool_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating profile_tool table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profile_tool (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            server_tool_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE,
            FOREIGN KEY (server_tool_id) REFERENCES server_tools (id) ON DELETE CASCADE,
            UNIQUE(profile_id, server_tool_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create profile_tool table: {}", e);
        anyhow::anyhow!("Failed to create profile_tool table: {}", e)
    })?;

    tracing::debug!("profile_tool table created or already exists");
    Ok(())
}

/// Create indexes on profile_tool table for performance
async fn create_profile_tool_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating indexes on profile_tool table for performance");

    // Index for lookup by profile_id and enabled status
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_profile_tool_lookup
        ON profile_tool(profile_id, enabled)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on profile_tool lookup: {}", e);
        anyhow::anyhow!("Failed to create index on profile_tool lookup: {}", e)
    })?;

    // Index for lookup by server_tool_id
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_profile_tool_server_tool
        ON profile_tool(server_tool_id)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on profile_tool server_tool: {}", e);
        anyhow::anyhow!("Failed to create index on profile_tool server_tool: {}", e)
    })?;

    tracing::debug!("Indexes on profile_tool table created or already exists");
    Ok(())
}

/// Create profile_resource table if it doesn't exist
async fn create_profile_resource_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating profile_resource table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profile_resource (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            resource_uri TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(profile_id, server_id, resource_uri)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create profile_resource table: {}", e);
        anyhow::anyhow!("Failed to create profile_resource table: {}", e)
    })?;

    tracing::debug!("profile_resource table created or already exists");
    Ok(())
}

/// Create index on profile_resource for performance
async fn create_profile_resource_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating index on profile_resource for performance");

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_profile_resource_lookup
        ON profile_resource(profile_id, enabled)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on profile_resource: {}", e);
        anyhow::anyhow!("Failed to create index on profile_resource: {}", e)
    })?;

    tracing::debug!("Index on profile_resource created or already exists");
    Ok(())
}

/// Create profile_resource_template table if it doesn't exist
async fn create_profile_resource_template_table(pool: &Pool<Sqlite>) -> Result<()> {
    use crate::common::constants::database::tables;
    tracing::debug!(
        "Creating {} table if it doesn't exist",
        tables::PROFILE_RESOURCE_TEMPLATE
    );

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profile_resource_template (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            uri_template TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(profile_id, server_id, uri_template)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create {} table: {}", tables::PROFILE_RESOURCE_TEMPLATE, e);
        anyhow::anyhow!("Failed to create {} table: {}", tables::PROFILE_RESOURCE_TEMPLATE, e)
    })?;

    tracing::debug!("{} table created or already exists", tables::PROFILE_RESOURCE_TEMPLATE);
    Ok(())
}

/// Create index on profile_resource_template for performance
async fn create_profile_resource_template_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating index on profile_resource_template for performance");

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_profile_resource_template_lookup
        ON profile_resource_template(profile_id, enabled)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on profile_resource_template: {}", e);
        anyhow::anyhow!("Failed to create index on profile_resource_template: {}", e)
    })?;

    tracing::debug!("Index on profile_resource_template created or already exists");
    Ok(())
}

/// Create profile_prompt table if it doesn't exist
async fn create_profile_prompt_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating profile_prompt table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profile_prompt (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            prompt_name TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(profile_id, server_id, prompt_name)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create profile_prompt table: {}", e);
        anyhow::anyhow!("Failed to create profile_prompt table: {}", e)
    })?;

    tracing::debug!("profile_prompt table created or already exists");
    Ok(())
}

/// Create index on profile_prompt for performance
async fn create_profile_prompt_index(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating index on profile_prompt for performance");

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_profile_prompt_lookup
        ON profile_prompt(profile_id, enabled)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on profile_prompt: {}", e);
        anyhow::anyhow!("Failed to create index on profile_prompt: {}", e)
    })?;

    tracing::debug!("Index on profile_prompt created or already exists");
    Ok(())
}

/// Verify that all profile tables were created successfully
async fn verify_profile_tables(pool: &Pool<Sqlite>) -> Result<()> {
    for table in [
        tables::PROFILE,
        tables::PROFILE_SERVER,
        tables::SERVER_TOOLS,
        tables::SERVER_PROMPTS,
        tables::SERVER_RESOURCES,
        tables::SERVER_RESOURCE_TEMPLATES,
        tables::PROFILE_TOOL,
        tables::PROFILE_RESOURCE,
        tables::PROFILE_RESOURCE_TEMPLATE,
        tables::PROFILE_PROMPT,
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
