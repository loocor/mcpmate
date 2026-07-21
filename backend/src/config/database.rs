// Configuration module for MCPMate
// Contains database connection and configuration management

use anyhow::{Context, Result};
use sqlx::{
    Pool, Sqlite,
    migrate::MigrateDatabase,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tracing;

use crate::{
    common::paths::global_paths,
    common::profile::ProfileType,
    config::{import, initialization, models},
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

fn sqlite_connect_options(database_url: &str) -> Result<SqliteConnectOptions> {
    Ok(SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5)))
}

pub(crate) async fn initialize_capability_catalog(pool: &Pool<Sqlite>) -> Result<()> {
    mcpmate_capability_store::SqliteCapabilityCatalog::new(pool.clone())
        .ensure_schema()
        .await
        .context("Failed to initialize capability catalog schema")
}

/// Database connection pool
#[derive(Debug, Clone)]
pub struct Database {
    /// SQLite connection pool
    pub pool: Pool<Sqlite>,
    /// Database file path
    pub path: PathBuf,
    /// Node-local derived capability caches owned by this database composition root.
    pub capability_cache: std::sync::Arc<mcpmate_capability_store::DerivedCapabilityCache>,
}

impl Database {
    pub(crate) async fn load_capability_snapshot_typed(
        &self,
        server_id: &str,
    ) -> mcpmate_capability_store::Result<(Option<Arc<mcpmate_capability_store::CatalogSnapshot>>, bool)> {
        let catalog = mcpmate_capability_store::SqliteCapabilityCatalog::new(self.pool.clone());
        let loaded_from_sqlite = Arc::new(AtomicBool::new(false));
        let loader_flag = loaded_from_sqlite.clone();
        let snapshot = self
            .capability_cache
            .get_or_load_current_snapshot(server_id, || async {
                loader_flag.store(true, Ordering::Relaxed);
                mcpmate_capability_store::CapabilityCatalog::load_snapshot(&catalog, server_id).await
            })
            .await?;
        let memory_hit = snapshot.is_some() && !loaded_from_sqlite.load(Ordering::Relaxed);
        Ok((snapshot, memory_hit))
    }

    /// Load the current capability snapshot through the node-local LRU before SQLite.
    pub async fn load_capability_snapshot(
        &self,
        server_id: &str,
    ) -> Result<(Option<Arc<mcpmate_capability_store::CatalogSnapshot>>, bool)> {
        self.load_capability_snapshot_typed(server_id)
            .await
            .context("Failed to load capability snapshot")
    }

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
        let connection_options =
            sqlite_connect_options(&database_url).context("Failed to configure SQLite connection options")?;
        let pool = match SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connection_options)
            .await
        {
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

        // Run initialization
        if let Err(e) = initialization::run_initialization(&pool).await {
            tracing::error!("Failed to run database initialization: {}", e);
            return Err(e);
        }
        // Create database instance
        let db = Self {
            pool,
            path: db_path,
            capability_cache: std::sync::Arc::new(mcpmate_capability_store::DerivedCapabilityCache::default()),
        };

        // Initialize default values
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn main_database_connections_enable_wal_busy_timeout_and_foreign_keys() {
        let directory = tempfile::tempdir().unwrap();
        let database_url = format!("sqlite://{}", directory.path().join("catalog.db").display());
        let options = sqlite_connect_options(&database_url).unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();

        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(&pool)
            .await
            .unwrap();
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap();
        let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous").fetch_one(&pool).await.unwrap();

        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
        assert_eq!(busy_timeout, 5_000);
        assert_eq!(foreign_keys, 1);
        assert_eq!(synchronous, 1, "SQLite NORMAL synchronous mode is encoded as 1");
    }

    #[tokio::test]
    async fn database_initialization_creates_capability_catalog_schema() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        initialize_capability_catalog(&pool).await.unwrap();

        for table in [
            "capability_server_snapshots",
            "capability_kind_states",
            "capability_records",
        ] {
            let exists: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
                    .bind(table)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(exists, 1, "missing catalog table {table}");
        }
    }

    #[tokio::test]
    async fn typed_capability_load_preserves_catalog_decode_errors() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        initialize_capability_catalog(&pool).await.unwrap();
        sqlx::query(
            r#"
            INSERT INTO capability_server_snapshots (
                server_id, server_name, config_fingerprint, record_format_version,
                catalog_revision, snapshot_state, initialize_payload, observed_at,
                committed_at, last_error
            ) VALUES ('server-a', 'docs', 'fingerprint', 1, 1, 'ready',
                      '{corrupt-json', '2026-07-20T00:00:00Z',
                      '2026-07-20T00:00:00Z', NULL)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        let database = Database {
            pool,
            path: PathBuf::new(),
            capability_cache: Arc::new(mcpmate_capability_store::DerivedCapabilityCache::default()),
        };

        let error = database
            .load_capability_snapshot_typed("server-a")
            .await
            .expect_err("typed load should preserve corrupt catalog errors");

        assert!(matches!(error, mcpmate_capability_store::CatalogError::Json(_)));
    }
}
