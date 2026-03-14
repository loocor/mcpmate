use super::core::{ClientConfigService, ClientStateRow};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::BackupPolicySetting;
use crate::clients::source::ClientConfigSource;
use crate::system::paths::PathService;
use std::collections::HashMap;

impl ClientConfigService {
    pub(super) async fn fetch_client_states(&self) -> ConfigResult<HashMap<String, ClientStateRow>> {
        let rows = sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, managed, config_mode, transport, client_version, backup_policy, backup_limit FROM client",
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(rows.into_iter().map(|row| (row.identifier.clone(), row)).collect())
    }

    pub(super) async fn ensure_state_row(
        &self,
        identifier: &str,
    ) -> ConfigResult<ClientStateRow> {
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await
    }

    pub(super) async fn ensure_state_row_with_name(
        &self,
        identifier: &str,
        name: &str,
    ) -> ConfigResult<ClientStateRow> {
        if let Some(existing) = self.fetch_state(identifier).await? {
            if existing.name != name {
                sqlx::query(
                    r#"
                    UPDATE client
                    SET name = ?, updated_at = CURRENT_TIMESTAMP
                    WHERE identifier = ?
                    "#,
                )
                .bind(name)
                .bind(identifier)
                .execute(&*self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

                return self.fetch_state(identifier).await?.ok_or_else(|| {
                    ConfigError::DataAccessError(format!("Failed to update management state for client {}", identifier))
                });
            }

            return Ok(existing);
        }

        let generated_id = crate::generate_id!("clnt");
        let insert_result = sqlx::query(
            r#"
            INSERT INTO client (id, name, identifier, managed, backup_policy, backup_limit)
            VALUES (?, ?, ?, 1, 'keep_n', 30)
            "#,
        )
        .bind(&generated_id)
        .bind(name)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await;

        if let Err(err) = insert_result {
            if let sqlx::Error::Database(db_err) = &err {
                if db_err.code().map(|code| code == "2067").unwrap_or(false) {
                    tracing::warn!(
                        client = %identifier,
                        "Concurrent client state insert detected; reusing existing row"
                    );
                } else {
                    return Err(ConfigError::DataAccessError(err.to_string()));
                }
            } else {
                return Err(ConfigError::DataAccessError(err.to_string()));
            }
        }

        self.fetch_state(identifier).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Failed to create management state for client {}", identifier))
        })
    }

    pub(super) async fn fetch_state(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<ClientStateRow>> {
        sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, managed, config_mode, transport, client_version, backup_policy, backup_limit FROM client WHERE identifier = ?",
        )
        .bind(identifier)
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))
    }

    pub(super) async fn resolve_client_name(
        &self,
        identifier: &str,
    ) -> ConfigResult<String> {
        let platform = PathService::get_current_platform();
        if let Some(template) = self.template_source.get_template(identifier, platform).await? {
            if let Some(display_name) = template.display_name {
                return Ok(display_name);
            }
        }
        Ok(identifier.to_string())
    }

    pub async fn set_client_managed(
        &self,
        identifier: &str,
        managed: bool,
    ) -> ConfigResult<bool> {
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await?;

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?, managed = ?, updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(name)
        .bind(if managed { 1 } else { 0 })
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(managed)
    }

    pub async fn is_client_managed(
        &self,
        identifier: &str,
    ) -> ConfigResult<bool> {
        let row = sqlx::query_scalar::<_, i64>("SELECT managed FROM client WHERE identifier = ?")
            .bind(identifier)
            .fetch_optional(&*self.db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        match row {
            Some(value) => Ok(value != 0),
            None => {
                let state = self.ensure_state_row(identifier).await?;
                Ok(state.managed())
            }
        }
    }

    pub async fn get_backup_policy(
        &self,
        identifier: &str,
    ) -> ConfigResult<BackupPolicySetting> {
        let state = self.ensure_state_row(identifier).await?;
        Ok(state.to_setting())
    }

    pub async fn set_backup_policy(
        &self,
        identifier: &str,
        policy: BackupPolicySetting,
    ) -> ConfigResult<BackupPolicySetting> {
        let (policy_label, limit) = policy.as_db_pair();
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await?;

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?,
                backup_policy = ?,
                backup_limit = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(name)
        .bind(policy_label)
        .bind(limit.map(|v| v as i64))
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(policy)
    }
}
