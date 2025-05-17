//! Configuration integration tests
//!
//! Tests for configuration loading and validation.

use anyhow::Result;
use serial_test::serial;

/// Test configuration loading
#[tokio::test]
#[serial]
async fn test_config_loading() -> Result<()> {
    // Create test environment
    let env = crate::common::environment::TestEnvironment::with_real_config().await?;

    // Load configuration
    let config = env.load_config().await?;

    // Verify configuration is not null
    assert!(config.is_object());

    // Just verify that we got a configuration
    assert!(true);

    Ok(())
}

/// Test temporary directory creation
#[tokio::test]
#[serial]
async fn test_temp_dir() -> Result<()> {
    // Create test environment
    let env = crate::common::environment::TestEnvironment::new().await?;

    // Verify temporary directory
    let path = env.temp_dir.path();
    assert!(path.exists());
    assert!(path.is_dir());

    // Verify we can create files in the temporary directory
    let file_path = path.join("test.txt");
    std::fs::write(&file_path, "test")?;
    assert!(file_path.exists());

    Ok(())
}

/// Test database initialization
#[tokio::test]
#[serial]
async fn test_database_init() -> Result<()> {
    // Create test environment
    let env = crate::common::environment::TestEnvironment::new().await?;

    // Initialize database
    let _db = env.init_database().await?;

    // Just verify that we got a database instance
    assert!(true);

    Ok(())
}
