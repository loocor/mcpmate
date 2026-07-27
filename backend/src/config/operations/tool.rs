// Tool operations for MCPMate
// Contains operations for tool configuration

use anyhow::Result;
use sqlx::{Pool, Sqlite};

/// Check if a tool is enabled in the database
pub async fn is_tool_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<bool> {
    // Get the server ID
    let server = crate::config::server::get_server(pool, server_name).await?;

    if let Some(server) = server {
        if let Some(server_id) = &server.id {
            // Get all active profile
            let active_profile = crate::config::profile::get_active_profile(pool).await?;

            // If there are no active profile, try to get the default profile
            if active_profile.is_empty() {
                let default_profile = crate::config::profile::get_default_profile(pool).await?;

                let default_profile = match default_profile {
                    Some(profile) => profile,
                    None => {
                        tracing::debug!("No active or default profile found, tool is enabled by default");
                        return Ok(true);
                    }
                };

                let profile_id = default_profile.id.unwrap();
                return is_tool_enabled_in_profile(pool, server_id, tool_name, &profile_id).await;
            }

            // Create a map to track tool enabled status with its priority
            // Higher priority value means higher precedence
            let mut tool_status: Option<(bool, i32)> = None;
            let mut server_status: Option<(bool, i32)> = None;

            // Process all active profile in priority order (already sorted by priority DESC)
            for profile in &active_profile {
                if let Some(profile_id) = &profile.id {
                    // Check server status in this profile
                    let server_configs = crate::config::profile::get_profile_servers(pool, profile_id).await?;

                    for server_config in server_configs {
                        if server_config.server_id == *server_id {
                            // Update server status if this is the first time we see it or if this profile has higher priority
                            if server_status.is_none() || server_status.as_ref().unwrap().1 < profile.priority {
                                server_status = Some((server_config.enabled, profile.priority));
                            }
                            break;
                        }
                    }

                    // Check tool status in this profile (using new architecture)
                    let tools = crate::config::profile::get_profile_tools(pool, profile_id).await?;

                    for tool_config in tools {
                        if tool_config.server_id == *server_id && tool_config.tool_name == tool_name {
                            // Update tool status if this is the first time we see it or if this profile has higher priority
                            if tool_status.is_none() || tool_status.as_ref().unwrap().1 < profile.priority {
                                tool_status = Some((tool_config.enabled, profile.priority));
                            }
                            break;
                        }
                    }
                }
            }

            // If server is disabled in any active profile, the tool is also disabled
            if let Some((server_enabled, _)) = server_status {
                if !server_enabled {
                    tracing::debug!(
                        "Server '{}' is disabled in an active profile, tool '{}' is also disabled",
                        server_name,
                        tool_name
                    );
                    return Ok(false);
                }
            }

            // If we found a specific tool configuration, use its enabled status
            if let Some((tool_enabled, _)) = tool_status {
                tracing::debug!(
                    "Tool '{}' from server '{}' is {} in an active profile",
                    tool_name,
                    server_name,
                    if tool_enabled { "enabled" } else { "disabled" }
                );
                return Ok(tool_enabled);
            }

            // If no specific tool configuration found but server is enabled, check if there are any tool configurations for this server
            if server_status.is_some() {
                // Check if there are any tool configurations for this server in any active profile
                let mut has_tool_configs = false;

                for profile in &active_profile {
                    if let Some(profile_id) = &profile.id {
                        let tools = crate::config::profile::get_profile_tools(pool, profile_id).await?;
                        if tools.iter().any(|t| t.server_id == *server_id) {
                            has_tool_configs = true;
                            break;
                        }
                    }
                }

                if !has_tool_configs {
                    // If there are no tool configurations for this server in any active profile,
                    // all tools are enabled by default (semi-blacklist mode)
                    tracing::debug!(
                        "No tool configurations for server '{}' in any active profile, tool '{}' is enabled by default (semi-blacklist mode)",
                        server_name,
                        tool_name
                    );
                } else {
                    // If there are tool configurations for this server but not for this specific tool,
                    // the tool is still enabled by default (semi-blacklist mode)
                    tracing::debug!(
                        "No specific configuration for tool '{}' from server '{}' in any active profile, enabled by default (semi-blacklist mode)",
                        tool_name,
                        server_name
                    );
                }

                return Ok(true);
            }
        }
    }

    // If no configuration found, the tool is enabled by default (semi-blacklist mode)
    tracing::debug!(
        "No configuration found for tool '{}' from server '{}', enabled by default (semi-blacklist mode)",
        tool_name,
        server_name
    );
    Ok(true)
}

/// Helper function to check if a tool is enabled in a specific profile
async fn is_tool_enabled_in_profile(
    pool: &Pool<Sqlite>,
    server_id: &str,
    tool_name: &str,
    profile_id: &str,
) -> Result<bool> {
    // Get server configuration in this profile
    let servers = crate::config::profile::get_profile_servers(pool, profile_id).await?;

    // Find the server in this profile
    for server_config in servers {
        if server_config.server_id == *server_id {
            // If the server is disabled in this profile, the tool is also disabled
            if !server_config.enabled {
                tracing::debug!(
                    "Server with ID '{}' is disabled in profile {}, tool '{}' is also disabled",
                    server_id,
                    profile_id,
                    tool_name
                );
                return Ok(false);
            }

            // Check if there's a specific tool configuration in this profile (using new architecture)
            let tools = crate::config::profile::get_profile_tools(pool, profile_id).await?;

            // Count tools for this server
            let server_tools = tools.iter().filter(|t| t.server_id == *server_id).count();

            // Find the tool in this profile
            let tool_config = tools
                .iter()
                .find(|t| t.server_id == *server_id && t.tool_name == tool_name);

            if let Some(config) = tool_config {
                // Return the tool's enabled status in this profile
                tracing::debug!(
                    "Tool '{}' from server ID '{}' is {} in profile {}",
                    tool_name,
                    server_id,
                    if config.enabled { "enabled" } else { "disabled" },
                    profile_id
                );
                return Ok(config.enabled);
            }

            if server_tools == 0 {
                // If there are no tool configurations for this server, all tools are enabled by default (semi-blacklist mode)
                tracing::debug!(
                    "No tool configurations for server ID '{}' in profile {}, tool '{}' is enabled by default (semi-blacklist mode)",
                    server_id,
                    profile_id,
                    tool_name
                );
                return Ok(true);
            } else {
                // If there are tool configurations for this server but not for this specific tool,
                // the tool is still enabled by default (semi-blacklist mode)
                tracing::debug!(
                    "No specific configuration for tool '{}' from server ID '{}' in profile {}, enabled by default (semi-blacklist mode)",
                    tool_name,
                    server_id,
                    profile_id
                );
                return Ok(true);
            }
        }
    }

    // If server not found in this profile, the tool is enabled by default
    tracing::debug!(
        "Server ID '{}' not found in profile {}, tool '{}' is enabled by default",
        server_id,
        profile_id,
        tool_name
    );
    Ok(true)
}
