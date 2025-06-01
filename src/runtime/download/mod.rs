//! Runtime download and installation module

mod downloader;
mod extractor;
mod progress;

use crate::common::env::Environment;
use crate::common::paths::global_paths;
use crate::runtime::{
    installers::{bun::BunInstaller, node::NodeInstaller, uv::UvInstaller},
    types::{DownloadConfig, DownloadProgress, DownloadStage, RuntimeType},
};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub use downloader::FileDownloader;
pub use extractor::ArchiveExtractor;
pub use progress::{InlineProgressBar, MultiLineProgress, supports_inline_progress};

/// Main runtime downloader that coordinates all installers
#[derive(Debug)]
pub struct RuntimeDownloader {
    environment: Environment,
    file_downloader: FileDownloader,
    extractor: ArchiveExtractor,
}

impl RuntimeDownloader {
    /// Create a new runtime downloader with default configuration
    pub fn new(environment: Environment) -> Result<Self> {
        let file_downloader = FileDownloader::new(environment.clone());
        let extractor = ArchiveExtractor::new();

        Ok(Self {
            environment,
            file_downloader,
            extractor,
        })
    }

    /// Create a new runtime downloader with custom download configuration
    pub fn with_config(
        environment: Environment,
        config: DownloadConfig,
    ) -> Result<Self> {
        let file_downloader = FileDownloader::with_config(environment.clone(), config);
        let extractor = ArchiveExtractor::new();

        Ok(Self {
            environment,
            file_downloader,
            extractor,
        })
    }

    /// Download and install runtime with progress tracking
    pub async fn download_and_install(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<PathBuf> {
        let version = version.unwrap_or(runtime_type.default_version());
        let paths = global_paths();

        // Create necessary directories
        paths.ensure_runtime_directories(runtime_type.as_str(), version)?;

        // Get download URL based on runtime type
        let download_url = self.get_download_url(runtime_type, version)?;

        // Download file with progress tracking
        let temp_file = self
            .file_downloader
            .download_file(&download_url, runtime_type, version, &paths.downloads_dir())
            .await?;

        // Report extraction stage
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::Extracting,
            message: Some("Extracting archive...".to_string()),
        });

        // Extract archive
        let install_dir = paths.runtime_version_dir(runtime_type.as_str(), version);
        self.extractor.extract(&temp_file, &install_dir)?;

        // Report post-processing stage
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::PostProcessing,
            message: Some("Configuring installation...".to_string()),
        });

        // Post-installation processing
        self.post_install(runtime_type, version, &install_dir)
            .await?;

        // Clean up temporary file
        if temp_file.exists() {
            std::fs::remove_file(&temp_file)?;
        }

        // Report completion
        self.report_progress(DownloadProgress {
            downloaded: 0,
            total: None,
            speed: None,
            stage: DownloadStage::Complete,
            message: Some(format!(
                "{} v{} installed successfully",
                runtime_type, version
            )),
        });

        // Return executable file path
        crate::runtime::executable::get_runtime_executable_path(runtime_type, Some(version))
    }

    /// Get download URL for the specified runtime type
    fn get_download_url(
        &self,
        runtime_type: RuntimeType,
        version: &str,
    ) -> Result<String> {
        match runtime_type {
            RuntimeType::Node => {
                let installer = NodeInstaller::new(self.environment.clone());
                installer.get_download_url(version)
            }
            RuntimeType::Uv => {
                let installer = UvInstaller::new(self.environment.clone());
                installer.get_download_url(version)
            }
            RuntimeType::Bun => {
                let installer = BunInstaller::new(self.environment.clone());
                installer.get_download_url(version)
            }
        }
    }

    /// Post-installation processing
    async fn post_install(
        &self,
        runtime_type: RuntimeType,
        version: &str,
        install_dir: &Path,
    ) -> Result<()> {
        match runtime_type {
            RuntimeType::Node => {
                let installer = NodeInstaller::new(self.environment.clone());
                installer.post_install(install_dir, version)?;
            }
            RuntimeType::Uv => {
                let installer = UvInstaller::new(self.environment.clone());
                installer.post_install(install_dir, version)?;
                // uv will automatically manage Python through environment variables
                tracing::info!(
                    "uv installed successfully. Python will be managed automatically when needed."
                );
            }
            RuntimeType::Bun => {
                let installer = BunInstaller::new(self.environment.clone());
                installer.post_install(install_dir, version)?;
            }
        }

        Ok(())
    }

    /// Report progress if file downloader has a callback configured
    fn report_progress(
        &self,
        progress: DownloadProgress,
    ) {
        // The file downloader handles progress reporting through its callback
        // This method is for additional progress reporting from the main downloader
        self.file_downloader.report_progress_external(progress);
    }
}

/// Convenience function: download runtime with default configuration
pub async fn download_runtime(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf> {
    use crate::common::env::detect_environment;

    let environment = detect_environment()?;
    let downloader = RuntimeDownloader::new(environment)?;
    downloader.download_and_install(runtime_type, version).await
}

/// Convenience function: download runtime with custom configuration
pub async fn download_runtime_with_config(
    runtime_type: RuntimeType,
    version: Option<&str>,
    config: DownloadConfig,
) -> Result<PathBuf> {
    use crate::common::env::detect_environment;

    let environment = detect_environment()?;
    let downloader = RuntimeDownloader::with_config(environment, config)?;
    downloader.download_and_install(runtime_type, version).await
}

/// Simple check for interactive support (simplified version)
pub fn supports_interactive() -> bool {
    // Simplified: just check if we're in a terminal
    atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout)
}
