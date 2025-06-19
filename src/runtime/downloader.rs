//! Simplified Runtime Downloader
//!
//! Provides basic download functionality for runtime binaries.
//! Replaces the complex download/ directory with a simple, focused implementation.

use anyhow::{Context, Result};
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use super::types::RuntimeType;
use crate::common::env::{Architecture, OperatingSystem, detect_environment};

/// Simple runtime downloader
pub struct RuntimeDownloader {
    client: Client,
}

impl RuntimeDownloader {
    /// Create a new downloader
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Download a runtime to the specified directory
    pub async fn download_runtime(
        &self,
        runtime_type: RuntimeType,
        target_dir: &PathBuf,
    ) -> Result<PathBuf> {
        // Ensure target directory exists
        tokio::fs::create_dir_all(target_dir)
            .await
            .context("Failed to create target directory")?;

        // Get download URL
        let download_url = self.get_download_url(runtime_type)?;

        tracing::info!(
            "Downloading {} from {}",
            runtime_type.as_str(),
            download_url
        );

        // Download the file
        let response = self
            .client
            .get(&download_url)
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        // Determine file name and path
        let file_name = self.get_file_name(runtime_type)?;
        let file_path = target_dir.join(&file_name);

        // Write file
        let mut file = File::create(&file_path)
            .await
            .context("Failed to create download file")?;

        let content = response
            .bytes()
            .await
            .context("Failed to read download content")?;

        file.write_all(&content)
            .await
            .context("Failed to write download file")?;

        tracing::info!(
            "Downloaded {} to {}",
            runtime_type.as_str(),
            file_path.display()
        );
        Ok(file_path)
    }

    /// Get download URL for a runtime
    fn get_download_url(
        &self,
        runtime_type: RuntimeType,
    ) -> Result<String> {
        let env = detect_environment()?;

        match runtime_type {
            RuntimeType::Bun => self.get_bun_download_url(&env),
            RuntimeType::Uv => self.get_uv_download_url(&env),
        }
    }

    /// Get Bun download URL
    fn get_bun_download_url(
        &self,
        env: &crate::common::env::Environment,
    ) -> Result<String> {
        let platform = match env.os {
            OperatingSystem::MacOS => "darwin",
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
        };

        let arch = match env.arch {
            Architecture::X86_64 => "x64",
            Architecture::Aarch64 => "aarch64",
        };

        Ok(format!(
            "https://github.com/oven-sh/bun/releases/latest/download/bun-{}-{}.zip",
            platform, arch
        ))
    }

    /// Get UV download URL
    fn get_uv_download_url(
        &self,
        env: &crate::common::env::Environment,
    ) -> Result<String> {
        let platform = match env.os {
            OperatingSystem::MacOS => "apple-darwin",
            OperatingSystem::Linux => "unknown-linux-gnu",
            OperatingSystem::Windows => "pc-windows-msvc",
        };

        let arch = match env.arch {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        };

        Ok(format!(
            "https://github.com/astral-sh/uv/releases/latest/download/uv-{}-{}.tar.gz",
            arch, platform
        ))
    }

    /// Get file name for downloaded runtime
    fn get_file_name(
        &self,
        runtime_type: RuntimeType,
    ) -> Result<String> {
        let env = detect_environment()?;

        match runtime_type {
            RuntimeType::Bun => {
                let platform = match env.os {
                    OperatingSystem::MacOS => "darwin",
                    OperatingSystem::Linux => "linux",
                    OperatingSystem::Windows => "windows",
                };
                let arch = match env.arch {
                    Architecture::X86_64 => "x64",
                    Architecture::Aarch64 => "aarch64",
                };
                Ok(format!("bun-{}-{}.zip", platform, arch))
            }
            RuntimeType::Uv => {
                let platform = match env.os {
                    OperatingSystem::MacOS => "apple-darwin",
                    OperatingSystem::Linux => "unknown-linux-gnu",
                    OperatingSystem::Windows => "pc-windows-msvc",
                };
                let arch = match env.arch {
                    Architecture::X86_64 => "x86_64",
                    Architecture::Aarch64 => "aarch64",
                };
                Ok(format!("uv-{}-{}.tar.gz", arch, platform))
            }
        }
    }
}

impl Default for RuntimeDownloader {
    fn default() -> Self {
        Self::new()
    }
}
