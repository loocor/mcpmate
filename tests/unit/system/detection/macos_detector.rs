// macOS detector specific tests

#[cfg(target_os = "macos")]
mod macos_tests {
    use anyhow::Result;
    use mcpmate::system::detection::models::{DetectionMethod, DetectionResult};
    use mcpmate::system::detection::platform::macos::MacOSDetector;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_macos_detector_creation() {
        // Given: macOS environment
        // When: Creating a MacOSDetector
        let _detector = MacOSDetector::new();

        // Then: Should succeed (just test instantiation)
        // Detector should be created successfully
    }

    #[tokio::test]
    async fn test_detect_by_file_path_nonexistent() -> Result<()> {
        // Given: A MacOSDetector
        let detector = MacOSDetector::new();

        // When: Detecting a non-existent file path
        let result = detector.detect_by_file_path("/nonexistent/path").await?;

        // Then: Should return failure result
        assert!(!result.success);
        assert_eq!(result.method.as_str(), "file_path");
        assert_eq!(result.confidence, 0.0);
        assert!(result.install_path.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_detect_by_bundle_id_nonexistent() -> Result<()> {
        // Given: A MacOSDetector
        let detector = MacOSDetector::new();

        // When: Detecting a non-existent bundle ID
        let result = detector.detect_by_bundle_id("com.nonexistent.app").await?;

        // Then: Should return failure result
        assert!(!result.success);
        assert_eq!(result.method.as_str(), "bundle_id");
        assert_eq!(result.confidence, 0.0);
        assert!(result.install_path.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_detect_by_file_path_invalid_bundle() -> Result<()> {
        // Given: A MacOSDetector and fake app bundle path
        let detector = MacOSDetector::new();
        let fake_path = "/fake/app.app";

        // When: Detecting fake app bundle
        let result = detector.detect_by_file_path(fake_path).await?;

        // Then: Should return failure for non-existent path
        assert!(!result.success);
        assert_eq!(result.method.as_str(), "file_path");

        Ok(())
    }

    #[tokio::test]
    async fn test_detect_by_file_path_system_app() -> Result<()> {
        // Given: A MacOSDetector and system app path
        let detector = MacOSDetector::new();
        let calculator_path = "/System/Applications/Calculator.app";

        // When: Detecting system app (only if it exists)
        let result = detector.detect_by_file_path(calculator_path).await?;

        // Then: Result depends on whether app exists
        if PathBuf::from(calculator_path).exists() {
            // If app exists, should succeed
            assert!(result.success);
            assert_eq!(result.method.as_str(), "file_path");
            assert!(result.confidence > 0.0);
            assert!(result.install_path.is_some());
        } else {
            // If app doesn't exist, should fail
            assert!(!result.success);
        }

        Ok(())
    }

    // Note: Private method tests removed to respect encapsulation
    // These methods are tested indirectly through public API

    #[tokio::test]
    async fn test_detection_result_success() {
        // Given: Success parameters
        let install_path = PathBuf::from("/test/path");
        let version = Some("1.0.0".to_string());
        let method = DetectionMethod::FilePath;
        let confidence = 0.8;

        // When: Creating success result
        let result = DetectionResult::success(install_path.clone(), version.clone(), method, confidence);

        // Then: Should have correct properties
        assert!(result.success);
        assert_eq!(result.install_path, Some(install_path));
        assert_eq!(result.version, version);
        assert_eq!(result.method.as_str(), "file_path");
        assert_eq!(result.confidence, confidence);
    }

    #[tokio::test]
    async fn test_detection_result_failure() {
        // Given: Failure method
        let method = DetectionMethod::BundleId;

        // When: Creating failure result
        let result = DetectionResult::failure(method);

        // Then: Should have correct properties
        assert!(!result.success);
        assert!(result.install_path.is_none());
        assert!(result.version.is_none());
        assert_eq!(result.method.as_str(), "bundle_id");
        assert_eq!(result.confidence, 0.0);
    }

    // Note: fallback_bundle_search is a private method
    // It's tested indirectly through detect_by_bundle_id when mdfind fails
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
