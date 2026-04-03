use super::core::{ClientConfigService, ClientStateRow};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::BackupPolicySetting;
use crate::clients::source::ClientConfigSource;
use crate::system::paths::PathService;
use std::collections::HashMap;

impl ClientConfigService {
    pub(super) async fn fetch_client_states(&self) -> ConfigResult<HashMap<String, ClientStateRow>> {
        let rows = sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, managed, config_mode, transport, client_version, backup_policy, backup_limit, capability_source, selected_profile_ids, custom_profile_id, approval_status, template_id, template_version, approval_metadata FROM client",
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

        let policy = self.get_onboarding_policy().await?;

        match policy {
            crate::clients::models::OnboardingPolicy::Manual => {
                return Err(ConfigError::DataAccessError(format!(
                    "Client {} not found and onboarding policy is manual",
                    identifier
                )));
            }
            crate::clients::models::OnboardingPolicy::AutoManage => {
                let generated_id = crate::generate_id!("clnt");
                let insert_result = sqlx::query(
                    r#"
                    INSERT INTO client (id, name, identifier, managed, backup_policy, backup_limit, approval_status)
                    VALUES (?, ?, ?, 1, 'keep_n', 30, 'approved')
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
            }
            crate::clients::models::OnboardingPolicy::RequireApproval => {
                let generated_id = crate::generate_id!("clnt");
                let insert_result = sqlx::query(
                    r#"
                    INSERT INTO client (id, name, identifier, managed, backup_policy, backup_limit, approval_status)
                    VALUES (?, ?, ?, 0, 'keep_n', 30, 'pending')
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
            }
        }

        self.fetch_state(identifier).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Failed to create management state for client {}", identifier))
        })
    }

    pub async fn fetch_state(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<ClientStateRow>> {
        sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, managed, config_mode, transport, client_version, backup_policy, backup_limit, capability_source, selected_profile_ids, custom_profile_id, approval_status, template_id, template_version, approval_metadata FROM client WHERE identifier = ?",
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

    pub async fn get_onboarding_policy(&self) -> ConfigResult<crate::clients::models::OnboardingPolicy> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM system_settings WHERE key = 'onboarding_policy'",
        )
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        match result {
            Some((value,)) => value
                .parse()
                .map_err(|_| ConfigError::DataAccessError("Invalid onboarding_policy value".to_string())),
            None => Ok(crate::clients::models::OnboardingPolicy::default()),
        }
    }

    pub async fn set_onboarding_policy(
        &self,
        policy: crate::clients::models::OnboardingPolicy,
    ) -> ConfigResult<()> {
        sqlx::query(
            r#"
            INSERT INTO system_settings (key, value, updated_at)
            VALUES ('onboarding_policy', ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind(policy.as_str())
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(())
    }

    pub async fn approve_client(&self, identifier: &str) -> ConfigResult<(String, bool)> {
        let row = self.fetch_state(identifier).await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} not found", identifier)))?;

        if row.approval_status() == "approved" {
            return Ok((row.approval_status().to_string(), row.managed()));
        }

        sqlx::query(
            r#"
            UPDATE client
            SET managed = 1, approval_status = 'approved', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(("approved".to_string(), true))
    }

    pub async fn reject_client(&self, identifier: &str) -> ConfigResult<(String, bool)> {
        let row = self.fetch_state(identifier).await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} not found", identifier)))?;

        if row.approval_status() == "rejected" {
            return Ok((row.approval_status().to_string(), row.managed()));
        }

        sqlx::query(
            r#"
            UPDATE client
            SET approval_status = 'rejected', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        let updated_row = self.fetch_state(identifier).await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Failed to fetch client {} after rejection", identifier)))?;

        Ok((updated_row.approval_status().to_string(), updated_row.managed()))
    }

    pub async fn suspend_client(&self, identifier: &str) -> ConfigResult<(String, bool)> {
        let row = self.fetch_state(identifier).await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} not found", identifier)))?;

        if row.approval_status() == "suspended" {
            return Ok((row.approval_status().to_string(), row.managed()));
        }

        sqlx::query(
            r#"
            UPDATE client
            SET managed = 0, approval_status = 'suspended', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(("suspended".to_string(), false))
    }

    #[allow(dead_code)]
    pub(super) async fn ensure_pending_unknown_row(
        &self,
        identifier: &str,
        name: &str,
    ) -> ConfigResult<ClientStateRow> {
        if let Some(existing) = self.fetch_state(identifier).await? {
            return Ok(existing);
        }

        let generated_id = crate::generate_id!("clnt");
        let insert_result = sqlx::query(
            r#"
            INSERT INTO client (id, name, identifier, managed, backup_policy, backup_limit, approval_status)
            VALUES (?, ?, ?, 0, 'keep_n', 30, 'pending')
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
                        "Concurrent pending unknown client insert detected; reusing existing row"
                    );
                } else {
                    return Err(ConfigError::DataAccessError(err.to_string()));
                }
            } else {
                return Err(ConfigError::DataAccessError(err.to_string()));
            }
        }

        self.fetch_state(identifier).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Failed to create pending unknown client {}", identifier))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::models::OnboardingPolicy;
    use crate::clients::source::{FileTemplateSource, TemplateRoot};
    use crate::common::constants::database::tables;
    use crate::config::{
        client::init::{initialize_client_table, initialize_system_settings_table},
        profile::init::initialize_profile_tables,
        server::init::initialize_server_tables,
    };
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_service() -> (TempDir, ClientConfigService) {
        let temp_dir = TempDir::new().expect("temp dir");
        let pool = Arc::new(
            SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .expect("sqlite pool"),
        );

        initialize_server_tables(pool.as_ref())
            .await
            .expect("init server tables");
        initialize_profile_tables(pool.as_ref())
            .await
            .expect("init profile tables");
        initialize_client_table(pool.as_ref())
            .await
            .expect("init client table");
        initialize_system_settings_table(pool.as_ref())
            .await
            .expect("init system settings table");

        let template_root = TemplateRoot::new(temp_dir.path().join("client-templates"));
        let source = Arc::new(
            FileTemplateSource::bootstrap(template_root)
                .await
                .expect("template source"),
        );
        let service = ClientConfigService::with_source(pool, source)
            .await
            .expect("client config service");

        (temp_dir, service)
    }

    async fn set_onboarding_policy(
        service: &ClientConfigService,
        policy: OnboardingPolicy,
    ) -> ConfigResult<()> {
        sqlx::query(&format!(
            "UPDATE {} SET value = ? WHERE key = 'onboarding_policy'",
            tables::SYSTEM_SETTINGS
        ))
        .bind(policy.as_str())
        .execute(&*service.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        Ok(())
    }

    #[tokio::test]
    async fn auto_manage_policy_creates_approved_managed_client() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::AutoManage)
            .await
            .expect("set policy");

        let state = service
            .ensure_state_row_with_name("test.client", "Test Client")
            .await
            .expect("ensure state");

        assert_eq!(state.identifier, "test.client");
        assert_eq!(state.name, "Test Client");
        assert_eq!(state.managed, 1);
        assert_eq!(state.approval_status.as_deref(), Some("approved"));
    }

    #[tokio::test]
    async fn require_approval_policy_creates_pending_unmanaged_client() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::RequireApproval)
            .await
            .expect("set policy");

        let state = service
            .ensure_state_row_with_name("test.client", "Test Client")
            .await
            .expect("ensure state");

        assert_eq!(state.identifier, "test.client");
        assert_eq!(state.name, "Test Client");
        assert_eq!(state.managed, 0);
        assert_eq!(state.approval_status.as_deref(), Some("pending"));
    }

    #[tokio::test]
    async fn manual_policy_rejects_client_creation() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::Manual)
            .await
            .expect("set policy");

        let result = service
            .ensure_state_row_with_name("test.client", "Test Client")
            .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not found") && err_msg.contains("manual"),
            "Expected error about manual policy, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn default_policy_is_auto_manage() {
        let (_temp_dir, service) = create_test_service().await;

        let policy = service
            .get_onboarding_policy()
            .await
            .expect("get policy");

        assert_eq!(policy, OnboardingPolicy::AutoManage);
    }

    #[tokio::test]
    async fn existing_client_is_returned_regardless_of_policy() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::AutoManage)
            .await
            .expect("set policy");

        let first = service
            .ensure_state_row_with_name("test.client", "Test Client")
            .await
            .expect("first ensure");

        set_onboarding_policy(&service, OnboardingPolicy::Manual)
            .await
            .expect("change policy to manual");

        let second = service
            .ensure_state_row_with_name("test.client", "Test Client")
            .await
            .expect("second ensure should succeed");

        assert_eq!(first.id, second.id);
        assert_eq!(second.managed, 1);
        assert_eq!(second.approval_status.as_deref(), Some("approved"));
    }

    #[tokio::test]
    async fn client_name_update_works_with_existing_client() {
        let (_temp_dir, service) = create_test_service().await;

        let first = service
            .ensure_state_row_with_name("test.client", "Original Name")
            .await
            .expect("first ensure");

        let updated = service
            .ensure_state_row_with_name("test.client", "Updated Name")
            .await
            .expect("second ensure with new name");

        assert_eq!(first.id, updated.id);
        assert_eq!(updated.name, "Updated Name");
    }
}
