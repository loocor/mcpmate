//! Simplified Runtime Installer
//!
//! Provides basic installation functionality for runtime binaries.
//! Replaces the complex installers/ directory with a simple, focused implementation.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

use super::downloader::RuntimeDownloader;
use super::manager::RuntimeManager;
use super::types::RuntimeType;

/// Simple runtime installer
pub struct RuntimeInstaller {
    downloader: RuntimeDownloader,
    manager: RuntimeManager,
}

impl RuntimeInstaller {
    /// Create a new installer
    pub fn new() -> Self {
        Self {
            downloader: RuntimeDownloader::new(),
            manager: RuntimeManager::new(),
        }
    }

    /// Install a runtime
    pub async fn install_runtime(
        &self,
        runtime_type: RuntimeType,
    ) -> Result<PathBuf> {
        tracing::info!("Installing runtime: {}", runtime_type.as_str());

        // Ensure runtimes directory exists
        self.manager.ensure_runtimes_dir()?;

        // Create temporary download directory
        let temp_dir = std::env::temp_dir().join("mcpmate-runtime-install");
        tokio::fs::create_dir_all(&temp_dir)
            .await
            .context("Failed to create temp directory")?;

        // Download the runtime
        let downloaded_file = self
            .downloader
            .download_runtime(runtime_type, &temp_dir)
            .await
            .context("Failed to download runtime")?;

        // Extract and install
        let installed_path = self
            .extract_and_install(runtime_type, &downloaded_file)
            .await
            .context("Failed to extract and install runtime")?;

        // Clean up temp directory
        if let Err(e) = tokio::fs::remove_dir_all(&temp_dir).await {
            tracing::warn!("Failed to clean up temp directory: {}", e);
        }

        tracing::info!(
            "Successfully installed {} at {}",
            runtime_type.as_str(),
            installed_path.display()
        );
        Ok(installed_path)
    }

    /// Extract and install runtime from downloaded file
    async fn extract_and_install(
        &self,
        runtime_type: RuntimeType,
        downloaded_file: &PathBuf,
    ) -> Result<PathBuf> {
        let runtimes_dir = self.manager.runtimes_dir();

        // Create runtime-specific subdirectory
        let target_dir = match runtime_type {
            RuntimeType::Bun => runtimes_dir.join("bun"),
            RuntimeType::Uv => runtimes_dir.join("uv"),
        };

        match runtime_type {
            RuntimeType::Bun => self.install_bun(downloaded_file, &target_dir).await,
            RuntimeType::Uv => self.install_uv(downloaded_file, &target_dir).await,
        }
    }

    /// Install Bun runtime
    async fn install_bun(
        &self,
        zip_file: &PathBuf,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        // Ensure target directory exists
        tokio::fs::create_dir_all(target_dir).await?;

        // Extract zip file
        let extract_dir = std::env::temp_dir().join("mcpmate-bun-extract");
        self.extract_zip(zip_file, &extract_dir).await?;

        // Find bun executable in extracted directory
        let bun_exe = self.find_bun_executable(&extract_dir)?;

        // Copy to target directory
        let target_path = target_dir.join(if cfg!(windows) { "bun.exe" } else { "bun" });
        tokio::fs::copy(&bun_exe, &target_path)
            .await
            .context("Failed to copy bun executable")?;

        // Make executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&target_path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&target_path, perms).await?;
        }

        // Create bunx symlink/copy
        let bunx_path = target_dir.join(if cfg!(windows) { "bunx.exe" } else { "bunx" });
        #[cfg(unix)]
        {
            if bunx_path.exists() {
                tokio::fs::remove_file(&bunx_path).await?;
            }
            tokio::fs::hard_link(&target_path, &bunx_path)
                .await
                .context("Failed to create bunx link")?;
        }
        #[cfg(windows)]
        {
            tokio::fs::copy(&target_path, &bunx_path)
                .await
                .context("Failed to copy bunx executable")?;
        }

        // Clean up extract directory
        if let Err(e) = tokio::fs::remove_dir_all(&extract_dir).await {
            tracing::warn!("Failed to clean up extract directory: {}", e);
        }

        Ok(target_path)
    }

    /// Install UV runtime
    async fn install_uv(
        &self,
        tar_file: &PathBuf,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        // Ensure target directory exists
        tokio::fs::create_dir_all(target_dir).await?;

        // Extract tar.gz file
        let extract_dir = std::env::temp_dir().join("mcpmate-uv-extract");
        self.extract_tar_gz(tar_file, &extract_dir).await?;

        // Find uv executable in extracted directory
        let uv_exe = self.find_uv_executable(&extract_dir)?;

        // Copy uv to target directory
        let target_path = target_dir.join(if cfg!(windows) { "uv.exe" } else { "uv" });
        tokio::fs::copy(&uv_exe, &target_path)
            .await
            .context("Failed to copy uv executable")?;

        // Make executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&target_path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&target_path, perms).await?;
        }

        // Find and copy uvx executable (official uvx binary from the package)
        let uvx_exe = self.find_uvx_executable(&extract_dir)?;
        let uvx_path = target_dir.join(if cfg!(windows) { "uvx.exe" } else { "uvx" });

        tokio::fs::copy(&uvx_exe, &uvx_path)
            .await
            .context("Failed to copy uvx executable")?;

        // Make uvx executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&uvx_path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&uvx_path, perms).await?;
        }

        // Clean up extract directory
        if let Err(e) = tokio::fs::remove_dir_all(&extract_dir).await {
            tracing::warn!("Failed to clean up extract directory: {}", e);
        }

        Ok(target_path)
    }

    /// Extract zip file using system unzip command
    async fn extract_zip(
        &self,
        zip_file: &PathBuf,
        target_dir: &PathBuf,
    ) -> Result<()> {
        tokio::fs::create_dir_all(target_dir).await?;

        let output = Command::new("unzip")
            .arg("-q")
            .arg(zip_file)
            .arg("-d")
            .arg(target_dir)
            .output()
            .await
            .context("Failed to run unzip command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Unzip failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    /// Extract tar.gz file using system tar command
    async fn extract_tar_gz(
        &self,
        tar_file: &PathBuf,
        target_dir: &PathBuf,
    ) -> Result<()> {
        tokio::fs::create_dir_all(target_dir).await?;

        let output = Command::new("tar")
            .arg("-xzf")
            .arg(tar_file)
            .arg("-C")
            .arg(target_dir)
            .output()
            .await
            .context("Failed to run tar command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Tar extraction failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    /// Find bun executable in extracted directory
    fn find_bun_executable(
        &self,
        extract_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let exe_name = if cfg!(windows) { "bun.exe" } else { "bun" };

        // Look for bun executable recursively
        for entry in walkdir::WalkDir::new(extract_dir) {
            let entry = entry?;
            if entry.file_name() == exe_name {
                return Ok(entry.path().to_path_buf());
            }
        }

        Err(anyhow::anyhow!(
            "Bun executable not found in extracted files"
        ))
    }

    /// Find uv executable in extracted directory
    fn find_uv_executable(
        &self,
        extract_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let exe_name = if cfg!(windows) { "uv.exe" } else { "uv" };

        // Look for uv executable recursively
        for entry in walkdir::WalkDir::new(extract_dir) {
            let entry = entry?;
            if entry.file_name() == exe_name {
                return Ok(entry.path().to_path_buf());
            }
        }

        Err(anyhow::anyhow!(
            "UV executable not found in extracted files"
        ))
    }

    /// Find uvx executable in extracted directory
    fn find_uvx_executable(
        &self,
        extract_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let exe_name = if cfg!(windows) { "uvx.exe" } else { "uvx" };

        // Look for uvx executable recursively
        for entry in walkdir::WalkDir::new(extract_dir) {
            let entry = entry?;
            if entry.file_name() == exe_name {
                return Ok(entry.path().to_path_buf());
            }
        }

        Err(anyhow::anyhow!(
            "UVX executable not found in extracted files"
        ))
    }
}

impl Default for RuntimeInstaller {
    fn default() -> Self {
        Self::new()
    }
}
