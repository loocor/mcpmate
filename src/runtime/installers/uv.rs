//! UV installer implementation

use super::super::types::DownloadConfig;
use crate::common::env::{Architecture, Environment, OperatingSystem};
use crate::runtime::download::{ArchiveExtractor, FileDownloader};
use crate::runtime::types::RuntimeType;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// uv installer
pub struct UvInstaller {
    downloader: FileDownloader,
    extractor: ArchiveExtractor,
}

impl UvInstaller {
    /// Create a new uv installer
    pub fn new(environment: Environment) -> Self {
        Self {
            downloader: FileDownloader::new(environment),
            extractor: ArchiveExtractor::new(),
        }
    }

    pub fn new_with_config(
        environment: Environment,
        config: DownloadConfig,
    ) -> Self {
        Self {
            downloader: FileDownloader::with_config(environment, config),
            extractor: ArchiveExtractor::new(),
        }
    }

    pub async fn install(
        &self,
        version: &str,
        install_dir: &Path,
    ) -> Result<PathBuf> {
        let url = self.get_download_url(version)?;
        let temp_dir = std::env::temp_dir().join("mcpmate-downloads");
        std::fs::create_dir_all(&temp_dir)?;

        // Download the archive using our simplified download_file method
        let archive_path = self
            .downloader
            .download_file(&url, RuntimeType::Uv, version, &temp_dir)
            .await?;

        // Extract to installation directory
        self.extractor
            .extract(&archive_path, install_dir)
            .context("Failed to extract uv archive")?;

        // Find the actual uv executable path
        // uv extracts to a platform-specific subdirectory
        let platform_suffix = self.get_platform_suffix()?;
        let extracted_dir = install_dir.join(format!("uv-{}", platform_suffix));
        let exe_path = extracted_dir.join("uv");

        // Verify the executable exists
        if !exe_path.exists() {
            // Fallback: search for uv executable in subdirectories
            let search_paths = [
                install_dir.join("uv"),
                install_dir.join("bin").join("uv"),
                extracted_dir.join("uv"),
            ];

            for path in &search_paths {
                if path.exists() {
                    return Ok(path.clone());
                }
            }

            return Err(anyhow::anyhow!(
                "uv executable not found after extraction. Expected at: {}",
                exe_path.display()
            ));
        }

        Ok(exe_path)
    }

    /// Get download URL
    pub fn get_download_url(
        &self,
        version: &str,
    ) -> Result<String> {
        let platform_suffix = self.get_platform_suffix()?;
        let extension = if self.downloader.environment().os == OperatingSystem::Windows {
            "zip"
        } else {
            "tar.gz"
        };

        if version == "latest" {
            Ok(format!(
                "https://github.com/astral-sh/uv/releases/latest/download/uv-{}.{}",
                platform_suffix, extension
            ))
        } else {
            Ok(format!(
                "https://github.com/astral-sh/uv/releases/download/{}/uv-{}.{}",
                version, platform_suffix, extension
            ))
        }
    }

    fn get_platform_suffix(&self) -> Result<String> {
        let env = self.downloader.environment();

        match (&env.os, &env.arch) {
            // macOS
            (OperatingSystem::MacOS, Architecture::X86_64) => Ok("x86_64-apple-darwin".to_string()),
            (OperatingSystem::MacOS, Architecture::Aarch64) => {
                Ok("aarch64-apple-darwin".to_string())
            }

            // Linux
            (OperatingSystem::Linux, Architecture::X86_64) => {
                Ok("x86_64-unknown-linux-gnu".to_string())
            }
            (OperatingSystem::Linux, Architecture::Aarch64) => {
                Ok("aarch64-unknown-linux-gnu".to_string())
            }

            // Windows
            (OperatingSystem::Windows, Architecture::X86_64) => {
                Ok("x86_64-pc-windows-msvc".to_string())
            }
            (OperatingSystem::Windows, Architecture::Aarch64) => {
                Ok("aarch64-pc-windows-msvc".to_string())
            }
        }
    }

    /// Post-install verification and configuration
    pub fn post_install(
        &self,
        install_path: &Path,
        version: &str,
    ) -> Result<()> {
        // Add platform-specific post-install steps if needed
        match (
            self.downloader.environment().os,
            self.downloader.environment().arch,
        ) {
            (OperatingSystem::Windows, Architecture::X86_64) => {
                // Windows-specific steps for x64
            }
            (OperatingSystem::Windows, Architecture::Aarch64) => {
                // Windows-specific steps for ARM64
            }
            (OperatingSystem::MacOS, Architecture::X86_64) => {
                // macOS-specific steps for Intel
            }
            (OperatingSystem::MacOS, Architecture::Aarch64) => {
                // macOS-specific steps for Apple Silicon
            }
            (OperatingSystem::Linux, Architecture::X86_64) => {
                // Linux-specific steps for x64
            }
            (OperatingSystem::Linux, Architecture::Aarch64) => {
                // Linux-specific steps for ARM64
            }
        }

        // Verify installation
        if !install_path.exists() {
            return Err(anyhow::anyhow!(
                "uv installation verification failed: executable not found at {}",
                install_path.display()
            ));
        }

        tracing::info!(
            "uv {} installed successfully at {}",
            version,
            install_path.display()
        );
        Ok(())
    }
}
