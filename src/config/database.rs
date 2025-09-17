// Configuration module for MCPMate
// Contains database connection and configuration management

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, migrate::MigrateDatabase, sqlite::SqlitePoolOptions};
use std::path::{Path, PathBuf};
use tracing;

use crate::{
    common::profile::ProfileType,
    common::paths::global_paths,
    config::{import, initialization, models, profile, server},
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

        // Import configuration from JSON files if available
        let default_config_path = std::path::Path::new("config/mcp.json");
        if default_config_path.exists() {
            // Check if database already has server configurations
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

            // Import configuration if database is empty
            if !has_server_configs {
                if let Err(e) = db.import_from_files(default_config_path).await {
                    tracing::warn!("Failed to import MCP configuration: {}", e);
                } else {
                    tracing::debug!(
                        "Imported MCP server configuration from {}",
                        default_config_path.display()
                    );
                }
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
        // Create default profile if it doesn't exist
        let default_profile = profile::get_profile_by_name(&self.pool, "default").await?;

        let profile_id = if let Some(profile) = default_profile {
            // Check if the default profile is active and default
            let id = profile.id.clone().unwrap();

            if !profile.is_active || !profile.is_default {
                tracing::info!("Updating default profile to be active and default");

                // Set the profile as active and default
                if !profile.is_active {
                    profile::set_profile_active(&self.pool, &id, true).await?;
                }
                if !profile.is_default {
                    profile::set_profile_default(&self.pool, &id).await?;
                }
            }
            id
        } else {
            tracing::info!("Creating default profile");

            // Create a new default profile
            let mut new_profile = models::Profile::new_with_description(
                "default".to_string(),
                Some("Default profile".to_string()),
                ProfileType::Shared,
            );

            // Set active and default flags
            new_profile.is_active = true;
            new_profile.is_default = true;
            new_profile.multi_select = true;

            // Insert the default profile
            let id = profile::upsert_profile(&self.pool, &new_profile).await?;
            tracing::info!("Created default profile with ID {}", id);
            id
        };

        // Check if we need to add servers to the default profile
        let profile_servers = profile::get_profile_servers(&self.pool, &profile_id).await?;

        if profile_servers.is_empty() {
            let all_servers = server::get_all_servers(&self.pool).await?;

            for server in &all_servers {
                if let Some(server_id) = &server.id {
                    profile::add_server_to_profile(&self.pool, &profile_id, server_id, true).await?;
                }
            }

            if !all_servers.is_empty() {
                tracing::info!("Added {} servers to default profile", all_servers.len());
            }
        }

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
