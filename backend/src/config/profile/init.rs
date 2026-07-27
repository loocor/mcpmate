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
    create_profile_server_relationships_table(pool).await?;
    create_server_tools_table(pool).await?;
    create_server_tools_index(pool).await?;
    create_server_prompts_table(pool).await?;
    create_server_prompts_index(pool).await?;
    create_server_resources_table(pool).await?;
    create_server_resources_index(pool).await?;
    create_server_resource_templates_table(pool).await?;
    create_server_resource_templates_index(pool).await?;
    create_server_issued_resources_table(pool).await?;
    create_server_issued_resources_index(pool).await?;
    create_profile_capability_refs_table(pool).await?;
    create_direct_exposure_refs_table(pool).await?;
    create_direct_exposure_servers_table(pool).await?;

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

/// Create profile-level server relationships if they do not exist.
async fn create_profile_server_relationships_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating profile_server_relationships table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profile_server_relationships (
            profile_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            new_ref_policy TEXT NOT NULL CHECK (new_ref_policy IN ('follow', 'review')),
            FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE,
            PRIMARY KEY(profile_id, server_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create profile_server_relationships table: {}", e);
        anyhow::anyhow!("Failed to create profile_server_relationships table: {}", e)
    })?;

    tracing::debug!("profile_server_relationships table created or already exists");
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
            route_uri TEXT,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, uri_template),
            UNIQUE(unique_name),
            UNIQUE(route_uri)
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

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_resource_templates_route_uri
        ON server_resource_templates(route_uri)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create index on server_resource_templates route_uri: {}", e);
        anyhow::anyhow!("Failed to create index on server_resource_templates route_uri: {}", e)
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

async fn create_server_issued_resources_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_issued_resources table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_issued_resources (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            resource_uri TEXT NOT NULL,
            unique_uri TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_seen_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, resource_uri),
            UNIQUE(unique_uri)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!("Failed to create server_issued_resources table: {}", error);
        anyhow::anyhow!("Failed to create server_issued_resources table: {}", error)
    })?;

    Ok(())
}

async fn create_server_issued_resources_index(pool: &Pool<Sqlite>) -> Result<()> {
    for statement in [
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_issued_resources_lookup
        ON server_issued_resources(server_id, resource_uri)
        "#,
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_issued_resources_unique_uri
        ON server_issued_resources(unique_uri)
        "#,
    ] {
        sqlx::query(statement).execute(pool).await.map_err(|error| {
            tracing::error!("Failed to create server_issued_resources index: {}", error);
            anyhow::anyhow!("Failed to create server_issued_resources index: {}", error)
        })?;
    }

    Ok(())
}

async fn create_profile_capability_refs_table(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profile_capability_refs (
            profile_id TEXT NOT NULL,
            ref_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL,
            FOREIGN KEY (profile_id) REFERENCES profile (id) ON DELETE CASCADE,
            FOREIGN KEY (ref_id) REFERENCES capability_refs (ref_id) ON DELETE CASCADE,
            PRIMARY KEY(profile_id, ref_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|error| anyhow::anyhow!("Failed to create profile_capability_refs table: {error}"))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_profile_capability_refs_ref ON profile_capability_refs(ref_id)")
        .execute(pool)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to index profile_capability_refs: {error}"))?;
    Ok(())
}

async fn create_direct_exposure_refs_table(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS direct_exposure_refs (
            consumer_id TEXT NOT NULL,
            ref_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL,
            FOREIGN KEY (consumer_id) REFERENCES client (identifier) ON DELETE CASCADE,
            FOREIGN KEY (ref_id) REFERENCES capability_refs (ref_id) ON DELETE CASCADE,
            PRIMARY KEY(consumer_id, ref_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|error| anyhow::anyhow!("Failed to create direct_exposure_refs table: {error}"))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_direct_exposure_refs_ref ON direct_exposure_refs(ref_id)")
        .execute(pool)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to index direct_exposure_refs: {error}"))?;
    Ok(())
}

async fn create_direct_exposure_servers_table(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS direct_exposure_servers (
            consumer_id TEXT NOT NULL,
            server_id TEXT NOT NULL,
            new_ref_policy TEXT NOT NULL CHECK (new_ref_policy IN ('follow', 'review')),
            FOREIGN KEY (consumer_id) REFERENCES client (identifier) ON DELETE CASCADE,
            PRIMARY KEY(consumer_id, server_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|error| anyhow::anyhow!("Failed to create direct_exposure_servers table: {error}"))?;
    Ok(())
}

/// Verify that all profile tables were created successfully
async fn verify_profile_tables(pool: &Pool<Sqlite>) -> Result<()> {
    for table in [
        tables::PROFILE,
        "profile_server_relationships",
        tables::SERVER_TOOLS,
        tables::SERVER_PROMPTS,
        tables::SERVER_RESOURCES,
        tables::SERVER_RESOURCE_TEMPLATES,
        "server_issued_resources",
        "profile_capability_refs",
        "direct_exposure_refs",
        "direct_exposure_servers",
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

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    #[tokio::test]
    async fn resource_registry_schema_contains_template_routes_and_issued_resources() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");

        let template_columns =
            sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('server_resource_templates')")
                .fetch_all(&pool)
                .await
                .expect("load template columns");
        assert!(template_columns.iter().any(|column| column == "route_uri"));

        let issued_columns =
            sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('server_issued_resources')")
                .fetch_all(&pool)
                .await
                .expect("load issued resource columns");
        for expected in [
            "id",
            "server_id",
            "server_name",
            "resource_uri",
            "unique_uri",
            "created_at",
            "last_seen_at",
        ] {
            assert!(
                issued_columns.iter().any(|column| column == expected),
                "missing issued resource column {expected}"
            );
        }

        let issued_indexes =
            sqlx::query_scalar::<_, String>("SELECT name FROM pragma_index_list('server_issued_resources')")
                .fetch_all(&pool)
                .await
                .expect("load issued resource indexes");
        assert!(
            issued_indexes
                .iter()
                .any(|index| index == "idx_server_issued_resources_lookup")
        );
        assert!(
            issued_indexes
                .iter()
                .any(|index| index == "idx_server_issued_resources_unique_uri")
        );
    }

    #[tokio::test]
    async fn authoring_schema_uses_capability_refs_without_legacy_capability_tables() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .expect("initialize client table");
        crate::config::database::initialize_capability_catalog(&pool)
            .await
            .expect("initialize capability catalog");
        initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");

        for table in [
            "profile_capability_refs",
            "profile_server_relationships",
            "direct_exposure_refs",
            "direct_exposure_servers",
        ] {
            let exists: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
                    .bind(table)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(exists, 1, "missing authoring table {table}");
        }
        for legacy in [
            "profile_tool",
            "profile_prompt",
            "profile_resource",
            "profile_resource_template",
        ] {
            let exists: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
                    .bind(legacy)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(exists, 0, "legacy authoring table {legacy} must not exist");
        }

        let profile_server_columns =
            sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('profile_server_relationships')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(profile_server_columns.iter().any(|column| column == "enabled"));
        assert!(profile_server_columns.iter().any(|column| column == "new_ref_policy"));
        let direct_server_columns =
            sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('direct_exposure_servers')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(direct_server_columns.iter().any(|column| column == "new_ref_policy"));
    }
}
