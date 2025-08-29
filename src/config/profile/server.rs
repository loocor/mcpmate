// Server association operations for Profile
// Contains operations for managing server associations with profile

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    common::constants::database::{columns, tables},
    config::models::ProfileServer,
    generate_id,
};

/// Get all servers for a profile from the database
pub async fn get_profile_servers(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<ProfileServer>> {
    tracing::debug!("Executing SQL query to get servers for profile with ID {}", profile_id);

    let servers = sqlx::query_as::<_, ProfileServer>(&format!(
        r#"
        SELECT * FROM {}
        WHERE {} = ?
        ORDER BY {}
        "#,
        tables::PROFILE_SERVER,
        columns::PROFILE_ID,
        columns::SERVER_ID
    ))
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch profile servers")?;

    tracing::debug!(
        "Successfully fetched {} servers for profile with ID {}",
        servers.len(),
        profile_id
    );
    Ok(servers)
}

/// Add a server to a profile in the database
///
/// This function adds a server to a profile in the database.
/// If the server is added or updated, it also publishes a ServerEnabledInProfileChanged event.
pub async fn add_server_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding server ID {} to profile ID {}, enabled: {}",
        server_id,
        profile_id,
        enabled
    );

    // Generate an ID for the association
    let association_id = generate_id!("ssrv");

    // Get the server name
    let server_name = match sqlx::query_scalar::<_, String>(
        r#"
        SELECT name FROM server_config
        WHERE id = ?
        "#,
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get server name")?
    {
        Some(name) => name.replace(' ', "_"), // Replace spaces with underscores
        None => {
            tracing::warn!("Server ID {} not found, using 'unknown' as server_name", server_id);
            "unknown".to_string()
        }
    };

    // Check if the server already exists in the profile and get its current enabled status
    let existing_enabled = sqlx::query_scalar::<_, bool>(&format!(
        r#"
        SELECT {} FROM {}
        WHERE {} = ? AND {} = ?
        "#,
        columns::ENABLED,
        tables::PROFILE_SERVER,
        columns::PROFILE_ID,
        columns::SERVER_ID
    ))
    .bind(profile_id)
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get existing server enabled status")?;

    let result = sqlx::query(&format!(
        r#"
        INSERT INTO {} ({}, {}, {}, {}, {})
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT({}, {}) DO UPDATE SET
            {} = excluded.{},
            {} = excluded.{},
            {} = CURRENT_TIMESTAMP
        "#,
        tables::PROFILE_SERVER,
        columns::ID,
        columns::PROFILE_ID,
        columns::SERVER_ID,
        columns::SERVER_NAME,
        columns::ENABLED,
        columns::PROFILE_ID,
        columns::SERVER_ID,
        columns::SERVER_NAME,
        columns::SERVER_NAME,
        columns::ENABLED,
        columns::ENABLED,
        columns::UPDATED_AT
    ))
    .bind(&association_id)
    .bind(profile_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add server to profile")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        association_id.clone()
    } else {
        // If no rows were affected, get the existing ID
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM profile_server
            WHERE profile_id = ? AND server_id = ?
            "#,
        )
        .bind(profile_id)
        .bind(server_id)
        .fetch_one(pool)
        .await
        .context("Failed to get profile server association ID")?
    };

    // Publish event if the server is new or its enabled status has changed
    if is_new || (existing_enabled != Some(enabled)) {
        // Get the original server name (without underscore replacement)
        let original_server_name = sqlx::query_scalar::<_, String>(
            r#"
            SELECT name FROM server_config
            WHERE id = ?
            "#,
        )
        .bind(server_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get original server name")?
        .unwrap_or_else(|| "unknown".to_string());

        // Publish the event
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerEnabledInProfileChanged {
            server_id: server_id.to_string(),
            server_name: original_server_name,
            profile_id: profile_id.to_string(),
            enabled,
        });

        // tracing::info!(
        //     "Published ServerEnabledInProfileChanged event for server ID {} in profile ID {} ({})",
        //     server_id,
        //     profile_id,
        //     enabled
        // );
    }

    Ok(id_to_return)
}

/// Remove a server from a profile in the database
pub async fn remove_server_from_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
) -> Result<bool> {
    tracing::debug!("Removing server ID {} from profile ID {}", server_id, profile_id);

    let result = sqlx::query(&format!(
        r#"
        DELETE FROM {}
        WHERE {} = ? AND {} = ?
        "#,
        tables::PROFILE_SERVER,
        columns::PROFILE_ID,
        columns::SERVER_ID
    ))
    .bind(profile_id)
    .bind(server_id)
    .execute(pool)
    .await
    .context("Failed to remove server from profile")?;

    Ok(result.rows_affected() > 0)
}

/// TODO: Sync server capabilities to a profile
///
/// This function retrieves all capabilities (tools, prompts, resources) from a server
/// and creates corresponding records in the profile with enabled=false by default.
/// This ensures that capabilities are available for viewing even when the server is not enabled.
pub async fn sync_server_capabilities_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
) -> Result<()> {
    tracing::debug!(
        "Starting capability sync for server ID {} to profile ID {}",
        server_id,
        profile_id
    );

    // Check if capabilities already exist to avoid duplicate work
    let existing_tools_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM profile_tool cst
         JOIN server_tools st ON cst.server_tool_id = st.id
         WHERE cst.profile_id = ? AND st.server_id = ?",
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if existing_tools_count > 0 {
        tracing::debug!(
            "Server {} already has {} tools in profile {}. Skipping capability sync.",
            server_id,
            existing_tools_count,
            profile_id
        );
        return Ok(());
    }

    Ok(())
}
