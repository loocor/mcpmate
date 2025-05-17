//! Test environment utilities
//!
//! Provides utilities for setting up and managing test environments,
//! including temporary directories, configuration files, and databases.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use mcpmate::conf::database::Database;
use tempfile::TempDir;

/// Test environment for MCPMate tests
///
/// Provides a controlled environment for tests, including:
/// - Temporary directory for test files
/// - Configuration file path
/// - Database path
/// - Utilities for setup and cleanup
pub struct TestEnvironment {
    /// Temporary directory for test files
    pub temp_dir: TempDir,

    /// Path to the configuration file
    pub config_path: PathBuf,

    /// Path to the database file
    pub db_path: PathBuf,

    /// Whether this environment uses the real config
    pub uses_real_config: bool,
}

impl TestEnvironment {
    /// Create a new test environment with a temporary configuration
    pub async fn new() -> Result<Self> {
        // Create a temporary directory
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

        // Set up paths
        let config_path = temp_dir.path().join("mcp.json");
        let db_path = temp_dir.path().join("mcpmate.db");

        // Create an empty environment
        let env = Self {
            temp_dir,
            config_path,
            db_path,
            uses_real_config: false,
        };

        Ok(env)
    }

    /// Create a test environment using the real configuration file
    pub async fn with_real_config() -> Result<Self> {
        // Create a temporary directory
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

        // Set up paths
        let real_config_path = Path::new("config/mcp.json");
        let config_path = temp_dir.path().join("mcp.json");
        let db_path = temp_dir.path().join("mcpmate.db");

        // Copy the real config to the temp directory
        fs::copy(real_config_path, &config_path).context("Failed to copy real config file")?;

        // Create the environment
        let env = Self {
            temp_dir,
            config_path,
            db_path,
            uses_real_config: true,
        };

        Ok(env)
    }

    /// Initialize the database for this environment
    pub async fn init_database(&self) -> Result<Database> {
        // Set the database path environment variable
        // This is safe because we're only setting an environment variable for our process
        unsafe {
            std::env::set_var("DATABASE_URL", format!("sqlite:{}", self.db_path.display()));
        }

        // Initialize the database
        let db = Database::new()
            .await
            .context("Failed to initialize database")?;

        // Initialize default tables and data
        db.initialize_defaults()
            .await
            .context("Failed to initialize database defaults")?;

        Ok(db)
    }

    /// Load the configuration from this environment
    pub async fn load_config(&self) -> Result<serde_json::Value> {
        // Read and parse the config file
        let config_str =
            fs::read_to_string(&self.config_path).context("Failed to read config file")?;

        let config: serde_json::Value =
            serde_json::from_str(&config_str).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Modify the configuration file
    pub async fn modify_config<F>(
        &self,
        modifier: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut serde_json::Value) -> (),
    {
        // Read the current config
        let mut config = self.load_config().await?;

        // Apply the modifier
        modifier(&mut config);

        // Write the modified config back
        let config_str =
            serde_json::to_string_pretty(&config).context("Failed to serialize modified config")?;

        fs::write(&self.config_path, config_str).context("Failed to write modified config")?;

        Ok(())
    }
}
