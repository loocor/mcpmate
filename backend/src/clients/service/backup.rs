use super::core::{ClientBackupRecord, ClientConfigService};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::BackupPolicySetting;
use crate::clients::storage::BackupFile;

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
        let config_path = state
            .config_path()
            .ok_or_else(|| ConfigError::PathResolutionError(format!("No config_path for client {}", identifier)))?;
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
            vec![
                self.fetch_state(id)
                    .await?
                    .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} not found", id)))?,
            ]
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
        let state = self
            .fetch_state(identifier)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} not found", identifier)))?;
        let config_path = state
            .config_path()
            .ok_or_else(|| ConfigError::PathResolutionError(format!("No config_path for client {}", identifier)))?;
        let storage = self.template_engine.storage_for_client(&state)?;
        storage.delete_backup(identifier, config_path, backup).await
    }

    pub async fn restore_backup(
        &self,
        identifier: &str,
        backup: &str,
    ) -> ConfigResult<Option<String>> {
        let state = self
            .fetch_state(identifier)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} not found", identifier)))?;
        let config_path = state
            .config_path()
            .ok_or_else(|| ConfigError::PathResolutionError(format!("No config_path for client {}", identifier)))?;
        let storage = self.template_engine.storage_for_client(&state)?;
        let policy = self.get_backup_policy(identifier).await?;
        let restored = storage.restore_backup(identifier, config_path, backup, &policy).await?;
        self.enforce_backup_retention(identifier, &policy).await?;
        Ok(restored)
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
