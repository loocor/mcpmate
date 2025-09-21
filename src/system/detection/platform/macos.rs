// macOS-specific application detection

use crate::system::detection::models::{DetectionMethod, DetectionResult};
use crate::system::paths::platform::macos::{get_applications_directories, get_user_applications_directories};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// macOS-specific application detector
pub struct MacOSDetector;

impl MacOSDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect application using bundle ID
    pub async fn detect_by_bundle_id(
        &self,
        bundle_id: &str,
    ) -> Result<DetectionResult> {
        // Use mdfind to search for applications with the given bundle ID
        let output = Command::new("mdfind")
            .arg(format!("kMDItemCFBundleIdentifier == '{}'", bundle_id))
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let paths: Vec<&str> = stdout.trim().lines().collect();

                if let Some(first_path) = paths.first() {
                    let app_path = PathBuf::from(first_path);
                    if app_path.exists() && app_path.extension().is_some_and(|ext| ext == "app") {
                        // Try to get version
                        let version = self.get_app_version(&app_path).await.ok();

                        return Ok(DetectionResult::success(
                            app_path,
                            version,
                            DetectionMethod::BundleId,
                            0.8, // High confidence for bundle ID detection
                        ));
                    }
                }
            }
            _ => {
                // mdfind failed, fallback to manual search
                return self.fallback_bundle_search(bundle_id).await;
            }
        }

        Ok(DetectionResult::failure(DetectionMethod::BundleId))
    }

    /// Detect application by file path
    pub async fn detect_by_file_path(
        &self,
        file_path: &str,
    ) -> Result<DetectionResult> {
        let path = PathBuf::from(file_path);

        if path.exists() {
            // For .app bundles, verify it's actually an application
            if path.extension().is_some_and(|ext| ext == "app") {
                if self.is_valid_app_bundle(&path).await {
                    let version = self.get_app_version(&path).await.ok();

                    return Ok(DetectionResult::success(
                        path,
                        version,
                        DetectionMethod::FilePath,
                        0.7, // Good confidence for file path detection
                    ));
                }
            } else if path.is_file() {
                // For executable files
                let version = self.get_executable_version(&path).await.ok();

                return Ok(DetectionResult::success(
                    path,
                    version,
                    DetectionMethod::FilePath,
                    0.6, // Lower confidence for non-bundle executables
                ));
            }
        }

        Ok(DetectionResult::failure(DetectionMethod::FilePath))
    }

    /// Fallback bundle search when mdfind fails
    async fn fallback_bundle_search(
        &self,
        bundle_id: &str,
    ) -> Result<DetectionResult> {
        let mut search_dirs = get_applications_directories();
        if let Ok(user_dirs) = get_user_applications_directories() {
            search_dirs.extend(user_dirs);
        }

        for dir in search_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "app") {
                        if let Ok(found_bundle_id) = self.get_bundle_id(&path).await {
                            if found_bundle_id == bundle_id {
                                let version = self.get_app_version(&path).await.ok();

                                return Ok(DetectionResult::success(
                                    path,
                                    version,
                                    DetectionMethod::BundleId,
                                    0.7, // Slightly lower confidence for fallback search
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(DetectionResult::failure(DetectionMethod::BundleId))
    }

    /// Check if a path is a valid app bundle
    async fn is_valid_app_bundle(
        &self,
        path: &Path,
    ) -> bool {
        let info_plist = path.join("Contents/Info.plist");
        info_plist.exists()
    }

    /// Get bundle ID from app bundle
    async fn get_bundle_id(
        &self,
        app_path: &Path,
    ) -> Result<String> {
        let output = Command::new("defaults")
            .arg("read")
            .arg(app_path.join("Contents/Info.plist"))
            .arg("CFBundleIdentifier")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok(bundle_id)
            }
            _ => Err(anyhow::anyhow!("Failed to read bundle ID")),
        }
    }

    /// Get application version from app bundle
    async fn get_app_version(
        &self,
        app_path: &Path,
    ) -> Result<String> {
        // Try CFBundleShortVersionString first
        let output = Command::new("defaults")
            .arg("read")
            .arg(app_path.join("Contents/Info.plist"))
            .arg("CFBundleShortVersionString")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !version.is_empty() {
                    return Ok(version);
                }
            }
        }

        // Fallback to CFBundleVersion
        let output = Command::new("defaults")
            .arg("read")
            .arg(app_path.join("Contents/Info.plist"))
            .arg("CFBundleVersion")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok(version)
            }
            _ => Err(anyhow::anyhow!("Failed to read app version")),
        }
    }

    /// Get version from executable file
    async fn get_executable_version(
        &self,
        _exe_path: &Path,
    ) -> Result<String> {
        // For now, return unknown. In a full implementation, we might try:
        // - Running the executable with --version flag
        // - Reading version from file metadata
        // - Other platform-specific methods
        Ok("Unknown".to_string())
    }
}

impl Default for MacOSDetector {
    fn default() -> Self {
        Self::new()
    }
}
