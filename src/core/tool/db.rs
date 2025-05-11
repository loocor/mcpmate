// Tool database operations module
// Contains functions for interacting with the database for tool operations

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

use crate::conf::operations;

/// Get custom prefixed names for tools from active config suits
///
/// This function retrieves custom prefixed names for tools from all active config suits.
/// It is used to override the automatically generated prefixed names with user-defined ones.
///
/// # Arguments
/// * `pool` - The database connection pool
///
/// # Returns
/// * `Result<HashMap<(String, String), String>>` - A mapping of (server_name, tool_name) to prefixed_name
pub async fn get_custom_prefixed_names(
    pool: &Pool<Sqlite>,
) -> Result<HashMap<(String, String), String>> {
    let mut result = HashMap::new();

    // Get all active config suits
    let active_suits = operations::suit::get_active_config_suits(pool).await?;

    // If there are no active suits, try to get the default suit
    if active_suits.is_empty() {
        let default_suit = operations::suit::get_default_config_suit(pool).await?;

        // If there's no default suit, try the legacy "default" named suit
        if default_suit.is_none() {
            let legacy_default = operations::get_config_suit_by_name(pool, "default").await?;

            // If there's no legacy default suit either, return empty map
            if legacy_default.is_none() {
                tracing::debug!(
                    "No active or default config suits found, no custom prefixed names"
                );
                return Ok(result);
            }

            // Use the legacy default suit
            let suit_id = legacy_default.unwrap().id.unwrap();
            return get_custom_prefixed_names_from_suit(pool, &suit_id).await;
        }

        // Use the default suit
        let suit_id = default_suit.unwrap().id.unwrap();
        return get_custom_prefixed_names_from_suit(pool, &suit_id).await;
    }

    // Create a map to track tool prefixed names with their priority
    // Higher priority value means higher precedence
    let mut prefixed_name_map: HashMap<(String, String), (String, i32)> = HashMap::new();

    // Process all active suits in priority order (already sorted by priority DESC)
    for suit in active_suits {
        if let Some(suit_id) = &suit.id {
            // Get all tool configs in this suit
            let tools = operations::get_config_suit_tools(pool, suit_id).await?;

            // Process each tool config
            for tool_config in tools {
                // Only consider tools with custom prefixed names
                if let Some(prefixed_name) = &tool_config.prefixed_name {
                    // Get the server name from the server ID
                    if let Ok(Some(server)) =
                        operations::get_server_by_id(pool, &tool_config.server_id).await
                    {
                        let key = (server.name.clone(), tool_config.tool_name.clone());

                        // Only update the map if this tool hasn't been seen yet or if the current suit has higher priority
                        if !prefixed_name_map.contains_key(&key)
                            || prefixed_name_map.get(&key).unwrap().1 < suit.priority
                        {
                            prefixed_name_map.insert(key, (prefixed_name.clone(), suit.priority));
                        }
                    }
                }
            }
        }
    }

    // Convert the map to the final result format
    for ((server_name, tool_name), (prefixed_name, _)) in prefixed_name_map {
        result.insert((server_name, tool_name), prefixed_name);
    }

    tracing::debug!(
        "Found {} custom prefixed names from active config suits",
        result.len()
    );
    Ok(result)
}

/// Helper function to get custom prefixed names from a specific suit
async fn get_custom_prefixed_names_from_suit(
    pool: &Pool<Sqlite>,
    suit_id: &str,
) -> Result<HashMap<(String, String), String>> {
    let mut result = HashMap::new();

    // Get all tool configs in this suit
    let tools = operations::get_config_suit_tools(pool, suit_id).await?;

    // Process each tool config
    for tool_config in tools {
        // Only consider tools with custom prefixed names
        if let Some(prefixed_name) = &tool_config.prefixed_name {
            // Get the server name from the server ID
            if let Ok(Some(server)) =
                operations::get_server_by_id(pool, &tool_config.server_id).await
            {
                result.insert(
                    (server.name.clone(), tool_config.tool_name.clone()),
                    prefixed_name.clone(),
                );
            }
        }
    }

    tracing::debug!(
        "Found {} custom prefixed names from suit {}",
        result.len(),
        suit_id
    );
    Ok(result)
}
