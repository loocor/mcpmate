//! Bun specific installer

use crate::runtime::{
    constants::*,
    detection::Environment,
    types::{RuntimeError, RuntimeType},
};
use anyhow::Result;
use std::path::Path;

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

    /// Get Bun download URL
    pub fn get_download_url(
        &self,
        version: &str,
    ) -> Result<String> {
        // Check if platform is supported
        if let (
            crate::runtime::detection::OperatingSystem::Windows,
            crate::runtime::detection::Architecture::Aarch64,
        ) = (self.environment.os, self.environment.arch)
        {
            return Err(RuntimeError::UnsupportedPlatform {
                os: "Windows".to_string(),
                arch: "ARM64".to_string(),
            }
            .into());
        }

        let os = match self.environment.os {
            crate::runtime::detection::OperatingSystem::Windows => "windows",
            crate::runtime::detection::OperatingSystem::MacOS => "darwin",
            crate::runtime::detection::OperatingSystem::Linux => "linux",
        };

        let arch = match self.environment.arch {
            crate::runtime::detection::Architecture::X86_64 => "x64",
            crate::runtime::detection::Architecture::Aarch64 => "aarch64",
        };

        let url = if version == "latest" {
            format!(
                "https://github.com/oven-sh/bun/releases/latest/download/bun-{}-{}.zip",
                os, arch
            )
        } else {
            format!(
                "https://github.com/oven-sh/bun/releases/download/bun-v{}/bun-{}-{}.zip",
                version, os, arch
            )
        };

        Ok(url)
    }

    /// Post-installation processing for Bun
    pub fn post_install(
        &self,
        target_dir: &Path,
        _version: &str,
    ) -> Result<()> {
        // Bun typically extracts directly to the target directory
        // Create bin directory for consistency
        let bin_dir = target_dir.join(BIN_DIR_NAME);
        std::fs::create_dir_all(&bin_dir)?;

        // Check if bun executable exists in the root directory
        let bun_exe_name = RuntimeType::Bun.executable_name();
        let bun_exe_path = target_dir.join(&bun_exe_name);

        // Move to bin directory
        if bun_exe_path.exists() {
            std::fs::rename(&bun_exe_path, bin_dir.join(&bun_exe_name))?;
        }

        // Check for other common locations
        let possible_dirs = [
            target_dir.join("bun-darwin-x64"),
            target_dir.join("bun-darwin-aarch64"),
            target_dir.join("bun-linux-x64"),
            target_dir.join("bun-linux-aarch64"),
            target_dir.join("bun-win-x64"),
        ];

        for dir in possible_dirs.iter() {
            if dir.exists() {
                // Check for bun executable
                let dir_bun_path = dir.join(&bun_exe_name);
                if dir_bun_path.exists() {
                    // Move to bin directory
                    std::fs::rename(&dir_bun_path, bin_dir.join(&bun_exe_name))?;
                    // Clean up directory
                    std::fs::remove_dir_all(dir)?;
                    break;
                }
            }
        }

        // Create bunx script/executable
        self.create_bunx_script(&bin_dir)?;

        Ok(())
    }

    /// Create bunx script that calls 'bun x'
    fn create_bunx_script(
        &self,
        bin_dir: &Path,
    ) -> Result<()> {
        let bun_exe_path = bin_dir.join(RuntimeType::Bun.executable_name());

        if !bun_exe_path.exists() {
            return Err(anyhow::anyhow!(
                "Bun executable not found at {}",
                bun_exe_path.display()
            ));
        }

        let bunx_path = if cfg!(windows) {
            // On Windows, create a batch file
            let bunx_path = bin_dir.join("bunx.cmd");
            let script_content = format!("@echo off\r\n\"{}\" x %*\r\n", bun_exe_path.display());
            std::fs::write(&bunx_path, script_content)?;
            bunx_path
        } else {
            // On Unix-like systems, create a shell script
            let bunx_path = bin_dir.join("bunx");
            let script_content =
                format!("#!/bin/sh\nexec \"{}\" x \"$@\"\n", bun_exe_path.display());
            std::fs::write(&bunx_path, script_content)?;

            // Make it executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&bunx_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&bunx_path, perms)?;
            }

            bunx_path
        };

        tracing::debug!("Created bunx script at: {}", bunx_path.display());
        Ok(())
    }
}
