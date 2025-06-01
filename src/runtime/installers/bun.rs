//! Bun specific installer

use anyhow::Result;
use std::path::Path;

use crate::common::env::{Architecture, Environment, OperatingSystem};

/// Bun installer
#[derive(Debug)]
pub struct BunInstaller {
    environment: Environment,
}

impl BunInstaller {
    /// Create a new Bun installer
    pub fn new(environment: Environment) -> Self {
        Self { environment }
    }

    /// Get download URL
    pub fn get_download_url(
        &self,
        version: &str,
    ) -> Result<String> {
        // Determine platform
        let platform = match self.environment.os {
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOS => "darwin",
            OperatingSystem::Linux => "linux",
        };

        // Check for unsupported platform
        if platform == "windows"
            && matches!(
                (self.environment.os, self.environment.arch),
                (OperatingSystem::Windows, Architecture::Aarch64),
            )
        {
            return Err(anyhow::anyhow!(
                "Bun is not available for Windows ARM64 architecture"
            ));
        }

        // Determine architecture
        let arch = match (platform, self.environment.arch) {
            (_, Architecture::X86_64) => "x64",
            (_, Architecture::Aarch64) => "aarch64",
        };

        // Construct URL
        let url = if version == "latest" {
            format!(
                "https://github.com/oven-sh/bun/releases/latest/download/bun-{}-{}.zip",
                platform, arch
            )
        } else {
            format!(
                "https://github.com/oven-sh/bun/releases/download/bun-v{}/bun-{}-{}.zip",
                version, platform, arch
            )
        };

        Ok(url)
    }

    /// Post-installation processing
    pub fn post_install(
        &self,
        _install_dir: &Path,
        version: &str,
    ) -> Result<()> {
        // Add platform-specific post-install steps if needed
        match (self.environment.os, self.environment.arch) {
            (OperatingSystem::Windows, Architecture::X86_64) => {
                // Windows-specific steps for x64
                tracing::debug!("Performing Windows x64 post-install steps for Bun");
            }
            (OperatingSystem::Windows, Architecture::Aarch64) => {
                // Windows-specific steps for ARM64
                tracing::debug!("Performing Windows ARM64 post-install steps for Bun");
            }
            (OperatingSystem::MacOS, Architecture::X86_64) => {
                // macOS-specific steps for x64
                tracing::debug!("Performing macOS x64 post-install steps for Bun");
            }
            (OperatingSystem::MacOS, Architecture::Aarch64) => {
                // macOS-specific steps for ARM64
                tracing::debug!("Performing macOS ARM64 post-install steps for Bun");
            }
            (OperatingSystem::Linux, Architecture::X86_64) => {
                // Linux-specific steps for x64
                tracing::debug!("Performing Linux x64 post-install steps for Bun");
            }
            (OperatingSystem::Linux, Architecture::Aarch64) => {
                // Linux-specific steps for ARM64
                tracing::debug!("Performing Linux ARM64 post-install steps for Bun");
            }
        }

        tracing::info!("Bun {} post-installation complete", version);
        Ok(())
    }
}
