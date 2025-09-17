// Server tools management module
// Manages global tool name mappings for servers

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{config::models::ServerTool, core::capability::naming::{ensure_unique_name, NamingKind}, generate_id};

/// Add or update a server tool mapping
pub async fn upsert_server_tool(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    tool_name: &str,
    description: Option<&str>,
) -> Result<String> {
    tracing::debug!(
        "Upserting server tool mapping: server_id={}, tool_name={}",
        server_id,
        tool_name
    );

    // Compute collision-free unique name via naming module
    let unique_name = ensure_unique_name(NamingKind::Tool, server_id, server_name, tool_name)
        .await
        .context("Failed to ensure unique name for tool")?;

    // Check if the tool already exists
    let existing_id =
        sqlx::query_scalar::<_, String>("SELECT id FROM server_tools WHERE server_id = ? AND tool_name = ?")
            .bind(server_id)
            .bind(tool_name)
            .fetch_optional(pool)
            .await
            .context("Failed to check existing server tool")?;

    if let Some(id) = existing_id {
        // Update existing tool
        sqlx::query(
            r#"
            UPDATE server_tools
            SET server_name = ?, unique_name = ?, description = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(server_name)
        .bind(&unique_name)
        .bind(description)
        .bind(&id)
        .execute(pool)
        .await
        .context("Failed to update server tool")?;

        tracing::debug!("Updated server tool mapping: id={}, unique_name={}", id, unique_name);
        Ok(id)
    } else {
        // Insert new tool
        let tool_id = generate_id!("stool");

        sqlx::query(
            r#"
            INSERT INTO server_tools (id, server_id, server_name, tool_name, unique_name, description)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&tool_id)
        .bind(server_id)
        .bind(server_name)
        .bind(tool_name)
        .bind(&unique_name)
        .bind(description)
        .execute(pool)
        .await
        .context("Failed to insert server tool")?;

        tracing::debug!(
            "Created server tool mapping: id={}, unique_name={}",
            tool_id,
            unique_name
        );
        Ok(tool_id)
    }
}

/// Get all server tools for a specific server
pub async fn get_server_tools(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<Vec<ServerTool>> {
    let tools = sqlx::query_as::<_, ServerTool>("SELECT * FROM server_tools WHERE server_id = ? ORDER BY tool_name")
        .bind(server_id)
        .fetch_all(pool)
        .await
        .context("Failed to fetch server tools")?;

    Ok(tools)
}

/// Get a server tool by unique name
pub async fn get_server_tool_by_unique_name(
    pool: &Pool<Sqlite>,
    unique_name: &str,
) -> Result<Option<ServerTool>> {
    let tool = sqlx::query_as::<_, ServerTool>("SELECT * FROM server_tools WHERE unique_name = ?")
        .bind(unique_name)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch server tool by unique name")?;

    Ok(tool)
}

/// Get a server tool by server_id and tool_name
pub async fn get_server_tool(
    pool: &Pool<Sqlite>,
    server_id: &str,
    tool_name: &str,
) -> Result<Option<ServerTool>> {
    let tool = sqlx::query_as::<_, ServerTool>("SELECT * FROM server_tools WHERE server_id = ? AND tool_name = ?")
        .bind(server_id)
        .bind(tool_name)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch server tool")?;

    Ok(tool)
}

/// Remove all server tools for a specific server
pub async fn remove_server_tools(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<u64> {
    let result = sqlx::query("DELETE FROM server_tools WHERE server_id = ?")
        .bind(server_id)
        .execute(pool)
        .await
        .context("Failed to remove server tools")?;

    tracing::debug!(
        "Removed {} server tools for server_id={}",
        result.rows_affected(),
        server_id
    );
    Ok(result.rows_affected())
}

/// Get all unique tool names (for API responses)
pub async fn get_all_unique_tool_names(pool: &Pool<Sqlite>) -> Result<Vec<String>> {
    let names = sqlx::query_scalar::<_, String>("SELECT unique_name FROM server_tools ORDER BY unique_name")
        .fetch_all(pool)
        .await
        .context("Failed to fetch unique tool names")?;

    Ok(names)
}

/// Batch upsert server tools (for server synchronization)
pub async fn batch_upsert_server_tools(
    pool: &Pool<Sqlite>,
    server_id: &str,
    server_name: &str,
    tools: &[(String, Option<String>)], // (tool_name, description)
) -> Result<Vec<String>> {
    let mut tool_ids = Vec::new();

    for (tool_name, description) in tools {
        let tool_id = upsert_server_tool(pool, server_id, server_name, tool_name, description.as_deref()).await?;
        tool_ids.push(tool_id);
    }

    tracing::debug!(
        "Batch upserted {} server tools for server_id={}",
        tool_ids.len(),
        server_id
    );

    Ok(tool_ids)
}
