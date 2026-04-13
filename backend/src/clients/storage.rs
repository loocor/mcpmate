use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs;

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{BackupPolicySetting, StorageKind};
use crate::system::paths::get_path_service;

pub type DynConfigStorage = Arc<dyn ConfigStorage>;

const MAX_DEFAULT_BACKUPS: usize = 30;

#[derive(Debug, Clone)]
pub struct BackupFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[async_trait]
pub trait ConfigStorage: Send + Sync {
    fn kind(&self) -> StorageKind;

    async fn read(
        &self,
        config_path: &str,
    ) -> ConfigResult<Option<String>>;

    async fn write_atomic(
        &self,
        identifier: &str,
        config_path: &str,
        content: &str,
        policy: &BackupPolicySetting,
    ) -> ConfigResult<Option<String>>;

    async fn list_backups(
        &self,
        identifier: &str,
        config_path: &str,
    ) -> ConfigResult<Vec<BackupFile>>;

    async fn delete_backup(
        &self,
        identifier: &str,
        config_path: &str,
        backup_name: &str,
    ) -> ConfigResult<()>;

    async fn restore_backup(
        &self,
        identifier: &str,
        config_path: &str,
        backup_name: &str,
        policy: &BackupPolicySetting,
    ) -> ConfigResult<Option<String>>;
}

/// File-based configuration storage adapter
pub struct FileConfigStorage;

impl Default for FileConfigStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl FileConfigStorage {
    pub fn new() -> Self {
        Self
    }

    async fn resolve_target_path(
        &self,
        config_path: &str,
    ) -> ConfigResult<PathBuf> {
        let path_service = get_path_service();
        path_service
            .resolve_user_path(config_path)
            .map_err(|err| ConfigError::PathResolutionError(err.to_string()))
    }
}

#[async_trait]
impl ConfigStorage for FileConfigStorage {
    fn kind(&self) -> StorageKind {
        StorageKind::File
    }

    async fn read(
        &self,
        config_path: &str,
    ) -> ConfigResult<Option<String>> {
        let path = match self.resolve_target_path(config_path).await {
            Ok(path) => path,
            Err(ConfigError::PathResolutionError(_)) => return Ok(None),
            Err(err) => return Err(err),
        };

        match fs::read_to_string(path).await {
            Ok(content) => Ok(Some(content)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(ConfigError::IoError(err)),
        }
    }

    async fn write_atomic(
        &self,
        identifier: &str,
        config_path: &str,
        content: &str,
        policy: &BackupPolicySetting,
    ) -> ConfigResult<Option<String>> {
        let path = self.resolve_target_path(config_path).await?;

        // Normalize JSON forward slashes to avoid "\/" in on-disk files.
        // Serde may legally emit escaped slashes; some editors persist them literally.
        // We prefer human-friendly URLs in configs.
        let normalized = content.replace("\\/", "/");

        if self.read(config_path).await?.as_deref() == Some(normalized.as_str()) {
            return Ok(None);
        }

        if !policy.should_backup() {
            let path_service = get_path_service();
            path_service
                .ensure_parent_dirs(&path)
                .await
                .map_err(|err| ConfigError::FileOperationError(err.to_string()))?;
            tokio::fs::write(&path, normalized.as_bytes())
                .await
                .map_err(ConfigError::IoError)?;
            return Ok(None);
        }

        let retention = policy.retention_limit();
        let max_backups = retention.unwrap_or(MAX_DEFAULT_BACKUPS);

        self.write_with_custom_limit(&path, &normalized, max_backups, identifier)
            .await
    }

    async fn list_backups(
        &self,
        identifier: &str,
        config_path: &str,
    ) -> ConfigResult<Vec<BackupFile>> {
        let target = match self.resolve_target_path(config_path).await {
            Ok(path) => path,
            Err(ConfigError::PathResolutionError(_)) => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };

        let path_service = get_path_service();
        let entries = path_service
            .list_backups_for(Some(identifier), &target)
            .await
            .map_err(|err| ConfigError::FileOperationError(err.to_string()))?;

        let mut backups = Vec::with_capacity(entries.len());
        for path in entries {
            let metadata = match fs::metadata(&path).await {
                Ok(meta) => meta,
                Err(err) => {
                    tracing::warn!("Skipping backup {} due to metadata error: {}", path.display(), err);
                    continue;
                }
            };

            let modified_at = metadata.modified().ok().map(chrono::DateTime::<chrono::Utc>::from);

            let name = path
                .file_name()
                .and_then(|os| os.to_str())
                .unwrap_or_default()
                .to_string();

            backups.push(BackupFile {
                name,
                path: path.clone(),
                size: metadata.len(),
                modified_at,
            });
        }

        Ok(backups)
    }

    async fn delete_backup(
        &self,
        identifier: &str,
        config_path: &str,
        backup_name: &str,
    ) -> ConfigResult<()> {
        let target = self.resolve_target_path(config_path).await?;
        let path_service = get_path_service();
        let backup_path = path_service
            .backup_path_for(Some(identifier), &target, backup_name)
            .map_err(|err| ConfigError::FileOperationError(err.to_string()))?;

        if !backup_path.exists() {
            return Ok(());
        }

        fs::remove_file(&backup_path).await.map_err(ConfigError::IoError)?;

        Ok(())
    }

    async fn restore_backup(
        &self,
        identifier: &str,
        config_path: &str,
        backup_name: &str,
        policy: &BackupPolicySetting,
    ) -> ConfigResult<Option<String>> {
        let target = self.resolve_target_path(config_path).await?;
        let path_service = get_path_service();
        let backup_path = path_service
            .backup_path_for(Some(identifier), &target, backup_name)
            .map_err(|err| ConfigError::FileOperationError(err.to_string()))?;

        if !backup_path.exists() {
            return Err(ConfigError::FileOperationError(format!(
                "Backup {} not found for client {}",
                backup_name, identifier
            )));
        }

        let content = fs::read_to_string(&backup_path).await.map_err(ConfigError::IoError)?;

        self.write_atomic(identifier, config_path, &content, policy).await
    }
}

impl FileConfigStorage {
    pub fn as_dyn(self) -> DynConfigStorage {
        Arc::new(self)
    }

    async fn write_with_custom_limit(
        &self,
        target: &std::path::Path,
        content: &str,
        max_backups: usize,
        identifier: &str,
    ) -> ConfigResult<Option<String>> {
        let path_service = get_path_service();
        let backup = path_service
            .atomic_write_with_backup(target, content.as_bytes(), Some(max_backups), Some(identifier))
            .await
            .map_err(|err| ConfigError::FileOperationError(err.to_string()))?;

        Ok(backup.map(|path| path.to_string_lossy().to_string()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn normalize_escaped_forward_slashes() {
        let input = r#"{"url": "https:\/\/example.com\/path"}"#;
        let expected = r#"{"url": "https://example.com/path"}"#;
        assert_eq!(input.replace("\\/", "/"), expected);
    }

    #[test]
    fn normalize_preserves_unescaped_slashes() {
        let input = r#"{"url": "https://example.com/path"}"#;
        assert_eq!(input.replace("\\/", "/"), input);
    }
}
