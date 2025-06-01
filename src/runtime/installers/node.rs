//! Node.js specific installer

use anyhow::Result;
use std::path::Path;

use crate::common::env::{Architecture, Environment, OperatingSystem};
use crate::runtime::types::RuntimeError;

/// Node.js installer
#[derive(Debug)]
pub struct NodeInstaller {
    environment: Environment,
}

impl NodeInstaller {
    /// Create a new Node.js installer
    pub fn new(environment: Environment) -> Self {
        Self { environment }
    }

    /// Get download URL for Node.js
    pub fn get_download_url(
        &self,
        version: &str,
    ) -> Result<String> {
        // Check for Windows ARM64 which has special handling
        if matches!(
            (self.environment.os, self.environment.arch),
            (OperatingSystem::Windows, Architecture::Aarch64)
        ) {
            return Err(RuntimeError::UnsupportedPlatform {
                os: "Windows ARM64".to_string(),
                arch: "Node.js".to_string(),
            }
            .into());
        }

        // Determine platform
        let platform = match self.environment.os {
            OperatingSystem::Windows => "win",
            OperatingSystem::MacOS => "darwin",
            OperatingSystem::Linux => "linux",
        };

        // Determine archive extension
        let ext = match self.environment.os {
            OperatingSystem::Windows => "zip",
            _ => "tar.gz",
        };

        // Format version string to handle latest
        let version_str = if version == "latest" {
            "22.16.0".to_string()
        } else {
            version.to_string()
        };

        // Use nodejs.org standard URL format
        let arch_suffix = if self.environment.arch == Architecture::X86_64 {
            "x64"
        } else {
            "arm64"
        };

        let url = format!(
            "https://nodejs.org/dist/v{}/node-v{}-{}-{}.{}",
            version_str, version_str, platform, arch_suffix, ext
        );

        Ok(url)
    }

    /// Post-installation processing
    pub fn post_install(
        &self,
        _install_dir: &Path,
        version: &str,
    ) -> Result<()> {
        // Add platform-specific post-install steps if needed
        match self.environment.os {
            OperatingSystem::Windows => {
                // Windows-specific steps
                tracing::debug!("Performing Windows post-install steps for Node.js");
            }
            _ => {
                // Unix-specific steps
                tracing::debug!("Performing Unix post-install steps for Node.js");
            }
        }

        tracing::info!("Node.js {} post-installation complete", version);
        Ok(())
    }
}
