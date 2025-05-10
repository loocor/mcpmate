// Tool operations for MCPMate
// Contains CRUD operations for tool configuration

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::conf::models::{Tool, ToolUpdate};

/// Get all tools from the database
pub async fn get_all_tools(pool: &Pool<Sqlite>) -> Result<Vec<Tool>> {
    tracing::debug!("Executing SQL query to get all tools");

    let tools = sqlx::query_as::<_, Tool>(
        r#"
        SELECT * FROM tool_config
        ORDER BY server_name, tool_name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch tools")?;

    tracing::debug!("Successfully fetched {} tools from database", tools.len());
    Ok(tools)
}

/// Get tools for a specific server from the database
pub async fn get_server_tools(pool: &Pool<Sqlite>, server_name: &str) -> Result<Vec<Tool>> {
    tracing::debug!(
        "Executing SQL query to get tools for server '{}'",
        server_name
    );

    let tools = sqlx::query_as::<_, Tool>(
        r#"
        SELECT * FROM tool_config
        WHERE server_name = ?
        ORDER BY tool_name
        "#,
    )
    .bind(server_name)
    .fetch_all(pool)
    .await
    .context("Failed to fetch server tools")?;

    tracing::debug!(
        "Successfully fetched {} tools for server '{}'",
        tools.len(),
        server_name
    );
    Ok(tools)
}

/// Get a specific tool from the database
pub async fn get_tool(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<Option<Tool>> {
    tracing::debug!(
        "Executing SQL query to get tool '{}' from server '{}'",
        tool_name,
        server_name
    );

    let tool = sqlx::query_as::<_, Tool>(
        r#"
        SELECT * FROM tool_config
        WHERE server_name = ? AND tool_name = ?
        "#,
    )
    .bind(server_name)
    .bind(tool_name)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch tool")?;

    if let Some(ref t) = tool {
        tracing::debug!(
            "Found tool '{}' from server '{}', enabled: {}",
            tool_name,
            server_name,
            t.enabled
        );
    } else {
        tracing::debug!(
            "No tool found with name '{}' from server '{}'",
            tool_name,
            server_name
        );
    }

    Ok(tool)
}

/// Create or update a tool in the database
pub async fn upsert_tool(pool: &Pool<Sqlite>, tool: &Tool) -> Result<i64> {
    tracing::debug!(
        "Upserting tool '{}' from server '{}', enabled: {}",
        tool.tool_name,
        tool.server_name,
        tool.enabled
    );

    let result = sqlx::query(
        r#"
        INSERT INTO tool_config (server_name, tool_name, alias_name, enabled)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(server_name, tool_name) DO UPDATE SET
            alias_name = excluded.alias_name,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&tool.server_name)
    .bind(&tool.tool_name)
    .bind(&tool.alias_name)
    .bind(tool.enabled)
    .execute(pool)
    .await
    .context("Failed to upsert tool")?;

    // Get the ID of the inserted or updated row
    let tool_id = if result.last_insert_rowid() > 0 {
        result.last_insert_rowid()
    } else {
        // If no new row was inserted, get the ID of the existing row
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM tool_config
            WHERE server_name = ? AND tool_name = ?
            "#,
        )
        .bind(&tool.server_name)
        .bind(&tool.tool_name)
        .fetch_one(pool)
        .await
        .context("Failed to get tool ID")?
    };

    Ok(tool_id)
}

/// Update a tool in the database
pub async fn update_tool(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
    update: &ToolUpdate,
) -> Result<bool> {
    tracing::debug!(
        "Updating tool '{}' from server '{}', enabled: {}",
        tool_name,
        server_name,
        update.enabled
    );

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
    .context("Failed to update tool")?;

    Ok(result.rows_affected() > 0)
}

/// Delete a tool from the database
pub async fn delete_tool(pool: &Pool<Sqlite>, server_name: &str, tool_name: &str) -> Result<bool> {
    tracing::debug!(
        "Deleting tool '{}' from server '{}'",
        tool_name,
        server_name
    );

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
    .context("Failed to delete tool")?;

    Ok(result.rows_affected() > 0)
}

/// Get all disabled tools from the database
pub async fn get_disabled_tools(pool: &Pool<Sqlite>) -> Result<Vec<Tool>> {
    tracing::debug!("Executing SQL query to get all disabled tools");

    let tools = sqlx::query_as::<_, Tool>(
        r#"
        SELECT * FROM tool_config
        WHERE enabled = 0
        ORDER BY server_name, tool_name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch disabled tools")?;

    tracing::debug!(
        "Successfully fetched {} disabled tools from database",
        tools.len()
    );
    Ok(tools)
}

/// Check if a tool is enabled in the database
pub async fn is_tool_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<bool> {
    // First check if there's a configuration for this tool
    let tool = get_tool(pool, server_name, tool_name).await?;

    // If there's no configuration, the tool is enabled by default
    if tool.is_none() {
        return Ok(true);
    }

    // Otherwise, return the enabled status from the configuration
    Ok(tool.unwrap().enabled)
}

// For backward compatibility
pub use delete_tool as delete_tool_config;
pub use get_all_tools as get_all_tool_configs;
pub use get_server_tools as get_server_tool_configs;
pub use get_tool as get_tool_config;
pub use update_tool as update_tool_config;
pub use upsert_tool as upsert_tool_config;
