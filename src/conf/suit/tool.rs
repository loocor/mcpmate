// Tool association operations for Config Suits
// Contains operations for managing tool associations with configuration suits

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{conf::models::ConfigSuitTool, generate_id};

/// Get all tools for a configuration suit from the database
pub async fn get_config_suit_tools(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
) -> Result<Vec<ConfigSuitTool>> {
    tracing::debug!(
        "Executing SQL query to get tools for configuration suit with ID {}",
        config_suit_id
    );

    let tools = sqlx::query_as::<_, ConfigSuitTool>(
        r#"
        SELECT * FROM config_suit_tool
        WHERE config_suit_id = ?
        ORDER BY server_id, tool_name
        "#,
    )
    .bind(config_suit_id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch configuration suit tools")?;

    tracing::debug!(
        "Successfully fetched {} tools for configuration suit with ID {}",
        tools.len(),
        config_suit_id
    );
    Ok(tools)
}

/// Add a tool to a configuration suit in the database
///
/// This function adds a tool to a configuration suit in the database.
/// If the tool is added or updated, it also publishes a ToolEnabledInSuitChanged event.
pub async fn add_tool_to_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
    server_id: &str,
    tool_name: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding tool '{}' from server ID {} to configuration suit ID {}, enabled: {}",
        tool_name,
        server_id,
        config_suit_id,
        enabled
    );

    // Generate an ID for the suit tool
    let tool_id = generate_id!("stol");

    // Check if the tool already exists in the suit and get its current enabled status
    let existing_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM config_suit_tool
        WHERE config_suit_id = ? AND server_id = ? AND tool_name = ?
        "#,
    )
    .bind(config_suit_id)
    .bind(server_id)
    .bind(tool_name)
    .fetch_optional(pool)
    .await
    .context("Failed to get existing tool enabled status")?;

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

    // Generate a unique name for the tool
    let base_unique_name = crate::core::tool::generate_unique_name(&server_name, tool_name);

    // Ensure the unique name doesn't conflict with existing names
    let unique_name =
        crate::core::tool::ensure_unique_name(pool, &base_unique_name, server_id, tool_name)
            .await
            .context("Failed to ensure unique name for tool")?;

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit_tool (id, config_suit_id, server_id, server_name, tool_name, unique_name, enabled)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(config_suit_id, server_id, tool_name) DO UPDATE SET
            server_name = excluded.server_name,
            unique_name = excluded.unique_name,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&tool_id)
    .bind(config_suit_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(tool_name)
    .bind(&unique_name)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add tool to configuration suit")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        tool_id.clone()
    } else {
        // If no rows were affected, get the existing ID
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM config_suit_tool
            WHERE config_suit_id = ? AND server_id = ? AND tool_name = ?
            "#,
        )
        .bind(config_suit_id)
        .bind(server_id)
        .bind(tool_name)
        .fetch_one(pool)
        .await
        .context("Failed to get configuration suit tool association ID")?
    };

    // Publish event if the tool is new or its enabled status has changed
    if is_new || (existing_enabled != Some(enabled)) {
        // Publish the event
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ToolEnabledInSuitChanged {
                tool_id: id_to_return.clone(),
                tool_name: tool_name.to_string(),
                suit_id: config_suit_id.to_string(),
                enabled,
            },
        );

        tracing::debug!(
            "Published ToolEnabledInSuitChanged event for tool '{}' in suit ID {} ({})",
            tool_name,
            config_suit_id,
            enabled
        );
    }

    Ok(id_to_return)
}

/// Remove a tool from a configuration suit in the database
pub async fn remove_tool_from_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
    server_id: &str,
    tool_name: &str,
) -> Result<bool> {
    tracing::debug!(
        "Removing tool '{}' from server ID {} from configuration suit ID {}",
        tool_name,
        server_id,
        config_suit_id
    );

    let result = sqlx::query(
        r#"
        DELETE FROM config_suit_tool
        WHERE config_suit_id = ? AND server_id = ? AND tool_name = ?
        "#,
    )
    .bind(config_suit_id)
    .bind(server_id)
    .bind(tool_name)
    .execute(pool)
    .await
    .context("Failed to remove tool from configuration suit")?;

    Ok(result.rows_affected() > 0)
}
