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
    relative_cache_path: Option<String>,
}

/// Migrate runtime configurations to ensure consistent path formats
pub async fn migrate_runtime_configs(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::info!("Migrating runtime configurations to ensure consistent path formats");

    // Get all runtime configurations
    // Use query_as to avoid compile-time verification
    let configs = sqlx::query_as::<_, RuntimeConfigRow>(
        r#"
        SELECT id, runtime_type, version, relative_bin_path, relative_cache_path
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

        // Check and fix cache path
        let cache_path = config.relative_cache_path.clone();
        let mut new_cache_path = cache_path.clone();

        if let Some(ref cache) = cache_path {
            if !cache.starts_with(".mcpmate/") {
                // If it's an absolute path, convert to relative
                if cache.starts_with("/") {
                    if let Some(rel_path) = cache.strip_prefix(&mcpmate_dir_str) {
                        new_cache_path = Some(format!(".mcpmate/{}", rel_path));
                    } else {
                        // Use standard format
                        let cache_dir = match runtime_type.as_str() {
                            "node" => "npm",
                            "uv" => "uv",
                            "bun" => "bun",
                            _ => runtime_type.as_str(),
                        };
                        new_cache_path = Some(format!(".mcpmate/cache/{}", cache_dir));
                    }
                } else if cache.starts_with("cache/") {
                    // If it starts with cache/, add .mcpmate/ prefix
                    new_cache_path = Some(format!(".mcpmate/{}", cache));
                } else {
                    // Use standard format
                    let cache_dir = match runtime_type.as_str() {
                        "node" => "npm",
                        "uv" => "uv",
                        "bun" => "bun",
                        _ => runtime_type.as_str(),
                    };
                    new_cache_path = Some(format!(".mcpmate/cache/{}", cache_dir));
                }

                tracing::info!(
                    "Updating cache path for {} {}: '{}' -> '{}'",
                    runtime_type,
                    version,
                    cache,
                    new_cache_path.as_ref().unwrap()
                );
            }
        } else {
            // If cache path is missing, add it
            let cache_dir = match runtime_type.as_str() {
                "node" => "npm",
                "uv" => "uv",
                "bun" => "bun",
                _ => runtime_type.as_str(),
            };
            new_cache_path = Some(format!(".mcpmate/cache/{}", cache_dir));

            tracing::info!(
                "Adding missing cache path for {} {}: '{}'",
                runtime_type,
                version,
                new_cache_path.as_ref().unwrap()
            );
        }

        // Update the database if paths changed
        if bin_path != new_bin_path || cache_path != new_cache_path {
            // Use query to avoid compile-time verification
            sqlx::query(
                r#"
                UPDATE runtime_config
                SET relative_bin_path = ?, relative_cache_path = ?, updated_at = CURRENT_TIMESTAMP
                WHERE id = ?
                "#,
            )
            .bind(&new_bin_path)
            .bind(&new_cache_path)
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
