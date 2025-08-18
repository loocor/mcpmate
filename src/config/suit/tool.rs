// Tool association operations for Config Suits (New Architecture)
// Contains operations for managing tool associations with configuration suits
// Uses server_tools table as the single source of truth for tool mappings

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{config::models::ConfigSuitToolWithDetails, generate_id};

/// Tool status information for API responses
#[derive(Debug, Clone)]
pub struct ToolStatus {
    /// Tool ID in config suit
    pub tool_id: String,
    /// Unique name for external display
    pub unique_name: Option<String>,
    /// Whether the tool is enabled
    pub enabled: bool,
}

/// Unified tool status service to eliminate code duplication
pub struct ToolStatusService;

impl ToolStatusService {
    /// Get comprehensive tool status information
    /// This replaces the scattered tool status checking logic
    pub async fn get_tool_status(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_name: &str,
        tool_name: &str,
    ) -> anyhow::Result<ToolStatus> {
        // Check if the tool is enabled using existing logic
        let enabled =
            crate::config::operations::tool::is_tool_enabled(pool, server_name, tool_name)
                .await
                .unwrap_or(true); // Default to enabled if there's an error

        // Get the tool ID using existing logic
        let tool_id =
            match crate::config::operations::tool::get_tool_id(pool, server_name, tool_name).await?
            {
                Some(id) => id,
                None => {
                    // Tool not found, create a default entry
                    Self::create_default_tool_entry(pool, server_name, tool_name).await?
                }
            };

        // Get unique name from server_tools table
        let unique_name = sqlx::query_scalar::<_, String>(
            r#"
            SELECT st.unique_name
            FROM config_suit_tool cst
            JOIN server_tools st ON cst.server_tool_id = st.id
            WHERE cst.id = ?
            "#,
        )
        .bind(&tool_id)
        .fetch_optional(pool)
        .await?;

        Ok(ToolStatus {
            tool_id,
            unique_name,
            enabled,
        })
    }

    /// Create a default tool entry when tool is not found
    async fn create_default_tool_entry(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_name: &str,
        tool_name: &str,
    ) -> anyhow::Result<String> {
        // Get or create default config suit
        let suit_id = Self::get_or_create_default_suit(pool).await?;

        // Get server ID
        let server = crate::config::server::get_server(pool, server_name).await?;
        let server_id = server
            .and_then(|s| s.id)
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

        // Add tool to config suit
        crate::config::suit::add_tool_to_config_suit(pool, &suit_id, &server_id, tool_name, true)
            .await
    }

    /// Get or create default configuration suit
    async fn get_or_create_default_suit(pool: &sqlx::Pool<sqlx::Sqlite>) -> anyhow::Result<String> {
        // Try to get default suit
        if let Some(suit) = crate::config::suit::get_default_config_suit(pool).await? {
            return Ok(suit.id.unwrap());
        }

        // Try legacy "default" named suit
        if let Some(suit) = crate::config::suit::get_config_suit_by_name(pool, "default").await? {
            return Ok(suit.id.unwrap());
        }

        // Create new default suit
        let mut new_suit = crate::config::models::ConfigSuit::new_with_description(
            "default".to_string(),
            Some("Default configuration suit".to_string()),
            crate::common::config::ConfigSuitType::Shared,
        );
        new_suit.is_active = true;
        new_suit.is_default = true;
        new_suit.multi_select = true;

        crate::config::suit::upsert_config_suit(pool, &new_suit).await
    }
}

/// Common query builder for enabled tools from active configuration suits
/// This helper reduces code duplication across the codebase
pub fn build_enabled_tools_query(additional_where: Option<&str>) -> String {
    let base_query = r#"
        SELECT DISTINCT st.unique_name, st.server_name, st.tool_name, st.server_id
        FROM config_suit_tool cst
        JOIN config_suit cs ON cst.config_suit_id = cs.id
        JOIN server_tools st ON cst.server_tool_id = st.id
        WHERE cs.is_active = true AND cst.enabled = true"#;

    match additional_where {
        Some(condition) => format!("{} AND {}", base_query, condition),
        None => base_query.to_string(),
    }
}

/// Common query builder for tool details with server information
/// This helper reduces code duplication for JOIN queries
pub fn build_tool_details_query(additional_where: Option<&str>) -> String {
    let base_query = r#"
        SELECT
            cst.id,
            cst.config_suit_id,
            cst.server_tool_id,
            cst.enabled,
            cst.created_at,
            cst.updated_at,
            st.server_id,
            st.server_name,
            st.tool_name,
            st.unique_name,
            st.description
        FROM config_suit_tool cst
        JOIN server_tools st ON cst.server_tool_id = st.id"#;

    match additional_where {
        Some(condition) => format!("{} WHERE {}", base_query, condition),
        None => base_query.to_string(),
    }
}

/// Get all tools for a configuration suit from the database (new architecture)
pub async fn get_config_suit_tools(
    pool: &Pool<Sqlite>,
    config_suit_id: &str,
) -> Result<Vec<ConfigSuitToolWithDetails>> {
    tracing::debug!(
        "Executing SQL query to get tools for configuration suit with ID {}",
        config_suit_id
    );

    let query = format!(
        "{} WHERE cst.config_suit_id = ? ORDER BY st.server_name, st.tool_name",
        build_tool_details_query(None)
    );

    let tools = sqlx::query_as::<_, ConfigSuitToolWithDetails>(&query)
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

/// Add a tool to a configuration suit (new architecture)
///
/// This function adds a tool to a configuration suit using the new architecture.
/// It first ensures the server tool mapping exists in server_tools table,
/// then creates the config suit association.
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

    // First, ensure the server tool mapping exists in server_tools table
    let server_name = crate::config::operations::server::get_server_name_safe(pool, server_id)
        .await
        .context("Failed to get server name")?;

    let server_tool_id = crate::config::server::tools::upsert_server_tool(
        pool,
        server_id,
        &server_name,
        tool_name,
        None, // description will be updated during tool sync
    )
    .await
    .context("Failed to upsert server tool")?;

    // Check if the tool already exists in the config suit
    let existing_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM config_suit_tool
        WHERE config_suit_id = ? AND server_tool_id = ?
        "#,
    )
    .bind(config_suit_id)
    .bind(&server_tool_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get existing tool enabled status")?;

    // Generate an ID for the config suit tool association
    let config_suit_tool_id = generate_id!("cstool");

    let result = sqlx::query(
        r#"
        INSERT INTO config_suit_tool (id, config_suit_id, server_tool_id, enabled)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(config_suit_id, server_tool_id) DO UPDATE SET
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&config_suit_tool_id)
    .bind(config_suit_id)
    .bind(&server_tool_id)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add tool to configuration suit")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        config_suit_tool_id.clone()
    } else {
        // If no rows were affected, get the existing ID
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM config_suit_tool
            WHERE config_suit_id = ? AND server_tool_id = ?
            "#,
        )
        .bind(config_suit_id)
        .bind(&server_tool_id)
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

/// Remove a tool from a configuration suit (new architecture)
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

    // First, find the server_tool_id
    let server_tool_id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM server_tools WHERE server_id = ? AND tool_name = ?",
    )
    .bind(server_id)
    .bind(tool_name)
    .fetch_optional(pool)
    .await
    .context("Failed to find server tool")?;

    if let Some(server_tool_id) = server_tool_id {
        let result = sqlx::query(
            r#"
            DELETE FROM config_suit_tool
            WHERE config_suit_id = ? AND server_tool_id = ?
            "#,
        )
        .bind(config_suit_id)
        .bind(&server_tool_id)
        .execute(pool)
        .await
        .context("Failed to remove tool from configuration suit")?;

        Ok(result.rows_affected() > 0)
    } else {
        tracing::warn!(
            "Server tool not found for server_id={}, tool_name={}",
            server_id,
            tool_name
        );
        Ok(false)
    }
}
