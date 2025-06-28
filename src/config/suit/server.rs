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
            tracing::warn!(
                "Server ID {} not found, using 'unknown' as server_name",
                server_id
            );
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
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ServerEnabledInSuitChanged {
                server_id: server_id.to_string(),
                server_name: original_server_name,
                suit_id: config_suit_id.to_string(),
                enabled,
            },
        );

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
    inspect_service: &crate::inspect::InspectService,
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
         WHERE cst.config_suit_id = ? AND st.server_id = ?"
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

    // Get server capabilities using inspect service with shorter timeout
    tracing::debug!("Fetching capabilities for server {}", server_id);
    let capabilities_result = tokio::time::timeout(
        std::time::Duration::from_secs(5), // Reduced to 5 second timeout
        inspect_service.get_server_capabilities(server_id, crate::inspect::RefreshStrategy::CacheFirst)
    ).await;

    let capabilities = match capabilities_result {
        Ok(Ok(caps)) => {
            tracing::debug!(
                "Successfully fetched capabilities for server {}: {} tools, {} prompts, {} resources",
                server_id,
                caps.tools.len(),
                caps.prompts.len(),
                caps.resources.len()
            );
            caps
        }
        Ok(Err(e)) => {
            tracing::warn!(
                "Failed to get capabilities for server {}: {}. Skipping capability sync.",
                server_id,
                e
            );
            return Ok(()); // Don't fail the server addition if capabilities can't be retrieved
        }
        Err(_) => {
            tracing::warn!(
                "Timeout (5s) getting capabilities for server {}. Skipping capability sync.",
                server_id
            );
            return Ok(()); // Don't fail the server addition if capabilities timeout
        }
    };

    let mut synced_tools = 0;
    let mut synced_prompts = 0;
    let mut synced_resources = 0;

    // Sync tools
    for tool in &capabilities.tools {
        match crate::config::suit::add_tool_to_config_suit(
            pool,
            config_suit_id,
            server_id,
            &tool.name,
            true, // Default to enabled for better user experience
        )
        .await
        {
            Ok(_) => synced_tools += 1,
            Err(e) => {
                tracing::warn!(
                    "Failed to sync tool '{}' for server {} to suit {}: {}",
                    tool.name,
                    server_id,
                    config_suit_id,
                    e
                );
            }
        }
    }

    // Sync prompts
    for prompt in &capabilities.prompts {
        match crate::config::suit::add_prompt_to_config_suit(
            pool,
            config_suit_id,
            server_id,
            &prompt.name,
            true, // Default to enabled for better user experience
        )
        .await
        {
            Ok(_) => synced_prompts += 1,
            Err(e) => {
                tracing::warn!(
                    "Failed to sync prompt '{}' for server {} to suit {}: {}",
                    prompt.name,
                    server_id,
                    config_suit_id,
                    e
                );
            }
        }
    }

    // Sync resources
    for resource in &capabilities.resources {
        match crate::config::suit::add_resource_to_config_suit(
            pool,
            config_suit_id,
            server_id,
            &resource.uri,
            true, // Default to enabled for better user experience
        )
        .await
        {
            Ok(_) => synced_resources += 1,
            Err(e) => {
                tracing::warn!(
                    "Failed to sync resource '{}' for server {} to suit {}: {}",
                    resource.uri,
                    server_id,
                    config_suit_id,
                    e
                );
            }
        }
    }

    tracing::info!(
        "Synced capabilities for server {} to suit {}: {} tools, {} prompts, {} resources",
        server_id,
        config_suit_id,
        synced_tools,
        synced_prompts,
        synced_resources
    );

    Ok(())
}
