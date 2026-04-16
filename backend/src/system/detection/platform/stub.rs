use crate::system::detection::models::{DetectionMethod, DetectionResult};
use anyhow::Result;
use std::path::PathBuf;

/// Temporary non-macOS detector used to unblock cross-platform packaging builds.
///
/// TODO(loocor): Replace this stub with real Windows/Linux detectors after desktop
/// packaging is green. At that stage we need to decide whether the detection layer
/// should be fully implemented, partially retained, or removed from desktop flows.
pub struct StubDetector;

impl StubDetector {
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

impl Default for StubDetector {
    fn default() -> Self {
        Self::new()
    }
}
