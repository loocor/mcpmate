// Tool operations for MCPMate
// Contains operations for tool configuration

use anyhow::Result;
use sqlx::{Pool, Sqlite};

/// Enable or disable a tool in the default config suit
pub async fn set_tool_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
    enabled: bool,
) -> Result<String> {
    // Get the server ID
    let server = crate::conf::operations::get_server(pool, server_name).await?;

    if let Some(server) = server {
        if let Some(server_id) = &server.id {
            // Get the default config suit
            let default_suit =
                crate::conf::operations::get_config_suit_by_name(pool, "default").await?;

            let suit_id = if let Some(suit) = default_suit {
                suit.id.unwrap()
            } else {
                // Create default config suit if it doesn't exist
                let new_suit = crate::conf::models::ConfigSuit::new(
                    "default".to_string(),
                    crate::conf::models::ConfigSuitType::Shared,
                );
                crate::conf::operations::upsert_config_suit(pool, &new_suit).await?
            };

            // Add the tool to the config suit
            let tool_id = crate::conf::operations::suit::add_tool_to_config_suit(
                pool, &suit_id, server_id, tool_name, enabled,
            )
            .await?;

            return Ok(tool_id);
        }
    }

    // If server not found, return error
    Err(anyhow::anyhow!("Server '{}' not found", server_name))
}

/// Check if a tool is enabled in the database
pub async fn is_tool_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<bool> {
    // Get the server ID
    let server = crate::conf::operations::get_server(pool, server_name).await?;

    if let Some(server) = server {
        if let Some(server_id) = &server.id {
            // Get the default config suit
            let default_suit =
                crate::conf::operations::get_config_suit_by_name(pool, "default").await?;

            // If there's no default suit, the tool is enabled by default
            if default_suit.is_none() {
                tracing::debug!("No default config suit found, tool is enabled by default");
                return Ok(true);
            }

            let suit_id = default_suit.unwrap().id.unwrap();

            // Get server configuration in this suit
            let servers = crate::conf::operations::get_config_suit_servers(pool, &suit_id).await?;

            // Find the server in this suit
            for server_config in servers {
                if &server_config.server_id == server_id {
                    // If the server is disabled in this suit, the tool is also disabled
                    if !server_config.enabled {
                        tracing::debug!(
                            "Server '{}' is disabled in default suit, tool '{}' is also disabled",
                            server_name,
                            tool_name
                        );
                        return Ok(false);
                    }

                    // Check if there's a specific tool configuration in this suit
                    let tools =
                        crate::conf::operations::get_config_suit_tools(pool, &suit_id).await?;

                    // Find the tool in this suit
                    for tool_config in tools {
                        if &tool_config.server_id == server_id && tool_config.tool_name == tool_name
                        {
                            // Return the tool's enabled status in this suit
                            tracing::debug!(
                                "Tool '{}' from server '{}' is {} in default suit",
                                tool_name,
                                server_name,
                                if tool_config.enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                }
                            );
                            return Ok(tool_config.enabled);
                        }
                    }

                    // If no specific tool configuration found, the tool is enabled by default
                    tracing::debug!(
                        "No specific configuration for tool '{}' from server '{}' in default suit, enabled by default",
                        tool_name,
                        server_name
                    );
                    return Ok(true);
                }
            }
        }
    }

    // If no configuration found, the tool is enabled by default
    tracing::debug!(
        "No configuration found for tool '{}' from server '{}', enabled by default",
        tool_name,
        server_name
    );
    Ok(true)
}
