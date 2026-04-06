// Server association operations for Profile
// Contains operations for managing server associations with profile

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    common::constants::database::{columns, tables},
    config::models::ProfileServer,
    generate_id,
};

/// Get all servers for a profile from the database
pub async fn get_profile_servers(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<ProfileServer>> {
    tracing::trace!("Executing SQL query to get servers for profile with ID {}", profile_id);

    let servers = sqlx::query_as::<_, ProfileServer>(&format!(
        r#"
        SELECT * FROM {}
        WHERE {} = ?
        ORDER BY {}
        "#,
        tables::PROFILE_SERVER,
        columns::PROFILE_ID,
        columns::SERVER_ID
    ))
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch profile servers")?;

    tracing::trace!(
        "Successfully fetched {} servers for profile with ID {}",
        servers.len(),
        profile_id
    );
    Ok(servers)
}

/// Add a server to a profile in the database
///
/// This function adds a server to a profile in the database.
/// If the server is added or updated, it also publishes a ServerEnabledInProfileChanged event.
pub async fn add_server_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    enabled: bool,
) -> Result<String> {
    tracing::debug!(
        "Adding server ID {} to profile ID {}, enabled: {}",
        server_id,
        profile_id,
        enabled
    );

    // Generate an ID for the association
    let association_id = generate_id!("ssrv");

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
            tracing::warn!("Server ID {} not found, using 'unknown' as server_name", server_id);
            "unknown".to_string()
        }
    };

    // Check if the server already exists in the profile and get its current enabled status
    let existing_enabled = sqlx::query_scalar::<_, bool>(&format!(
        r#"
        SELECT {} FROM {}
        WHERE {} = ? AND {} = ?
        "#,
        columns::ENABLED,
        tables::PROFILE_SERVER,
        columns::PROFILE_ID,
        columns::SERVER_ID
    ))
    .bind(profile_id)
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get existing server enabled status")?;

    let result = sqlx::query(&format!(
        r#"
        INSERT INTO {} ({}, {}, {}, {}, {})
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT({}, {}) DO UPDATE SET
            {} = excluded.{},
            {} = excluded.{},
            {} = CURRENT_TIMESTAMP
        "#,
        tables::PROFILE_SERVER,
        columns::ID,
        columns::PROFILE_ID,
        columns::SERVER_ID,
        columns::SERVER_NAME,
        columns::ENABLED,
        columns::PROFILE_ID,
        columns::SERVER_ID,
        columns::SERVER_NAME,
        columns::SERVER_NAME,
        columns::ENABLED,
        columns::ENABLED,
        columns::UPDATED_AT
    ))
    .bind(&association_id)
    .bind(profile_id)
    .bind(server_id)
    .bind(&server_name)
    .bind(enabled)
    .execute(pool)
    .await
    .context("Failed to add server to profile")?;

    let is_new = result.rows_affected() > 0;
    let id_to_return = if is_new {
        association_id.clone()
    } else {
        // If no rows were affected, get the existing ID
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM profile_server
            WHERE profile_id = ? AND server_id = ?
            "#,
        )
        .bind(profile_id)
        .bind(server_id)
        .fetch_one(pool)
        .await
        .context("Failed to get profile server association ID")?
    };

    // Publish event if the server is new or its enabled status has changed
    if is_new || (existing_enabled != Some(enabled)) {
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
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerEnabledInProfileChanged {
            server_id: server_id.to_string(),
            server_name: original_server_name,
            profile_id: profile_id.to_string(),
            enabled,
        });

        // tracing::info!(
        //     "Published ServerEnabledInProfileChanged event for server ID {} in profile ID {} ({})",
        //     server_id,
        //     profile_id,
        //     enabled
        // );
    }

    Ok(id_to_return)
}

/// Remove a server from a profile in the database
pub async fn remove_server_from_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
) -> Result<bool> {
    tracing::debug!("Removing server ID {} from profile ID {}", server_id, profile_id);

    let result = sqlx::query(&format!(
        r#"
        DELETE FROM {}
        WHERE {} = ? AND {} = ?
        "#,
        tables::PROFILE_SERVER,
        columns::PROFILE_ID,
        columns::SERVER_ID
    ))
    .bind(profile_id)
    .bind(server_id)
    .execute(pool)
    .await
    .context("Failed to remove server from profile")?;

    Ok(result.rows_affected() > 0)
}

/// Server capability synchronization actions
#[derive(Debug, Clone)]
pub enum ServerCapabilityAction {
    /// Add server: create all capabilities (disabled by default)
    Add,
    /// Enable server: enable all existing capabilities
    Enable,
    /// Disable server: disable all capabilities
    Disable,
    /// Remove server: delete all capabilities
    Remove,
}

/// Unified server capabilities synchronization function
/// Handles all capability management operations (add, enable, disable, remove) in one place
pub async fn sync_server_capabilities(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    action: ServerCapabilityAction,
) -> Result<()> {
    tracing::debug!(
        "Syncing server {} capabilities in profile {} with action: {:?}",
        server_id,
        profile_id,
        action
    );

    match action {
        ServerCapabilityAction::Add => add_server_capabilities_to_profile(pool, profile_id, server_id).await?,
        ServerCapabilityAction::Enable => {
            batch_server_capabilities_operation(pool, profile_id, server_id, CapabilityOperation::UpdateEnabled(true))
                .await?;
        }
        ServerCapabilityAction::Disable => {
            batch_server_capabilities_operation(pool, profile_id, server_id, CapabilityOperation::UpdateEnabled(false))
                .await?;
        }
        ServerCapabilityAction::Remove => {
            batch_server_capabilities_operation(pool, profile_id, server_id, CapabilityOperation::Delete).await?;
        }
    }

    // Always prune capability rows that lost their server association so profile metrics stay accurate.
    cleanup_orphan_capabilities(pool, profile_id).await?;

    Ok(())
}

/// Internal function to add server capabilities to a profile
/// Retrieves all capabilities from server and creates profile associations (disabled by default)
async fn add_server_capabilities_to_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
) -> Result<()> {
    tracing::debug!(
        "Starting capability sync for server ID {} to profile ID {}",
        server_id,
        profile_id
    );

    // Check individual capability types to ensure all are properly synced
    let existing_tools_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM profile_tool cst
         JOIN server_tools st ON cst.server_tool_id = st.id
         WHERE cst.profile_id = ? AND st.server_id = ?",
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let existing_resources_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM profile_resource
         WHERE profile_id = ? AND server_id = ?",
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let existing_prompts_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM profile_prompt
         WHERE profile_id = ? AND server_id = ?",
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let existing_templates_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM profile_resource_template
         WHERE profile_id = ? AND server_id = ?",
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    tracing::debug!(
        "Capability sync status for server {} in profile {}: {} tools, {} resources, {} prompts, {} templates existing",
        server_id,
        profile_id,
        existing_tools_count,
        existing_resources_count,
        existing_prompts_count,
        existing_templates_count
    );

    let mut tools_added = 0_u64;
    let mut resources_added = 0_u64;
    let mut prompts_added = 0_u64;
    let mut templates_added = 0_u64;

    // 1. Sync Tools from server_tools (only if not already synced)
    if existing_tools_count == 0 {
        let server_tools = sqlx::query_as::<_, (String,)>("SELECT tool_name FROM server_tools WHERE server_id = ?")
            .bind(server_id)
            .fetch_all(pool)
            .await
            .context("Failed to fetch server tools")?;

        for (tool_name,) in server_tools {
            match crate::config::profile::add_tool_to_profile(
                pool, profile_id, server_id, &tool_name, false, // disabled by default
            )
            .await
            {
                Ok(_) => tools_added += 1,
                Err(e) => tracing::warn!("Failed to add tool {} to profile {}: {}", tool_name, profile_id, e),
            }
        }
    } else {
        tracing::debug!(
            "Tools already synced for server {} in profile {}, skipping",
            server_id,
            profile_id
        );
    }

    // 2. Sync Resources from server_resources (if table exists and not already synced)
    if existing_resources_count == 0 {
        let server_resources_result =
            sqlx::query_as::<_, (String,)>("SELECT resource_uri FROM server_resources WHERE server_id = ? LIMIT 100")
                .bind(server_id)
                .fetch_all(pool)
                .await;

        if let Ok(server_resources) = server_resources_result {
            for (resource_uri,) in server_resources {
                match crate::config::profile::add_resource_to_profile(
                    pool,
                    profile_id,
                    server_id,
                    &resource_uri,
                    false, // disabled by default
                )
                .await
                {
                    Ok(_) => resources_added += 1,
                    Err(e) => tracing::warn!(
                        "Failed to add resource {} to profile {}: {}",
                        resource_uri,
                        profile_id,
                        e
                    ),
                }
            }
        } else {
            // No server_resources table or no resources found - this is normal
            tracing::debug!(
                "No server resources table found or no resources for server {}",
                server_id
            );
        }
    } else {
        tracing::debug!(
            "Resources already synced for server {} in profile {}, skipping",
            server_id,
            profile_id
        );
    }

    // 3. Sync Prompts from server_prompts (if table exists and not already synced)
    if existing_prompts_count == 0 {
        let server_prompts_result =
            sqlx::query_as::<_, (String,)>("SELECT prompt_name FROM server_prompts WHERE server_id = ? LIMIT 100")
                .bind(server_id)
                .fetch_all(pool)
                .await;

        if let Ok(server_prompts) = server_prompts_result {
            for (prompt_name,) in server_prompts {
                match crate::config::profile::add_prompt_to_profile(
                    pool,
                    profile_id,
                    server_id,
                    &prompt_name,
                    false, // disabled by default
                )
                .await
                {
                    Ok(_) => prompts_added += 1,
                    Err(e) => tracing::warn!("Failed to add prompt {} to profile {}: {}", prompt_name, profile_id, e),
                }
            }
        } else {
            // No server_prompts table or no prompts found - this is normal
            tracing::debug!("No server prompts table found or no prompts for server {}", server_id);
        }
    } else {
        tracing::debug!(
            "Prompts already synced for server {} in profile {}, skipping",
            server_id,
            profile_id
        );
    }

    // 4. Sync Resource Templates from server_resource_templates (if not already synced)
    if existing_templates_count == 0 {
        let server_templates_result = sqlx::query_as::<_, (String,)>(
            "SELECT uri_template FROM server_resource_templates WHERE server_id = ? LIMIT 100",
        )
        .bind(server_id)
        .fetch_all(pool)
        .await;

        if let Ok(server_templates) = server_templates_result {
            for (uri_template,) in server_templates {
                match crate::config::profile::add_resource_template_to_profile(
                    pool,
                    profile_id,
                    server_id,
                    &uri_template,
                    false,
                )
                .await
                {
                    Ok(_) => templates_added += 1,
                    Err(e) => tracing::warn!(
                        "Failed to add resource template {} to profile {}: {}",
                        uri_template,
                        profile_id,
                        e
                    ),
                }
            }
        }
    } else {
        tracing::debug!(
            "Resource templates already synced for server {} in profile {}, skipping",
            server_id,
            profile_id
        );
    }

    tracing::info!(
        "Successfully synced capabilities for server {} to profile {}: {} tools, {} resources, {} prompts added (disabled by default)",
        server_id,
        profile_id,
        tools_added,
        resources_added,
        prompts_added
    );

    if templates_added > 0 {
        tracing::info!(
            "Resource templates added for server {} in profile {}: {}",
            server_id,
            profile_id,
            templates_added
        );
    }

    Ok(())
}

/// Capability operation types for batch processing
#[derive(Debug, Clone)]
enum CapabilityOperation {
    UpdateEnabled(bool),
    Delete,
}

/// Batch operation results for different capability types
#[derive(Debug)]
struct CapabilityOperationResults {
    tools_affected: u64,
    resources_affected: u64,
    prompts_affected: u64,
}

/// Remove any capability rows that no longer have a matching server association in the profile.
async fn cleanup_orphan_capabilities(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<()> {
    // Remove orphaned tools first (missing matching profile_server/server_tools association)
    let orphan_tools = sqlx::query(
        r#"
        DELETE FROM profile_tool
        WHERE profile_id = ?
          AND NOT EXISTS (
              SELECT 1
              FROM server_tools st
              JOIN profile_server ps
                ON ps.profile_id = profile_tool.profile_id
               AND ps.server_id = st.server_id
              WHERE st.id = profile_tool.server_tool_id
          )
        "#,
    )
    .bind(profile_id)
    .execute(pool)
    .await
    .context("Failed to delete orphaned profile tools")?;

    // Remove orphaned resources (no corresponding server association)
    let orphan_resources = sqlx::query(
        r#"
        DELETE FROM profile_resource
        WHERE profile_id = ?
          AND NOT EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_resource.profile_id
                AND ps.server_id = profile_resource.server_id
          )
        "#,
    )
    .bind(profile_id)
    .execute(pool)
    .await
    .context("Failed to delete orphaned profile resources")?;

    // Remove orphaned prompts (no corresponding server association)
    let orphan_prompts = sqlx::query(
        r#"
        DELETE FROM profile_prompt
        WHERE profile_id = ?
          AND NOT EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_prompt.profile_id
                AND ps.server_id = profile_prompt.server_id
          )
        "#,
    )
    .bind(profile_id)
    .execute(pool)
    .await
    .context("Failed to delete orphaned profile prompts")?;

    // Remove orphaned resource templates (no corresponding server association)
    let orphan_templates = sqlx::query(
        r#"
        DELETE FROM profile_resource_template
        WHERE profile_id = ?
          AND NOT EXISTS (
              SELECT 1
              FROM profile_server ps
              WHERE ps.profile_id = profile_resource_template.profile_id
                AND ps.server_id = profile_resource_template.server_id
          )
        "#,
    )
    .bind(profile_id)
    .execute(pool)
    .await
    .context("Failed to delete orphaned profile resource templates")?;

    let total_removed = orphan_tools.rows_affected()
        + orphan_resources.rows_affected()
        + orphan_prompts.rows_affected()
        + orphan_templates.rows_affected();

    if total_removed > 0 {
        tracing::info!(
            "Cleaned up {} orphaned capability records for profile {} ({} tools, {} resources, {} prompts, {} templates)",
            total_removed,
            profile_id,
            orphan_tools.rows_affected(),
            orphan_resources.rows_affected(),
            orphan_prompts.rows_affected(),
            orphan_templates.rows_affected()
        );
    }

    Ok(())
}

/// Generic batch operations on server capabilities within a profile
/// Handles tools, resources, and prompts uniformly with transaction safety
async fn batch_server_capabilities_operation(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
    operation: CapabilityOperation,
) -> Result<CapabilityOperationResults> {
    let operation_name = match &operation {
        CapabilityOperation::UpdateEnabled(enabled) => format!("update enabled={}", enabled),
        CapabilityOperation::Delete => "delete".to_string(),
    };

    tracing::debug!(
        "Batch {} capabilities for server {} in profile {}",
        operation_name,
        server_id,
        profile_id
    );

    // Start a transaction for consistency
    let mut tx = pool.begin().await.context("Failed to start transaction")?;

    // Execute operation based on type
    let (tools_result, resources_result, prompts_result) = match operation {
        CapabilityOperation::UpdateEnabled(enabled) => {
            // Update tools: profile_tool -> server_tools -> server_id
            let tools = sqlx::query(
                r#"
                UPDATE profile_tool
                SET enabled = ?, updated_at = CURRENT_TIMESTAMP
                WHERE profile_id = ? AND server_tool_id IN (
                    SELECT id FROM server_tools WHERE server_id = ?
                )
                "#,
            )
            .bind(enabled)
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .context("Failed to update tools enabled status")?;

            // Update resources: direct server_id reference
            let resources = sqlx::query(
                r#"
                UPDATE profile_resource
                SET enabled = ?, updated_at = CURRENT_TIMESTAMP
                WHERE profile_id = ? AND server_id = ?
                "#,
            )
            .bind(enabled)
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .context("Failed to update resources enabled status")?;

            // Update prompts: direct server_id reference
            let prompts = sqlx::query(
                r#"
                UPDATE profile_prompt
                SET enabled = ?, updated_at = CURRENT_TIMESTAMP
                WHERE profile_id = ? AND server_id = ?
                "#,
            )
            .bind(enabled)
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .context("Failed to update prompts enabled status")?;

            // Update resource templates: direct server_id reference
            let _templates = sqlx::query(
                r#"
                UPDATE profile_resource_template
                SET enabled = ?, updated_at = CURRENT_TIMESTAMP
                WHERE profile_id = ? AND server_id = ?
                "#,
            )
            .bind(enabled)
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .context("Failed to update resource templates enabled status")?;

            (tools, resources, prompts)
        }
        CapabilityOperation::Delete => {
            // Delete tools: profile_tool -> server_tools -> server_id
            let tools = sqlx::query(
                r#"
                DELETE FROM profile_tool
                WHERE profile_id = ? AND server_tool_id IN (
                    SELECT id FROM server_tools WHERE server_id = ?
                )
                "#,
            )
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete tools from profile")?;

            // Delete resources: direct server_id reference
            let resources = sqlx::query(
                r#"
                DELETE FROM profile_resource
                WHERE profile_id = ? AND server_id = ?
                "#,
            )
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete resources from profile")?;

            // Delete prompts: direct server_id reference
            let prompts = sqlx::query(
                r#"
                DELETE FROM profile_prompt
                WHERE profile_id = ? AND server_id = ?
                "#,
            )
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete prompts from profile")?;

            // Delete resource templates: direct server_id reference
            let _templates = sqlx::query(
                r#"
                DELETE FROM profile_resource_template
                WHERE profile_id = ? AND server_id = ?
                "#,
            )
            .bind(profile_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete resource templates from profile")?;

            (tools, resources, prompts)
        }
    };

    // Commit transaction
    tx.commit().await.context("Failed to commit transaction")?;

    let results = CapabilityOperationResults {
        tools_affected: tools_result.rows_affected(),
        resources_affected: resources_result.rows_affected(),
        prompts_affected: prompts_result.rows_affected(),
    };

    tracing::info!(
        "Successfully {} capabilities for server {} in profile {}: {} tools, {} resources, {} prompts affected",
        operation_name,
        server_id,
        profile_id,
        results.tools_affected,
        results.resources_affected,
        results.prompts_affected
    );

    Ok(results)
}

/// Check whether any capability on the server remains enabled in the profile
async fn server_has_enabled_capabilities(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
) -> Result<bool> {
    // Check enabled tools
    let tools_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM profile_tool pt
            JOIN server_tools st ON st.id = pt.server_tool_id
            WHERE pt.profile_id = ?
              AND st.server_id = ?
              AND pt.enabled = 1
        )
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .context("Failed to check enabled tools for server")?;

    if tools_enabled {
        return Ok(true);
    }

    // Check enabled resources
    let resources_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM profile_resource
            WHERE profile_id = ?
              AND server_id = ?
              AND enabled = 1
        )
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .context("Failed to check enabled resources for server")?;

    if resources_enabled {
        return Ok(true);
    }

    // Check enabled prompts
    let prompts_enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM profile_prompt
            WHERE profile_id = ?
              AND server_id = ?
              AND enabled = 1
        )
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    .context("Failed to check enabled prompts for server")?;

    if prompts_enabled {
        return Ok(true);
    }

    // Check enabled resource templates; if the table doesn't exist yet, treat as no templates
    let templates_enabled = match sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM profile_resource_template
            WHERE profile_id = ?
              AND server_id = ?
              AND enabled = 1
        )
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            // Gracefully degrade when the table is not present (pre‑migration or test harness schema)
            let msg = e.to_string();
            if msg.contains("no such table: profile_resource_template") {
                tracing::debug!("profile_resource_template table missing; assuming no enabled templates");
                false
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to check enabled resource templates for server: {e}"
                ));
            }
        }
    };

    Ok(templates_enabled)
}

/// Ensure the server is enabled in the profile when any capability becomes active
pub async fn ensure_server_enabled_for_profile(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
) -> Result<bool> {
    let current = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM profile_server
        WHERE profile_id = ? AND server_id = ?
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch current server enabled status")?;

    if current == Some(true) {
        return Ok(false);
    }

    let _ = add_server_to_profile(pool, profile_id, server_id, true)
        .await
        .context("Failed to enable server for profile")?;

    Ok(true)
}

/// Disable the server if all of its capabilities are currently disabled
pub async fn disable_server_if_all_capabilities_disabled(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    server_id: &str,
) -> Result<bool> {
    let current = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM profile_server
        WHERE profile_id = ? AND server_id = ?
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch current server enabled status")?;

    if current != Some(true) {
        return Ok(false);
    }

    if server_has_enabled_capabilities(pool, profile_id, server_id).await? {
        return Ok(false);
    }

    let _ = add_server_to_profile(pool, profile_id, server_id, false)
        .await
        .context("Failed to disable server for profile")?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};
    use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};

    const PROFILE_ID: &str = "profile-1";
    const SERVER_ID: &str = "server-1";

    async fn setup_test_db() -> Result<Pool<Sqlite>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .context("Failed to create in-memory Sqlite pool")?;

        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;

        // Minimal schema required for capability/server linkage tests
        sqlx::query(
            r#"
            CREATE TABLE server_config (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE profile_server (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                server_id TEXT NOT NULL,
                server_name TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(profile_id, server_id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE server_tools (
                id TEXT PRIMARY KEY,
                server_id TEXT NOT NULL,
                server_name TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                unique_name TEXT,
                description TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE profile_tool (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                server_tool_id TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE profile_resource (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                server_id TEXT NOT NULL,
                server_name TEXT NOT NULL,
                resource_uri TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE profile_prompt (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                server_id TEXT NOT NULL,
                server_name TEXT NOT NULL,
                prompt_name TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    async fn seed_server(pool: &Pool<Sqlite>) -> Result<()> {
        sqlx::query("INSERT INTO server_config (id, name, enabled) VALUES (?, ?, 0)")
            .bind(SERVER_ID)
            .bind("Demo Server")
            .bind(0)
            .execute(pool)
            .await?;

        Ok(())
    }

    async fn seed_profile_server(
        pool: &Pool<Sqlite>,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO profile_server (id, profile_id, server_id, server_name, enabled) VALUES (?, ?, ?, ?, ?)",
        )
        .bind("assoc-1")
        .bind(PROFILE_ID)
        .bind(SERVER_ID)
        .bind("Demo_Server")
        .bind(if enabled { 1 } else { 0 })
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn seed_tool(
        pool: &Pool<Sqlite>,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query("INSERT INTO server_tools (id, server_id, server_name, tool_name) VALUES (?, ?, ?, ?)")
            .bind("tool-1")
            .bind(SERVER_ID)
            .bind("Demo_Server")
            .bind("demo-tool")
            .execute(pool)
            .await?;

        sqlx::query("INSERT INTO profile_tool (id, profile_id, server_tool_id, enabled) VALUES (?, ?, ?, ?)")
            .bind("pt-1")
            .bind(PROFILE_ID)
            .bind("tool-1")
            .bind(if enabled { 1 } else { 0 })
            .execute(pool)
            .await?;

        Ok(())
    }

    async fn seed_resource(
        pool: &Pool<Sqlite>,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO profile_resource (id, profile_id, server_id, server_name, resource_uri, enabled) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("res-1")
        .bind(PROFILE_ID)
        .bind(SERVER_ID)
        .bind("Demo_Server")
        .bind("resource://one")
        .bind(if enabled { 1 } else { 0 })
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn read_server_enabled(pool: &Pool<Sqlite>) -> Result<bool> {
        sqlx::query_scalar::<_, bool>("SELECT enabled FROM profile_server WHERE profile_id = ? AND server_id = ?")
            .bind(PROFILE_ID)
            .bind(SERVER_ID)
            .fetch_one(pool)
            .await
            .context("Failed to fetch server enabled state")
    }

    #[tokio::test]
    async fn enabling_capability_reactivates_server() -> Result<()> {
        let pool = setup_test_db().await?;
        seed_server(&pool).await?;
        seed_profile_server(&pool, false).await?;
        seed_tool(&pool, false).await?;

        crate::config::profile::update_tool_enabled_status(&pool, "pt-1", true).await?;

        let server_enabled = read_server_enabled(&pool).await?;
        assert!(
            server_enabled,
            "Server should be enabled when a capability is activated"
        );

        Ok(())
    }

    #[tokio::test]
    async fn disabling_last_capability_turns_off_server() -> Result<()> {
        let pool = setup_test_db().await?;
        seed_server(&pool).await?;
        seed_profile_server(&pool, true).await?;
        seed_tool(&pool, true).await?;
        seed_resource(&pool, true).await?;

        // Disable resource first; server stays enabled because tool is still active
        let resource_changed = crate::config::profile::update_resource_enabled_status(
            &pool,
            PROFILE_ID,
            SERVER_ID,
            "resource://one",
            false,
        )
        .await?;
        assert!(resource_changed);

        let server_enabled = read_server_enabled(&pool).await?;
        assert!(
            server_enabled,
            "Server should remain enabled while other capabilities stay active"
        );

        // Disable the remaining tool; server should auto-disable now
        crate::config::profile::update_tool_enabled_status(&pool, "pt-1", false).await?;

        let server_enabled = read_server_enabled(&pool).await?;
        assert!(
            !server_enabled,
            "Server should disable when all capabilities are inactive"
        );

        Ok(())
    }
}
