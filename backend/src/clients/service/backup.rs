use super::core::{ClientBackupRecord, ClientConfigService};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::BackupPolicySetting;
use crate::clients::source::ClientConfigSource;
use crate::clients::storage::BackupFile;
use crate::common::constants::timeouts;

impl ClientConfigService {
    pub async fn list_backups(
        &self,
        identifier: Option<&str>,
    ) -> ConfigResult<Vec<ClientBackupRecord>> {
        let templates = if let Some(id) = identifier {
            vec![self.get_client_template(id).await?]
        } else {
            self.template_source.list_client().await?
        };

        let mut records = Vec::new();
        for template in templates {
            let storage = self.template_engine.storage_for_template(&template)?;
            let backups = storage.list_backups(&template).await?;
            for backup in backups {
                records.push(Self::map_backup_record(&template.identifier, backup));
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
        let template = self.get_client_template(identifier).await?;
        let storage = self.template_engine.storage_for_template(&template)?;
        storage.delete_backup(&template, backup).await
    }

    pub async fn restore_backup(
        &self,
        identifier: &str,
        backup: &str,
    ) -> ConfigResult<Option<String>> {
        let template = self.get_client_template(identifier).await?;
        let storage = self.template_engine.storage_for_template(&template)?;
        let policy = self.get_backup_policy(identifier).await?;
        storage.restore_backup(&template, backup, &policy).await
    }

    /// Schedule a background write when the target storage is temporarily locked.
    pub fn schedule_write_after_unlock(
        &self,
        template: crate::clients::models::ClientTemplate,
        content: String,
        policy: BackupPolicySetting,
    ) -> ConfigResult<()> {
        let storage = self.template_engine.storage_for_template(&template)?;
        let identifier = template.identifier.clone();
        let retry_window = std::time::Duration::from_secs(timeouts::CHERRY_LOCK_RETRY_WINDOW_SEC);
        let interval = std::time::Duration::from_millis(timeouts::CHERRY_LOCK_RETRY_INTERVAL_MS);

        tokio::spawn(async move {
            use std::time::Instant;
            let deadline = Instant::now() + retry_window;
            loop {
                match storage.write_atomic(&template, &content, &policy).await {
                    Ok(_) => {
                        tracing::info!(client = %identifier, "Deferred Cherry config write applied after unlock");
                        break;
                    }
                    Err(ConfigError::FileOperationError(msg)) if msg.to_ascii_lowercase().contains("locked") => {
                        if Instant::now() >= deadline {
                            tracing::error!(client = %identifier, "Deferred write window expired; database remained locked");
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
