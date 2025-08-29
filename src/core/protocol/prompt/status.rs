//! Prompt status checking functionality
//!
//! Contains functions for checking if prompts are enabled in profile

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

/// Check if a prompt is enabled in any active profile
pub async fn is_prompt_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    prompt_name: &str,
) -> Result<bool> {
    tracing::debug!(
        "Checking if prompt '{}' from server '{}' is enabled",
        prompt_name,
        server_name
    );

    // Get all active profile
    let active_profile = crate::config::profile::get_active_profile(pool)
        .await
        .context("Failed to get active profile")?;

    if active_profile.is_empty() {
        tracing::debug!("No active profile found, prompt is disabled");
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
                tracing::warn!("Server '{}' has no ID, prompt is disabled", server_name);
                return Ok(false);
            }
        },
        None => {
            tracing::debug!("Server '{}' not found, prompt is disabled", server_name);
            return Ok(false);
        }
    };

    // Check each active profile
    for profile in active_profile {
        if let Some(profile_id) = &profile.id {
            // Get enabled prompts for this profile
            let enabled_prompts = crate::config::profile::get_enabled_prompts_for_profile(pool, profile_id)
                .await
                .context(format!("Failed to get enabled prompts for profile '{profile_id}'"))?;

            // Check if our prompt is in the enabled list
            for prompt in enabled_prompts {
                if prompt.server_id == server_id && prompt.prompt_name == prompt_name {
                    tracing::debug!(
                        "Prompt '{}' from server '{}' is enabled in profile '{}'",
                        prompt_name,
                        server_name,
                        profile.name
                    );
                    return Ok(true);
                }
            }
        }
    }

    tracing::debug!(
        "Prompt '{}' from server '{}' is not enabled in any active profile",
        prompt_name,
        server_name
    );
    Ok(false)
}

/// Get prompt status (ID, enabled status) for a specific prompt
pub async fn get_prompt_status(
    pool: &Pool<Sqlite>,
    server_name: &str,
    prompt_name: &str,
) -> Result<(String, bool)> {
    tracing::debug!(
        "Getting prompt status for '{}' from server '{}'",
        prompt_name,
        server_name
    );

    // Check if the prompt is enabled
    let enabled = is_prompt_enabled(pool, server_name, prompt_name).await?;

    // Get the prompt ID from any active profile
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

    // Look for the prompt in any active profile
    for profile in active_profile {
        if let Some(profile_id) = &profile.id {
            let prompts = crate::config::profile::get_prompts_for_profile(pool, profile_id)
                .await
                .context(format!("Failed to get prompts for profile '{profile_id}'"))?;

            for prompt in prompts {
                if prompt.server_id == server_id && prompt.prompt_name == prompt_name {
                    if let Some(prompt_id) = prompt.id {
                        return Ok((prompt_id, enabled));
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Prompt '{}' from server '{}' not found in any active profile",
        prompt_name,
        server_name
    ))
}
