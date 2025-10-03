// Server metadata management
// Contains operations for managing server metadata (description, author, etc.)

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::config::{models::ServerMeta, operations::utils::get_server_name};
use crate::generate_id;

/// Get server metadata from the database
pub async fn get_server_meta(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<Option<ServerMeta>> {
    tracing::debug!("Executing SQL query to get metadata for server ID {}", server_id);

    let meta = sqlx::query_as::<_, ServerMeta>(
        r#"
        SELECT * FROM server_meta
        WHERE server_id = ?
        "#,
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch server metadata")?;

    if meta.is_some() {
        tracing::debug!("Found metadata for server ID {}", server_id);
    } else {
        tracing::debug!("No metadata found for server ID {}", server_id);
    }

    Ok(meta)
}

/// Create or update server metadata in the database
pub async fn upsert_server_meta(
    pool: &Pool<Sqlite>,
    meta: &ServerMeta,
) -> Result<String> {
    tracing::debug!("Upserting metadata for server ID {}", meta.server_id);

    // Generate an ID for the metadata if it doesn't have one
    let meta_id = if let Some(id) = &meta.id {
        id.clone()
    } else {
        generate_id!("smet")
    };

    // Get the server name
    let server_name = get_server_name(pool, &meta.server_id).await?;

    let result = sqlx::query(
        r#"
        INSERT INTO server_meta (
            id, server_id, server_name, description, author, website, repository,
            category, recommended_scenario, rating, icons_json, server_version, protocol_version
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id) DO UPDATE SET
            server_name = excluded.server_name,
            description = excluded.description,
            author = excluded.author,
            website = excluded.website,
            repository = excluded.repository,
            category = excluded.category,
            recommended_scenario = excluded.recommended_scenario,
            rating = excluded.rating,
            icons_json = excluded.icons_json,
            server_version = excluded.server_version,
            protocol_version = excluded.protocol_version,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&meta_id)
    .bind(&meta.server_id)
    .bind(&server_name)
    .bind(&meta.description)
    .bind(&meta.author)
    .bind(&meta.website)
    .bind(&meta.repository)
    .bind(&meta.category)
    .bind(&meta.recommended_scenario)
    .bind(meta.rating)
    .bind(&meta.icons_json)
    .bind(&meta.server_version)
    .bind(&meta.protocol_version)
    .execute(pool)
    .await
    .context("Failed to upsert server metadata")?;

    if result.rows_affected() == 0 {
        // If no rows were affected, get the existing ID
        let existing_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM server_meta
            WHERE server_id = ?
            "#,
        )
        .bind(&meta.server_id)
        .fetch_one(pool)
        .await
        .context("Failed to get server metadata ID")?;

        return Ok(existing_id);
    }

    Ok(meta_id)
}

/// Update server icons based on the rmcp::Implementation metadata.
pub async fn update_server_icons(
    pool: &Pool<Sqlite>,
    server_id: &str,
    icons: Option<Vec<rmcp::model::Icon>>,
) -> Result<()> {
    let icons_json = if let Some(icons) = icons {
        Some(serde_json::to_string(&icons).context("Failed to serialize server icons")?)
    } else {
        None
    };

    let server_name = get_server_name(pool, server_id).await?;
    let icon_id = generate_id!("smet");

    sqlx::query(
        r#"
        INSERT INTO server_meta (id, server_id, server_name, icons_json)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(server_id) DO UPDATE SET
            server_name = excluded.server_name,
            icons_json = excluded.icons_json,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(icon_id)
    .bind(server_id)
    .bind(server_name)
    .bind(icons_json)
    .execute(pool)
    .await
    .context("Failed to upsert server icons")?;

    Ok(())
}

/// Update server and protocol version fields captured from peer info
pub async fn update_server_versions(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_version: Option<String>,
    protocol_version: String,
) -> Result<()> {
    let server_name = get_server_name(pool, server_id).await?;
    let meta_id = generate_id!("smet");

    sqlx::query(
        r#"
        INSERT INTO server_meta (id, server_id, server_name, server_version, protocol_version)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(server_id) DO UPDATE SET
            server_name = excluded.server_name,
            server_version = excluded.server_version,
            protocol_version = excluded.protocol_version,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(meta_id)
    .bind(server_id)
    .bind(server_name)
    .bind(server_version)
    .bind(protocol_version)
    .execute(pool)
    .await
    .context("Failed to upsert server versions")?;

    Ok(())
}
