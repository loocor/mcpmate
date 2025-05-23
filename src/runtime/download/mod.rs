//! Runtime download and installation module

mod downloader;
mod extractor;

use crate::runtime::{
    detection::Environment,
    installers::{bun::BunInstaller, node::NodeInstaller, uv::UvInstaller},
    paths::RuntimePaths,
    types::RuntimeType,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub use downloader::FileDownloader;
pub use extractor::ArchiveExtractor;

/// Main runtime downloader that coordinates all installers
#[derive(Debug)]
pub struct RuntimeDownloader {
    environment: Environment,
    paths: RuntimePaths,
    file_downloader: FileDownloader,
    extractor: ArchiveExtractor,
}

impl RuntimeDownloader {
    /// Create a new runtime downloader
    pub fn new(environment: Environment) -> Result<Self> {
        let paths = RuntimePaths::new()?;
        let file_downloader = FileDownloader::new(environment.clone());
        let extractor = ArchiveExtractor::new();

        Ok(Self {
            environment,
            paths,
            file_downloader,
            extractor,
        })
    }

    /// Download and install runtime
    pub async fn download_and_install(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<PathBuf> {
        let version = version.unwrap_or(runtime_type.default_version());

        // Create necessary directories
        self.paths.create_directories(runtime_type, Some(version))?;

        // Get download URL based on runtime type
        let download_url = self.get_download_url(runtime_type, version)?;

        // Download file
        let temp_file = self
            .file_downloader
            .download_file(
                &download_url,
                runtime_type,
                version,
                &self.paths.get_temp_dir(),
            )
            .await?;

        // Extract archive
        let install_dir = self.paths.get_runtime_dir(runtime_type, Some(version));
        self.extractor.extract(&temp_file, &install_dir)?;

        // Post-installation processing
        self.post_install(runtime_type, version, &install_dir)
            .await?;

        // Clean up temporary file
        if temp_file.exists() {
            std::fs::remove_file(&temp_file)?;
        }

        // Return executable file path
        self.paths.get_runtime_path(runtime_type, Some(version))
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
}

/// Convenience function: download runtime
pub async fn download_runtime(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf> {
    use crate::runtime::detection::detect_environment;

    let environment = detect_environment()?;
    let downloader = RuntimeDownloader::new(environment)?;
    downloader.download_and_install(runtime_type, version).await
}
