//! uv specific installer

use crate::runtime::{constants::*, detection::Environment, types::RuntimeType};
use anyhow::Result;
use std::path::Path;

/// uv installer
#[derive(Debug)]
pub struct UvInstaller {
    environment: Environment,
}

impl UvInstaller {
    /// Create a new uv installer
    pub fn new(environment: Environment) -> Self {
        Self { environment }
    }

    /// Get uv download URL
    pub fn get_download_url(
        &self,
        version: &str,
    ) -> Result<String> {
        // uv uses specific target triple format
        let target_triple = match (self.environment.os, self.environment.arch) {
            (
                crate::runtime::detection::OperatingSystem::Windows,
                crate::runtime::detection::Architecture::X86_64,
            ) => "x86_64-pc-windows-msvc",
            (
                crate::runtime::detection::OperatingSystem::Windows,
                crate::runtime::detection::Architecture::Aarch64,
            ) => "aarch64-pc-windows-msvc",
            (
                crate::runtime::detection::OperatingSystem::MacOS,
                crate::runtime::detection::Architecture::X86_64,
            ) => "x86_64-apple-darwin",
            (
                crate::runtime::detection::OperatingSystem::MacOS,
                crate::runtime::detection::Architecture::Aarch64,
            ) => "aarch64-apple-darwin",
            (
                crate::runtime::detection::OperatingSystem::Linux,
                crate::runtime::detection::Architecture::X86_64,
            ) => "x86_64-unknown-linux-gnu",
            (
                crate::runtime::detection::OperatingSystem::Linux,
                crate::runtime::detection::Architecture::Aarch64,
            ) => "aarch64-unknown-linux-gnu",
        };

        let ext = match self.environment.os {
            crate::runtime::detection::OperatingSystem::Windows => "zip",
            _ => "tar.gz",
        };

        let url = if version == "latest" {
            format!(
                "https://github.com/astral-sh/uv/releases/latest/download/uv-{}.{}",
                target_triple, ext
            )
        } else {
            format!(
                "https://github.com/astral-sh/uv/releases/download/{}/uv-{}.{}",
                version, target_triple, ext
            )
        };

        Ok(url)
    }

    /// Post-installation processing for uv
    pub fn post_install(
        &self,
        target_dir: &Path,
        _version: &str,
    ) -> Result<()> {
        // uv's directory structure varies by platform
        // For tar.gz files, uv typically extracts to a directory named after the target triple
        let target_triple = match (self.environment.os, self.environment.arch) {
            (
                crate::runtime::detection::OperatingSystem::Windows,
                crate::runtime::detection::Architecture::X86_64,
            ) => "x86_64-pc-windows-msvc",
            (
                crate::runtime::detection::OperatingSystem::Windows,
                crate::runtime::detection::Architecture::Aarch64,
            ) => "aarch64-pc-windows-msvc",
            (
                crate::runtime::detection::OperatingSystem::MacOS,
                crate::runtime::detection::Architecture::X86_64,
            ) => "x86_64-apple-darwin",
            (
                crate::runtime::detection::OperatingSystem::MacOS,
                crate::runtime::detection::Architecture::Aarch64,
            ) => "aarch64-apple-darwin",
            (
                crate::runtime::detection::OperatingSystem::Linux,
                crate::runtime::detection::Architecture::X86_64,
            ) => "x86_64-unknown-linux-gnu",
            (
                crate::runtime::detection::OperatingSystem::Linux,
                crate::runtime::detection::Architecture::Aarch64,
            ) => "aarch64-unknown-linux-gnu",
        };

        let uv_dir_name = format!("uv-{}", target_triple);
        let uv_dir = target_dir.join(&uv_dir_name);

        // Get executable name once at the top
        let uv_exe_name = RuntimeType::Uv.executable_name();

        if uv_dir.exists() {
            // Create bin directory and move executable files
            let bin_dir = target_dir.join(BIN_DIR_NAME);
            std::fs::create_dir_all(&bin_dir)?;

            // Move uv executable file
            let uv_exe = uv_dir.join(&uv_exe_name);
            if uv_exe.exists() {
                std::fs::rename(&uv_exe, bin_dir.join(&uv_exe_name))?;
            }

            // Move uvx executable file (uvx is a separate executable, not a symlink)
            let uvx_exe_name = if cfg!(windows) { "uvx.exe" } else { "uvx" };
            let uvx_exe = uv_dir.join(uvx_exe_name);
            if uvx_exe.exists() {
                std::fs::rename(&uvx_exe, bin_dir.join(uvx_exe_name))?;
            } else {
                tracing::warn!(
                    "uvx executable not found in extracted archive, this may cause issues"
                );
            }

            // Clean up original directory
            std::fs::remove_dir_all(&uv_dir)?;
        } else {
            // If the expected directory doesn't exist, check if files are directly in target_dir
            let direct_uv_exe = target_dir.join(&uv_exe_name);

            if direct_uv_exe.exists() {
                // Create bin directory and move the file
                let bin_dir = target_dir.join(BIN_DIR_NAME);
                std::fs::create_dir_all(&bin_dir)?;
                std::fs::rename(&direct_uv_exe, bin_dir.join(&uv_exe_name))?;

                // Move uvx executable (should be in the same directory as uv)
                let uvx_exe_name = if cfg!(windows) { "uvx.exe" } else { "uvx" };
                let direct_uvx_exe = target_dir.join(uvx_exe_name);

                if direct_uvx_exe.exists() {
                    std::fs::rename(&direct_uvx_exe, bin_dir.join(uvx_exe_name))?;
                } else {
                    tracing::warn!(
                        "uvx executable not found in extracted archive, this may cause issues"
                    );
                }
            }
        }

        tracing::info!("uv installation completed successfully");
        tracing::info!(
            "uv will automatically manage Python installations through environment variables"
        );

        Ok(())
    }
}
