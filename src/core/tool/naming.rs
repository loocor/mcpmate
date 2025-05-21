//! Tool naming module
//!
//! This module provides functions for generating and resolving unique tool names.
//! It implements a database-backed approach to handle tool name conflicts.
//!
//! The naming system follows these principles:
//! 1. Each tool has an original name (as provided by the upstream server)
//! 2. Each tool is assigned a unique name for external display and routing
//! 3. Name transformations only occur at API boundaries, not in internal logic

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

/// Generate a unique name for a tool
///
/// This function generates a unique name for a tool based on the server name and original tool name.
/// If the tool name already contains the server name as a prefix, it will be used as is.
/// Otherwise, the server name will be added as a prefix.
///
/// # Arguments
/// * `server_name` - The name of the server
/// * `original_tool_name` - The original name of the tool
///
/// # Returns
/// * `String` - The generated unique name
pub fn generate_unique_name(server_name: &str, original_tool_name: &str) -> String {
    // Normalize server name (lowercase, replace spaces with underscores)
    let normalized_server = server_name.to_lowercase().replace(' ', "_");
    
    // Check if the tool name already contains the server name as a prefix
    let server_prefix = format!("{}_", normalized_server);
    
    if original_tool_name.to_lowercase().starts_with(&server_prefix) {
        // Tool name already has the server name as a prefix, use it as is
        original_tool_name.to_string()
    } else {
        // Add the server name as a prefix
        format!("{}_{}", normalized_server, original_tool_name)
    }
}

/// Resolve a unique tool name to get the server name and original tool name
///
/// This function queries the database to find the server name and original tool name
/// associated with a unique tool name.
///
/// # Arguments
/// * `pool` - The database connection pool
/// * `unique_name` - The unique name to resolve
///
/// # Returns
/// * `Result<(String, String)>` - A tuple containing the server name and original tool name
pub async fn resolve_unique_name(
    pool: &Pool<Sqlite>,
    unique_name: &str,
) -> Result<(String, String)> {
    // Query the database to find the server name and original tool name
    let result = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT server_name, tool_name
        FROM config_suit_tool
        WHERE unique_name = ?
        "#
    )
    .bind(unique_name)
    .fetch_optional(pool)
    .await
    .context(format!("Failed to query unique name: {}", unique_name))?;
    
    match result {
        Some((server_name, tool_name)) => {
            tracing::debug!(
                "Resolved unique name '{}' -> server: '{}', tool: '{}'",
                unique_name,
                server_name,
                tool_name
            );
            Ok((server_name, tool_name))
        },
        None => {
            // If the unique name is not found in the database, return an error
            Err(anyhow::anyhow!(
                "Unique name '{}' not found in database",
                unique_name
            ))
        }
    }
}

/// Ensure a unique name doesn't conflict with existing names
///
/// This function checks if a generated unique name conflicts with existing names
/// in the database, and if so, adds a numeric suffix to make it unique.
///
/// # Arguments
/// * `pool` - The database connection pool
/// * `base_name` - The base unique name to check
/// * `server_id` - The server ID
/// * `tool_name` - The original tool name
///
/// # Returns
/// * `Result<String>` - A unique name that doesn't conflict with existing names
pub async fn ensure_unique_name(
    pool: &Pool<Sqlite>,
    base_name: &str,
    server_id: &str,
    tool_name: &str,
) -> Result<String> {
    // Check if the base name is already used by another tool
    let conflict = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM config_suit_tool
            WHERE unique_name = ?
            AND (server_id != ? OR tool_name != ?)
        )
        "#
    )
    .bind(base_name)
    .bind(server_id)
    .bind(tool_name)
    .fetch_one(pool)
    .await
    .context(format!("Failed to check for name conflicts: {}", base_name))?;
    
    if !conflict {
        // No conflict, use the base name
        return Ok(base_name.to_string());
    }
    
    // If there's a conflict, add a numeric suffix
    let mut counter = 1;
    loop {
        let suffixed_name = format!("{}_{}", base_name, counter);
        
        let conflict = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM config_suit_tool
                WHERE unique_name = ?
            )
            "#
        )
        .bind(&suffixed_name)
        .fetch_one(pool)
        .await
        .context(format!("Failed to check for name conflicts: {}", suffixed_name))?;
        
        if !conflict {
            // No conflict, use this name
            return Ok(suffixed_name);
        }
        
        // Increment counter and try again
        counter += 1;
        
        // Safety check to prevent infinite loops
        if counter > 1000 {
            return Err(anyhow::anyhow!(
                "Failed to generate a unique name after 1000 attempts"
            ));
        }
    }
}
