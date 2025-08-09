// Server association operations for Config Suits
// Contains operations for managing server associations with configuration suits

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{config::models::ConfigSuitServer, generate_id};

/// Get all servers for a configuration suit from the database
pub async fn get_config_suit_servers(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
) -> Result<Vec<ConfigSuitServer>> {
    tracing::debug!(
        "Executing SQL query to get servers for configuration suit with ID {}",
        config_suit_id
    );

    let servers = sqlx::query_as::<_, ConfigSuitServer>(
        r#"
        SELECT * FROM config_suit_server
        WHERE config_suit_id = ?
        ORDER BY server_id
        "#,
    )
    .bind(config_suit_id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch configuration suit servers")?;

    tracing::debug!(
        "Successfully fetched {} servers for configuration suit with ID {}",
        servers.len(),
        config_suit_id
    );
    Ok(servers)
}

/// Add a server to a configuration suit in the database
///
/// This function adds a server to a configuration suit in the database.
/// If the server is added or updated, it also publishes a ServerEnabledInSuitChanged event.
pub async fn add_server_to_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
    server_id: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding server ID {} to configuration suit ID {}, enabled: {}",
        server_id,
        config_suit_id,
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

    // Check if the server already exists in the suit and get its current enabled status
    let existing_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM config_suit_server
        WHERE config_suit_id = ? AND server_id = ?
        "#,
    )
    .bind(config_suit_id)
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get existing server enabled status")?;

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit_server (id, config_suit_id, server_id, server_name, enabled)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(config_suit_id, server_id) DO UPDATE SET
            server_name = excluded.server_name,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&association_id)
    .bind(config_suit_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add server to configuration suit")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        association_id.clone()
    } else {
        // If no rows were affected, get the existing ID
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM config_suit_server
            WHERE config_suit_id = ? AND server_id = ?
            "#,
        )
        .bind(config_suit_id)
        .bind(server_id)
        .fetch_one(pool)
        .await
        .context("Failed to get configuration suit server association ID")?
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
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerEnabledInSuitChanged {
            server_id: server_id.to_string(),
            server_name: original_server_name,
            suit_id: config_suit_id.to_string(),
            enabled,
        });

        // tracing::info!(
        //     "Published ServerEnabledInSuitChanged event for server ID {} in suit ID {} ({})",
        //     server_id,
        //     config_suit_id,
        //     enabled
        // );
    }

    Ok(id_to_return)
}

/// Remove a server from a configuration suit in the database
pub async fn remove_server_from_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
    server_id: &str,
) -> Result<bool> {
    tracing::debug!(
        "Removing server ID {} from configuration suit ID {}",
        server_id,
        config_suit_id
    );

    let result = sqlx::query(
        r#"
        DELETE FROM config_suit_server
        WHERE config_suit_id = ? AND server_id = ?
        "#,
    )
    .bind(config_suit_id)
    .bind(server_id)
    .execute(pool)
    .await
    .context("Failed to remove server from configuration suit")?;

    Ok(result.rows_affected() > 0)
}

/// Sync server capabilities to a configuration suit
///
/// This function retrieves all capabilities (tools, prompts, resources) from a server
/// and creates corresponding records in the configuration suit with enabled=false by default.
/// This ensures that capabilities are available for viewing even when the server is not enabled.
pub async fn sync_server_capabilities_to_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
    server_id: &str,
) -> Result<()> {
    tracing::debug!(
        "Starting capability sync for server ID {} to configuration suit ID {}",
        server_id,
        config_suit_id
    );

    // Check if capabilities already exist to avoid duplicate work
    let existing_tools_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM config_suit_tool cst
         JOIN server_tools st ON cst.server_tool_id = st.id
         WHERE cst.config_suit_id = ? AND st.server_id = ?",
    )
    .bind(config_suit_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if existing_tools_count > 0 {
        tracing::debug!(
            "Server {} already has {} tools in suit {}. Skipping capability sync.",
            server_id,
            existing_tools_count,
            config_suit_id
        );
        return Ok(());
    }

    // Inspect module removed; skip fetching capabilities (no-op for now)
    let _capabilities = None::<()>;

    // No-op: syncing removed with inspect module

    tracing::info!(
        "Capabilities sync skipped (inspect removed) for server {} to suit {}",
        server_id,
        config_suit_id
    );

    Ok(())
}
