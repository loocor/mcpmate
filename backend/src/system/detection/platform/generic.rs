// Generic non-macOS application detection

use crate::system::detection::models::{DetectionMethod, DetectionResult};
use anyhow::Result;
use std::path::PathBuf;

/// Generic detector for non-macOS platforms.
pub struct GenericDetector;

impl GenericDetector {
    pub fn new() -> Self {
        Self
    }

    /// Bundle identifiers are only supported on macOS.
    pub async fn detect_by_bundle_id(
        &self,
        _bundle_id: &str,
    ) -> Result<DetectionResult> {
        Ok(DetectionResult::failure(DetectionMethod::BundleId))
    }

    /// Detect application by file path.
    pub async fn detect_by_file_path(
        &self,
        file_path: &str,
    ) -> Result<DetectionResult> {
        let path = PathBuf::from(file_path);

        if path.exists() {
            return Ok(DetectionResult::success(
                path,
                Some("Unknown".to_string()),
                DetectionMethod::FilePath,
                0.6,
            ));
        }

        Ok(DetectionResult::failure(DetectionMethod::FilePath))
    }
}

impl Default for GenericDetector {
    fn default() -> Self {
        Self::new()
    }
}
