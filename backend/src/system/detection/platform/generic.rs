use crate::system::detection::models::{DetectionMethod, DetectionResult};
use anyhow::Result;
use std::path::PathBuf;

/// Minimal non-macOS detector used by Cleanup Foundation to keep cross-platform
/// desktop packaging unblocked without reintroducing platform-specific client
/// detection coupling into the backend mainline.
pub struct GenericDetector;

impl GenericDetector {
    pub fn new() -> Self {
        Self
    }

    pub async fn detect_by_bundle_id(
        &self,
        _bundle_id: &str,
    ) -> Result<DetectionResult> {
        Ok(DetectionResult::failure(DetectionMethod::BundleId))
    }

    pub async fn detect_by_file_path(
        &self,
        file_path: &str,
    ) -> Result<DetectionResult> {
        let path = PathBuf::from(file_path);

        if path.exists() {
            return Ok(DetectionResult::success(
                path,
                None,
                DetectionMethod::FilePath,
                0.4,
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
