// Database module for MCPMate
// Contains SQLite connection and operations for tool configuration persistence

use anyhow::Result;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use tracing;

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
        tracing::info!("Initializing database connection to {}", DB_URL);

        // Check if database exists
        let db_exists = match Sqlite::database_exists(DB_URL).await {
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
            tracing::info!("Creating database at {}", DB_URL);
            match Sqlite::create_database(DB_URL).await {
                Ok(_) => tracing::info!("Database created successfully"),
                Err(e) => {
                    tracing::error!("Failed to create SQLite database: {}", e);
                    return Err(anyhow::anyhow!("Failed to create SQLite database: {}", e));
                }
            }
        } else {
            tracing::info!("Database already exists at {}", DB_URL);
        }

        // Connect to the database
        tracing::debug!("Connecting to database with max 5 connections");
        let pool = match SqlitePoolOptions::new()
            .max_connections(5)
            .connect(DB_URL)
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

        // Run migrations
        if let Err(e) = Self::run_migrations(&pool).await {
            tracing::error!("Failed to run database migrations: {}", e);
            return Err(e);
        }

        tracing::info!("Database initialization completed successfully");
        Ok(Self { pool })
    }

    /// Run database migrations
    async fn run_migrations(pool: &Pool<Sqlite>) -> Result<()> {
        tracing::info!("Running database migrations");

        // Create tool_config table if it doesn't exist
        tracing::debug!("Creating tool_config table if it doesn't exist");
        match sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tool_config (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                server_name TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                alias_name TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(server_name, tool_name)
            )
            "#,
        )
        .execute(pool)
        .await
        {
            Ok(_) => tracing::debug!("tool_config table created or already exists"),
            Err(e) => {
                tracing::error!("Failed to create tool_config table: {}", e);
                return Err(anyhow::anyhow!("Failed to create tool_config table: {}", e));
            }
        };

        // Check if the table was created successfully
        match sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='tool_config'",
        )
        .fetch_optional(pool)
        .await
        {
            Ok(Some(_)) => tracing::info!("Verified tool_config table exists"),
            Ok(None) => {
                let err = "tool_config table not found after creation";
                tracing::error!("{}", err);
                return Err(anyhow::anyhow!(err));
            }
            Err(e) => {
                tracing::error!("Failed to verify tool_config table: {}", e);
                return Err(anyhow::anyhow!("Failed to verify tool_config table: {}", e));
            }
        };

        // Check if alias_name column exists
        tracing::debug!("Checking if alias_name column exists in tool_config table");
        let has_alias_name = match sqlx::query("PRAGMA table_info(tool_config)")
            .fetch_all(pool)
            .await
        {
            Ok(rows) => {
                let mut found = false;
                for row in rows {
                    let column_name: String = row.get(1);
                    if column_name == "alias_name" {
                        found = true;
                        break;
                    }
                }
                found
            }
            Err(e) => {
                tracing::error!("Failed to check if alias_name column exists: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to check if alias_name column exists: {}",
                    e
                ));
            }
        };

        // Add alias_name column if it doesn't exist
        if !has_alias_name {
            tracing::info!("Adding alias_name column to tool_config table");
            match sqlx::query("ALTER TABLE tool_config ADD COLUMN alias_name TEXT")
                .execute(pool)
                .await
            {
                Ok(_) => {
                    tracing::info!("Successfully added alias_name column to tool_config table")
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to add alias_name column to tool_config table: {}",
                        e
                    );
                    return Err(anyhow::anyhow!(
                        "Failed to add alias_name column to tool_config table: {}",
                        e
                    ));
                }
            };
        } else {
            tracing::debug!("alias_name column already exists in tool_config table");
        }

        tracing::info!("Database migrations completed successfully");
        Ok(())
    }

    /// Initialize the database with default values
    pub async fn initialize_defaults(&self) -> Result<()> {
        // This method can be used to insert default values if needed
        tracing::info!("Database initialized with default values");
        Ok(())
    }
}
