// Database operations for MCPMate
// Contains CRUD operations for tool configuration

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use super::models::{ToolConfig, ToolConfigUpdate};

/// Get all tool configurations
pub async fn get_all_tool_configs(pool: &Pool<Sqlite>) -> Result<Vec<ToolConfig>> {
    tracing::debug!("Executing SQL query to get all tool configurations");

    let configs = match sqlx::query_as::<_, ToolConfig>(
        r#"
        SELECT * FROM tool_config
        ORDER BY server_name, tool_name
        "#,
    )
    .fetch_all(pool)
    .await
    {
        Ok(configs) => {
            tracing::debug!(
                "Successfully fetched {} tool configurations from database",
                configs.len()
            );
            configs
        }
        Err(e) => {
            tracing::error!("Database error when fetching tool configurations: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to fetch tool configurations: {}",
                e
            ));
        }
    };

    Ok(configs)
}

/// Get tool configurations for a specific server
pub async fn get_server_tool_configs(
    pool: &Pool<Sqlite>,
    server_name: &str,
) -> Result<Vec<ToolConfig>> {
    let configs = sqlx::query_as::<_, ToolConfig>(
        r#"
        SELECT * FROM tool_config
        WHERE server_name = ?
        ORDER BY tool_name
        "#,
    )
    .bind(server_name)
    .fetch_all(pool)
    .await
    .context("Failed to fetch server tool configurations")?;

    Ok(configs)
}

/// Get a specific tool configuration
pub async fn get_tool_config(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<Option<ToolConfig>> {
    tracing::debug!(
        "Executing SQL query to get tool configuration for server '{}', tool '{}'",
        server_name,
        tool_name
    );

    let config = match sqlx::query_as::<_, ToolConfig>(
        r#"
        SELECT * FROM tool_config
        WHERE server_name = ? AND tool_name = ?
        "#,
    )
    .bind(server_name)
    .bind(tool_name)
    .fetch_optional(pool)
    .await
    {
        Ok(config) => {
            if let Some(ref c) = config {
                tracing::debug!(
                    "Found tool configuration for server '{}', tool '{}', enabled: {}",
                    server_name,
                    tool_name,
                    c.enabled
                );
            } else {
                tracing::debug!(
                    "No tool configuration found for server '{}', tool '{}'",
                    server_name,
                    tool_name
                );
            }
            config
        }
        Err(e) => {
            tracing::error!(
                "Database error when fetching tool configuration for server '{}', tool '{}': {}",
                server_name,
                tool_name,
                e
            );
            return Err(anyhow::anyhow!("Failed to fetch tool configuration: {}", e));
        }
    };

    Ok(config)
}

/// Create or update a tool configuration
pub async fn upsert_tool_config(pool: &Pool<Sqlite>, config: &ToolConfig) -> Result<i64> {
    tracing::debug!(
        "Upserting tool configuration for server '{}', tool '{}', alias '{}', enabled: {}",
        config.server_name,
        config.tool_name,
        config.alias_name.as_deref().unwrap_or("none"),
        config.enabled
    );

    let result =
        match sqlx::query(
            r#"
        INSERT INTO tool_config (server_name, tool_name, alias_name, enabled)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(server_name, tool_name) DO UPDATE SET
            alias_name = excluded.alias_name,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
        )
        .bind(&config.server_name)
        .bind(&config.tool_name)
        .bind(&config.alias_name)
        .bind(config.enabled)
        .execute(pool)
        .await
        {
            Ok(result) => {
                tracing::debug!(
                    "Successfully upserted tool configuration for server '{}', tool '{}'",
                    config.server_name,
                    config.tool_name
                );
                result
            }
            Err(e) => {
                tracing::error!(
                "Database error when upserting tool configuration for server '{}', tool '{}': {}",
                config.server_name, config.tool_name, e
            );
                return Err(anyhow::anyhow!(
                    "Failed to upsert tool configuration: {}",
                    e
                ));
            }
        };

    // Get the ID of the inserted or updated row
    let id = if result.last_insert_rowid() > 0 {
        result.last_insert_rowid()
    } else {
        // If no new row was inserted, get the ID of the existing row
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM tool_config
            WHERE server_name = ? AND tool_name = ?
            "#,
        )
        .bind(&config.server_name)
        .bind(&config.tool_name)
        .fetch_one(pool)
        .await
        .context("Failed to get tool configuration ID")?
    };

    Ok(id)
}

/// Update a tool configuration
pub async fn update_tool_config(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
    update: &ToolConfigUpdate,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE tool_config
        SET enabled = ?, alias_name = ?, updated_at = CURRENT_TIMESTAMP
        WHERE server_name = ? AND tool_name = ?
        "#,
    )
    .bind(update.enabled)
    .bind(&update.alias_name)
    .bind(server_name)
    .bind(tool_name)
    .execute(pool)
    .await
    .context("Failed to update tool configuration")?;

    Ok(result.rows_affected() > 0)
}

/// Delete a tool configuration
pub async fn delete_tool_config(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM tool_config
        WHERE server_name = ? AND tool_name = ?
        "#,
    )
    .bind(server_name)
    .bind(tool_name)
    .execute(pool)
    .await
    .context("Failed to delete tool configuration")?;

    Ok(result.rows_affected() > 0)
}

/// Get all disabled tools
pub async fn get_disabled_tools(pool: &Pool<Sqlite>) -> Result<Vec<ToolConfig>> {
    let configs = sqlx::query_as::<_, ToolConfig>(
        r#"
        SELECT * FROM tool_config
        WHERE enabled = 0
        ORDER BY server_name, tool_name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch disabled tool configurations")?;

    Ok(configs)
}

/// Check if a tool is enabled
pub async fn is_tool_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<bool> {
    // First check if there's a configuration for this tool
    let config = get_tool_config(pool, server_name, tool_name).await?;

    // If there's no configuration, the tool is enabled by default
    if config.is_none() {
        return Ok(true);
    }

    // Otherwise, return the enabled status from the configuration
    Ok(config.unwrap().enabled)
}
