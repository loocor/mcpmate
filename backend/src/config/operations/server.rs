// Server operations for database
// Contains common server-related database operations

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

use crate::common::database::fetch_scalar;

/// Get server name by server ID with underscore replacement for safe usage
///
/// This function retrieves the server name from the database and replaces spaces
/// with underscores to make it safe for use in various contexts.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID to look up
///
/// # Returns
/// * `Ok(server_name)` - Server name with spaces replaced by underscores
/// * `Err(_)` - If server not found or database error
pub async fn get_server_name_safe(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<String> {
    tracing::debug!("Getting safe server name for server ID: {}", server_id);

    let server_name: Option<String> = fetch_scalar(pool, "server_config", "name", "id", server_id).await?;

    match server_name {
        Some(name) => {
            let safe_name = name.replace(' ', "_"); // Replace spaces with underscores
            tracing::debug!(
                "Found server name '{}' for ID {}, safe name: '{}'",
                name,
                server_id,
                safe_name
            );
            Ok(safe_name)
        }
        None => {
            tracing::warn!("Server ID {} not found, using 'unknown' as server_name", server_id);
            Ok("unknown".to_string())
        }
    }
}

/// Get original server name by server ID without any modifications
///
/// This function retrieves the original server name from the database without
/// any modifications like underscore replacement.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID to look up
///
/// # Returns
/// * `Ok(server_name)` - Original server name
/// * `Err(_)` - If server not found or database error
pub async fn get_server_name_original(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<String> {
    tracing::debug!("Getting original server name for server ID: {}", server_id);

    let server_name = sqlx::query_scalar::<_, String>(
        r#"
        SELECT name FROM server_config
        WHERE id = ?
        "#,
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get server name")?;

    match server_name {
        Some(name) => {
            tracing::debug!("Found server name '{}' for ID {}", name, server_id);
            Ok(name)
        }
        None => {
            let error_msg = format!("Server with ID '{}' not found", server_id);
            tracing::error!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }
}

/// Check if a server exists in the database
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID to check
///
/// # Returns
/// * `Ok(true)` - Server exists
/// * `Ok(false)` - Server does not exist
/// * `Err(_)` - Database error
pub async fn server_exists(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<bool> {
    tracing::debug!("Checking if server exists: {}", server_id);

    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(SELECT 1 FROM server_config WHERE id = ?)
        "#,
    )
    .bind(server_id)
    .fetch_one(pool)
    .await
    .context("Failed to check if server exists")?;

    tracing::debug!("Server {} exists: {}", server_id, exists);
    Ok(exists)
}
