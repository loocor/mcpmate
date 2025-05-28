// Basic application detector tests

use anyhow::Result;
use mcpmate::conf::initialization::run_initialization;
use mcpmate::system::detection::AppDetector;
use sqlx::SqlitePool;
use std::sync::Arc;

/// Test helper to create an in-memory database with initialized schema
async fn create_test_database() -> Result<Arc<SqlitePool>> {
    let pool = SqlitePool::connect(":memory:").await?;
    run_initialization(&pool).await?;
    Ok(Arc::new(pool))
}

#[tokio::test]
async fn test_app_detector_creation() -> Result<()> {
    // Given: An initialized database
    let db_pool = create_test_database().await?;

    // When: Creating an AppDetector
    let detector = AppDetector::new(db_pool).await;

    // Then: Should succeed
    assert!(detector.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_preloaded_apps_are_disabled_by_default() -> Result<()> {
    // Given: An initialized database with preloaded apps
    let db_pool = create_test_database().await?;
    let detector = AppDetector::new(db_pool).await?;

    // When: Getting enabled apps
    let enabled_apps = detector.get_enabled_apps().await?;

    // Then: Should return empty list (all apps disabled by default)
    assert_eq!(enabled_apps.len(), 0);
    Ok(())
}

#[tokio::test]
async fn test_can_query_all_known_apps() -> Result<()> {
    // Given: An initialized database with preloaded apps
    let db_pool = create_test_database().await?;
    let detector = AppDetector::new(db_pool).await?;

    // When: Getting all known apps
    let all_apps = detector.get_all_known_apps().await?;

    // Then: Should include preloaded clients
    assert!(all_apps.len() >= 3); // claude_desktop, cursor, windsurf

    // Verify specific apps are present
    let identifiers: Vec<&str> = all_apps.iter().map(|app| app.identifier.as_str()).collect();
    assert!(identifiers.contains(&"claude_desktop"));
    assert!(identifiers.contains(&"cursor"));
    assert!(identifiers.contains(&"windsurf"));

    // Verify all are disabled by default
    for app in &all_apps {
        assert!(
            !app.enabled,
            "App {} should be disabled by default",
            app.identifier
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_enable_client_app() -> Result<()> {
    // Given: An initialized database with preloaded apps
    let db_pool = create_test_database().await?;
    let detector = AppDetector::new(db_pool).await?;

    // When: Enabling a client app
    detector.enable_client_app("claude_desktop").await?;

    // Then: The app should be enabled
    let enabled_apps = detector.get_enabled_apps().await?;
    assert_eq!(enabled_apps.len(), 1);
    assert_eq!(enabled_apps[0].identifier, "claude_desktop");
    assert!(enabled_apps[0].enabled);

    Ok(())
}

#[tokio::test]
async fn test_disable_client_app() -> Result<()> {
    // Given: An initialized database with an enabled app
    let db_pool = create_test_database().await?;
    let detector = AppDetector::new(db_pool).await?;
    detector.enable_client_app("claude_desktop").await?;

    // Verify it's enabled first
    let enabled_apps = detector.get_enabled_apps().await?;
    assert_eq!(enabled_apps.len(), 1);

    // When: Disabling the client app
    detector.disable_client_app("claude_desktop").await?;

    // Then: The app should be disabled
    let enabled_apps = detector.get_enabled_apps().await?;
    assert_eq!(enabled_apps.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_detect_by_identifier_existing() -> Result<()> {
    // Given: An initialized database with preloaded apps
    let db_pool = create_test_database().await?;
    let detector = AppDetector::new(db_pool).await?;

    // When: Detecting by identifier (even if app is not installed)
    let result = detector.detect_by_identifier("claude_desktop").await?;

    // Then: Should return None (app not actually installed in test environment)
    // But the method should not error - it should handle missing apps gracefully
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_detect_by_identifier_nonexistent() -> Result<()> {
    // Given: An initialized database
    let db_pool = create_test_database().await?;
    let detector = AppDetector::new(db_pool).await?;

    // When: Detecting by non-existent identifier
    let result = detector.detect_by_identifier("nonexistent_app").await?;

    // Then: Should return None
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_scan_all_known_apps() -> Result<()> {
    // Given: An initialized database with preloaded apps
    let db_pool = create_test_database().await?;
    let detector = AppDetector::new(db_pool).await?;

    // When: Scanning all known apps
    let detected_apps = detector.scan_all_known_apps().await?;

    // Then: Should return empty list in test environment (no apps actually installed)
    // But should not error
    assert!(detected_apps.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_detect_enabled_apps() -> Result<()> {
    // Given: An initialized database with no enabled apps
    let db_pool = create_test_database().await?;
    let detector = AppDetector::new(db_pool).await?;

    // When: Detecting enabled apps
    let detected_apps = detector.detect_enabled_apps().await?;

    // Then: Should return empty list (no apps enabled)
    assert!(detected_apps.is_empty());

    Ok(())
}

/// Test that simulates the Tauri command interface
#[tokio::test]
async fn test_tauri_command_simulation() -> Result<()> {
    // Given: An initialized database (simulating Tauri app state)
    let db_pool = create_test_database().await?;

    // When: Simulating Tauri commands
    let detector = AppDetector::new(db_pool).await?;

    // Simulate: scan_installed_apps command
    let scan_result = detector.scan_all_known_apps().await?;
    assert!(scan_result.is_empty()); // No apps in test environment

    // Simulate: get_enabled_apps command
    let enabled_result = detector.get_enabled_apps().await?;
    assert!(enabled_result.is_empty()); // No apps enabled by default

    // Simulate: enable_client_app command
    detector.enable_client_app("claude_desktop").await?;

    // Verify to enable worked
    let enabled_after = detector.get_enabled_apps().await?;
    assert_eq!(enabled_after.len(), 1);

    Ok(())
}
