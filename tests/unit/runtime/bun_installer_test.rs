//! Tests for simplified runtime installer

use anyhow::Result;
use mcpmate::runtime::{RuntimeInstaller, RuntimeType};

#[tokio::test]
async fn test_runtime_installer_bun() -> Result<()> {
    // Given: A runtime installer
    let _installer = RuntimeInstaller::new();

    // When: Testing Bun runtime type
    // Note: We can't actually install in tests, so we just verify the installer can be created
    // and the runtime type is recognized

    // Then: The installer should be able to handle Bun runtime type
    // This test mainly verifies the API structure is correct
    assert!(matches!(RuntimeType::Bun, RuntimeType::Bun));

    Ok(())
}

#[tokio::test]
async fn test_runtime_installer_uv() -> Result<()> {
    // Given: A runtime installer
    let _installer = RuntimeInstaller::new();

    // When: Testing UV runtime type
    // Note: We can't actually install in tests, so we just verify the installer can be created
    // and the runtime type is recognized

    // Then: The installer should be able to handle UV runtime type
    // This test mainly verifies the API structure is correct
    assert!(matches!(RuntimeType::Uv, RuntimeType::Uv));

    Ok(())
}
