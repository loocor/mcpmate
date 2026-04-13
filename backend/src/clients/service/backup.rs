use super::core::{ClientBackupRecord, ClientConfigService};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::BackupPolicySetting;
use crate::clients::storage::BackupFile;
use crate::common::constants::timeouts;

impl ClientConfigService {
    pub async fn enforce_backup_retention(
        &self,
        identifier: &str,
        policy: &BackupPolicySetting,
    ) -> ConfigResult<()> {
        let Some(retention) = policy.retention_limit() else {
            return Ok(());
        };

        let state = match self.fetch_state(identifier).await? {
            Some(state) => state,
            None => return Ok(()),
        };
        let config_path = state.config_path().ok_or_else(|| {
            ConfigError::PathResolutionError(format!("No config_path for client {}", identifier))
        })?;
        let storage = self.template_engine.storage_for_client(&state)?;
        let mut backups = storage.list_backups(identifier, config_path).await?;

        if backups.len() <= retention {
            return Ok(());
        }

        backups.sort_by(|left, right| {
            left.modified_at
                .cmp(&right.modified_at)
                .then_with(|| left.name.cmp(&right.name))
        });

        let remove_count = backups.len() - retention;
        for backup in backups.into_iter().take(remove_count) {
            storage.delete_backup(identifier, config_path, &backup.name).await?;
        }

        Ok(())
    }

    pub async fn delete_all_client_backups(
        &self,
        identifier: &str,
    ) -> ConfigResult<usize> {
        let state = match self.fetch_state(identifier).await? {
            Some(state) => state,
            None => return Ok(0),
        };
        let Some(config_path) = state.config_path() else {
            return Ok(0);
        };
        let storage = self.template_engine.storage_for_client(&state)?;
        let backups = storage.list_backups(identifier, config_path).await?;
        let deleted = backups.len();

        for backup in backups {
            storage.delete_backup(identifier, config_path, &backup.name).await?;
        }

        Ok(deleted)
    }

    pub async fn list_backups(
        &self,
        identifier: Option<&str>,
    ) -> ConfigResult<Vec<ClientBackupRecord>> {
        let states = if let Some(id) = identifier {
            vec![self.fetch_state(id).await?.ok_or_else(|| {
                ConfigError::DataAccessError(format!("Client {} not found", id))
            })?]
        } else {
            self.fetch_client_states().await?.into_values().collect()
        };

        let mut records = Vec::new();
        for state in states {
            let identifier = state.identifier();
            let config_path = match state.config_path() {
                Some(path) => path,
                None => continue,
            };
            let storage = match self.template_engine.storage_for_client(&state) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let backups = storage.list_backups(identifier, config_path).await?;
            for backup in backups {
                records.push(Self::map_backup_record(identifier, backup));
            }
        }

        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(records)
    }

    pub async fn delete_backup(
        &self,
        identifier: &str,
        backup: &str,
    ) -> ConfigResult<()> {
        let state = self.fetch_state(identifier).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Client {} not found", identifier))
        })?;
        let config_path = state.config_path().ok_or_else(|| {
            ConfigError::PathResolutionError(format!("No config_path for client {}", identifier))
        })?;
        let storage = self.template_engine.storage_for_client(&state)?;
        storage.delete_backup(identifier, config_path, backup).await
    }

    pub async fn restore_backup(
        &self,
        identifier: &str,
        backup: &str,
    ) -> ConfigResult<Option<String>> {
        let state = self.fetch_state(identifier).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Client {} not found", identifier))
        })?;
        let config_path = state.config_path().ok_or_else(|| {
            ConfigError::PathResolutionError(format!("No config_path for client {}", identifier))
        })?;
        let storage = self.template_engine.storage_for_client(&state)?;
        let policy = self.get_backup_policy(identifier).await?;
        let restored = storage.restore_backup(identifier, config_path, backup, &policy).await?;
        self.enforce_backup_retention(identifier, &policy).await?;
        Ok(restored)
    }

    /// Schedule a background write when the target storage is temporarily locked.
    pub fn schedule_write_after_unlock(
        &self,
        identifier: String,
        config_path: String,
        content: String,
        policy: BackupPolicySetting,
    ) -> ConfigResult<()> {
        let db_pool = self.db_pool.clone();
        let engine = self.template_engine.clone();
        let detector = self.detector.clone();
        let template_source = self.template_source.clone();
        let retry_window = std::time::Duration::from_secs(timeouts::CHERRY_LOCK_RETRY_WINDOW_SEC);
        let interval = std::time::Duration::from_millis(timeouts::CHERRY_LOCK_RETRY_INTERVAL_MS);

        tokio::spawn(async move {
            use std::time::Instant;
            let deadline = Instant::now() + retry_window;

            // Get storage from state
            let service = ClientConfigService {
                template_source,
                template_engine: engine.clone(),
                detector,
                db_pool: db_pool.clone(),
            };

            let state = match service.fetch_state(&identifier).await {
                Ok(Some(s)) => s,
                _ => {
                    tracing::error!(client = %identifier, "Failed to fetch state for deferred write");
                    return;
                }
            };

            let storage = match engine.storage_for_client(&state) {
                Ok(s) => s,
                Err(err) => {
                    tracing::error!(client = %identifier, error = %err, "Failed to get storage for deferred write");
                    return;
                }
            };

            loop {
                match storage.write_atomic(&identifier, &config_path, &content, &policy).await {
                    Ok(_) => {
                        if let Err(err) = service.enforce_backup_retention(&identifier, &policy).await {
                            tracing::warn!(client = %identifier, error = %err, "Deferred write retention enforcement failed");
                        }
                        tracing::info!(client = %identifier, "Deferred config write applied after unlock");
                        break;
                    }
                    Err(ConfigError::FileOperationError(msg)) if msg.to_ascii_lowercase().contains("locked") => {
                        if Instant::now() >= deadline {
                            tracing::error!(client = %identifier, "Deferred write window expired; storage remained locked");
                            break;
                        }
                        tokio::time::sleep(interval).await;
                    }
                    Err(err) => {
                        tracing::error!(client = %identifier, error = %err, "Deferred write aborted due to non-lock error");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    fn map_backup_record(
        identifier: &str,
        backup: BackupFile,
    ) -> ClientBackupRecord {
        ClientBackupRecord {
            identifier: identifier.to_string(),
            backup: backup.name,
            path: backup.path.to_string_lossy().to_string(),
            size: backup.size,
            created_at: backup.modified_at,
        }
    }
}
