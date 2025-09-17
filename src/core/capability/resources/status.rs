//! Resource status checking functionality
//!
//! Contains functions for checking if resources are enabled in profile

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

/// Check if a resource is enabled in any active profile
///
/// This function checks if a resource is enabled by looking at all active profile.
/// A resource is considered enabled if it's enabled in at least one active profile.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_name` - Name of the server providing the resource
/// * `resource_uri` - URI of the resource to check
///
/// # Returns
/// * `Ok(true)` if the resource is enabled in at least one active profile
/// * `Ok(false)` if the resource is not enabled in any active profile
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

    // Get all active profile
    let active_profile = crate::config::profile::get_active_profile(pool)
        .await
        .context("Failed to get active profile")?;

    if active_profile.is_empty() {
        tracing::debug!("No active profile found, resource is disabled");
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

    // Check each active profile
    for profile in active_profile {
        if let Some(profile_id) = &profile.id {
            // Get enabled resources for this profile
            let enabled_resources = crate::config::profile::get_enabled_resources_for_profile(pool, profile_id)
                .await
                .context(format!("Failed to get enabled resources for profile '{profile_id}'"))?;

            // Check if our resource is in the enabled list
            for resource in enabled_resources {
                if resource.server_id == server_id && resource.resource_uri == resource_uri {
                    tracing::debug!(
                        "Resource '{}' from server '{}' is enabled in profile '{}'",
                        resource_uri,
                        server_name,
                        profile.name
                    );
                    return Ok(true);
                }
            }
        }
    }

    tracing::debug!(
        "Resource '{}' from server '{}' is not enabled in any active profile",
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

    // Get the resource ID from any active profile
    let active_profile = crate::config::profile::get_active_profile(pool)
        .await
        .context("Failed to get active profile")?;

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

    // Look for the resource in any active profile
    for profile in active_profile {
        if let Some(profile_id) = &profile.id {
            let resources = crate::config::profile::get_resources_for_profile(pool, profile_id)
                .await
                .context(format!("Failed to get resources for profile '{profile_id}'"))?;

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
        "Resource '{}' from server '{}' not found in any active profile",
        resource_uri,
        server_name
    ))
}
