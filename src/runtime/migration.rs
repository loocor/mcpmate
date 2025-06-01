//! Runtime configuration migration utilities
//!
//! This module provides functions for migrating runtime configurations
//! to ensure consistent path formats.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tracing;

use super::constants::get_mcpmate_dir;

/// Runtime configuration row from database
#[derive(Debug, sqlx::FromRow)]
struct RuntimeConfigRow {
    id: i64,
    runtime_type: String,
    version: String,
    relative_bin_path: String,
}

/// Migrate runtime configurations to ensure consistent path formats
pub async fn migrate_runtime_configs(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::info!("Migrating runtime configurations to ensure consistent path formats");

    // Get all runtime configurations
    // Use query_as to avoid compile-time verification
    let configs = sqlx::query_as::<_, RuntimeConfigRow>(
        r#"
        SELECT id, runtime_type, version, relative_bin_path
        FROM runtime_config
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch runtime configurations")?;

    tracing::info!("Found {} runtime configurations to check", configs.len());

    // Get the MCPMate directory
    let mcpmate_dir = get_mcpmate_dir()?;
    let mcpmate_dir_str = mcpmate_dir.to_string_lossy().to_string();

    // Iterate through configurations and fix paths
    for config in configs {
        let id = config.id;
        let runtime_type = config.runtime_type;
        let version = config.version;

        // Check and fix bin path
        let bin_path = config.relative_bin_path;
        let mut new_bin_path = bin_path.clone();

        // If path doesn't start with .mcpmate, fix it
        if !bin_path.starts_with(".mcpmate/") {
            // If it's an absolute path, convert to relative
            if bin_path.starts_with("/") {
                if let Some(rel_path) = bin_path.strip_prefix(&mcpmate_dir_str) {
                    new_bin_path = format!(".mcpmate/{}", rel_path);
                } else {
                    // Try to construct a standard path based on runtime type and version
                    new_bin_path = format!(".mcpmate/runtimes/{}/{}/bin", runtime_type, version);
                }
            } else if bin_path.starts_with("runtimes/") {
                // If it starts with runtimes/, add .mcpmate/ prefix
                new_bin_path = format!(".mcpmate/{}", bin_path);
            } else {
                // Use standard format
                new_bin_path = format!(".mcpmate/runtimes/{}/{}/bin", runtime_type, version);
            }

            tracing::info!(
                "Updating bin path for {} {}: '{}' -> '{}'",
                runtime_type,
                version,
                bin_path,
                new_bin_path
            );
        }

        // Update the database if path changed
        if bin_path != new_bin_path {
            // Use query to avoid compile-time verification
            sqlx::query(
                r#"
                UPDATE runtime_config
                SET relative_bin_path = ?, updated_at = CURRENT_TIMESTAMP
                WHERE id = ?
                "#,
            )
            .bind(&new_bin_path)
            .bind(id)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to update runtime config for {}", id))?;

            tracing::info!("Updated runtime config for {} {}", runtime_type, version);
        }
    }

    tracing::info!("Runtime configuration migration completed successfully");
    Ok(())
}
