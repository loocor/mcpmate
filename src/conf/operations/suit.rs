// Config Suit operations for MCPMate
// Contains CRUD operations for configuration suits

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Transaction};

use crate::conf::models::{ConfigSuit, ConfigSuitServer, ConfigSuitTool, ConfigSuitType};

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
pub async fn get_config_suit(pool: &Pool<Sqlite>, id: i64) -> Result<Option<ConfigSuit>> {
    tracing::debug!("Executing SQL query to get configuration suit with ID {}", id);

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
            s.id.unwrap_or(0),
            s.suit_type
        );
    } else {
        tracing::debug!("No configuration suit found with name '{}'", name);
    }

    Ok(suit)
}

/// Create or update a configuration suit in the database
pub async fn upsert_config_suit(pool: &Pool<Sqlite>, suit: &ConfigSuit) -> Result<i64> {
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

/// Create or update a configuration suit in the database (transaction version)
pub async fn upsert_config_suit_tx(
    tx: &mut Transaction<'_, Sqlite>,
    suit: &ConfigSuit,
) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO config_suit (name, description, type, multi_select, priority)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(name) DO UPDATE SET
            description = excluded.description,
            type = excluded.type,
            multi_select = excluded.multi_select,
            priority = excluded.priority,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&suit.name)
    .bind(&suit.description)
    .bind(&suit.suit_type)
    .bind(suit.multi_select)
    .bind(suit.priority)
    .execute(&mut **tx)
    .await
    .context("Failed to upsert configuration suit")?;

    // Get the ID of the inserted or updated row
    let suit_id = if result.last_insert_rowid() > 0 {
        result.last_insert_rowid()
    } else {
        // If no new row was inserted, get the ID of the existing row
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM config_suit
            WHERE name = ?
            "#,
        )
        .bind(&suit.name)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to get configuration suit ID")?
    };

    Ok(suit_id)
}

/// Delete a configuration suit from the database
pub async fn delete_config_suit(pool: &Pool<Sqlite>, id: i64) -> Result<bool> {
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
    config_suit_id: i64,
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
pub async fn add_server_to_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: i64,
    server_id: i64,
    enabled: bool,
) -> Result<i64> {
    tracing::debug!(
        "Adding server ID {} to configuration suit ID {}, enabled: {}",
        server_id,
        config_suit_id,
        enabled
    );

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit_server (config_suit_id, server_id, enabled)
        VALUES (?, ?, ?)
        ON CONFLICT(config_suit_id, server_id) DO UPDATE SET
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(config_suit_id)
    .bind(server_id)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add server to configuration suit")?;

    // Get the ID of the inserted or updated row
    let association_id = if result.last_insert_rowid() > 0 {
        result.last_insert_rowid()
    } else {
        // If no new row was inserted, get the ID of the existing row
        sqlx::query_scalar::<_, i64>(
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

    Ok(association_id)
}

/// Remove a server from a configuration suit in the database
pub async fn remove_server_from_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: i64,
    server_id: i64,
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
    config_suit_id: i64,
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
pub async fn add_tool_to_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: i64,
    server_id: i64,
    tool_name: &str,
    enabled: bool,
) -> Result<i64> {
    tracing::debug!(
        "Adding tool '{}' from server ID {} to configuration suit ID {}, enabled: {}",
        tool_name,
        server_id,
        config_suit_id,
        enabled
    );

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit_tool (config_suit_id, server_id, tool_name, enabled)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(config_suit_id, server_id, tool_name) DO UPDATE SET
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(config_suit_id)
    .bind(server_id)
    .bind(tool_name)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add tool to configuration suit")?;

    // Get the ID of the inserted or updated row
    let association_id = if result.last_insert_rowid() > 0 {
        result.last_insert_rowid()
    } else {
        // If no new row was inserted, get the ID of the existing row
        sqlx::query_scalar::<_, i64>(
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

    Ok(association_id)
}

/// Remove a tool from a configuration suit in the database
pub async fn remove_tool_from_config_suit(
    pool: &Pool<Sqlite>,
    config_suit_id: i64,
    server_id: i64,
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
