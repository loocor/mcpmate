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
            // Create bin directory for consistent structure
            let bin_dir = target_dir.join("bin");
            std::fs::create_dir_all(&bin_dir)?;

            // Handle different platforms
            match self.environment.os {
                crate::runtime::detection::OperatingSystem::Windows => {
                    // On Windows, copy executables to bin directory
                    let node_exe = node_dir.join("node.exe");
                    let npm_exe = node_dir.join("npm.cmd");
                    let npx_exe = node_dir.join("npx.cmd");

                    if node_exe.exists() {
                        std::fs::copy(&node_exe, bin_dir.join("node.exe"))?;
                    }
                    if npm_exe.exists() {
                        std::fs::copy(&npm_exe, bin_dir.join("npm.cmd"))?;
                    }
                    if npx_exe.exists() {
                        std::fs::copy(&npx_exe, bin_dir.join("npx.cmd"))?;
                    }

                    // Also check for .exe versions of npm/npx
                    let npm_exe_alt = node_dir.join("npm.exe");
                    let npx_exe_alt = node_dir.join("npx.exe");
                    if npm_exe_alt.exists() {
                        std::fs::copy(&npm_exe_alt, bin_dir.join("npm.exe"))?;
                    }
                    if npx_exe_alt.exists() {
                        std::fs::copy(&npx_exe_alt, bin_dir.join("npx.exe"))?;
                    }
                }
                _ => {
                    // On Unix-like systems, move the bin directory
                    let source_bin = node_dir.join("bin");
                    if source_bin.exists() {
                        // Copy all files from source bin to target bin
                        for entry in std::fs::read_dir(&source_bin)? {
                            let entry = entry?;
                            let source_file = entry.path();
                            let target_file = bin_dir.join(entry.file_name());
                            if source_file.is_file() {
                                std::fs::copy(&source_file, &target_file)?;
                                // Make executable on Unix
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    let mut perms = std::fs::metadata(&target_file)?.permissions();
                                    perms.set_mode(0o755);
                                    std::fs::set_permissions(&target_file, perms)?;
                                }
                            }
                        }
                    }
                }
            }

            // Clean up original directory
            std::fs::remove_dir_all(&node_dir)?;
        }

        tracing::info!("Node.js installation completed successfully");
        Ok(())
    }
}
