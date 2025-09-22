// Unified path service to eliminate path handling duplication
// Centralizes all path resolution, template processing, and platform-specific logic

use super::PathMapper;
use anyhow::{Context, Result};
use chrono::Utc;
use nanoid::nanoid;
use std::ffi::OsStr;
#[cfg(windows)]
use std::path::PrefixComponent;
use std::path::{Component, Path, PathBuf};
use tokio::fs;

/// Unified path service for consistent path handling across the application
pub struct PathService {
    path_mapper: PathMapper,
}

const MAX_BACKUPS_PER_FILE: usize = 5;

impl PathService {
    /// Create a new path service with system variables
    pub fn new() -> Result<Self> {
        Ok(Self {
            path_mapper: PathMapper::new()?,
        })
    }

    /// Resolve any path template with consistent logic
    /// This replaces scattered template resolution logic
    pub fn resolve_path_template(
        &self,
        template: &str,
    ) -> Result<PathBuf> {
        self.path_mapper
            .resolve_template(template)
            .context(format!("Failed to resolve path template: {}", template))
    }

    /// Resolve a user-provided path (supports ~ and template variables)
    pub fn resolve_user_path(
        &self,
        template: &str,
    ) -> Result<PathBuf> {
        self.resolve_path_template(template)
    }

    /// Get runtime binary path with unified logic
    /// This replaces scattered runtime path logic
    pub fn resolve_runtime_path(
        &self,
        relative_bin_path: &str,
    ) -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Unable to determine home directory"))?;

        let bin_path = if relative_bin_path.starts_with('/') {
            // Absolute path (system runtime) - use as-is
            PathBuf::from(relative_bin_path)
        } else if relative_bin_path.starts_with(".mcpmate/") {
            // Already properly formatted relative path
            home_dir.join(relative_bin_path)
        } else {
            // Relative path that needs .mcpmate prefix
            home_dir.join(format!(".mcpmate/{}", relative_bin_path))
        };

        Ok(bin_path)
    }

    /// Get detection rule path with unified logic
    /// This replaces scattered detection path logic
    pub fn resolve_detection_path(
        &self,
        detection_value: &str,
    ) -> Result<PathBuf> {
        self.path_mapper
            .resolve_template(detection_value)
            .context(format!("Failed to resolve detection path: {}", detection_value))
    }

    /// Get current platform string consistently
    /// This replaces scattered platform detection logic
    pub fn get_current_platform() -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "macos"
        }
        #[cfg(target_os = "windows")]
        {
            "windows"
        }
        #[cfg(target_os = "linux")]
        {
            "linux"
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            "unknown"
        }
    }

    /// Create parent directories if they don't exist
    /// This replaces scattered directory creation logic
    pub async fn ensure_parent_dirs(
        &self,
        file_path: &Path,
    ) -> Result<()> {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.context(format!(
                "Failed to create parent directories for: {}",
                file_path.display()
            ))?;
        }
        Ok(())
    }

    /// Atomically write content to target with optional backup of existing file
    pub async fn atomic_write_with_backup(
        &self,
        target: &Path,
        content: &[u8],
        max_backups: Option<usize>,
        identifier: Option<&str>,
    ) -> Result<Option<PathBuf>> {
        self.ensure_parent_dirs(target).await?;

        let tmp_suffix = nanoid!(8);
        let tmp_path = target.with_extension(format!("tmp.{}", tmp_suffix));
        fs::write(&tmp_path, content)
            .await
            .context(format!("Failed to write temporary file: {}", tmp_path.display()))?;

        let target_buf = target.to_path_buf();
        let exists = self.validate_path_exists(&target_buf).await?;
        let mut backup_path = None;

        if exists {
            let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
            let (backup_dir, candidate, file_prefix) =
                self.build_backup_destination(identifier, &target_buf, &timestamp)?;

            fs::create_dir_all(&backup_dir)
                .await
                .context(format!("Failed to create backup directory: {}", backup_dir.display()))?;

            fs::copy(&target_buf, &candidate)
                .await
                .context(format!("Failed to create backup file: {}", candidate.display()))?;

            let retention = max_backups.unwrap_or(MAX_BACKUPS_PER_FILE);
            if let Err(err) = self.prune_old_backups(&backup_dir, &file_prefix, retention).await {
                tracing::warn!("Failed to prune old backups in {}: {}", backup_dir.display(), err);
            }

            backup_path = Some(candidate);
        }

        match fs::rename(&tmp_path, &target_buf).await {
            Ok(_) => Ok(backup_path),
            Err(err) => {
                let _ = fs::remove_file(&tmp_path).await;
                if let Some(ref backup) = backup_path {
                    let _ = fs::copy(backup, &target_buf).await;
                }
                Err(anyhow::anyhow!(
                    "Failed to replace file {}: {}",
                    target_buf.display(),
                    err
                ))
            }
        }
    }

    fn backups_root(&self) -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot get user home directory"))?;
        Ok(home_dir.join(".mcpmate").join("backups").join("client"))
    }

    fn sanitize_component(component: &OsStr) -> String {
        let sanitized: String = component
            .to_string_lossy()
            .chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' | '_' => c,
                _ => '_',
            })
            .collect();

        if sanitized.is_empty() {
            "_".to_string()
        } else {
            sanitized
        }
    }

    fn sanitize_identifier(identifier: &str) -> String {
        let value: String = identifier
            .chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' | '_' => c,
                _ => '_',
            })
            .collect();

        if value.is_empty() { "_".to_string() } else { value }
    }

    fn build_backup_destination(
        &self,
        identifier: Option<&str>,
        target: &Path,
        timestamp: &str,
    ) -> Result<(PathBuf, PathBuf, String)> {
        let (dir, file_component) = self.backup_dir_and_prefix(identifier, target)?;
        let file_name = format!("{}.{}.bak", file_component, timestamp);
        let backup_path = dir.join(file_name);

        Ok((dir, backup_path, file_component))
    }

    fn backup_dir_and_prefix(
        &self,
        identifier: Option<&str>,
        target: &Path,
    ) -> Result<(PathBuf, String)> {
        let mut dir = self.backups_root()?;

        if let Some(id) = identifier {
            dir.push(Self::sanitize_identifier(id));
        } else if let Some(parent) = target.parent() {
            for component in parent.components() {
                match component {
                    Component::Normal(os) => dir.push(Self::sanitize_component(os)),
                    #[cfg(windows)]
                    Component::Prefix(prefix) => {
                        let prefix_str: &OsStr = prefix.as_os_str();
                        dir.push(Self::sanitize_component(prefix_str));
                    }
                    _ => {}
                }
            }
        }

        let file_component = target
            .file_name()
            .map(|os| Self::sanitize_component(os))
            .unwrap_or_else(|| "config".to_string());

        Ok((dir, file_component))
    }

    pub async fn list_backups_for(
        &self,
        identifier: Option<&str>,
        target: &Path,
    ) -> Result<Vec<PathBuf>> {
        let (dir, file_prefix) = self.backup_dir_and_prefix(identifier, target)?;
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&dir)
            .await
            .context(format!("Failed to read backup directory: {}", dir.display()))?;

        let mut backups = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|os| os.to_str()) {
                    if name.starts_with(&file_prefix) && name.ends_with(".bak") {
                        backups.push(path);
                    }
                }
            }
        }

        backups.sort();
        Ok(backups)
    }

    pub fn backup_path_for(
        &self,
        identifier: Option<&str>,
        target: &Path,
        backup_name: &str,
    ) -> Result<PathBuf> {
        let (dir, _) = self.backup_dir_and_prefix(identifier, target)?;
        Ok(dir.join(backup_name))
    }

    async fn prune_old_backups(
        &self,
        dir: &Path,
        file_prefix: &str,
        retention: usize,
    ) -> Result<()> {
        let mut entries = fs::read_dir(dir)
            .await
            .context(format!("Failed to read backup directory: {}", dir.display()))?;

        let mut backups = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|os| os.to_str()) {
                    if name.starts_with(file_prefix) && name.ends_with(".bak") {
                        backups.push(path);
                    }
                }
            }
        }

        if backups.len() <= retention {
            return Ok(());
        }

        backups.sort();
        let remove_count = backups.len() - retention;
        for path in backups.into_iter().take(remove_count) {
            if let Err(err) = fs::remove_file(&path).await {
                tracing::warn!("Failed to remove old backup {}: {}", path.display(), err);
            }
        }

        Ok(())
    }

    /// Validate that a path exists and is accessible
    /// This adds consistent path validation across the application
    pub async fn validate_path_exists(
        &self,
        path: &PathBuf,
    ) -> Result<bool> {
        match tokio::fs::metadata(path).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Failed to check path {}: {}", path.display(), e)),
        }
    }

    /// Get the path mapper for advanced operations
    pub fn path_mapper(&self) -> &PathMapper {
        &self.path_mapper
    }
}

impl Default for PathService {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            path_mapper: PathMapper::default(),
        })
    }
}

/// Global path service instance for consistent usage
static PATH_SERVICE: std::sync::OnceLock<PathService> = std::sync::OnceLock::new();

/// Get the global path service instance
pub fn get_path_service() -> &'static PathService {
    PATH_SERVICE.get_or_init(PathService::default)
}
