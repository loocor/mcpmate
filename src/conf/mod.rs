// Configuration module for MCPMate
// Contains database connection and configuration management

use anyhow::{Context, Result};
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;
use tracing;

pub mod initialization;
pub mod migration;
pub mod models;
pub mod operations;

/// Database URL for SQLite
const DB_URL: &str = "sqlite:./config/mcpmate.db";

/// Database connection pool
#[derive(Debug)]
pub struct Database {
    /// SQLite connection pool
    pub pool: Pool<Sqlite>,
}

impl Database {
    /// Create a new database connection
    pub async fn new() -> Result<Self> {
        // Get database URL from environment or use default
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DB_URL.to_string());

        tracing::info!("Initializing database connection to {}", database_url);

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
        let db = Self { pool };

        // TODO: Add version tracking for database schema
        // In the future, we should add a version table to track schema changes
        // and perform necessary migrations when the schema is updated.
        // This would involve:
        // 1. Creating a version table with a schema_version field
        // 2. Checking the current version against the expected version
        // 3. Running appropriate migration scripts if needed

        // Check if we need to migrate configuration from files
        let default_config_path = std::path::Path::new("config/mcp.json");
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
                    tracing::info!("Successfully migrated configuration from files");
                }
            }
        }

        tracing::info!("Database initialization completed successfully");
        Ok(db)
    }

    /// Migrate configuration from files to database
    pub async fn migrate_from_files(&self, mcp_config_path: &Path) -> Result<()> {
        migration::migrate_from_files(&self.pool, mcp_config_path).await
    }

    /// Initialize the database with some default values
    pub async fn initialize_defaults(&self) -> Result<()> {
        // This method can be used to insert default values if needed
        tracing::info!("Database initialized with default values");
        Ok(())
    }

    /// Close the database connection
    pub async fn close(self) -> Result<()> {
        tracing::info!("Closing database connection");
        self.pool.close().await;
        Ok(())
    }
}
