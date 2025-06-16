//! Resource status checking functionality
//!
//! Contains functions for checking if resources are enabled in configuration suits

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

/// Check if a resource is enabled in any active configuration suit
///
/// This function checks if a resource is enabled by looking at all active configuration suits.
/// A resource is considered enabled if it's enabled in at least one active configuration suit.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_name` - Name of the server providing the resource
/// * `resource_uri` - URI of the resource to check
///
/// # Returns
/// * `Ok(true)` if the resource is enabled in at least one active configuration suit
/// * `Ok(false)` if the resource is not enabled in any active configuration suit
/// * `Err(_)` if there was a database error
pub async fn is_resource_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    resource_uri: &str,
) -> Result<bool> {
    tracing::debug!(
        "Checking if resource '{}' from server '{}' is enabled",
        resource_uri,
        server_name
    );

    // Get all active configuration suits
    let active_suits = crate::config::suit::get_active_config_suits(pool)
        .await
        .context("Failed to get active configuration suits")?;

    if active_suits.is_empty() {
        tracing::debug!("No active configuration suits found, resource is disabled");
        return Ok(false);
    }

    // Get the server ID
    let server = crate::config::server::get_server(pool, server_name)
        .await
        .context(format!("Failed to get server '{server_name}'"))?;

    let server_id = match server {
        Some(server) => match server.id {
            Some(id) => id,
            None => {
                tracing::warn!("Server '{}' has no ID, resource is disabled", server_name);
                return Ok(false);
            }
        },
        None => {
            tracing::debug!("Server '{}' not found, resource is disabled", server_name);
            return Ok(false);
        }
    };

    // Check each active configuration suit
    for suit in active_suits {
        if let Some(suit_id) = &suit.id {
            // Get enabled resources for this configuration suit
            let enabled_resources =
                crate::config::suit::get_enabled_resources_for_config_suit(pool, suit_id)
                    .await
                    .context(format!(
                        "Failed to get enabled resources for suit '{suit_id}'"
                    ))?;

            // Check if our resource is in the enabled list
            for resource in enabled_resources {
                if resource.server_id == server_id && resource.resource_uri == resource_uri {
                    tracing::debug!(
                        "Resource '{}' from server '{}' is enabled in configuration suit '{}'",
                        resource_uri,
                        server_name,
                        suit.name
                    );
                    return Ok(true);
                }
            }
        }
    }

    tracing::debug!(
        "Resource '{}' from server '{}' is not enabled in any active configuration suit",
        resource_uri,
        server_name
    );
    Ok(false)
}

/// Get resource status (ID, enabled status) for a specific resource
///
/// This function returns the resource ID and enabled status for a resource.
/// It's used by API handlers to get resource information.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_name` - Name of the server providing the resource
/// * `resource_uri` - URI of the resource to check
///
/// # Returns
/// * `Ok((resource_id, enabled))` if the resource is found
/// * `Err(_)` if the resource is not found or there was a database error
pub async fn get_resource_status(
    pool: &Pool<Sqlite>,
    server_name: &str,
    resource_uri: &str,
) -> Result<(String, bool)> {
    tracing::debug!(
        "Getting resource status for '{}' from server '{}'",
        resource_uri,
        server_name
    );

    // Check if the resource is enabled
    let enabled = is_resource_enabled(pool, server_name, resource_uri).await?;

    // Get the resource ID from any active configuration suit
    let active_suits = crate::config::suit::get_active_config_suits(pool)
        .await
        .context("Failed to get active configuration suits")?;

    // Get the server ID
    let server = crate::config::server::get_server(pool, server_name)
        .await
        .context(format!("Failed to get server '{server_name}'"))?;

    let server_id = match server {
        Some(server) => match server.id {
            Some(id) => id,
            None => {
                return Err(anyhow::anyhow!("Server '{}' has no ID", server_name));
            }
        },
        None => {
            return Err(anyhow::anyhow!("Server '{}' not found", server_name));
        }
    };

    // Look for the resource in any active configuration suit
    for suit in active_suits {
        if let Some(suit_id) = &suit.id {
            let resources = crate::config::suit::get_resources_for_config_suit(pool, suit_id)
                .await
                .context(format!("Failed to get resources for suit '{suit_id}'"))?;

            for resource in resources {
                if resource.server_id == server_id && resource.resource_uri == resource_uri {
                    if let Some(resource_id) = resource.id {
                        return Ok((resource_id, enabled));
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Resource '{}' from server '{}' not found in any active configuration suit",
        resource_uri,
        server_name
    ))
}
