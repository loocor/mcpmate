//! Simplified Runtime Installer
//!
//! Provides basic installation functionality for runtime binaries.
//! Replaces the complex installers/ directory with a simple, focused implementation.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use nanoid::nanoid;
use std::fs::File as StdFile;
use std::io;
use std::path::{Component, Path, PathBuf};
use tar::{Archive, EntryType};
use tokio::task;
use walkdir::WalkDir;
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
        version: Option<&str>,
    ) -> Result<PathBuf> {
        tracing::info!(
            "Installing runtime: {}{}",
            runtime_type.as_str(),
            version.map(|value| format!(" ({})", value.trim())).unwrap_or_default()
        );

        self.manager.ensure_runtimes_dir()?;

        let temp_dir = Self::unique_temp_dir("mcpmate-runtime-install");
        tokio::fs::create_dir_all(&temp_dir)
            .await
            .context("Failed to create temp directory")?;

        let downloaded_file = self
            .downloader
            .download_runtime(runtime_type, version, &temp_dir)
            .await
            .context("Failed to download runtime")?;

        let installed_path = self
            .extract_and_install(runtime_type, &downloaded_file)
            .await
            .context("Failed to extract and install runtime")?;

        if let Err(error) = tokio::fs::remove_dir_all(&temp_dir).await {
            tracing::warn!("Failed to clean up temp directory: {}", error);
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
        let target_dir = runtimes_dir.join(runtime_type.as_str());

        match runtime_type {
            RuntimeType::Bun => self.install_bun(downloaded_file, &target_dir).await,
            RuntimeType::Uv => self.install_uv(downloaded_file, &target_dir).await,
            RuntimeType::Node => self.install_node(downloaded_file, &target_dir).await,
        }
    }

    /// Install Bun runtime
    async fn install_bun(
        &self,
        zip_file: &Path,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        tokio::fs::create_dir_all(target_dir).await?;

        let extract_dir = Self::unique_temp_dir("mcpmate-bun-extract");
        self.prepare_extract_dir(&extract_dir).await?;
        self.extract_zip(zip_file, &extract_dir).await?;

        let bun_exe = self.find_bun_executable(&extract_dir)?;
        let target_path = target_dir.join(if cfg!(windows) { "bun.exe" } else { "bun" });
        tokio::fs::copy(&bun_exe, &target_path)
            .await
            .context("Failed to copy bun executable")?;

        Self::set_executable_permissions(&target_path).await?;

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

        Self::set_executable_permissions(&bunx_path).await?;

        if let Err(error) = tokio::fs::remove_dir_all(&extract_dir).await {
            tracing::warn!("Failed to clean up extract directory: {}", error);
        }

        Ok(target_path)
    }

    async fn install_node(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        let extract_dir = Self::unique_temp_dir("mcpmate-node-extract");
        self.prepare_extract_dir(&extract_dir).await?;
        self.prepare_extract_dir(target_dir).await?;

        let is_zip = archive_file
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));

        if is_zip {
            self.extract_zip(archive_file, &extract_dir).await?;
        } else {
            self.extract_tar_gz_skipping_links(archive_file, &extract_dir).await?;
        }

        let install_root = self.find_node_install_root(&extract_dir)?;
        self.copy_directory_contents(&install_root, target_dir).await?;

        #[cfg(unix)]
        self.create_node_unix_shims(target_dir).await?;

        let target_path = target_dir.join(RuntimeType::Node.executable_name());
        if !target_path.exists() {
            return Err(anyhow::anyhow!(
                "Installed Node.js executable not found at {}",
                target_path.display()
            ));
        }

        Self::set_executable_permissions(&target_path).await?;

        if let Err(error) = tokio::fs::remove_dir_all(&extract_dir).await {
            tracing::warn!("Failed to clean up extract directory: {}", error);
        }

        Ok(target_path)
    }

    /// Install UV runtime
    ///
    /// UV publishes `.zip` on Windows and `.tar.gz` on macOS/Linux.
    async fn install_uv(
        &self,
        archive_file: &Path,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        tokio::fs::create_dir_all(target_dir).await?;

        let extract_dir = Self::unique_temp_dir("mcpmate-uv-extract");
        self.prepare_extract_dir(&extract_dir).await?;

        let is_zip = archive_file
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));

        if is_zip {
            self.extract_zip(archive_file, &extract_dir).await?;
        } else {
            self.extract_tar_gz(archive_file, &extract_dir).await?;
        }

        let uv_exe = self.find_uv_executable(&extract_dir)?;
        let target_path = target_dir.join(if cfg!(windows) { "uv.exe" } else { "uv" });
        tokio::fs::copy(&uv_exe, &target_path)
            .await
            .context("Failed to copy uv executable")?;

        Self::set_executable_permissions(&target_path).await?;

        let uvx_exe = self.find_uvx_executable(&extract_dir)?;
        let uvx_path = target_dir.join(if cfg!(windows) { "uvx.exe" } else { "uvx" });

        tokio::fs::copy(&uvx_exe, &uvx_path)
            .await
            .context("Failed to copy uvx executable")?;

        Self::set_executable_permissions(&uvx_path).await?;

        if let Err(error) = tokio::fs::remove_dir_all(&extract_dir).await {
            tracing::warn!("Failed to clean up extract directory: {}", error);
        }

        Ok(target_path)
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanoid!(10)))
    }

    fn safe_archive_entry_path(
        target_dir: &Path,
        entry_path: &Path,
    ) -> Result<PathBuf> {
        let mut relative_path = PathBuf::new();

        for component in entry_path.components() {
            match component {
                Component::Normal(path) => relative_path.push(path),
                Component::CurDir => {}
                _ => {
                    return Err(anyhow::anyhow!(
                        "Archive entry path escapes target directory: {}",
                        entry_path.display()
                    ));
                }
            }
        }

        Ok(target_dir.join(relative_path))
    }

    async fn prepare_extract_dir(
        &self,
        target_dir: &Path,
    ) -> Result<()> {
        if let Err(error) = tokio::fs::remove_dir_all(target_dir).await {
            if error.kind() != io::ErrorKind::NotFound {
                return Err(error).context("Failed to clean extract directory");
            }
        }

        tokio::fs::create_dir_all(target_dir)
            .await
            .context("Failed to create extract directory")?;
        Ok(())
    }

    async fn set_executable_permissions(path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(path, perms).await?;
        }

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
        Self::extract_tar_gz_internal(tar_file, target_dir, false).await
    }

    async fn extract_tar_gz_skipping_links(
        &self,
        tar_file: &Path,
        target_dir: &Path,
    ) -> Result<()> {
        Self::extract_tar_gz_internal(tar_file, target_dir, true).await
    }

    async fn extract_tar_gz_internal(
        tar_file: &Path,
        target_dir: &Path,
        skip_links: bool,
    ) -> Result<()> {
        let tar_file = tar_file.to_path_buf();
        let target_dir = target_dir.to_path_buf();

        task::spawn_blocking(move || -> Result<()> {
            let file = StdFile::open(&tar_file).context("Failed to open tar.gz file")?;
            let decoder = GzDecoder::new(file);
            let mut archive = Archive::new(decoder);

            for entry in archive.entries().context("Failed to read tar entries")? {
                let mut entry = entry.context("Failed to read tar entry")?;
                let entry_path = entry.path().context("Failed to read tar entry path")?.to_path_buf();
                let safe_path =
                    Self::safe_archive_entry_path(&target_dir, &entry_path).context("Invalid tar entry path")?;

                let file_type = entry.header().entry_type();

                if file_type.is_dir() {
                    std::fs::create_dir_all(&safe_path)
                        .with_context(|| format!("Failed to create tar directory: {}", safe_path.display()))?;
                    continue;
                }

                if skip_links && (file_type == EntryType::Symlink || file_type == EntryType::Link) {
                    tracing::debug!(
                        "Skipping tar link entry during runtime install: {}",
                        entry_path.display()
                    );
                    continue;
                }

                if !file_type.is_file() {
                    return Err(anyhow::anyhow!(
                        "Unsupported tar entry type for {}: expected regular file or directory",
                        entry_path.display()
                    ));
                }

                if let Some(parent) = safe_path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create tar parent directory: {}", parent.display()))?;
                }

                entry
                    .unpack(&safe_path)
                    .with_context(|| format!("Failed to extract tar entry: {}", safe_path.display()))?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(mode) = entry.header().mode() {
                        std::fs::set_permissions(&safe_path, std::fs::Permissions::from_mode(mode))
                            .with_context(|| format!("Failed to set tar entry permissions: {}", safe_path.display()))?;
                    }
                }
            }

            Ok(())
        })
        .await
        .context("Failed to join tar extraction task")??;

        Ok(())
    }

    async fn copy_directory_contents(
        &self,
        source_dir: &Path,
        target_dir: &Path,
    ) -> Result<()> {
        let source_dir = source_dir.to_path_buf();
        let target_dir = target_dir.to_path_buf();

        task::spawn_blocking(move || -> Result<()> {
            std::fs::create_dir_all(&target_dir)
                .with_context(|| format!("Failed to create target dir: {}", target_dir.display()))?;

            for entry in WalkDir::new(&source_dir) {
                let entry = entry.context("Failed to walk source directory")?;
                let path = entry.path();
                let relative_path = path
                    .strip_prefix(&source_dir)
                    .with_context(|| format!("Failed to relativize path: {}", path.display()))?;

                if relative_path.as_os_str().is_empty() {
                    continue;
                }

                let destination = target_dir.join(relative_path);
                let file_type = entry.file_type();

                if file_type.is_dir() {
                    std::fs::create_dir_all(&destination).with_context(|| {
                        format!("Failed to create destination directory: {}", destination.display())
                    })?;
                    continue;
                }

                if !file_type.is_file() {
                    continue;
                }

                if let Some(parent) = destination.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create destination parent: {}", parent.display()))?;
                }

                std::fs::copy(path, &destination).with_context(|| {
                    format!(
                        "Failed to copy runtime file from {} to {}",
                        path.display(),
                        destination.display()
                    )
                })?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = std::fs::metadata(path)
                        .with_context(|| format!("Failed to read metadata for {}", path.display()))?
                        .permissions()
                        .mode();
                    std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(mode))
                        .with_context(|| format!("Failed to set permissions on {}", destination.display()))?;
                }
            }

            Ok(())
        })
        .await
        .context("Failed to join runtime copy task")??;

        Ok(())
    }

    fn find_node_install_root(
        &self,
        extract_dir: &Path,
    ) -> Result<PathBuf> {
        if cfg!(windows) {
            for entry in WalkDir::new(extract_dir) {
                let entry = entry?;
                if entry.file_name() == "node.exe" {
                    let parent = entry.path().parent().ok_or_else(|| {
                        anyhow::anyhow!("Node.js install root missing for {}", entry.path().display())
                    })?;
                    return Ok(parent.to_path_buf());
                }
            }
        } else {
            for entry in WalkDir::new(extract_dir) {
                let entry = entry?;
                if entry.file_name() == "node"
                    && entry
                        .path()
                        .parent()
                        .and_then(Path::file_name)
                        .is_some_and(|part| part == "bin")
                {
                    let root = entry
                        .path()
                        .parent()
                        .and_then(Path::parent)
                        .ok_or_else(|| anyhow::anyhow!("Node.js install root missing"))?;
                    return Ok(root.to_path_buf());
                }
            }
        }

        Err(anyhow::anyhow!("Node.js install root not found in extracted files"))
    }

    #[cfg(unix)]
    async fn create_node_unix_shims(
        &self,
        target_dir: &Path,
    ) -> Result<()> {
        let bin_node = target_dir.join("bin").join("node");
        let root_node = target_dir.join("node");
        let npm_path = target_dir.join("npm");
        let npx_path = target_dir.join("npx");

        if !bin_node.exists() {
            return Err(anyhow::anyhow!("Extracted Node.js archive does not contain bin/node"));
        }

        Self::recreate_hard_link_or_copy(&bin_node, &root_node).await?;
        Self::set_executable_permissions(&root_node).await?;

        let npm_script = r#"#!/bin/sh
DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
exec "$DIR/node" "$DIR/lib/node_modules/npm/bin/npm-cli.js" "$@"
"#;
        tokio::fs::write(&npm_path, npm_script)
            .await
            .context("Failed to write npm launcher")?;
        Self::set_executable_permissions(&npm_path).await?;

        let npx_script = r#"#!/bin/sh
DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
exec "$DIR/node" "$DIR/lib/node_modules/npm/bin/npx-cli.js" "$@"
"#;
        tokio::fs::write(&npx_path, npx_script)
            .await
            .context("Failed to write npx launcher")?;
        Self::set_executable_permissions(&npx_path).await?;

        Ok(())
    }

    #[cfg(unix)]
    async fn recreate_hard_link_or_copy(
        source: &Path,
        destination: &Path,
    ) -> Result<()> {
        if destination.exists() {
            tokio::fs::remove_file(destination)
                .await
                .with_context(|| format!("Failed to remove existing file: {}", destination.display()))?;
        }

        match tokio::fs::hard_link(source, destination).await {
            Ok(()) => Ok(()),
            Err(_) => {
                tokio::fs::copy(source, destination).await.with_context(|| {
                    format!(
                        "Failed to copy runtime binary from {} to {}",
                        source.display(),
                        destination.display()
                    )
                })?;
                Ok(())
            }
        }
    }

    /// Find bun executable in extracted directory
    fn find_bun_executable(
        &self,
        extract_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let exe_name = if cfg!(windows) { "bun.exe" } else { "bun" };

        for entry in WalkDir::new(extract_dir) {
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

        for entry in WalkDir::new(extract_dir) {
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

        for entry in WalkDir::new(extract_dir) {
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
    use tar::{Builder, EntryType};
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

    #[tokio::test]
    async fn extract_tar_gz_rejects_symlink_entries() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let tar_path = temp_dir.path().join("uv.tar.gz");
        let file = StdFile::create(&tar_path).expect("create tar.gz");
        let encoder = GzEncoder::new(file, Compression::default());
        let mut builder = Builder::new(encoder);
        let mut header = tar::Header::new_gnu();

        header.set_entry_type(EntryType::Symlink);
        header.set_path("uv-x86_64-pc-windows-msvc/uv.exe").expect("set path");
        header.set_link_name("../../outside").expect("set link path");
        header.set_size(0);
        header.set_cksum();
        builder.append(&header, std::io::empty()).expect("append symlink");
        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish gzip");

        let target_dir = temp_dir.path().join("extract");
        let installer = RuntimeInstaller::new();
        installer
            .prepare_extract_dir(&target_dir)
            .await
            .expect("prepare extract dir");

        let err = installer
            .extract_tar_gz(&tar_path, &target_dir)
            .await
            .expect_err("reject symlink");

        assert!(err.to_string().contains("Unsupported tar entry type"));
    }

    #[tokio::test]
    async fn extract_tar_gz_skips_symlink_entries_for_node_archives() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let tar_path = temp_dir.path().join("node.tar.gz");
        let file = StdFile::create(&tar_path).expect("create tar.gz");
        let encoder = GzEncoder::new(file, Compression::default());
        let mut builder = Builder::new(encoder);

        let mut file_header = tar::Header::new_gnu();
        file_header
            .set_path("node-v24.15.0-darwin-arm64/bin/node")
            .expect("set file path");
        file_header.set_mode(0o755);
        file_header.set_size(4);
        file_header.set_cksum();
        builder.append(&file_header, &b"node"[..]).expect("append node");

        let mut link_header = tar::Header::new_gnu();
        link_header.set_entry_type(EntryType::Symlink);
        link_header
            .set_path("node-v24.15.0-darwin-arm64/bin/npm")
            .expect("set link path");
        link_header
            .set_link_name("../lib/node_modules/npm/bin/npm-cli.js")
            .expect("set link name");
        link_header.set_size(0);
        link_header.set_cksum();
        builder.append(&link_header, std::io::empty()).expect("append symlink");

        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish gzip");

        let target_dir = temp_dir.path().join("extract");
        let installer = RuntimeInstaller::new();
        installer
            .prepare_extract_dir(&target_dir)
            .await
            .expect("prepare extract dir");
        installer
            .extract_tar_gz_skipping_links(&tar_path, &target_dir)
            .await
            .expect("extract tar.gz while skipping links");

        assert!(target_dir.join("node-v24.15.0-darwin-arm64/bin/node").exists());
        assert!(!target_dir.join("node-v24.15.0-darwin-arm64/bin/npm").exists());
    }

    #[test]
    fn unique_temp_dir_creates_distinct_paths() {
        assert_ne!(
            RuntimeInstaller::unique_temp_dir("mcpmate-test"),
            RuntimeInstaller::unique_temp_dir("mcpmate-test")
        );
    }
}
