//! Simplified Runtime Installer
//!
//! Provides basic installation functionality for runtime binaries.
//! Replaces the complex installers/ directory with a simple, focused implementation.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::File as StdFile;
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::task;
use zip::ZipArchive;

use super::downloader::RuntimeDownloader;
use super::manager::RuntimeManager;
use crate::common::RuntimeType;

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
        let temp_dir = std::env::temp_dir().join(format!("mcpmate-runtime-install-{}", std::process::id()));
        if let Err(e) = tokio::fs::remove_dir_all(&temp_dir).await {
            if e.kind() != io::ErrorKind::NotFound {
                tracing::warn!("Failed to clean existing temp directory: {}", e);
            }
        }
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
        downloaded_file: &Path,
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
        zip_file: &Path,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        // Ensure target directory exists
        tokio::fs::create_dir_all(target_dir).await?;

        // Extract zip file
        let extract_dir = std::env::temp_dir().join(format!("mcpmate-bun-extract-{}", std::process::id()));
        self.prepare_extract_dir(&extract_dir).await?;
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
        tar_file: &Path,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        // Ensure target directory exists
        tokio::fs::create_dir_all(target_dir).await?;

        // Extract tar.gz file
        let extract_dir = std::env::temp_dir().join(format!("mcpmate-uv-extract-{}", std::process::id()));
        self.prepare_extract_dir(&extract_dir).await?;
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

    async fn prepare_extract_dir(
        &self,
        target_dir: &Path,
    ) -> Result<()> {
        if let Err(e) = tokio::fs::remove_dir_all(target_dir).await {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e).context("Failed to clean extract directory");
            }
        }

        tokio::fs::create_dir_all(target_dir)
            .await
            .context("Failed to create extract directory")?;
        Ok(())
    }

    /// Extract zip file using Rust zip support
    async fn extract_zip(
        &self,
        zip_file: &Path,
        target_dir: &Path,
    ) -> Result<()> {
        let zip_file = zip_file.to_path_buf();
        let target_dir = target_dir.to_path_buf();

        task::spawn_blocking(move || -> Result<()> {
            let file = StdFile::open(&zip_file).context("Failed to open zip file")?;
            let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;

            for index in 0..archive.len() {
                let mut entry = archive.by_index(index).context("Failed to read zip entry")?;
                let Some(safe_path) = entry.enclosed_name().map(|path| target_dir.join(path)) else {
                    continue;
                };

                if entry.name().ends_with('/') {
                    std::fs::create_dir_all(&safe_path).context("Failed to create zip directory")?;
                    continue;
                }

                if let Some(parent) = safe_path.parent() {
                    std::fs::create_dir_all(parent).context("Failed to create zip parent directory")?;
                }

                let mut output = StdFile::create(&safe_path).context("Failed to create extracted zip file")?;
                io::copy(&mut entry, &mut output).context("Failed to extract zip entry")?;
            }

            Ok(())
        })
        .await
        .context("Failed to join zip extraction task")??;

        Ok(())
    }

    /// Extract tar.gz file using Rust tar support
    async fn extract_tar_gz(
        &self,
        tar_file: &Path,
        target_dir: &Path,
    ) -> Result<()> {
        let tar_file = tar_file.to_path_buf();
        let target_dir = target_dir.to_path_buf();

        task::spawn_blocking(move || -> Result<()> {
            let file = StdFile::open(&tar_file).context("Failed to open tar.gz file")?;
            let decoder = GzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            archive
                .unpack(&target_dir)
                .context("Failed to extract tar.gz archive")?;
            Ok(())
        })
        .await
        .context("Failed to join tar extraction task")??;

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

        Err(anyhow::anyhow!("Bun executable not found in extracted files"))
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

        Err(anyhow::anyhow!("UV executable not found in extracted files"))
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

        Err(anyhow::anyhow!("UVX executable not found in extracted files"))
    }
}

impl Default for RuntimeInstaller {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::{fs, io::Write as _};
    use tar::Builder;
    use zip::write::SimpleFileOptions;

    #[tokio::test]
    async fn extract_zip_reads_archive_without_system_unzip() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let zip_path = temp_dir.path().join("bun.zip");
        let file = StdFile::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);

        zip.start_file("bun-windows-x64/bun.exe", SimpleFileOptions::default())
            .expect("start zip file");
        zip.write_all(b"bun").expect("write zip file");
        zip.finish().expect("finish zip");

        let target_dir = temp_dir.path().join("extract");
        let installer = RuntimeInstaller::new();
        installer
            .prepare_extract_dir(&target_dir)
            .await
            .expect("prepare extract dir");
        installer
            .extract_zip(&zip_path, &target_dir)
            .await
            .expect("extract zip");

        assert_eq!(
            fs::read(target_dir.join("bun-windows-x64").join("bun.exe")).expect("read bun"),
            b"bun"
        );
    }

    #[tokio::test]
    async fn extract_tar_gz_reads_archive_without_system_tar() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let tar_path = temp_dir.path().join("uv.tar.gz");
        let file = StdFile::create(&tar_path).expect("create tar.gz");
        let encoder = GzEncoder::new(file, Compression::default());
        let mut builder = Builder::new(encoder);
        let mut header = tar::Header::new_gnu();

        header.set_path("uv-x86_64-pc-windows-msvc/uv.exe").expect("set path");
        header.set_size(2);
        header.set_cksum();
        builder.append(&header, &b"uv"[..]).expect("append uv");
        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish gzip");

        let target_dir = temp_dir.path().join("extract");
        let installer = RuntimeInstaller::new();
        installer
            .prepare_extract_dir(&target_dir)
            .await
            .expect("prepare extract dir");
        installer
            .extract_tar_gz(&tar_path, &target_dir)
            .await
            .expect("extract tar.gz");

        assert_eq!(
            fs::read(target_dir.join("uv-x86_64-pc-windows-msvc").join("uv.exe")).expect("read uv"),
            b"uv"
        );
    }
}
