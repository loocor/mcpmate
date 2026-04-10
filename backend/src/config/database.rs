// Configuration module for MCPMate
// Contains database connection and configuration management

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, migrate::MigrateDatabase, sqlite::SqlitePoolOptions};
use std::path::{Path, PathBuf};
use tracing;

use crate::{
    common::paths::global_paths,
    common::profile::ProfileType,
    config::{defaults, import, initialization, models},
    core::capability::naming,
};

/// Get the database URL for SQLite
fn get_database_url() -> Result<String> {
    // Check environment variable first
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        return Ok(db_url);
    }

    // Use centralized path manager for consistency
    Ok(global_paths().database_url())
}

/// Database connection pool
#[derive(Debug, Clone)]
pub struct Database {
    /// SQLite connection pool
    pub pool: Pool<Sqlite>,
    /// Database file path
    pub path: PathBuf,
}

impl Database {
    /// Create a new database connection
    pub async fn new() -> Result<Self> {
        // Get database URL from environment or use default in user directory
        let database_url = get_database_url()?;
        let db_path = if database_url.starts_with("sqlite:") {
            PathBuf::from(database_url.strip_prefix("sqlite:").unwrap())
        } else {
            global_paths().database_path()
        };

        tracing::info!("Initializing database connection to {}", database_url);

        let uses_default_database_path = !database_url.starts_with("sqlite:");
        if uses_default_database_path {
            global_paths()
                .ensure_directories()
                .context("Failed to initialize MCPMate runtime directories")?;
        }

        // Ensure the parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create database directory: {}", parent.display()))?;
        }

        // Check if database exists
        let db_exists = match Sqlite::database_exists(&database_url).await {
            Ok(exists) => {
                tracing::debug!("Database existence check result: {}", exists);
                exists
            }
            Err(e) => {
                tracing::warn!("Failed to check if database exists: {}", e);
                false
            }
        };

        // Create database if it doesn't exist
        if !db_exists {
            tracing::debug!("Creating database at {}", database_url);
            match Sqlite::create_database(&database_url).await {
                Ok(_) => tracing::debug!("Database created successfully"),
                Err(e) => {
                    tracing::error!("Failed to create SQLite database: {}", e);
                    return Err(anyhow::anyhow!("Failed to create SQLite database: {}", e));
                }
            }
        } else {
            tracing::debug!("Database already exists at {}", database_url);
        }

        // Connect to the database
        tracing::debug!("Connecting to database with max 5 connections");
        let pool = match SqlitePoolOptions::new().max_connections(5).connect(&database_url).await {
            Ok(pool) => {
                tracing::debug!("Successfully connected to database");
                pool
            }
            Err(e) => {
                tracing::error!("Failed to connect to SQLite database: {}", e);
                return Err(anyhow::anyhow!("Failed to connect to SQLite database: {}", e));
            }
        };

        // Initialize naming store as early as possible so other components can rely on it
        naming::initialize(pool.clone());

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .context("Failed to enable foreign keys")?;

        // Run initialization
        if let Err(e) = initialization::run_initialization(&pool).await {
            tracing::error!("Failed to run database initialization: {}", e);
            return Err(e);
        }

        // Create database instance
        let db = Self { pool, path: db_path };

        let has_server_configs = match sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {}",
            crate::common::constants::database::tables::SERVER_CONFIG
        ))
        .fetch_one(&db.pool)
        .await
        {
            Ok(count) => count > 0,
            Err(e) => {
                tracing::error!("Failed to check if server_config table has data: {}", e);
                false
            }
        };

        if !has_server_configs {
            if let Err(e) = defaults::seed_default_servers(&db.pool).await {
                tracing::warn!("Failed to import bundled MCP configuration: {}", e);
            } else {
                tracing::debug!("Imported bundled MCP server configuration into empty database");
            }
        }

        // Initialize default values (after importing servers from config files)
        if let Err(e) = db.initialize_defaults().await {
            tracing::error!("Failed to initialize default values: {}", e);
            tracing::warn!("Continuing with database initialization");
        }

        // Publish DatabaseChanged event
        crate::core::events::EventBus::global().publish(crate::core::events::Event::DatabaseChanged);

        Ok(db)
    }

    /// Get the database file path
    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// Import configuration from JSON files to database
    pub async fn import_from_files(
        &self,
        mcp_config_path: &Path,
    ) -> Result<()> {
        import::import_from_mcp_config(&self.pool, mcp_config_path).await
    }

    /// Initialize the database with some default values
    pub async fn initialize_defaults(&self) -> Result<()> {
        use crate::config::profile::{
            self, DEFAULT_ANCHOR_INITIAL_NAME, DEFAULT_ANCHOR_ROLE, DEFAULT_PROFILE_DESCRIPTION,
        };

        // Ensure the default anchor profile exists
        let default_profile = profile::get_default_profile(&self.pool).await?;

        if let Some(mut profile) = default_profile {
            let profile_id = profile
                .id
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Default profile has no ID"))?;

            let mut needs_update = false;

            if profile.role != DEFAULT_ANCHOR_ROLE {
                tracing::info!(
                    "Setting profile '{}' (ID {}) to default anchor role",
                    profile.name,
                    profile_id
                );
                profile.role = DEFAULT_ANCHOR_ROLE;
                needs_update = true;
            }

            if !profile.is_active || !profile.is_default || !profile.multi_select {
                tracing::info!("Normalizing default anchor profile flags for '{}'", profile_id);
                profile.is_active = true;
                profile.is_default = true;
                profile.multi_select = true;
                needs_update = true;
            }

            if profile.profile_type != ProfileType::Shared {
                tracing::info!("Normalizing default anchor profile type to shared for '{}'", profile_id);
                profile.profile_type = ProfileType::Shared;
                needs_update = true;
            }

            if needs_update {
                profile::update_profile(&self.pool, &profile).await?;
            }
        } else {
            tracing::info!("Creating default anchor profile '{}'", DEFAULT_ANCHOR_INITIAL_NAME);

            // Create a new default profile
            let mut new_profile = models::Profile::new_with_description(
                DEFAULT_ANCHOR_INITIAL_NAME.to_string(),
                Some(DEFAULT_PROFILE_DESCRIPTION.to_string()),
                ProfileType::Shared,
            );

            // Set active and default flags
            new_profile.is_active = true;
            new_profile.is_default = true;
            new_profile.multi_select = true;
            new_profile.role = DEFAULT_ANCHOR_ROLE;

            // Insert the default profile
            let id = profile::upsert_profile(&self.pool, &new_profile).await?;
            tracing::info!("Created default profile with ID {}", id);
        };
        // Publish DatabaseChanged event
        crate::core::events::EventBus::global().publish(crate::core::events::Event::DatabaseChanged);

        Ok(())
    }

    /// Close the database connection
    pub async fn close(self) -> Result<()> {
        tracing::info!("Closing database connection");
        self.pool.close().await;
        Ok(())
    }
}
