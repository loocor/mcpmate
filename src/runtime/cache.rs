//! Runtime Cache Module
//!
//! Provides simple in-memory caching for runtime paths to enable fast
//! runtime queries during stdio connections.

use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

use super::types::RuntimeType;

/// Simple in-memory cache for runtime paths
#[derive(Debug, Clone)]
pub struct RuntimeCache {
    paths: Arc<RwLock<HashMap<RuntimeType, PathBuf>>>,
}

impl RuntimeCache {
    /// Create a new empty runtime cache
    pub fn new() -> Self {
        Self {
            paths: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set a runtime path in cache
    pub async fn set_available(
        &self,
        runtime_type: RuntimeType,
        path: PathBuf,
    ) {
        let mut paths = self.paths.write().await;
        paths.insert(runtime_type, path);
        tracing::debug!("Runtime {:?} cached", runtime_type);
    }

    /// Remove a runtime from cache
    pub async fn remove(
        &self,
        runtime_type: RuntimeType,
    ) {
        let mut paths = self.paths.write().await;
        paths.remove(&runtime_type);
        tracing::debug!("Runtime {:?} removed from cache", runtime_type);
    }

    /// Get runtime path for a command (npx -> Node, uvx -> Uv, etc.)
    pub async fn get_runtime_for_command(
        &self,
        command: &str,
    ) -> Option<PathBuf> {
        let runtime_type = match command {
            "npx" => RuntimeType::Node,
            "uvx" => RuntimeType::Uv,
            "bunx" => RuntimeType::Bun,
            _ => return None,
        };

        let paths = self.paths.read().await;
        let base_path = paths.get(&runtime_type)?.clone();

        // For commands like npx, uvx, bunx, find the correct executable
        match command {
            "npx" => {
                let parent_dir = base_path.parent()?;

                if cfg!(windows) {
                    // Try .exe first, then .cmd
                    for ext in ["npx.exe", "npx.cmd"] {
                        let npx_path = parent_dir.join(ext);
                        if npx_path.exists() {
                            return Some(npx_path);
                        }
                    }
                } else {
                    let npx_path = parent_dir.join("npx");
                    if npx_path.exists() {
                        return Some(npx_path);
                    }
                }

                Some(base_path)
            }
            "uvx" | "bunx" => {
                let parent_dir = base_path.parent()?;
                let exe_name = if cfg!(windows) {
                    format!("{}.exe", command)
                } else {
                    command.to_string()
                };

                let exe_path = parent_dir.join(exe_name);
                if exe_path.exists() {
                    Some(exe_path)
                } else {
                    Some(base_path)
                }
            }
            _ => Some(base_path),
        }
    }

    /// Clear all cached paths
    pub async fn clear(&self) {
        let mut paths = self.paths.write().await;
        paths.clear();
        tracing::debug!("Runtime cache cleared");
    }

    /// Initialize runtime cache from database configurations
    pub async fn initialize_from_database(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> anyhow::Result<()> {
        use super::config::get_all_configs;
        use crate::common::paths::get_mcpmate_dir;

        tracing::info!("Initializing runtime cache from database...");

        let configs = match get_all_configs(pool).await {
            Ok(configs) => configs,
            Err(e) => {
                tracing::warn!("Failed to load runtime configs from database: {}", e);
                return Ok(());
            }
        };

        let mcpmate_dir = get_mcpmate_dir()?;
        let mut loaded_count = 0;

        for config in configs {
            let runtime_type = match config.get_runtime_type() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::warn!("Invalid runtime type '{}': {}", config.runtime_type, e);
                    continue;
                }
            };

            // Construct full path from relative path
            let full_path = if config.relative_bin_path.starts_with("/") {
                PathBuf::from(&config.relative_bin_path)
            } else if config.relative_bin_path.starts_with(".mcpmate/") {
                mcpmate_dir.join(&config.relative_bin_path[9..])
            } else {
                mcpmate_dir.join(&config.relative_bin_path)
            };

            // Only cache if the file actually exists
            if full_path.exists() {
                self.set_available(runtime_type, full_path.clone()).await;
                tracing::debug!("Runtime {:?} loaded: {}", runtime_type, full_path.display());
                loaded_count += 1;
            } else {
                tracing::warn!(
                    "Runtime {:?} file missing: {}",
                    runtime_type,
                    full_path.display()
                );
            }
        }

        tracing::info!("Runtime cache initialized with {} runtimes", loaded_count);
        Ok(())
    }

    /// Start background maintenance (simplified - just periodic cache refresh)
    pub fn start_background_maintenance(
        cache: Arc<RuntimeCache>,
        database_pool: sqlx::Pool<sqlx::Sqlite>,
    ) {
        tokio::spawn(async move {
            use tokio::time::{Duration, sleep};

            loop {
                // Refresh cache every hour
                sleep(Duration::from_secs(3600)).await;

                tracing::debug!("Refreshing runtime cache from database");
                if let Err(e) = cache.initialize_from_database(&database_pool).await {
                    tracing::warn!("Failed to refresh runtime cache: {}", e);
                }
            }
        });
    }
}

impl Default for RuntimeCache {
    fn default() -> Self {
        Self::new()
    }
}
