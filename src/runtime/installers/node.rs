//! Node.js specific installer

use crate::runtime::{detection::Environment, types::RuntimeError};
use anyhow::Result;
use std::path::Path;

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

    /// Get Node.js download URL
    pub fn get_download_url(
        &self,
        version: &str,
    ) -> Result<String> {
        // Check Windows ARM64 support (only v20+)
        if matches!(
            (self.environment.os, self.environment.arch),
            (
                crate::runtime::detection::OperatingSystem::Windows,
                crate::runtime::detection::Architecture::Aarch64
            )
        ) {
            // Extract version number for comparison
            let version_num = version.trim_start_matches('v');
            if let Ok(major_version) = version_num.split('.').next().unwrap_or("0").parse::<u32>() {
                if major_version < 20 {
                    return Err(RuntimeError::UnsupportedPlatform {
                        os: "Windows ARM64".to_string(),
                        arch: format!("Node.js {} (requires v20.0.0+)", version),
                    }
                    .into());
                }
            }
        }

        let os = match self.environment.os {
            crate::runtime::detection::OperatingSystem::Windows => "win",
            crate::runtime::detection::OperatingSystem::MacOS => "darwin",
            crate::runtime::detection::OperatingSystem::Linux => "linux",
        };

        let arch = self.environment.arch.node_arch();

        // Node.js uses different extensions for different platforms
        let ext = match self.environment.os {
            crate::runtime::detection::OperatingSystem::Windows => "zip",
            _ => "tar.gz",
        };

        let url = format!(
            "https://nodejs.org/dist/{}/node-{}-{}-{}.{}",
            version, version, os, arch, ext
        );

        Ok(url)
    }

    /// Post-installation processing for Node.js
    pub fn post_install(
        &self,
        target_dir: &Path,
        version: &str,
    ) -> Result<()> {
        let node_dir_name = format!(
            "node-{}-{}-{}",
            version,
            match self.environment.os {
                crate::runtime::detection::OperatingSystem::Windows => "win",
                crate::runtime::detection::OperatingSystem::MacOS => "darwin",
                crate::runtime::detection::OperatingSystem::Linux => "linux",
            },
            self.environment.arch.node_arch()
        );
        let node_dir = target_dir.join(&node_dir_name);

        if node_dir.exists() {
            // Move entire Node.js directory to correct location
            let final_dir = target_dir.join("node");
            if final_dir.exists() {
                std::fs::remove_dir_all(&final_dir)?;
            }
            std::fs::rename(&node_dir, &final_dir)?;
        }

        Ok(())
    }
}
