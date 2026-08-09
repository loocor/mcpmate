// Unified path service to eliminate path handling duplication
// Centralizes all path resolution, template processing, and platform-specific logic

use super::PathMapper;
use anyhow::{Context, Result};
use chrono::Local;
use nanoid::nanoid;
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::path::PrefixComponent;
use std::path::{Component, Path, PathBuf};
use tokio::fs;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW};

/// Unified path service for consistent path handling across the application
pub struct PathService {
    path_mapper: PathMapper,
    backup_root: Option<PathBuf>,
}

const MAX_BACKUPS_PER_FILE: usize = 5;

fn format_backup_timestamp(timestamp: chrono::DateTime<chrono::FixedOffset>) -> String {
    timestamp.format("%Y%m%d%H%M%S%z").to_string()
}

fn sort_backups_by_modified(backups: &mut [(std::time::SystemTime, PathBuf)]) {
    backups.sort_by(|(left_time, left_path), (right_time, right_path)| {
        left_time.cmp(right_time).then_with(|| left_path.cmp(right_path))
    });
}

fn set_backup_modified_time(
    path: &Path,
    modified_at: std::time::SystemTime,
) -> Result<()> {
    std::fs::File::options()
        .write(true)
        .open(path)
        .context(format!("Failed to open backup file: {}", path.display()))?
        .set_modified(modified_at)
        .context(format!("Failed to set backup modified time: {}", path.display()))
}

fn create_backup_file(
    source: &Path,
    destination: &Path,
    created_at: std::time::SystemTime,
) -> Result<()> {
    let staging = destination.with_extension(format!("pending.{}", nanoid!(8)));
    let result = (|| {
        std::fs::copy(source, &staging).context(format!("Failed to create backup file: {}", destination.display()))?;
        set_backup_modified_time(&staging, created_at)?;
        replace_existing_file(&staging, destination)
            .context(format!("Failed to publish backup file: {}", destination.display()))
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(staging);
    }
    result
}

impl PathService {
    /// Create a new path service with system variables
    pub fn new() -> Result<Self> {
        Ok(Self {
            path_mapper: PathMapper::new()?,
            backup_root: None,
        })
    }

    pub fn with_backup_root(
        mut self,
        backup_root: impl Into<PathBuf>,
    ) -> Self {
        self.backup_root = Some(backup_root.into());
        self
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
            let created_at = std::time::SystemTime::now();
            let timestamp = format_backup_timestamp(chrono::DateTime::<Local>::from(created_at).fixed_offset());
            let (backup_dir, candidate, file_prefix) =
                self.build_backup_destination(identifier, &target_buf, &timestamp)?;

            fs::create_dir_all(&backup_dir)
                .await
                .context(format!("Failed to create backup directory: {}", backup_dir.display()))?;

            let backup_source = target_buf.clone();
            let backup_destination = candidate.clone();
            let backup_result = tokio::task::spawn_blocking(move || {
                create_backup_file(&backup_source, &backup_destination, created_at)
            })
            .await
            .context("Backup creation task failed")
            .and_then(|result| result);
            if let Err(err) = backup_result {
                let _ = fs::remove_file(&tmp_path).await;
                return Err(err);
            }

            let retention = max_backups.unwrap_or(MAX_BACKUPS_PER_FILE);
            if let Err(err) = self.prune_old_backups(&backup_dir, &file_prefix, retention).await {
                tracing::warn!("Failed to prune old backups in {}: {}", backup_dir.display(), err);
            }

            backup_path = Some(candidate);
        }

        match replace_existing_file(&tmp_path, &target_buf) {
            Ok(()) => Ok(backup_path),
            Err(err) => {
                let _ = fs::remove_file(&tmp_path).await;
                if let Some(ref backup) = backup_path {
                    let _ = fs::copy(backup, &target_buf).await;
                }
                Err(err)
            }
        }
    }

    /// Atomically replace content without creating or pruning backups.
    pub async fn atomic_write(
        &self,
        target: &Path,
        content: &[u8],
    ) -> Result<()> {
        self.ensure_parent_dirs(target).await?;

        let tmp_suffix = nanoid!(8);
        let tmp_path = target.with_extension(format!("tmp.{}", tmp_suffix));
        fs::write(&tmp_path, content)
            .await
            .context(format!("Failed to write temporary file: {}", tmp_path.display()))?;

        if let Err(err) = replace_existing_file(&tmp_path, target) {
            let _ = fs::remove_file(&tmp_path).await;
            return Err(err);
        }

        Ok(())
    }

    pub fn atomic_write_with_backup_sync(
        &self,
        target: &Path,
        content: &[u8],
        max_backups: Option<usize>,
        identifier: Option<&str>,
    ) -> Result<Option<PathBuf>> {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .context(format!("Failed to create parent directories for: {}", target.display()))?;
        }

        let tmp_suffix = nanoid!(8);
        let tmp_path = target.with_extension(format!("tmp.{}", tmp_suffix));
        std::fs::write(&tmp_path, content)
            .context(format!("Failed to write temporary file: {}", tmp_path.display()))?;

        let target_buf = target.to_path_buf();
        let exists = target_buf.exists();
        let mut backup_path = None;

        if exists {
            let created_at = std::time::SystemTime::now();
            let timestamp = format_backup_timestamp(chrono::DateTime::<Local>::from(created_at).fixed_offset());
            let (backup_dir, candidate, file_prefix) =
                self.build_backup_destination(identifier, &target_buf, &timestamp)?;

            std::fs::create_dir_all(&backup_dir)
                .context(format!("Failed to create backup directory: {}", backup_dir.display()))?;

            if let Err(err) = create_backup_file(&target_buf, &candidate, created_at) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(err);
            }

            let retention = max_backups.unwrap_or(MAX_BACKUPS_PER_FILE);
            if let Err(err) = self.prune_old_backups_sync(&backup_dir, &file_prefix, retention) {
                tracing::warn!("Failed to prune old backups in {}: {}", backup_dir.display(), err);
            }

            backup_path = Some(candidate);
        }

        match replace_existing_file(&tmp_path, &target_buf) {
            Ok(()) => Ok(backup_path),
            Err(err) => {
                let _ = std::fs::remove_file(&tmp_path);
                if let Some(ref backup) = backup_path {
                    let _ = std::fs::copy(backup, &target_buf);
                }
                Err(err)
            }
        }
    }

    fn backups_root(&self) -> Result<PathBuf> {
        if let Some(backup_root) = &self.backup_root {
            return Ok(backup_root.clone());
        }
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
            .map(Self::sanitize_component)
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
                        let modified = entry
                            .metadata()
                            .await
                            .context(format!("Failed to read backup metadata: {}", path.display()))?
                            .modified()
                            .context(format!("Failed to read backup modified time: {}", path.display()))?;
                        backups.push((modified, path));
                    }
                }
            }
        }

        if backups.len() <= retention {
            return Ok(());
        }

        sort_backups_by_modified(&mut backups);
        let remove_count = backups.len() - retention;
        for (_, path) in backups.into_iter().take(remove_count) {
            if let Err(err) = fs::remove_file(&path).await {
                tracing::warn!("Failed to remove old backup {}: {}", path.display(), err);
            }
        }

        Ok(())
    }

    fn prune_old_backups_sync(
        &self,
        dir: &Path,
        file_prefix: &str,
        retention: usize,
    ) -> Result<()> {
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(dir).context(format!("Failed to read backup directory: {}", dir.display()))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|os| os.to_str()) {
                    if name.starts_with(file_prefix) && name.ends_with(".bak") {
                        let modified = entry
                            .metadata()
                            .context(format!("Failed to read backup metadata: {}", path.display()))?
                            .modified()
                            .context(format!("Failed to read backup modified time: {}", path.display()))?;
                        backups.push((modified, path));
                    }
                }
            }
        }

        if backups.len() <= retention {
            return Ok(());
        }

        sort_backups_by_modified(&mut backups);
        let remove_count = backups.len() - retention;
        for (_, path) in backups.into_iter().take(remove_count) {
            if let Err(err) = std::fs::remove_file(&path) {
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

#[cfg(windows)]
fn replace_existing_file(
    source: &Path,
    target: &Path,
) -> Result<()> {
    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

    let replaced = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            target_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };

    if replaced == 0 {
        return Err(std::io::Error::last_os_error()).context(format!(
            "Failed to replace file {} with {}",
            target.display(),
            source.display()
        ));
    }

    Ok(())
}

#[cfg(not(windows))]
fn replace_existing_file(
    source: &Path,
    target: &Path,
) -> Result<()> {
    std::fs::rename(source, target).context(format!(
        "Failed to replace file {} with {}",
        target.display(),
        source.display()
    ))?;
    Ok(())
}

impl Default for PathService {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            path_mapper: PathMapper::default(),
            backup_root: None,
        })
    }
}

/// Global path service instance for consistent usage
static PATH_SERVICE: std::sync::OnceLock<PathService> = std::sync::OnceLock::new();

/// Get the global path service instance
pub fn get_path_service() -> &'static PathService {
    PATH_SERVICE.get_or_init(PathService::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::time::{Duration, SystemTime};

    fn write_file_at(
        path: &Path,
        modified_at: SystemTime,
    ) {
        std::fs::write(path, "content").expect("write file");
        File::options()
            .write(true)
            .open(path)
            .expect("open file")
            .set_modified(modified_at)
            .expect("set file modified time");
    }

    fn make_read_only(path: &Path) -> std::fs::Permissions {
        let original = std::fs::metadata(path).expect("file metadata").permissions();
        let mut read_only = original.clone();
        read_only.set_readonly(true);
        std::fs::set_permissions(path, read_only).expect("make file read-only");
        original
    }

    fn assert_backup_creation_time(
        backup: &Path,
        target_name: &str,
    ) {
        let name = backup.file_name().and_then(OsStr::to_str).expect("backup name");
        let timestamp = name
            .strip_prefix(&format!("{target_name}."))
            .and_then(|value| value.strip_suffix(".bak"))
            .expect("backup timestamp");
        let modified_at = std::fs::metadata(backup)
            .expect("backup metadata")
            .modified()
            .expect("backup modified time");
        let modified_at = chrono::DateTime::<Local>::from(modified_at);

        assert_eq!(timestamp, modified_at.format("%Y%m%d%H%M%S%z").to_string());
    }

    fn seed_backups() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let directory = tempfile::tempdir().expect("temp backup directory");
        let older = directory.path().join("z.bak");
        let newer = directory.path().join("a.bak");

        for (path, modified_seconds) in [(&older, 1), (&newer, 2)] {
            write_file_at(path, SystemTime::UNIX_EPOCH + Duration::from_secs(modified_seconds));
        }

        (directory, older, newer)
    }

    #[tokio::test]
    async fn async_and_sync_backups_use_creation_time_for_name_and_metadata() {
        let directory = tempfile::tempdir().expect("temp directory");
        let service = PathService::default().with_backup_root(directory.path().join("backups"));
        let async_target = directory.path().join("async.json");
        let sync_target = directory.path().join("sync.json");
        write_file_at(&async_target, SystemTime::UNIX_EPOCH);
        write_file_at(&sync_target, SystemTime::UNIX_EPOCH);

        let async_backup = service
            .atomic_write_with_backup(&async_target, b"updated", Some(1), Some("test"))
            .await
            .expect("async write")
            .expect("async backup");
        let sync_backup = service
            .atomic_write_with_backup_sync(&sync_target, b"updated", Some(1), Some("test"))
            .expect("sync write")
            .expect("sync backup");

        assert_backup_creation_time(&async_backup, "async.json");
        assert_backup_creation_time(&sync_backup, "sync.json");

        let created_at = std::fs::metadata(&sync_backup)
            .expect("sync backup metadata")
            .modified()
            .expect("sync backup modified time");
        create_backup_file(&sync_target, &sync_backup, created_at).expect("replace same-second backup");
        assert_eq!(
            std::fs::read_to_string(sync_backup).expect("read replaced backup"),
            "updated"
        );
    }

    #[tokio::test]
    async fn failed_backup_timestamp_does_not_leave_discoverable_or_temporary_files() {
        let directory = tempfile::tempdir().expect("temp directory");
        let service = PathService::default().with_backup_root(directory.path().join("backups"));
        let async_target = directory.path().join("async.json");
        let sync_target = directory.path().join("sync.json");
        write_file_at(&async_target, SystemTime::UNIX_EPOCH);
        write_file_at(&sync_target, SystemTime::UNIX_EPOCH);
        let async_permissions = make_read_only(&async_target);
        let sync_permissions = make_read_only(&sync_target);

        let async_result = service
            .atomic_write_with_backup(&async_target, b"updated", Some(1), Some("test"))
            .await;
        let sync_result = service.atomic_write_with_backup_sync(&sync_target, b"updated", Some(1), Some("test"));
        std::fs::set_permissions(&async_target, async_permissions).expect("restore async permissions");
        std::fs::set_permissions(&sync_target, sync_permissions).expect("restore sync permissions");

        assert!(async_result.is_err());
        assert!(sync_result.is_err());
        for target in [&async_target, &sync_target] {
            assert!(
                service
                    .list_backups_for(Some("test"), target)
                    .await
                    .expect("list backups")
                    .is_empty()
            );
        }
        assert!(
            std::fs::read_dir(directory.path())
                .expect("read target directory")
                .all(|entry| !entry
                    .expect("target directory entry")
                    .file_name()
                    .to_string_lossy()
                    .contains(".tmp."))
        );
    }

    #[tokio::test]
    async fn async_and_sync_pruning_use_file_modified_time() {
        let (async_directory, async_older, async_newer) = seed_backups();
        let (sync_directory, sync_older, sync_newer) = seed_backups();

        PathService::default()
            .prune_old_backups(async_directory.path(), "", 1)
            .await
            .expect("prune async backups");
        PathService::default()
            .prune_old_backups_sync(sync_directory.path(), "", 1)
            .expect("prune sync backups");

        assert!(!async_older.exists());
        assert!(async_newer.exists());
        assert!(!sync_older.exists());
        assert!(sync_newer.exists());
    }
}
