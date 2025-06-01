// Configuration module for MCPMate
// Contains database connection and configuration management

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, migrate::MigrateDatabase, sqlite::SqlitePoolOptions};
use tracing;

use crate::{
    common::types::ConfigSuitType,
    config::{initialization, migration, models, server, suit},
    runtime::constants::get_mcpmate_dir,
};

/// Get the database file path in user directory
fn get_database_path() -> Result<PathBuf> {
    let mcpmate_dir = get_mcpmate_dir()?;
    Ok(mcpmate_dir.join("config.db"))
}

/// Get the database URL for SQLite
fn get_database_url() -> Result<String> {
    // Check environment variable first
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        return Ok(db_url);
    }

    // Use default path in user directory
    let db_path = get_database_path()?;
    Ok(format!("sqlite:{}", db_path.display()))
}

/// Database connection pool
#[derive(Debug)]
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
            get_database_path()?
        };

        tracing::info!("Initializing database connection to {}", database_url);

        // Ensure the parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create database directory: {}", parent.display())
            })?;
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
            tracing::info!("Creating database at {}", database_url);
            match Sqlite::create_database(&database_url).await {
                Ok(_) => tracing::info!("Database created successfully"),
                Err(e) => {
                    tracing::error!("Failed to create SQLite database: {}", e);
                    return Err(anyhow::anyhow!("Failed to create SQLite database: {}", e));
                }
            }
        } else {
            tracing::info!("Database already exists at {}", database_url);
        }

        // Connect to the database
        tracing::debug!("Connecting to database with max 5 connections");
        let pool = match SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
        {
            Ok(pool) => {
                tracing::info!("Successfully connected to database");
                pool
            }
            Err(e) => {
                tracing::error!("Failed to connect to SQLite database: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to connect to SQLite database: {}",
                    e
                ));
            }
        };

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
        let db = Self {
            pool,
            path: db_path,
        };

        // Check if we need to migrate configuration from files
        let default_config_path = std::path::Path::new("config/mcp.json");
        if default_config_path.exists() {
            tracing::info!(
                "Found MCP configuration file at {}",
                default_config_path.display()
            );
        } else {
            tracing::warn!(
                "MCP configuration file not found at {}. No servers will be available.",
                default_config_path.display()
            );
        }

        if default_config_path.exists() {
            // Check if database already has server configurations
            let has_server_configs =
                match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM server_config")
                    .fetch_one(&db.pool)
                    .await
                {
                    Ok(count) => count > 0,
                    Err(e) => {
                        tracing::error!("Failed to check if server_config table has data: {}", e);
                        false
                    }
                };

            // If database is empty, migrate configuration from files
            if !has_server_configs {
                tracing::info!("Database is empty, migrating configuration from files");
                if let Err(e) = db.migrate_from_files(default_config_path).await {
                    tracing::error!("Failed to migrate configuration from files: {}", e);
                    tracing::warn!("Continuing with empty database");
                } else {
                    // Check if servers were actually migrated
                    let server_count = match sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM server_config",
                    )
                    .fetch_one(&db.pool)
                    .await
                    {
                        Ok(count) => count,
                        Err(e) => {
                            tracing::error!(
                                "Failed to check if server_config table has data after migration: {}",
                                e
                            );
                            0
                        }
                    };

                    tracing::info!(
                        "Successfully migrated configuration from files. Found {} servers.",
                        server_count
                    );
                }
            }
        }

        // Initialize default values (after migrating servers from config files)
        if let Err(e) = db.initialize_defaults().await {
            tracing::error!("Failed to initialize default values: {}", e);
            tracing::warn!("Continuing with database initialization");
        }

        tracing::info!("Database initialization completed successfully");

        // Publish DatabaseChanged event
        crate::core::events::EventBus::global()
            .publish(crate::core::events::Event::DatabaseChanged);
        tracing::info!("Published DatabaseChanged event");

        Ok(db)
    }

    /// Get the database file path
    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// Migrate configuration from files to database
    pub async fn migrate_from_files(
        &self,
        mcp_config_path: &Path,
    ) -> Result<()> {
        migration::migrate_from_files(&self.pool, mcp_config_path).await
    }

    /// Initialize the database with some default values
    pub async fn initialize_defaults(&self) -> Result<()> {
        // Create default configuration suit if it doesn't exist
        let default_suit = suit::get_config_suit_by_name(&self.pool, "default").await?;

        let suit_id = if let Some(suit) = default_suit {
            // Check if the default suit is active and default
            let id = suit.id.clone().unwrap();

            if !suit.is_active || !suit.is_default {
                tracing::info!("Updating default configuration suit to be active and default");

                // Set the suit as active and default
                if !suit.is_active {
                    suit::set_config_suit_active(&self.pool, &id, true).await?;
                }
                if !suit.is_default {
                    suit::set_config_suit_default(&self.pool, &id).await?;
                }
            }
            id
        } else {
            tracing::info!("Creating default configuration suit");

            // Create a new default configuration suit
            let mut new_suit = models::ConfigSuit::new_with_description(
                "default".to_string(),
                Some("Default configuration suit".to_string()),
                ConfigSuitType::Shared,
            );

            // Set active and default flags
            new_suit.is_active = true;
            new_suit.is_default = true;
            new_suit.multi_select = true;

            // Insert the default suit
            let id = suit::upsert_config_suit(&self.pool, &new_suit).await?;
            tracing::info!("Created default configuration suit with ID {}", id);
            id
        };

        // Check if we need to add servers to the default configuration suit
        let suit_servers = suit::get_config_suit_servers(&self.pool, &suit_id).await?;

        if suit_servers.is_empty() {
            tracing::info!("Adding servers to default configuration suit");

            // Get all servers from the database
            let all_servers = server::get_all_servers(&self.pool).await?;
            let server_count = all_servers.len();

            tracing::info!(
                "Found {} servers in the database to add to default configuration suit",
                server_count
            );

            if server_count == 0 {
                tracing::warn!(
                    "No servers found in the database. Make sure mcp.json exists and contains valid server configurations."
                );
                tracing::warn!(
                    "You may need to restart the application after adding server configurations."
                );
            }

            // Add each server to the default configuration suit
            for server in &all_servers {
                if let Some(server_id) = &server.id {
                    // Add the server to the default configuration suit with enabled=true
                    suit::add_server_to_config_suit(&self.pool, &suit_id, server_id, true).await?;

                    tracing::debug!(
                        "Added server '{}' to default configuration suit",
                        server.name
                    );
                }
            }

            tracing::info!(
                "Added {} servers to default configuration suit",
                server_count
            );
        }

        // Verify that servers were added to the default configuration suit
        let suit_servers = suit::get_config_suit_servers(&self.pool, &suit_id).await?;
        if suit_servers.is_empty() {
            tracing::warn!(
                "No servers were added to the default configuration suit. This may indicate a database issue."
            );
        } else {
            tracing::info!(
                "Verified {} servers were added to the default configuration suit",
                suit_servers.len()
            );
        }

        tracing::info!("Database initialized with default values");

        // Publish DatabaseChanged event
        crate::core::events::EventBus::global()
            .publish(crate::core::events::Event::DatabaseChanged);
        tracing::info!("Published DatabaseChanged event after initializing defaults");

        Ok(())
    }

    /// Close the database connection
    pub async fn close(self) -> Result<()> {
        tracing::info!("Closing database connection");
        self.pool.close().await;
        Ok(())
    }
}
