//! Runtime integration utilities for MCPMate
//!
//! This module provides integration functions for runtime management,
//! including database operations and event publishing for the main application.

use anyhow::{Context, Result};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::path::Path;

use super::config::{RuntimeConfig, save_config};
use super::types::RuntimeType;
use crate::common::paths::get_mcpmate_dir;
use crate::core::events::{Event, EventBus};

/// Send runtime events to the global event bus
pub fn send_runtime_event(event: Event) {
    let event_bus = EventBus::global();
    event_bus.publish(event);
}

/// Save runtime configuration to database after installation
pub async fn save_runtime_config_to_db(
    database_path: &str,
    runtime_type: RuntimeType,
    version: &str,
    install_path: &Path,
) -> Result<()> {
    // Connect to database
    let database_url = format!("sqlite:{}", database_path);

    // Ensure database exists
    if !Sqlite::database_exists(&database_url).await? {
        Sqlite::create_database(&database_url).await?;
    }

    let pool = SqlitePool::connect(&database_url).await?;

    // Note: runtime_config table is created during database initialization

    // Get relative path from MCPMate directory
    let mcpmate_dir = get_mcpmate_dir()?;

    // For Node.js, we need to point to the npx executable, not just the bin directory
    let executable_path = match runtime_type {
        RuntimeType::Node => {
            // Check if npx exists in the bin directory
            let npx_path = install_path.join("bin").join("npx");
            if npx_path.exists() {
                npx_path
            } else {
                // Fall back to node executable
                let node_path = install_path.join("bin").join("node");
                if node_path.exists() {
                    node_path
                } else {
                    install_path.to_path_buf()
                }
            }
        }
        RuntimeType::Uv => {
            // For uv, point to the uv executable
            let uv_path = install_path.join("bin").join("uv");
            if uv_path.exists() {
                uv_path
            } else {
                install_path.to_path_buf()
            }
        }
        RuntimeType::Bun => {
            // For bun, point to the bun executable
            let bun_path = install_path.join("bin").join("bun");
            if bun_path.exists() {
                bun_path
            } else {
                install_path.to_path_buf()
            }
        }
    };

    let relative_path = executable_path
        .strip_prefix(&mcpmate_dir)
        .unwrap_or(&executable_path)
        .to_string_lossy()
        .to_string();

    // Create runtime config with complete information
    let config = RuntimeConfig::new(runtime_type, version, &relative_path);

    // Save to database
    save_config(&pool, &config)
        .await
        .context("Failed to save runtime config to database")?;

    Ok(())
}

/// Send runtime events during download progress
pub fn send_download_progress_events(
    runtime_type: RuntimeType,
    stage: &crate::runtime::DownloadStage,
    version: Option<&str>,
) {
    let runtime_type_str = runtime_type.as_str().to_string();
    let version_str = version.unwrap_or("latest").to_string();

    match stage {
        crate::runtime::DownloadStage::Initializing => {
            send_runtime_event(Event::RuntimeCheckStarted {
                runtime_type: runtime_type_str,
                version: Some(version_str),
            });
        }
        crate::runtime::DownloadStage::Downloading => {
            send_runtime_event(Event::RuntimeDownloadStarted {
                runtime_type: runtime_type_str,
                version: version_str,
            });
        }
        crate::runtime::DownloadStage::Complete => {
            send_runtime_event(Event::RuntimeDownloadCompleted {
                runtime_type: runtime_type_str,
                version: version_str,
                install_path: "".to_string(), // Will be updated after installation
            });
        }
        crate::runtime::DownloadStage::Failed(error) => {
            send_runtime_event(Event::RuntimeSetupFailed {
                runtime_type: runtime_type_str,
                error: error.clone(),
            });
        }
        _ => {} // Other stages don't need events
    }
}

/// Send runtime ready event after successful installation
pub fn send_runtime_ready_event(
    runtime_type: RuntimeType,
    version: &str,
    bin_path: &Path,
) {
    send_runtime_event(Event::RuntimeReady {
        runtime_type: runtime_type.as_str().to_string(),
        version: version.to_string(),
        bin_path: bin_path.to_string_lossy().to_string(),
    });
}

/// Send runtime setup failed event
pub fn send_runtime_setup_failed_event(
    runtime_type: RuntimeType,
    error: &str,
) {
    send_runtime_event(Event::RuntimeSetupFailed {
        runtime_type: runtime_type.as_str().to_string(),
        error: error.to_string(),
    });
}
