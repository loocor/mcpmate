// Server enabled status management
// Contains complex logic for determining which servers are enabled based on config suits

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use super::crud::get_server_by_id;
use crate::config::models::Server;

/// Get all enabled servers from the database based on config suits
pub async fn get_enabled_servers(pool: &Pool<Sqlite>) -> Result<Vec<Server>> {
    tracing::debug!("Getting all enabled servers from config suits");

    // Get all servers first
    let all_servers = super::crud::get_all_servers(pool).await?;

    // If there are no servers, return empty list
    if all_servers.is_empty() {
        return Ok(Vec::new());
    }

    // Get all active config suits
    let active_suits = crate::config::suit::get_active_config_suits(pool).await?;

    // If there are no active suits, try to get the default suit
    if active_suits.is_empty() {
        let default_suit = crate::config::suit::get_default_config_suit(pool).await?;

        // If there's no default suit, try the legacy "default" named suit
        if default_suit.is_none() {
            let legacy_default =
                crate::config::suit::get_config_suit_by_name(pool, "default").await?;

            // If there's no legacy default suit either, return no servers (whitelist mode)
            if legacy_default.is_none() {
                tracing::info!(
                    "No active or default config suits found, returning no servers (whitelist mode)"
                );
                return Ok(Vec::new());
            }

            // Use the legacy default suit
            let suit_id = legacy_default.unwrap().id.unwrap();
            return get_enabled_servers_from_suit(pool, &suit_id, &all_servers).await;
        }

        // Use the default suit
        let suit_id = default_suit.unwrap().id.unwrap();
        return get_enabled_servers_from_suit(pool, &suit_id, &all_servers).await;
    }

    // Create a map to track enabled server IDs with their priority
    // Higher priority value means higher precedence
    let mut enabled_server_map: HashMap<String, (bool, i32)> = HashMap::new();

    // Process all active suits in priority order (already sorted by priority DESC)
    for suit in active_suits {
        if let Some(suit_id) = &suit.id {
            // Get all server configs in this suit
            let server_configs = crate::config::suit::get_config_suit_servers(pool, suit_id).await?;

            // Process each server config
            for server_config in server_configs {
                // Only update the map if this server hasn't been seen yet or if the current suit has higher priority
                if !enabled_server_map.contains_key(&server_config.server_id)
                    || enabled_server_map.get(&server_config.server_id).unwrap().1 < suit.priority
                {
                    enabled_server_map.insert(
                        server_config.server_id.clone(),
                        (server_config.enabled, suit.priority),
                    );
                }
            }
        }
    }

    // If no server configurations were found in any active suits, return no servers (whitelist mode)
    if enabled_server_map.is_empty() {
        tracing::info!(
            "No server configurations in any active suits, returning no servers (whitelist mode)"
        );
        return Ok(Vec::new());
    }

    // Filter servers by enabled status
    let enabled_servers: Vec<Server> = all_servers
        .into_iter()
        .filter(|server| {
            if let Some(id) = &server.id {
                // Check both the suit-level enabled status AND the global enabled status
                enabled_server_map
                    .get(id)
                    .is_some_and(|(enabled, _)| *enabled)
                    && server.enabled.as_bool() // Add this check for global enabled status
            } else {
                false // Server without ID is not enabled
            }
        })
        .collect();

    tracing::info!(
        "Found {} enabled servers across all active config suits",
        enabled_servers.len()
    );

    Ok(enabled_servers)
}

/// Helper function to get enabled servers from a specific suit
async fn get_enabled_servers_from_suit(
    pool: &Pool<Sqlite>,
    suit_id: &str,
    all_servers: &[Server],
) -> Result<Vec<Server>> {
    // Get all enabled servers in the suit
    let server_configs = crate::config::suit::get_config_suit_servers(pool, suit_id).await?;

    // If there are no server configs in the suit, return no servers (whitelist mode)
    if server_configs.is_empty() {
        tracing::info!(
            "No server configurations in suit {}, returning no servers (whitelist mode)",
            suit_id
        );
        return Ok(Vec::new());
    }

    // Create a set of enabled server IDs
    let mut enabled_server_ids = HashSet::new();
    for server_config in server_configs {
        if server_config.enabled {
            enabled_server_ids.insert(server_config.server_id);
        }
    }

    // Filter servers by enabled status
    let enabled_servers: Vec<Server> = all_servers
        .iter()
        .filter(|server| {
            if let Some(id) = &server.id {
                // Check both the suit-level enabled status AND the global enabled status
                enabled_server_ids.contains(id) && server.enabled.as_bool()
            } else {
                false // Server without ID is not enabled
            }
        })
        .cloned()
        .collect();

    tracing::info!(
        "Found {} enabled servers in suit {}",
        enabled_servers.len(),
        suit_id
    );

    Ok(enabled_servers)
}

/// Check if a server is enabled in any active config suit
///
/// This function checks if a server is enabled in any active config suit.
/// Returns true if the server is enabled in at least one active suit, false otherwise.
pub async fn is_server_enabled_in_any_suit(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<bool> {
    // Get all active config suits
    let active_suits = crate::config::suit::get_active_config_suits(pool).await?;

    // If there are no active suits, try to get the default suit
    if active_suits.is_empty() {
        let default_suit = crate::config::suit::get_default_config_suit(pool).await?;

        // If there's no default suit, try the legacy "default" named suit
        if default_suit.is_none() {
            let legacy_default =
                crate::config::suit::get_config_suit_by_name(pool, "default").await?;

            // If there's no legacy default suit either, return false (not enabled)
            if legacy_default.is_none() {
                tracing::debug!("No active or default config suits found, server is not enabled");
                return Ok(false);
            }

            // Use the legacy default suit
            let suit_id = legacy_default.unwrap().id.unwrap();
            return is_server_enabled_in_suit(pool, server_id, &suit_id).await;
        }

        // Use the default suit
        let suit_id = default_suit.unwrap().id.unwrap();
        return is_server_enabled_in_suit(pool, server_id, &suit_id).await;
    }

    // Check each active suit
    for suit in active_suits {
        if let Some(suit_id) = &suit.id {
            if is_server_enabled_in_suit(pool, server_id, suit_id).await? {
                return Ok(true);
            }
        }
    }

    // Server is not enabled in any active suit
    Ok(false)
}

/// Check if a server is enabled in a specific config suit
///
/// This function checks if a server is enabled in a specific config suit.
/// Returns true if the server is enabled in the suit, false otherwise.
async fn is_server_enabled_in_suit(
    pool: &Pool<Sqlite>,
    server_id: &str,
    suit_id: &str,
) -> Result<bool> {
    // Get all server configs in this suit
    let server_configs = crate::config::suit::get_config_suit_servers(pool, suit_id).await?;

    // Check if the server is enabled in this suit
    for server_config in server_configs {
        if server_config.server_id == server_id {
            // We found the server in this suit, now we need to check the global status
            let server = get_server_by_id(pool, server_id).await?;
            if let Some(server) = server {
                // Return true only if both the suit-level and global status are enabled
                return Ok(server_config.enabled && server.enabled.as_bool());
            }
            // If we couldn't find the server (shouldn't happen), just return the suit status
            return Ok(server_config.enabled);
        }
    }

    // Server is not in this suit, so it's not enabled
    Ok(false)
}

/// Check if a server is in a specific config suit
///
/// This function checks if a server is in a specific config suit, regardless of enabled status.
/// Returns true if the server is in the suit, false otherwise.
pub async fn is_server_in_suit(
    pool: &Pool<Sqlite>,
    server_id: &str,
    suit_id: &str,
) -> Result<bool> {
    // Get all server configs in this suit
    let server_configs = crate::config::suit::get_config_suit_servers(pool, suit_id).await?;

    // Check if the server is in this suit
    for server_config in server_configs {
        if server_config.server_id == server_id {
            return Ok(true);
        }
    }

    // Server is not in this suit
    Ok(false)
}

/// Update a server's global enabled status
///
/// This function updates the global enabled status of a server in the database.
/// Returns true if the server was updated, false if the server was not found.
/// If the status is updated, it also publishes a ServerGlobalStatusChanged event.
pub async fn update_server_global_status(
    pool: &Pool<Sqlite>,
    server_id: &str,
    enabled: bool,
) -> Result<bool> {
    tracing::debug!(
        "Updating global enabled status for server ID {} to {}",
        server_id,
        enabled
    );

    let result = sqlx::query(
        r#"
        UPDATE server_config
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(enabled)
    .bind(server_id)
    .execute(pool)
    .await
    .context("Failed to update server global status")?;

    let updated = result.rows_affected() > 0;

    // If the server was updated, publish an event
    if updated {
        // Get the server name
        if let Ok(Some(server)) = get_server_by_id(pool, server_id).await {
            // Publish the event
            crate::core::events::EventBus::global().publish(
                crate::core::events::Event::ServerGlobalStatusChanged {
                    server_id: server_id.to_string(),
                    server_name: server.name,
                    enabled,
                },
            );

            tracing::info!(
                "Published ServerGlobalStatusChanged event for server ID {} ({})",
                server_id,
                enabled
            );
        }
    }

    Ok(updated)
}

/// Get a server's global enabled status
///
/// This function retrieves the global enabled status of a server from the database.
/// Returns Some(bool) if the server was found, None if the server was not found.
pub async fn get_server_global_status(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<Option<bool>> {
    tracing::debug!("Getting global enabled status for server ID {}", server_id);

    let enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM server_config
        WHERE id = ?
        "#,
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get server global status")?;

    Ok(enabled)
}
