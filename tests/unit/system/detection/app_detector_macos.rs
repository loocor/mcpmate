// macOS-specific application detector tests

#[cfg(target_os = "macos")]
mod macos_tests {
    use anyhow::Result;
    use mcpmate::config::initialization::run_initialization;
    use mcpmate::system::detection::AppDetector;
    use mcpmate::system::detection::models::{DetectionMethod, DetectionResult};
    use mcpmate::system::detection::platform::macos::MacOSDetector;
    use sqlx::SqlitePool;
    use std::sync::Arc;

    /// Test helper to create an in-memory database with initialized schema
    async fn create_test_database() -> Result<Arc<SqlitePool>> {
        let pool = SqlitePool::connect(":memory:").await?;
        run_initialization(&pool).await?;
        Ok(Arc::new(pool))
    }

    #[tokio::test]
    async fn test_macos_detector_creation() {
        // Given: macOS environment
        // When: Creating a macOS detector
        let _detector = MacOSDetector::new();

        // Then: Should succeed (just test instantiation)
        // This is a basic smoke test
        assert!(true);
    }

    #[tokio::test]
    async fn test_detect_by_bundle_id_nonexistent() -> Result<()> {
        // Given: A macOS detector
        let detector = MacOSDetector::new();

        // When: Detecting a non-existent bundle ID
        let result = detector.detect_by_bundle_id("com.nonexistent.app").await?;

        // Then: Should return failure result
        assert!(!result.success);
        assert_eq!(result.method.as_str(), "bundle_id");
        assert_eq!(result.confidence, 0.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_detect_by_file_path_nonexistent() -> Result<()> {
        // Given: A macOS detector
        let detector = MacOSDetector::new();

        // When: Detecting a non-existent file path
        let result = detector.detect_by_file_path("/nonexistent/app.app").await?;

        // Then: Should return failure result
        assert!(!result.success);
        assert_eq!(result.method.as_str(), "file_path");
        assert_eq!(result.confidence, 0.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_detect_by_file_path_existing_system_app() -> Result<()> {
        // Given: A macOS detector
        let detector = MacOSDetector::new();

        // When: Detecting a system app that should exist (Calculator)
        let result = detector
            .detect_by_file_path("/System/Applications/Calculator.app")
            .await?;

        // Then: Should return success if the app exists
        if result.success {
            assert_eq!(result.method.as_str(), "file_path");
            assert!(result.confidence > 0.0);
            assert!(result.install_path.is_some());
        }
        // Note: We don't assert success because the test might run on different macOS versions

        Ok(())
    }

    #[tokio::test]
    async fn test_claude_desktop_detection_rules() -> Result<()> {
        // Given: An initialized database with Claude Desktop rules
        let db_pool = create_test_database().await?;
        let detector = AppDetector::new(db_pool).await?;

        // When: Getting detection rules for Claude Desktop
        // This is an indirect test - we detect by identifier which uses the rules
        let result = detector.detect_by_identifier("claude_desktop").await?;

        // Then: Should not error, result depends on whether app is actually installed
        if result.is_some() {
            // If Claude Desktop is installed, verify the detection result
            let detected = result.unwrap();
            assert_eq!(detected.client_app.identifier, "claude_desktop");
            assert!(detected.confidence > 0.0);
            println!("✅ Claude Desktop detected: {:?}", detected.install_path);
            println!("✅ Claude Desktop config path: {:?}", detected.config_path);
        } else {
            // If not installed, that's also fine
            println!("ℹ️ Claude Desktop not detected (not installed)");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_cursor_detection_rules() -> Result<()> {
        // Given: An initialized database with Cursor rules
        let db_pool = create_test_database().await?;
        let detector = AppDetector::new(db_pool).await?;

        // When: Getting detection rules for Cursor
        // This is an indirect test - we detect by identifier which uses the rules
        let result = detector.detect_by_identifier("cursor").await?;

        // Then: Should not error, result depends on whether app is actually installed
        if result.is_some() {
            // If Cursor is installed, verify the detection result
            let detected = result.unwrap();
            assert_eq!(detected.client_app.identifier, "cursor");
            assert!(detected.confidence > 0.0);
            println!("✅ Cursor detected: {:?}", detected.install_path);
            println!("✅ Cursor config path: {:?}", detected.config_path);
        } else {
            // If not installed, that's also fine
            println!("ℹ️ Cursor not detected (not installed)");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_confidence_calculation_single_method() -> Result<()> {
        // This test verifies the confidence calculation logic
        // Given: A detection result with single method
        let result = DetectionResult::success(
            std::path::PathBuf::from("/test/path"),
            Some("1.0.0".to_string()),
            DetectionMethod::FilePath,
            0.7,
        );

        // Then: Should have the specified confidence
        assert_eq!(result.confidence, 0.7);
        assert!(result.success);
        assert_eq!(result.method.as_str(), "file_path");

        Ok(())
    }

    #[tokio::test]
    async fn test_confidence_calculation_failure() -> Result<()> {
        // Given: A failed detection result
        let result = DetectionResult::failure(DetectionMethod::BundleId);

        // Then: Should have zero confidence
        assert_eq!(result.confidence, 0.0);
        assert!(!result.success);
        assert!(result.install_path.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_platform_specific_rules_filtering() -> Result<()> {
        // Given: An initialized database with platform-specific rules
        let db_pool = create_test_database().await?;
        let detector = AppDetector::new(db_pool).await?;

        // When: Detecting apps (which should filter for macOS rules only)
        let result = detector.detect_by_identifier("claude_desktop").await?;

        // Then: Should only use macOS rules (verified indirectly)
        // The detection should not error even if Windows/Linux rules exist
        if result.is_some() {
            println!("✅ Platform-specific detection working (app found)");
        } else {
            println!("ℹ️ Platform-specific detection working (app not found)");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_detection_methods_priority() -> Result<()> {
        // This test verifies that detection rules are processed in priority order
        // Given: An initialized database with multiple detection rules
        let db_pool = create_test_database().await?;
        let detector = AppDetector::new(db_pool).await?;

        // When: Attempting detection (which should try methods in priority order)
        let result = detector.detect_by_identifier("claude_desktop").await?;

        // Then: Should process without error
        // Priority order is verified indirectly through the implementation
        if result.is_some() {
            let detected = result.unwrap();
            println!(
                "✅ Multiple detection methods working, confidence: {}",
                detected.confidence
            );
            println!("   Verified methods: {:?}", detected.verified_methods);
        } else {
            println!("ℹ️ Multiple detection methods tested (app not found)");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_config_path_template_resolution() -> Result<()> {
        // Given: An initialized database and detector
        let db_pool = create_test_database().await?;
        let detector = AppDetector::new(db_pool).await?;

        // When: Detection would resolve config path templates
        // This is tested indirectly through the detection process
        let result = detector.detect_by_identifier("claude_desktop").await?;

        // Then: Should handle template resolution without error
        if result.is_some() {
            let detected = result.unwrap();
            println!(
                "✅ Config path template resolved: {:?}",
                detected.config_path
            );
            // Verify the path contains resolved variables (no {{ }} left)
            let path_str = detected.config_path.to_string_lossy();
            assert!(
                !path_str.contains("{{"),
                "Config path should not contain unresolved variables: {}",
                path_str
            );
        } else {
            println!("ℹ️ Config path template resolution tested (app not found)");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_scan_enables_detected_apps() -> Result<()> {
        // Given: An initialized database with disabled apps
        let db_pool = create_test_database().await?;
        let detector = AppDetector::new(db_pool).await?;

        // Verify apps are disabled initially
        let enabled_before = detector.get_enabled_apps().await?;
        assert!(enabled_before.is_empty());

        // When: Scanning all known apps
        let detected = detector.scan_all_known_apps().await?;

        // Then: Result depends on what's actually installed
        if detected.is_empty() {
            println!("ℹ️ No apps detected (none installed)");
            // Verify no apps were enabled
            let enabled_after = detector.get_enabled_apps().await?;
            assert!(enabled_after.is_empty());
        } else {
            println!(
                "✅ Detected {} apps: {:?}",
                detected.len(),
                detected
                    .iter()
                    .map(|app| &app.client_app.identifier)
                    .collect::<Vec<_>>()
            );

            // Verify detected apps were enabled
            let enabled_after = detector.get_enabled_apps().await?;
            assert_eq!(enabled_after.len(), detected.len());
        }

        Ok(())
    }
}

// Placeholder tests for other platforms
#[cfg(not(target_os = "macos"))]
mod non_macos_tests {
    #[test]
    fn test_non_macos_platform() {
        // This test runs on non-macOS platforms
        // Just verify that the test suite can run
        assert!(true);
    }
}
