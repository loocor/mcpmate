// Tool operations for MCPMate
// Contains operations for tool configuration

use anyhow::Result;
use sqlx::{Pool, Sqlite};

use crate::conf::models::ConfigSuitTool;

/// Enable or disable a tool in the active config suits
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
            // Get all active config suits
            let active_suits = crate::conf::operations::suit::get_active_config_suits(pool).await?;

            // If there are no active suits, try to get the default suit
            if active_suits.is_empty() {
                let default_suit =
                    crate::conf::operations::suit::get_default_config_suit(pool).await?;

                // If there's no default suit, try the legacy "default" named suit
                let suit_id = if let Some(suit) = default_suit {
                    suit.id.unwrap()
                } else {
                    let legacy_default =
                        crate::conf::operations::get_config_suit_by_name(pool, "default").await?;

                    // If there's no legacy default suit either, create a new default suit
                    if let Some(suit) = legacy_default {
                        suit.id.unwrap()
                    } else {
                        // Create default config suit if it doesn't exist
                        let mut new_suit = crate::conf::models::ConfigSuit::new_with_description(
                            "default".to_string(),
                            Some("Default configuration suit".to_string()),
                            crate::common::types::ConfigSuitType::Shared,
                        );

                        // Set active and default flags
                        new_suit.is_active = true;
                        new_suit.is_default = true;
                        new_suit.multi_select = true;
                        crate::conf::operations::upsert_config_suit(pool, &new_suit).await?
                    }
                };

                // Add the tool to the config suit
                let tool_id = crate::conf::operations::suit::add_tool_to_config_suit(
                    pool, &suit_id, server_id, tool_name, enabled,
                )
                .await?;

                return Ok(tool_id);
            }

            // If there are active suits, update the tool in all of them
            let mut last_tool_id = String::new();

            for suit in active_suits {
                if let Some(suit_id) = &suit.id {
                    // Add the tool to the config suit
                    let tool_id = crate::conf::operations::suit::add_tool_to_config_suit(
                        pool, suit_id, server_id, tool_name, enabled,
                    )
                    .await?;

                    // Save the last tool ID (from the highest priority suit)
                    if last_tool_id.is_empty() {
                        last_tool_id = tool_id;
                    }
                }
            }

            return Ok(last_tool_id);
        }
    }

    // If server not found, return error
    Err(anyhow::anyhow!("Server '{}' not found", server_name))
}

/// Get the ID of a tool in the database
pub async fn get_tool_id(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<Option<String>> {
    // Get the server ID
    let server = crate::conf::operations::get_server(pool, server_name).await?;

    if let Some(server) = server {
        if let Some(server_id) = &server.id {
            // Get the default config suit
            let default_suit = crate::conf::operations::suit::get_default_config_suit(pool).await?;

            // If there's no default suit, try the legacy "default" named suit
            let suit_id = if let Some(suit) = default_suit {
                suit.id.unwrap()
            } else {
                let legacy_default =
                    crate::conf::operations::get_config_suit_by_name(pool, "default").await?;

                // If there's no legacy default suit either, return None
                if legacy_default.is_none() {
                    return Ok(None);
                }

                legacy_default.unwrap().id.unwrap()
            };

            // Get all tools in this suit
            let tools = crate::conf::operations::get_config_suit_tools(pool, &suit_id).await?;

            // Find the tool in this suit
            for tool_config in tools {
                if tool_config.server_id == *server_id && tool_config.tool_name == tool_name {
                    // Return the tool's ID
                    return Ok(tool_config.id);
                }
            }
        }
    }

    // If no tool found, return None
    Ok(None)
}

/// Get all tools in a configuration suit by ID
pub async fn get_tools_by_suit_id(
    pool: &Pool<Sqlite>,
    suit_id: &str,
) -> Result<Vec<ConfigSuitTool>> {
    // Get all tools in the suit
    let tools = sqlx::query_as::<_, ConfigSuitTool>(
        r#"
        SELECT * FROM config_suit_tool
        WHERE config_suit_id = ?
        "#,
    )
    .bind(suit_id)
    .fetch_all(pool)
    .await?;

    Ok(tools)
}

/// Get a specific tool in a configuration suit by ID
pub async fn get_config_suit_tool_by_id(
    pool: &Pool<Sqlite>,
    tool_id: &str,
) -> Result<Option<ConfigSuitTool>> {
    // Get the tool
    let tool = sqlx::query_as::<_, ConfigSuitTool>(
        r#"
        SELECT * FROM config_suit_tool
        WHERE id = ?
        "#,
    )
    .bind(tool_id)
    .fetch_optional(pool)
    .await?;

    Ok(tool)
}

/// Enable or disable a tool in a configuration suit by ID
pub async fn set_tool_enabled_by_id(
    pool: &Pool<Sqlite>,
    tool_id: &str,
    enabled: bool,
) -> Result<()> {
    // Update the tool
    sqlx::query(
        r#"
        UPDATE config_suit_tool
        SET enabled = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(enabled)
    .bind(tool_id)
    .execute(pool)
    .await?;

    Ok(())
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
            // Get all active config suits
            let active_suits = crate::conf::operations::suit::get_active_config_suits(pool).await?;

            // If there are no active suits, try to get the default suit
            if active_suits.is_empty() {
                let default_suit =
                    crate::conf::operations::suit::get_default_config_suit(pool).await?;

                // If there's no default suit, try the legacy "default" named suit
                if default_suit.is_none() {
                    let legacy_default =
                        crate::conf::operations::get_config_suit_by_name(pool, "default").await?;

                    // If there's no legacy default suit either, the tool is enabled by default
                    if legacy_default.is_none() {
                        tracing::debug!(
                            "No active or default config suits found, tool is enabled by default"
                        );
                        return Ok(true);
                    }

                    // Use the legacy default suit
                    let suit_id = legacy_default.unwrap().id.unwrap();
                    return is_tool_enabled_in_suit(pool, server_id, tool_name, &suit_id).await;
                }

                // Use the default suit
                let suit_id = default_suit.unwrap().id.unwrap();
                return is_tool_enabled_in_suit(pool, server_id, tool_name, &suit_id).await;
            }

            // Create a map to track tool enabled status with its priority
            // Higher priority value means higher precedence
            let mut tool_status: Option<(bool, i32)> = None;
            let mut server_status: Option<(bool, i32)> = None;

            // Process all active suits in priority order (already sorted by priority DESC)
            for suit in &active_suits {
                if let Some(suit_id) = &suit.id {
                    // Check server status in this suit
                    let server_configs =
                        crate::conf::operations::get_config_suit_servers(pool, suit_id).await?;

                    for server_config in server_configs {
                        if server_config.server_id == *server_id {
                            // Update server status if this is the first time we see it or if this suit has higher priority
                            if server_status.is_none()
                                || server_status.as_ref().unwrap().1 < suit.priority
                            {
                                server_status = Some((server_config.enabled, suit.priority));
                            }
                            break;
                        }
                    }

                    // Check tool status in this suit
                    let tools =
                        crate::conf::operations::get_config_suit_tools(pool, suit_id).await?;

                    for tool_config in tools {
                        if tool_config.server_id == *server_id && tool_config.tool_name == tool_name
                        {
                            // Update tool status if this is the first time we see it or if this suit has higher priority
                            if tool_status.is_none()
                                || tool_status.as_ref().unwrap().1 < suit.priority
                            {
                                tool_status = Some((tool_config.enabled, suit.priority));
                            }
                            break;
                        }
                    }
                }
            }

            // If server is disabled in any active suit, the tool is also disabled
            if let Some((server_enabled, _)) = server_status {
                if !server_enabled {
                    tracing::debug!(
                        "Server '{}' is disabled in an active suit, tool '{}' is also disabled",
                        server_name,
                        tool_name
                    );
                    return Ok(false);
                }
            }

            // If we found a specific tool configuration, use its enabled status
            if let Some((tool_enabled, _)) = tool_status {
                tracing::debug!(
                    "Tool '{}' from server '{}' is {} in an active suit",
                    tool_name,
                    server_name,
                    if tool_enabled { "enabled" } else { "disabled" }
                );
                return Ok(tool_enabled);
            }

            // If no specific tool configuration found but server is enabled, check if there are any tool configurations for this server
            if server_status.is_some() {
                // Check if there are any tool configurations for this server in any active suit
                let mut has_tool_configs = false;

                for suit in &active_suits {
                    if let Some(suit_id) = &suit.id {
                        let tools =
                            crate::conf::operations::get_config_suit_tools(pool, suit_id).await?;
                        if tools.iter().any(|t| t.server_id == *server_id) {
                            has_tool_configs = true;
                            break;
                        }
                    }
                }

                if !has_tool_configs {
                    // If there are no tool configurations for this server in any active suit,
                    // all tools are enabled by default (semi-blacklist mode)
                    tracing::debug!(
                        "No tool configurations for server '{}' in any active suit, tool '{}' is enabled by default (semi-blacklist mode)",
                        server_name,
                        tool_name
                    );
                } else {
                    // If there are tool configurations for this server but not for this specific tool,
                    // the tool is still enabled by default (semi-blacklist mode)
                    tracing::debug!(
                        "No specific configuration for tool '{}' from server '{}' in any active suit, enabled by default (semi-blacklist mode)",
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

/// Helper function to check if a tool is enabled in a specific suit
async fn is_tool_enabled_in_suit(
    pool: &Pool<Sqlite>,
    server_id: &str,
    tool_name: &str,
    suit_id: &str,
) -> Result<bool> {
    // Get server configuration in this suit
    let servers = crate::conf::operations::get_config_suit_servers(pool, suit_id).await?;

    // Find the server in this suit
    for server_config in servers {
        if server_config.server_id == *server_id {
            // If the server is disabled in this suit, the tool is also disabled
            if !server_config.enabled {
                tracing::debug!(
                    "Server with ID '{}' is disabled in suit {}, tool '{}' is also disabled",
                    server_id,
                    suit_id,
                    tool_name
                );
                return Ok(false);
            }

            // Check if there's a specific tool configuration in this suit
            let tools = crate::conf::operations::get_config_suit_tools(pool, suit_id).await?;

            // Count tools for this server
            let server_tools = tools.iter().filter(|t| t.server_id == *server_id).count();

            // Find the tool in this suit
            let tool_config = tools
                .iter()
                .find(|t| t.server_id == *server_id && t.tool_name == tool_name);

            if let Some(config) = tool_config {
                // Return the tool's enabled status in this suit
                tracing::debug!(
                    "Tool '{}' from server ID '{}' is {} in suit {}",
                    tool_name,
                    server_id,
                    if config.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    suit_id
                );
                return Ok(config.enabled);
            }

            if server_tools == 0 {
                // If there are no tool configurations for this server, all tools are enabled by default (semi-blacklist mode)
                tracing::debug!(
                    "No tool configurations for server ID '{}' in suit {}, tool '{}' is enabled by default (semi-blacklist mode)",
                    server_id,
                    suit_id,
                    tool_name
                );
                return Ok(true);
            } else {
                // If there are tool configurations for this server but not for this specific tool,
                // the tool is still enabled by default (semi-blacklist mode)
                tracing::debug!(
                    "No specific configuration for tool '{}' from server ID '{}' in suit {}, enabled by default (semi-blacklist mode)",
                    tool_name,
                    server_id,
                    suit_id
                );
                return Ok(true);
            }
        }
    }

    // If server not found in this suit, the tool is enabled by default
    tracing::debug!(
        "Server ID '{}' not found in suit {}, tool '{}' is enabled by default",
        server_id,
        suit_id,
        tool_name
    );
    Ok(true)
}
