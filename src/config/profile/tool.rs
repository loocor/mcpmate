// Tool association operations for Profile (New Architecture)
// Contains operations for managing tool associations with profile
// Uses server_tools table as the single source of truth for tool mappings

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    config::{
        models::ProfileToolWithDetails,
        profile::{DEFAULT_ANCHOR_INITIAL_NAME, DEFAULT_ANCHOR_ROLE, DEFAULT_PROFILE_DESCRIPTION},
    },
    core::capability::naming::{NamingKind, strip_server_prefix},
    generate_id,
};

/// Tool status information for API responses
#[derive(Debug, Clone)]
pub struct ToolStatus {
    /// Tool ID in profile
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
        let enabled = crate::config::operations::tool::is_tool_enabled(pool, server_name, tool_name)
            .await
            .unwrap_or(true); // Default to enabled if there's an error

        // Get the tool ID using existing logic
        let tool_id = match crate::config::operations::tool::get_tool_id(pool, server_name, tool_name).await? {
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
            FROM profile_tool cst
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
        // Get or create default profile
        let profile_id = Self::get_or_create_default_profile(pool).await?;

        // Get server ID
        let server = crate::config::server::get_server(pool, server_name).await?;
        let server_id = server
            .and_then(|s| s.id)
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

        // Add tool to profile
        crate::config::profile::add_tool_to_profile(pool, &profile_id, &server_id, tool_name, true).await
    }

    /// Get or create default profile
    async fn get_or_create_default_profile(pool: &sqlx::Pool<sqlx::Sqlite>) -> anyhow::Result<String> {
        // Try to get default profile
        if let Some(profile) = crate::config::profile::get_default_profile(pool).await? {
            return Ok(profile.id.unwrap());
        }

        // Create new default anchor profile when none exists
        let mut new_profile = crate::config::models::Profile::new_with_description(
            DEFAULT_ANCHOR_INITIAL_NAME.to_string(),
            Some(DEFAULT_PROFILE_DESCRIPTION.to_string()),
            crate::common::profile::ProfileType::Shared,
        );
        new_profile.is_active = true;
        new_profile.is_default = true;
        new_profile.multi_select = true;
        new_profile.role = DEFAULT_ANCHOR_ROLE;

        crate::config::profile::upsert_profile(pool, &new_profile).await
    }
}

/// Common query builder for enabled tools from active profile
/// This helper reduces code duplication across the codebase
pub fn build_enabled_tools_query(additional_where: Option<&str>) -> String {
    let base_query = r#"
        SELECT DISTINCT st.unique_name, st.server_name, st.tool_name, st.server_id
        FROM profile_tool cst
        JOIN profile cs ON cst.profile_id = cs.id
        JOIN server_tools st ON cst.server_tool_id = st.id
        JOIN server_config sc ON st.server_id = sc.id
        WHERE cs.is_active = true
          AND cst.enabled = true
          AND sc.enabled = 1"#;

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
            cst.profile_id,
            cst.server_tool_id,
            cst.enabled,
            cst.created_at,
            cst.updated_at,
            st.server_id,
            st.server_name,
            st.tool_name,
            st.unique_name,
            st.description
        FROM profile_tool cst
        JOIN server_tools st ON cst.server_tool_id = st.id"#;

    match additional_where {
        Some(condition) => format!("{} WHERE {}", base_query, condition),
        None => base_query.to_string(),
    }
}

/// Get all tools for a profile from the database (new architecture)
pub async fn get_profile_tools(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<ProfileToolWithDetails>> {
    tracing::debug!("Executing SQL query to get tools for profile with ID {}", profile_id);

    let query = format!(
        "{} WHERE cst.profile_id = ? ORDER BY st.server_name, st.tool_name",
        build_tool_details_query(None)
    );

    let tools = sqlx::query_as::<_, ProfileToolWithDetails>(&query)
        .bind(profile_id)
        .fetch_all(pool)
        .await
        .context("Failed to fetch profile tools")?;

    tracing::debug!(
        "Successfully fetched {} tools for profile with ID {}",
        tools.len(),
        profile_id
    );
    Ok(tools)
}

/// Add a tool to a profile (new architecture)
///
/// This function adds a tool to a profile using the new architecture.
/// It first ensures the server tool mapping exists in server_tools table,
/// then creates the profile association.
/// If the tool is added or updated, it also publishes a ToolEnabledInProfileChanged event.
pub async fn add_tool_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    tool_name: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding tool '{}' from server ID {} to profile ID {}, enabled: {}",
        tool_name,
        server_id,
        profile_id,
        enabled
    );

    // First, ensure the server tool mapping exists in server_tools table
    let server_name = crate::config::operations::server::get_server_name_safe(pool, server_id)
        .await
        .context("Failed to get server name")?;

    let original_tool_name = if let Some(existing) =
        crate::config::server::tools::get_server_tool_by_unique_name(pool, tool_name)
            .await
            .context("Failed to lookup tool by unique name")?
    {
        if existing.server_id == server_id {
            existing.tool_name
        } else {
            tool_name.to_string()
        }
    } else if let Some(stripped) = strip_server_prefix(NamingKind::Tool, &server_name, tool_name) {
        stripped
    } else {
        tool_name.to_string()
    };

    let server_tool_id = if let Some(existing_tool) =
        crate::config::server::tools::get_server_tool(pool, server_id, &original_tool_name)
            .await
            .context("Failed to check existing server tool")?
    {
        existing_tool.id
    } else {
        crate::config::server::tools::upsert_server_tool(
            pool,
            server_id,
            &server_name,
            &original_tool_name,
            None, // description will be updated during tool sync
            None,
        )
        .await
        .context("Failed to upsert server tool")?
        .tool_id
    };

    // Check if the tool already exists in the profile
    let existing_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM profile_tool
        WHERE profile_id = ? AND server_tool_id = ?
        "#,
    )
    .bind(profile_id)
    .bind(&server_tool_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get existing tool enabled status")?;

    // Generate an ID for the profile tool association
    let profile_tool_id = generate_id!("cstool");

    let result = sqlx::query(
        r#"
        INSERT INTO profile_tool (id, profile_id, server_tool_id, enabled)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(profile_id, server_tool_id) DO UPDATE SET
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&profile_tool_id)
    .bind(profile_id)
    .bind(&server_tool_id)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add tool to profile")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        profile_tool_id.clone()
    } else {
        // If no rows were affected, get the existing ID
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM profile_tool
            WHERE profile_id = ? AND server_tool_id = ?
            "#,
        )
        .bind(profile_id)
        .bind(&server_tool_id)
        .fetch_one(pool)
        .await
        .context("Failed to get profile tool association ID")?
    };

    // Publish event if the tool is new or its enabled status has changed
    if is_new || (existing_enabled != Some(enabled)) {
        // Publish the event
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ToolEnabledInProfileChanged {
            tool_id: id_to_return.clone(),
            tool_name: tool_name.to_string(),
            profile_id: profile_id.to_string(),
            enabled,
        });

        tracing::debug!(
            "Published ToolEnabledInProfileChanged event for tool '{}' in profile ID {} ({})",
            tool_name,
            profile_id,
            enabled
        );
    }

    Ok(id_to_return)
}

/// Remove a tool from a profile (new architecture)
pub async fn remove_tool_from_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    tool_name: &str,
) -> Result<bool> {
    tracing::debug!(
        "Removing tool '{}' from server ID {} from profile ID {}",
        tool_name,
        server_id,
        profile_id
    );

    // First, find the server_tool_id
    let server_tool_id =
        sqlx::query_scalar::<_, String>("SELECT id FROM server_tools WHERE server_id = ? AND tool_name = ?")
            .bind(server_id)
            .bind(tool_name)
            .fetch_optional(pool)
            .await
            .context("Failed to find server tool")?;

    if let Some(server_tool_id) = server_tool_id {
        let result = sqlx::query(
            r#"
            DELETE FROM profile_tool
            WHERE profile_id = ? AND server_tool_id = ?
            "#,
        )
        .bind(profile_id)
        .bind(&server_tool_id)
        .execute(pool)
        .await
        .context("Failed to remove tool from profile")?;

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

/// Update tool enabled status in a profile by profile_tool id
/// Publishes ToolEnabledInProfileChanged when status changes
pub async fn update_tool_enabled_status(
    pool: &Pool<Sqlite>,
    profile_tool_id: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    tracing::debug!(
        "Updating tool enabled status: id={}, enabled={}",
        profile_tool_id,
        enabled
    );

    // Fetch context for event publishing
    let (tool_name, profile_id): (String, String) = sqlx::query_as(
        r#"
        SELECT st.tool_name, cst.profile_id
        FROM profile_tool cst
        JOIN server_tools st ON cst.server_tool_id = st.id
        WHERE cst.id = ?
        "#,
    )
    .bind(profile_tool_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get tool info for event publishing")?
    .ok_or_else(|| anyhow::anyhow!("Tool association not found: {}", profile_tool_id))?;

    let result = sqlx::query(
        r#"
        UPDATE profile_tool
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(enabled)
    .bind(profile_tool_id)
    .execute(pool)
    .await
    .context("Failed to update tool enabled status")?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("No rows updated for tool id {}", profile_tool_id));
    }

    crate::core::events::EventBus::global().publish(crate::core::events::Event::ToolEnabledInProfileChanged {
        tool_id: profile_tool_id.to_string(),
        tool_name,
        profile_id,
        enabled,
    });

    Ok(())
}
