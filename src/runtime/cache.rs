//! Runtime Cache Module
//!
//! Provides in-memory caching for runtime states to enable fast, zero-overhead
//! runtime queries during stdio connections.

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};
use tokio::sync::RwLock;

use super::types::RuntimeType;

/// In-memory cache for runtime states
#[derive(Debug, Clone)]
pub struct RuntimeCache {
    states: Arc<RwLock<HashMap<RuntimeType, RuntimeState>>>,
}

/// Represents the current state of a runtime
#[derive(Debug, Clone)]
pub enum RuntimeState {
    /// Runtime is available and ready to use
    Available { path: PathBuf, verified_at: Instant },
    /// Runtime is not available (not installed or failed verification)
    Unavailable,
    /// Runtime is currently being installed
    Installing { started_at: Instant },
    /// Runtime installation failed
    Failed { error: String, failed_at: Instant },
}

impl RuntimeCache {
    /// Create a new empty runtime cache
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set a runtime as available with the given path
    pub async fn set_available(
        &self,
        runtime_type: RuntimeType,
        path: PathBuf,
    ) {
        let mut states = self.states.write().await;
        states.insert(
            runtime_type,
            RuntimeState::Available {
                path,
                verified_at: Instant::now(),
            },
        );
        tracing::debug!("Runtime {:?} marked as available", runtime_type);
    }

    /// Set a runtime as unavailable
    pub async fn set_unavailable(
        &self,
        runtime_type: RuntimeType,
    ) {
        let mut states = self.states.write().await;
        states.insert(runtime_type, RuntimeState::Unavailable);
        tracing::debug!("Runtime {:?} marked as unavailable", runtime_type);
    }

    /// Set a runtime as currently installing
    pub async fn set_installing(
        &self,
        runtime_type: RuntimeType,
    ) {
        let mut states = self.states.write().await;
        states.insert(
            runtime_type,
            RuntimeState::Installing {
                started_at: Instant::now(),
            },
        );
        tracing::debug!("Runtime {:?} marked as installing", runtime_type);
    }

    /// Set a runtime as failed with error message
    pub async fn set_failed(
        &self,
        runtime_type: RuntimeType,
        error: String,
    ) {
        let mut states = self.states.write().await;
        states.insert(
            runtime_type,
            RuntimeState::Failed {
                error,
                failed_at: Instant::now(),
            },
        );
        tracing::debug!("Runtime {:?} marked as failed", runtime_type);
    }

    /// Get the path for a runtime if it's available
    pub async fn get_available_path(
        &self,
        runtime_type: RuntimeType,
    ) -> Option<PathBuf> {
        let states = self.states.read().await;
        match states.get(&runtime_type) {
            Some(RuntimeState::Available { path, .. }) => Some(path.clone()),
            _ => None,
        }
    }

    /// Check if a runtime is currently being installed
    pub async fn is_installing(
        &self,
        runtime_type: RuntimeType,
    ) -> bool {
        let states = self.states.read().await;
        matches!(
            states.get(&runtime_type),
            Some(RuntimeState::Installing { .. })
        )
    }

    /// Check if a runtime is available
    pub async fn is_available(
        &self,
        runtime_type: RuntimeType,
    ) -> bool {
        let states = self.states.read().await;
        matches!(
            states.get(&runtime_type),
            Some(RuntimeState::Available { .. })
        )
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

        // Get the base runtime path
        let base_path = self.get_available_path(runtime_type).await?;

        // For commands like npx, uvx, bunx, we need to find the correct executable
        // The base_path might point to node.exe, but we need npx.cmd on Windows
        match command {
            "npx" => {
                // Try to find npx in the same directory as the base runtime
                let parent_dir = base_path.parent()?;

                // On Windows, check for both .exe and .cmd versions
                if cfg!(windows) {
                    let npx_exe = parent_dir.join("npx.exe");
                    if npx_exe.exists() {
                        return Some(npx_exe);
                    }
                    let npx_cmd = parent_dir.join("npx.cmd");
                    if npx_cmd.exists() {
                        return Some(npx_cmd);
                    }
                } else {
                    let npx_path = parent_dir.join("npx");
                    if npx_path.exists() {
                        return Some(npx_path);
                    }
                }

                // If npx is not found, fall back to the base path
                Some(base_path)
            }
            "uvx" => {
                // Try to find uvx in the same directory as the base runtime
                let parent_dir = base_path.parent()?;

                let uvx_name = if cfg!(windows) { "uvx.exe" } else { "uvx" };
                let uvx_path = parent_dir.join(uvx_name);
                if uvx_path.exists() {
                    return Some(uvx_path);
                }

                // If uvx is not found, fall back to the base path
                Some(base_path)
            }
            "bunx" => {
                // Try to find bunx in the same directory as the base runtime
                let parent_dir = base_path.parent()?;

                let bunx_name = if cfg!(windows) { "bunx.exe" } else { "bunx" };
                let bunx_path = parent_dir.join(bunx_name);
                if bunx_path.exists() {
                    return Some(bunx_path);
                }

                // If bunx is not found, fall back to the base path
                Some(base_path)
            }
            _ => Some(base_path),
        }
    }

    /// Get all unavailable runtimes for background maintenance
    pub async fn get_unavailable_runtimes(&self) -> Vec<RuntimeType> {
        let states = self.states.read().await;
        states
            .iter()
            .filter_map(|(runtime_type, state)| match state {
                RuntimeState::Unavailable | RuntimeState::Failed { .. } => Some(*runtime_type),
                _ => None,
            })
            .collect()
    }

    /// Get all available runtimes with their paths
    pub async fn get_available_runtimes(&self) -> Vec<(RuntimeType, PathBuf)> {
        let states = self.states.read().await;
        states
            .iter()
            .filter_map(|(runtime_type, state)| match state {
                RuntimeState::Available { path, .. } => Some((*runtime_type, path.clone())),
                _ => None,
            })
            .collect()
    }

    /// Get the current state of a runtime
    pub async fn get_state(
        &self,
        runtime_type: RuntimeType,
    ) -> Option<RuntimeState> {
        let states = self.states.read().await;
        states.get(&runtime_type).cloned()
    }

    /// Get statistics about cached runtimes
    pub async fn get_stats(&self) -> RuntimeCacheStats {
        let states = self.states.read().await;
        let mut stats = RuntimeCacheStats::default();

        for state in states.values() {
            match state {
                RuntimeState::Available { .. } => stats.available += 1,
                RuntimeState::Unavailable => stats.unavailable += 1,
                RuntimeState::Installing { .. } => stats.installing += 1,
                RuntimeState::Failed { .. } => stats.failed += 1,
            }
        }

        stats.total = states.len();
        stats
    }

    /// Clear all cached states (useful for testing)
    pub async fn clear(&self) {
        let mut states = self.states.write().await;
        states.clear();
        tracing::debug!("Runtime cache cleared");
    }

    /// Initialize runtime cache from database configurations
    pub async fn initialize_from_database(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> anyhow::Result<()> {
        use super::config::get_all_configs;
        use super::constants::get_mcpmate_dir;

        tracing::info!("Initializing runtime cache from database...");

        // Get all runtime configurations from database
        let configs = match get_all_configs(pool).await {
            Ok(configs) => configs,
            Err(e) => {
                tracing::warn!("Failed to load runtime configs from database: {}", e);
                tracing::info!("Starting with empty runtime cache");
                return Ok(());
            }
        };

        let mcpmate_dir = get_mcpmate_dir()?;
        let mut initialized_count = 0;
        let mut failed_count = 0;

        for config in configs {
            let runtime_type = match config.get_runtime_type() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::warn!("Invalid runtime type '{}': {}", config.runtime_type, e);
                    failed_count += 1;
                    continue;
                }
            };

            // Construct full path from relative path
            let full_path = if config.relative_bin_path.starts_with("/") {
                // Absolute path (system runtime)
                PathBuf::from(&config.relative_bin_path)
            } else if config.relative_bin_path.starts_with(".mcpmate/") {
                // Relative path with .mcpmate prefix
                mcpmate_dir.join(&config.relative_bin_path[9..]) // Remove ".mcpmate/" prefix
            } else {
                // Relative path without prefix
                mcpmate_dir.join(&config.relative_bin_path)
            };

            // Verify if the runtime file actually exists
            if full_path.exists() {
                self.set_available(runtime_type, full_path.clone()).await;
                tracing::debug!(
                    "Runtime {:?} loaded from database: {}",
                    runtime_type,
                    full_path.display()
                );
                initialized_count += 1;
            } else {
                self.set_unavailable(runtime_type).await;
                tracing::warn!(
                    "Runtime {:?} recorded in database but file missing: {}",
                    runtime_type,
                    full_path.display()
                );
                failed_count += 1;
            }
        }

        tracing::info!(
            "Runtime cache initialized: {} available, {} unavailable",
            initialized_count,
            failed_count
        );

        Ok(())
    }

    /// Start background maintenance task for runtime cache
    pub fn start_background_maintenance(
        cache: Arc<RuntimeCache>,
        database_pool: sqlx::Pool<sqlx::Sqlite>,
    ) {
        tokio::spawn(async move {
            Self::background_maintenance_loop(cache, database_pool).await;
        });
    }

    /// Background maintenance loop
    async fn background_maintenance_loop(
        cache: Arc<RuntimeCache>,
        database_pool: sqlx::Pool<sqlx::Sqlite>,
    ) {
        use tokio::time::{Duration, sleep};

        tracing::info!("Starting runtime cache background maintenance");

        loop {
            // Wait for 1 hour between maintenance cycles
            sleep(Duration::from_secs(3600)).await;

            tracing::debug!("Running runtime cache maintenance cycle");

            if let Err(e) = Self::run_maintenance_cycle(&cache, &database_pool).await {
                tracing::warn!("Runtime cache maintenance cycle failed: {}", e);
            }
        }
    }

    /// Run a single maintenance cycle
    async fn run_maintenance_cycle(
        cache: &RuntimeCache,
        database_pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> anyhow::Result<()> {
        let mut maintenance_stats = MaintenanceStats::default();

        // 1. Check and install unavailable runtimes
        let unavailable_runtimes = cache.get_unavailable_runtimes().await;
        tracing::debug!("Found {} unavailable runtimes", unavailable_runtimes.len());

        for runtime_type in unavailable_runtimes {
            if cache.is_installing(runtime_type).await {
                tracing::debug!(
                    "Runtime {:?} is already being installed, skipping",
                    runtime_type
                );
                continue;
            }

            tracing::info!("Attempting to install missing runtime: {:?}", runtime_type);
            cache.set_installing(runtime_type).await;

            match Self::try_install_runtime_silently(runtime_type, database_pool).await {
                Ok(path) => {
                    cache.set_available(runtime_type, path.clone()).await;
                    tracing::info!(
                        "Successfully installed runtime {:?}: {}",
                        runtime_type,
                        path.display()
                    );
                    maintenance_stats.installed += 1;
                }
                Err(e) => {
                    cache.set_failed(runtime_type, e.to_string()).await;
                    tracing::warn!("Failed to install runtime {:?}: {}", runtime_type, e);
                    maintenance_stats.failed += 1;
                }
            }
        }

        // 2. Verify existing runtimes are still available
        let available_runtimes = cache.get_available_runtimes().await;
        tracing::debug!("Verifying {} available runtimes", available_runtimes.len());

        for (runtime_type, path) in available_runtimes {
            if !path.exists() {
                cache.set_unavailable(runtime_type).await;
                tracing::warn!(
                    "Runtime {:?} became unavailable: {}",
                    runtime_type,
                    path.display()
                );
                maintenance_stats.became_unavailable += 1;
            } else {
                maintenance_stats.verified += 1;
            }
        }

        // 3. Clean up stale database records
        if let Err(e) = Self::cleanup_stale_runtime_records(database_pool).await {
            tracing::warn!("Failed to cleanup stale runtime records: {}", e);
        } else {
            maintenance_stats.cleaned_up += 1;
        }

        // 4. Update last_verified timestamps for available runtimes
        if let Err(e) = Self::update_verification_timestamps(database_pool).await {
            tracing::warn!("Failed to update verification timestamps: {}", e);
        }

        tracing::info!(
            "Maintenance cycle completed: {} installed, {} verified, {} became unavailable, {} failed, {} cleaned up",
            maintenance_stats.installed,
            maintenance_stats.verified,
            maintenance_stats.became_unavailable,
            maintenance_stats.failed,
            maintenance_stats.cleaned_up
        );

        Ok(())
    }

    /// Try to install a runtime silently (for background maintenance)
    async fn try_install_runtime_silently(
        runtime_type: RuntimeType,
        database_pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> anyhow::Result<PathBuf> {
        use super::RuntimeManager;
        use super::constants::get_mcpmate_dir;

        tracing::debug!(
            "Attempting silent installation of runtime: {:?}",
            runtime_type
        );

        // Create runtime manager
        let runtime_manager = RuntimeManager::new()?;

        // Install the runtime using ensure method
        let install_path = runtime_manager.ensure(runtime_type, None).await?;

        // Get the executable path
        let mcpmate_dir = get_mcpmate_dir()?;
        let executable_path = match runtime_type {
            RuntimeType::Node => {
                // For node, point to npx
                let npx_path = install_path.join("bin").join("npx");
                if npx_path.exists() {
                    npx_path
                } else {
                    install_path.to_path_buf()
                }
            }
            RuntimeType::Uv => {
                // For uv, point to uvx
                let uvx_path = install_path.join("bin").join("uvx");
                if uvx_path.exists() {
                    uvx_path
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

        // Create relative path
        let relative_path = executable_path
            .strip_prefix(&mcpmate_dir)
            .unwrap_or(&executable_path)
            .to_string_lossy()
            .to_string();

        // Create runtime config
        let config = super::config::RuntimeConfig::new(runtime_type, "latest", &relative_path);

        // Save to database
        super::config::save_config(database_pool, &config).await?;

        tracing::debug!(
            "Silent installation completed for {:?}: {}",
            runtime_type,
            executable_path.display()
        );
        Ok(executable_path)
    }

    /// Clean up stale runtime records from database
    async fn cleanup_stale_runtime_records(
        database_pool: &sqlx::Pool<sqlx::Sqlite>
    ) -> anyhow::Result<()> {
        use super::constants::get_mcpmate_dir;

        let mcpmate_dir = get_mcpmate_dir()?;
        let configs = super::config::get_all_configs(database_pool).await?;

        for config in configs {
            // Construct full path from relative path
            let full_path = if config.relative_bin_path.starts_with("/") {
                // Absolute path (system runtime)
                PathBuf::from(&config.relative_bin_path)
            } else if config.relative_bin_path.starts_with(".mcpmate/") {
                // Relative path with .mcpmate prefix
                mcpmate_dir.join(&config.relative_bin_path[9..]) // Remove ".mcpmate/" prefix
            } else {
                // Relative path without prefix
                mcpmate_dir.join(&config.relative_bin_path)
            };

            // If file doesn't exist, remove the database record
            if !full_path.exists() {
                tracing::debug!(
                    "Removing stale runtime record: {} -> {}",
                    config.runtime_type,
                    full_path.display()
                );

                if let Err(e) = sqlx::query("DELETE FROM runtime_config WHERE runtime_type = ?")
                    .bind(&config.runtime_type)
                    .execute(database_pool)
                    .await
                {
                    tracing::warn!(
                        "Failed to remove stale runtime record for {}: {}",
                        config.runtime_type,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Update last_verified timestamps for available runtimes
    async fn update_verification_timestamps(
        database_pool: &sqlx::Pool<sqlx::Sqlite>
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE runtime_config SET last_verified = CURRENT_TIMESTAMP")
            .execute(database_pool)
            .await?;

        tracing::debug!("Updated verification timestamps for all runtime configs");
        Ok(())
    }
}

/// Statistics about the runtime cache
#[derive(Debug, Default)]
pub struct RuntimeCacheStats {
    pub total: usize,
    pub available: usize,
    pub unavailable: usize,
    pub installing: usize,
    pub failed: usize,
}

/// Statistics for maintenance operations
#[derive(Debug, Default)]
struct MaintenanceStats {
    pub installed: usize,
    pub verified: usize,
    pub became_unavailable: usize,
    pub failed: usize,
    pub cleaned_up: usize,
}

impl Default for RuntimeCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_runtime_cache_basic_operations() {
        let cache = RuntimeCache::new();

        // Initially empty
        assert!(!cache.is_available(RuntimeType::Node).await);
        assert!(cache.get_available_path(RuntimeType::Node).await.is_none());

        // Set as available
        let test_path = PathBuf::from("/test/path");
        cache
            .set_available(RuntimeType::Node, test_path.clone())
            .await;

        assert!(cache.is_available(RuntimeType::Node).await);
        assert_eq!(
            cache.get_available_path(RuntimeType::Node).await,
            Some(test_path)
        );

        // Set as unavailable
        cache.set_unavailable(RuntimeType::Node).await;
        assert!(!cache.is_available(RuntimeType::Node).await);
        assert!(cache.get_available_path(RuntimeType::Node).await.is_none());
    }

    #[tokio::test]
    async fn test_runtime_cache_installing_state() {
        let cache = RuntimeCache::new();

        // Set as installing
        cache.set_installing(RuntimeType::Uv).await;
        assert!(cache.is_installing(RuntimeType::Uv).await);
        assert!(!cache.is_available(RuntimeType::Uv).await);

        // Set as available after installation
        let test_path = PathBuf::from("/uv/path");
        cache
            .set_available(RuntimeType::Uv, test_path.clone())
            .await;
        assert!(!cache.is_installing(RuntimeType::Uv).await);
        assert!(cache.is_available(RuntimeType::Uv).await);
    }

    #[tokio::test]
    async fn test_get_runtime_for_command() {
        let cache = RuntimeCache::new();

        // Set up Node runtime
        let node_path = PathBuf::from("/node/bin/npx");
        cache
            .set_available(RuntimeType::Node, node_path.clone())
            .await;

        // Test command mapping
        assert_eq!(cache.get_runtime_for_command("npx").await, Some(node_path));
        assert_eq!(cache.get_runtime_for_command("uvx").await, None);
        assert_eq!(cache.get_runtime_for_command("unknown").await, None);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = RuntimeCache::new();

        // Add different states
        cache
            .set_available(RuntimeType::Node, PathBuf::from("/node"))
            .await;
        cache.set_unavailable(RuntimeType::Uv).await;
        cache.set_installing(RuntimeType::Bun).await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.available, 1);
        assert_eq!(stats.unavailable, 1);
        assert_eq!(stats.installing, 1);
        assert_eq!(stats.failed, 0);
    }
}
