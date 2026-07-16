// Server operations for database
// Contains common server-related database operations

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

/// Get the persisted MCPMate namespace by server ID.
///
/// This function returns the canonical value stored in `server_config.name`.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID to look up
///
/// # Returns
/// * `Ok(server_name)` - Persisted MCPMate namespace
/// * `Err(_)` - If server not found or database error
pub async fn get_server_namespace(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<String> {
    tracing::debug!("Getting server namespace for server ID: {}", server_id);

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
