// Config Suit operations for MCPMate
// Contains CRUD operations for configuration suits

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Transaction};

use crate::{
    common::types::ConfigSuitType,
    conf::models::{ConfigSuit, ConfigSuitServer, ConfigSuitTool},
};

/// Get all configuration suits from the database
pub async fn get_all_config_suits(pool: &Pool<Sqlite>) -> Result<Vec<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get all configuration suits");

    let suits = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch configuration suits")?;

    tracing::debug!(
        "Successfully fetched {} configuration suits from database",
        suits.len()
    );
    Ok(suits)
}

/// Get all active configuration suits from the database
pub async fn get_active_config_suits(pool: &Pool<Sqlite>) -> Result<Vec<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get all active configuration suits");

    let suits = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE is_active = 1
        ORDER BY priority DESC, created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch active configuration suits")?;

    tracing::debug!(
        "Successfully fetched {} active configuration suits from database",
        suits.len()
    );
    Ok(suits)
}

/// Get the default configuration suit from the database
pub async fn get_default_config_suit(pool: &Pool<Sqlite>) -> Result<Option<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get default configuration suit");

    let suit = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE is_default = 1
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to fetch default configuration suit")?;

    if let Some(ref s) = suit {
        tracing::debug!(
            "Found default configuration suit '{}' with ID {}",
            s.name,
            s.id.as_ref().unwrap_or(&"unknown".to_string())
        );
    } else {
        tracing::debug!("No default configuration suit found");
    }

    Ok(suit)
}

/// Get configuration suits by type from the database
pub async fn get_config_suits_by_type(
    pool: &Pool<Sqlite>,
    suit_type: ConfigSuitType,
) -> Result<Vec<ConfigSuit>> {
    tracing::debug!(
        "Executing SQL query to get configuration suits of type '{}'",
        suit_type.as_str()
    );

    let suits = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE type = ?
        ORDER BY name
        "#,
    )
    .bind(suit_type.as_str())
    .fetch_all(pool)
    .await
    .context("Failed to fetch configuration suits by type")?;

    tracing::debug!(
        "Successfully fetched {} configuration suits of type '{}'",
        suits.len(),
        suit_type.as_str()
    );
    Ok(suits)
}

/// Get a specific configuration suit from the database
pub async fn get_config_suit(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<Option<ConfigSuit>> {
    tracing::debug!(
        "Executing SQL query to get configuration suit with ID {}",
        id
    );

    let suit = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch configuration suit")?;

    if let Some(ref s) = suit {
        tracing::debug!(
            "Found configuration suit '{}' with ID {}, type: {}",
            s.name,
            id,
            s.suit_type
        );
    } else {
        tracing::debug!("No configuration suit found with ID {}", id);
    }

    Ok(suit)
}

/// Get a specific configuration suit by name from the database
pub async fn get_config_suit_by_name(
    pool: &Pool<Sqlite>,
    name: &str,
) -> Result<Option<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get configuration suit '{}'", name);

    let suit = sqlx::query_as::<_, ConfigSuit>(
        r#"
        SELECT * FROM config_suit
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch configuration suit by name")?;

    if let Some(ref s) = suit {
        tracing::debug!(
            "Found configuration suit '{}' with ID {}, type: {}",
            name,
            s.id.as_ref().unwrap_or(&"unknown".to_string()),
            s.suit_type
        );
    } else {
        tracing::debug!("No configuration suit found with name '{}'", name);
    }

    Ok(suit)
}

/// Create or update a configuration suit in the database
pub async fn upsert_config_suit(
    pool: &Pool<Sqlite>,
    suit: &ConfigSuit,
) -> Result<String> {
    tracing::debug!(
        "Upserting configuration suit '{}', type: {}",
        suit.name,
        suit.suit_type
    );

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;
    let suit_id = upsert_config_suit_tx(&mut tx, suit).await?;
    tx.commit().await.context("Failed to commit transaction")?;

    Ok(suit_id)
}

/// Set a configuration suit as active or inactive
///
/// This function updates the active status of a configuration suit in the database.
/// If the status is updated, it also publishes a ConfigSuitStatusChanged event.
pub async fn set_config_suit_active(
    pool: &Pool<Sqlite>,
    suit_id: &str,
    active: bool,
) -> Result<()> {
    tracing::debug!(
        "Setting configuration suit with ID {} as {}",
        suit_id,
        if active { "active" } else { "inactive" }
    );

    // Get the configuration suit to check multi_select
    let suit = get_config_suit(pool, suit_id).await?;
    if suit.is_none() {
        return Err(anyhow::anyhow!(
            "Configuration suit with ID {} not found",
            suit_id
        ));
    }
    let suit = suit.unwrap();

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // If activating and multi_select is false, deactivate all other suits
    if active && !suit.multi_select {
        sqlx::query(
            r#"
            UPDATE config_suit
            SET is_active = 0,
                updated_at = CURRENT_TIMESTAMP
            WHERE id != ?
            "#,
        )
        .bind(suit_id)
        .execute(&mut *tx)
        .await
        .context("Failed to deactivate other configuration suits")?;
    }

    // Update the specified suit
    sqlx::query(
        r#"
        UPDATE config_suit
        SET is_active = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(active)
    .bind(suit_id)
    .execute(&mut *tx)
    .await
    .context("Failed to update configuration suit active status")?;

    tx.commit().await.context("Failed to commit transaction")?;

    // Publish the event
    crate::core::events::EventBus::global().publish(
        crate::core::events::Event::ConfigSuitStatusChanged {
            suit_id: suit_id.to_string(),
            enabled: active,
        },
    );

    tracing::info!(
        "Published ConfigSuitStatusChanged event for suit ID {} ({})",
        suit_id,
        active
    );

    Ok(())
}

/// Set a configuration suit as the default
pub async fn set_config_suit_default(
    pool: &Pool<Sqlite>,
    suit_id: &str,
) -> Result<()> {
    tracing::debug!("Setting configuration suit with ID {} as default", suit_id);

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // Clear default flag from all suits
    sqlx::query(
        r#"
        UPDATE config_suit
        SET is_default = 0,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .execute(&mut *tx)
    .await
    .context("Failed to clear default flag from all configuration suits")?;

    // Set the specified suit as default
    sqlx::query(
        r#"
        UPDATE config_suit
        SET is_default = 1,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(suit_id)
    .execute(&mut *tx)
    .await
    .context("Failed to set configuration suit as default")?;

    tx.commit().await.context("Failed to commit transaction")?;
    Ok(())
}

/// Create or update a configuration suit in the database (transaction version)
pub async fn upsert_config_suit_tx(
    tx: &mut Transaction<'_, Sqlite>,
    suit: &ConfigSuit,
) -> Result<String> {
    // Generate a UUID for the suit if it doesn't have one
    let suit_id = if let Some(id) = &suit.id {
        id.clone()
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit (id, name, description, type, multi_select, priority, is_active, is_default)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(name) DO UPDATE SET
            description = excluded.description,
            type = excluded.type,
            multi_select = excluded.multi_select,
            priority = excluded.priority,
            is_active = excluded.is_active,
            is_default = excluded.is_default,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&suit_id)
    .bind(&suit.name)
    .bind(&suit.description)
    .bind(&suit.suit_type)
    .bind(suit.multi_select)
    .bind(suit.priority)
    .bind(suit.is_active)
    .bind(suit.is_default)
    .execute(&mut **tx)
    .await
    .context("Failed to upsert configuration suit")?;

    if result.rows_affected() == 0 {
        // If no rows were affected, get the existing ID
        let existing_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM config_suit
            WHERE name = ?
            "#,
        )
        .bind(&suit.name)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to get configuration suit ID")?;

        return Ok(existing_id);
    }

    Ok(suit_id)
}

/// Delete a configuration suit from the database
pub async fn delete_config_suit(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<bool> {
    tracing::debug!("Deleting configuration suit with ID {}", id);

    let result = sqlx::query(
        r#"
        DELETE FROM config_suit
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to delete configuration suit")?;

    Ok(result.rows_affected() > 0)
}

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

    // Generate a UUID for the association
    let association_id = uuid::Uuid::new_v4().to_string();

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
    if is_new || existing_enabled.map_or(true, |e| e != enabled) {
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

    // Generate a UUID for the tool
    let tool_id = uuid::Uuid::new_v4().to_string();

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

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit_tool (id, config_suit_id, server_id, server_name, tool_name, enabled)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(config_suit_id, server_id, tool_name) DO UPDATE SET
            server_name = excluded.server_name,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&tool_id)
    .bind(config_suit_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(tool_name)
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
    if is_new || existing_enabled.map_or(true, |e| e != enabled) {
        // Publish the event
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ToolEnabledInSuitChanged {
                tool_id: id_to_return.clone(),
                tool_name: tool_name.to_string(),
                suit_id: config_suit_id.to_string(),
                enabled,
            },
        );

        tracing::info!(
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
