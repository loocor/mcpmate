use super::core::{ClientConfigService, ClientStateRow, PersistedTemplateConfig};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    AttachmentState, BackupPolicySetting, ClientConnectionMode, ClientGovernanceKind, FirstContactBehavior,
};
use std::collections::HashMap;

impl ClientConfigService {
    pub(super) async fn fetch_client_states(&self) -> ConfigResult<HashMap<String, ClientStateRow>> {
        let rows = sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, display_name, config_path, config_mode, transport, client_version, backup_policy, backup_limit, capability_source, governance_kind, connection_mode, template_identifier, selected_profile_ids, custom_profile_id, unify_direct_exposure_intent, approval_status, attachment_state, template_id, template_version, approval_metadata, config_format, protocol_revision, container_type, container_keys, storage_kind, storage_adapter, storage_path_strategy, merge_strategy, keep_original_config, managed_source, transports, config_file_parse FROM client",
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
            return self.refresh_existing_state_name(identifier, name, existing).await;
        }

        let first_contact_behavior = self.get_first_contact_behavior().await?;
        let approval_status = match first_contact_behavior {
            FirstContactBehavior::Deny => "suspended",
            FirstContactBehavior::Review => "pending",
            FirstContactBehavior::Allow => "approved",
        };

        self.create_state_row(identifier, name, ClientGovernanceKind::Passive, approval_status, None)
            .await
    }

    pub async fn fetch_state(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<ClientStateRow>> {
        sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, display_name, config_path, config_mode, transport, client_version, backup_policy, backup_limit, capability_source, governance_kind, connection_mode, template_identifier, selected_profile_ids, custom_profile_id, unify_direct_exposure_intent, approval_status, attachment_state, template_id, template_version, approval_metadata, config_format, protocol_revision, container_type, container_keys, storage_kind, storage_adapter, storage_path_strategy, merge_strategy, keep_original_config, managed_source, transports, config_file_parse FROM client WHERE identifier = ?",
        )
        .bind(identifier)
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))
    }

    pub async fn delete_client_record(
        &self,
        identifier: &str,
    ) -> ConfigResult<bool> {
        let Some(state) = self.fetch_state(identifier).await? else {
            return Ok(false);
        };

        if let Err(err) = self.delete_all_client_backups(identifier).await {
            tracing::warn!(
                client = %identifier,
                error = %err,
                "Failed to delete all client backups before record removal; continuing"
            );
        }

        if let Some(custom_profile_id) = state.custom_profile_id.as_deref() {
            crate::config::profile::delete_profile(self.db_pool.as_ref(), custom_profile_id)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        }

        sqlx::query("DELETE FROM client_template_runtime WHERE identifier = ?")
            .bind(identifier)
            .execute(&*self.db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        let result = sqlx::query("DELETE FROM client WHERE identifier = ?")
            .bind(identifier)
            .execute(&*self.db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }

        Ok(true)
    }

    pub(crate) async fn resolve_client_name(
        &self,
        identifier: &str,
    ) -> ConfigResult<String> {
        if let Some(state) = self.fetch_state(identifier).await? {
            if !state.display_name().trim().is_empty() {
                return Ok(state.display_name().to_string());
            }
            if !state.name.trim().is_empty() {
                return Ok(state.name);
            }
        }
        Ok(identifier.to_string())
    }

    pub async fn is_client_approved(
        &self,
        identifier: &str,
    ) -> ConfigResult<bool> {
        let status = sqlx::query_scalar::<_, String>("SELECT approval_status FROM client WHERE identifier = ?")
            .bind(identifier)
            .fetch_optional(&*self.db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        match status {
            Some(value) => Ok(value == "approved"),
            None => {
                let state = self.ensure_state_row(identifier).await?;
                Ok(state.is_approved())
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
        self.ensure_active_state_row_with_name(identifier, &name, Some("approved"))
            .await?;

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?,
                backup_policy = ?,
                backup_limit = ?,
                governance_kind = 'active',
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
        Ok(match self.get_first_contact_behavior().await? {
            FirstContactBehavior::Allow => crate::clients::models::OnboardingPolicy::AutoManage,
            FirstContactBehavior::Review => crate::clients::models::OnboardingPolicy::RequireApproval,
            FirstContactBehavior::Deny => crate::clients::models::OnboardingPolicy::Manual,
        })
    }

    pub async fn get_first_contact_behavior(&self) -> ConfigResult<FirstContactBehavior> {
        let result: Option<(String,)> =
            sqlx::query_as("SELECT value FROM system_settings WHERE key = 'first_contact_behavior'")
                .fetch_optional(&*self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        match result {
            Some((value,)) => value
                .parse()
                .map_err(|_| ConfigError::DataAccessError("Invalid first_contact_behavior value".to_string())),
            None => Ok(FirstContactBehavior::default()),
        }
    }

    pub async fn set_first_contact_behavior(
        &self,
        behavior: FirstContactBehavior,
    ) -> ConfigResult<()> {
        sqlx::query(
            r#"
            INSERT INTO system_settings (key, value, updated_at)
            VALUES ('first_contact_behavior', ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind(behavior.as_str())
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(())
    }

    pub async fn set_onboarding_policy(
        &self,
        policy: crate::clients::models::OnboardingPolicy,
    ) -> ConfigResult<()> {
        let behavior = match policy {
            crate::clients::models::OnboardingPolicy::AutoManage => FirstContactBehavior::Allow,
            crate::clients::models::OnboardingPolicy::RequireApproval => FirstContactBehavior::Review,
            crate::clients::models::OnboardingPolicy::Manual => FirstContactBehavior::Deny,
        };

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

        self.set_first_contact_behavior(behavior).await
    }

    pub async fn approve_client(
        &self,
        identifier: &str,
    ) -> ConfigResult<String> {
        let name = self.resolve_client_name(identifier).await?;
        let row = self
            .ensure_active_state_row_with_name(identifier, &name, Some("approved"))
            .await?;

        if row.approval_status() == "approved" {
            return Ok(row.approval_status().to_string());
        }

        sqlx::query(
            r#"
            UPDATE client
            SET approval_status = 'approved', governance_kind = 'active', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok("approved".to_string())
    }

    pub async fn suspend_client(
        &self,
        identifier: &str,
    ) -> ConfigResult<String> {
        let name = self.resolve_client_name(identifier).await?;
        let row = self
            .ensure_active_state_row_with_name(identifier, &name, Some("suspended"))
            .await?;

        if row.approval_status() == "suspended" {
            return Ok(row.approval_status().to_string());
        }

        sqlx::query(
            r#"
            UPDATE client
            SET approval_status = 'suspended', governance_kind = 'active', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok("suspended".to_string())
    }

    pub async fn mark_client_attached(
        &self,
        identifier: &str,
    ) -> ConfigResult<()> {
        sqlx::query(
            r#"
            UPDATE client
            SET attachment_state = CASE
                    WHEN connection_mode = 'local_config_detected'
                        AND config_path IS NOT NULL
                        AND TRIM(config_path) <> ''
                    THEN 'attached'
                    ELSE 'not_applicable'
                END,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(())
    }

    pub async fn mark_client_detached(
        &self,
        identifier: &str,
    ) -> ConfigResult<()> {
        sqlx::query(
            r#"
            UPDATE client
            SET attachment_state = CASE
                    WHEN connection_mode = 'local_config_detected'
                        AND config_path IS NOT NULL
                        AND TRIM(config_path) <> ''
                    THEN 'detached'
                    ELSE 'not_applicable'
                END,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(())
    }

    /// Used by detection/list and by the MCP proxy when registering an unknown client under `review` policy.
    pub async fn ensure_passive_observed_row(
        &self,
        identifier: &str,
        name: &str,
        config_path: Option<&str>,
    ) -> ConfigResult<ClientStateRow> {
        if let Some(existing) = self.fetch_state(identifier).await? {
            return self.refresh_existing_state_name(identifier, name, existing).await;
        }
        let first_contact_behavior = self.get_first_contact_behavior().await?;
        let approval_status = match first_contact_behavior {
            FirstContactBehavior::Deny => "suspended",
            FirstContactBehavior::Review => "pending",
            FirstContactBehavior::Allow => "approved",
        };

        self.create_state_row(
            identifier,
            name,
            ClientGovernanceKind::Passive,
            approval_status,
            config_path,
        )
        .await
    }

    pub(crate) async fn ensure_active_state_row_with_name(
        &self,
        identifier: &str,
        name: &str,
        approval_status: Option<&str>,
    ) -> ConfigResult<ClientStateRow> {
        if let Some(existing) = self.fetch_state(identifier).await? {
            self.promote_existing_state(identifier, name, approval_status, existing)
                .await
        } else {
            self.create_state_row(
                identifier,
                name,
                ClientGovernanceKind::Active,
                approval_status.unwrap_or("approved"),
                None,
            )
            .await
        }
    }

    async fn refresh_existing_state_name(
        &self,
        identifier: &str,
        name: &str,
        existing: ClientStateRow,
    ) -> ConfigResult<ClientStateRow> {
        if existing.name == name {
            return Ok(existing);
        }

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

        self.fetch_state(identifier).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Failed to update management state for client {}", identifier))
        })
    }

    async fn promote_existing_state(
        &self,
        identifier: &str,
        name: &str,
        approval_status: Option<&str>,
        existing: ClientStateRow,
    ) -> ConfigResult<ClientStateRow> {
        let approval_status = approval_status.unwrap_or(existing.approval_status());

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?,
                governance_kind = 'active',
                approval_status = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(name)
        .bind(approval_status)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        self.fetch_state(identifier).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Failed to promote management state for client {}", identifier))
        })
    }

    async fn create_state_row(
        &self,
        identifier: &str,
        name: &str,
        governance_kind: ClientGovernanceKind,
        approval_status: &str,
        observed_config_path: Option<&str>,
    ) -> ConfigResult<ClientStateRow> {
        let platform = crate::system::paths::PathService::get_current_platform();
        let template = self.template_source.get_template(identifier, platform).await?;
        let display_name = template
            .as_ref()
            .and_then(|entry| entry.display_name.clone())
            .unwrap_or_else(|| name.to_string());
        let config_path = observed_config_path.map(str::to_string).or_else(|| {
            template
                .as_ref()
                .and_then(Self::extract_runtime_config_path_from_template)
        });
        let connection_mode = if config_path.is_some() {
            ClientConnectionMode::LocalConfigDetected.as_str()
        } else {
            ClientConnectionMode::Manual.as_str()
        };
        let attachment_state = if config_path.is_some() {
            AttachmentState::Detached.as_str()
        } else {
            AttachmentState::NotApplicable.as_str()
        };
        let template_identifier = template.as_ref().map(|entry| entry.identifier.clone());
        let generated_id = crate::generate_id!("clnt");
        let persisted_config = template
            .as_ref()
            .map(PersistedTemplateConfig::from_template)
            .unwrap_or_default();

        let insert_result = sqlx::query(
            r#"
            INSERT INTO client (
                id, name, display_name, identifier, config_path, backup_policy, backup_limit,
                approval_status, governance_kind, connection_mode, template_identifier,
                config_format, protocol_revision, container_type, container_keys,
                storage_kind, storage_adapter, storage_path_strategy,
                merge_strategy, keep_original_config, managed_source, transports, config_file_parse, attachment_state
            )
            VALUES (?, ?, ?, ?, ?, 'keep_n', 5, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&generated_id)
        .bind(name)
        .bind(display_name)
        .bind(identifier)
        .bind(config_path)
        .bind(approval_status)
        .bind(governance_kind.as_str())
        .bind(connection_mode)
        .bind(template_identifier)
        .bind(persisted_config.config_format)
        .bind(persisted_config.protocol_revision)
        .bind(persisted_config.container_type)
        .bind(persisted_config.container_keys)
        .bind(persisted_config.storage_kind)
        .bind(persisted_config.storage_adapter)
        .bind(persisted_config.storage_path_strategy)
        .bind(persisted_config.merge_strategy)
        .bind(persisted_config.keep_original_config)
        .bind(persisted_config.managed_source)
        .bind(persisted_config.transports)
        .bind(persisted_config.config_file_parse)
        .bind(attachment_state)
        .execute(&*self.db_pool)
        .await;

        if let Err(err) = insert_result {
            if let sqlx::Error::Database(db_err) = &err {
                if db_err.code().map(|code| code == "2067").unwrap_or(false) {
                    tracing::warn!(client = %identifier, "Concurrent client state insert detected; reusing existing row");
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::models::OnboardingPolicy;
    use crate::clients::source::{ClientConfigSource, DbTemplateSource, FileTemplateSource, TemplateRoot};
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
        initialize_client_table(pool.as_ref()).await.expect("init client table");
        initialize_system_settings_table(pool.as_ref())
            .await
            .expect("init system settings table");

        let template_root = TemplateRoot::new(temp_dir.path().join("client-templates"));
        let source = Arc::new(
            FileTemplateSource::bootstrap(template_root)
                .await
                .expect("template source"),
        );
        ClientConfigService::seed_runtime_template_snapshots(pool.as_ref(), source.as_ref())
            .await
            .expect("seed runtime templates");
        ClientConfigService::seed_client_runtime_rows(pool.as_ref(), source.as_ref())
            .await
            .expect("seed runtime rows");
        let runtime_source: Arc<dyn ClientConfigSource> =
            Arc::new(DbTemplateSource::new(pool.clone()).expect("runtime source"));
        let service = ClientConfigService::with_source(pool, runtime_source)
            .await
            .expect("client config service");

        (temp_dir, service)
    }

    async fn set_onboarding_policy(
        service: &ClientConfigService,
        policy: OnboardingPolicy,
    ) -> ConfigResult<()> {
        service.set_onboarding_policy(policy).await
    }

    #[tokio::test]
    async fn auto_manage_policy_creates_approved_client() {
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
        assert_eq!(state.approval_status.as_deref(), Some("approved"));
        assert_eq!(state.governance_kind().as_str(), "passive");
    }

    #[tokio::test]
    async fn require_approval_policy_creates_pending_client() {
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
        assert_eq!(state.approval_status.as_deref(), Some("pending"));
        assert_eq!(state.governance_kind().as_str(), "passive");
    }

    #[tokio::test]
    async fn manual_policy_creates_suspended_client() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::Manual)
            .await
            .expect("set policy");

        let state = service
            .ensure_state_row_with_name("test.client", "Test Client")
            .await
            .expect("ensure state");

        assert_eq!(state.identifier, "test.client");
        assert_eq!(state.name, "Test Client");
        assert_eq!(state.approval_status.as_deref(), Some("suspended"));
        assert_eq!(state.governance_kind().as_str(), "passive");
    }

    #[tokio::test]
    async fn default_policy_is_require_approval() {
        let (_temp_dir, service) = create_test_service().await;

        let policy = service.get_onboarding_policy().await.expect("get policy");

        assert_eq!(policy, OnboardingPolicy::RequireApproval);
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

    #[tokio::test]
    async fn active_updates_promote_existing_passive_client() {
        let (_temp_dir, service) = create_test_service().await;

        let passive = service
            .ensure_state_row_with_name("test.client", "Test Client")
            .await
            .expect("create passive client");
        assert_eq!(passive.governance_kind().as_str(), "passive");

        service
            .set_client_settings("test.client", Some("hosted".to_string()), None, None)
            .await
            .expect("promote via settings update");

        let promoted = service
            .fetch_state("test.client")
            .await
            .expect("fetch state")
            .expect("state exists");
        assert_eq!(promoted.governance_kind().as_str(), "active");
        assert_eq!(promoted.config_mode.as_deref(), Some("hosted"));
    }

    #[tokio::test]
    async fn handshake_observation_fills_existing_passive_client_once() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::RequireApproval)
            .await
            .expect("set policy");
        service
            .ensure_passive_observed_row("test.observed", "Observed", None)
            .await
            .expect("create passive observed row");

        service
            .persist_handshake_observation(
                "test.observed",
                Some("Observed App"),
                Some("1.2.3"),
                Some("streamable_http"),
                Some("remote_http"),
                Some("Observed description"),
                Some("https://example.com"),
                Some("https://example.com/logo.png"),
            )
            .await
            .expect("persist first handshake observation");

        service
            .persist_handshake_observation(
                "test.observed",
                Some("Changed App"),
                Some("9.9.9"),
                Some("sse"),
                Some("remote_http"),
                Some("Changed description"),
                Some("https://changed.example.com"),
                Some("https://changed.example.com/logo.png"),
            )
            .await
            .expect("ignore later handshake observation");

        let state = service
            .fetch_state("test.observed")
            .await
            .expect("fetch state")
            .expect("state exists");
        assert_eq!(state.governance_kind().as_str(), "passive");
        assert_eq!(state.approval_status.as_deref(), Some("pending"));
        assert_eq!(state.display_name.as_deref(), Some("Observed App"));
        assert_eq!(state.client_version.as_deref(), Some("1.2.3"));
        assert_eq!(state.transport.as_deref(), Some("streamable_http"));
        assert_eq!(state.connection_mode.as_deref(), Some("remote_http"));

        let metadata = state.runtime_client_metadata();
        assert_eq!(metadata.description.as_deref(), Some("Observed description"));
        assert_eq!(metadata.homepage_url.as_deref(), Some("https://example.com"));
        assert_eq!(metadata.logo_url.as_deref(), Some("https://example.com/logo.png"));

        let transports = state.parsed_transports().expect("transports json");
        assert!(transports.contains_key("streamable_http"));
        assert!(!transports.contains_key("sse"));
    }

    #[tokio::test]
    async fn handshake_observation_does_not_override_template_client() {
        let (_temp_dir, service) = create_test_service().await;
        service
            .ensure_passive_observed_row("template.client", "Template Client", None)
            .await
            .expect("create template-like client");
        sqlx::query("UPDATE client SET template_identifier = ? WHERE identifier = ?")
            .bind("template.client")
            .bind("template.client")
            .execute(service.db_pool.as_ref())
            .await
            .expect("mark template identifier");
        let template_client = service
            .fetch_state("template.client")
            .await
            .expect("fetch template-like state")
            .expect("template-like state exists");
        let identifier = template_client.identifier.clone();
        let original_display_name = template_client.display_name.clone();
        let original_client_version = template_client.client_version.clone();
        let original_metadata = template_client.runtime_client_metadata();

        service
            .persist_handshake_observation(
                &identifier,
                Some("Changed Template"),
                Some("9.9.9"),
                Some("streamable_http"),
                Some("remote_http"),
                Some("Changed description"),
                Some("https://changed.example.com"),
                Some("https://changed.example.com/logo.png"),
            )
            .await
            .expect("template handshake observation should be ignored");

        let state = service
            .fetch_state(&identifier)
            .await
            .expect("fetch template state")
            .expect("template state exists");
        assert_eq!(state.display_name, original_display_name);
        assert_eq!(state.client_version, original_client_version);
        assert_eq!(state.runtime_client_metadata(), original_metadata);
    }

    #[tokio::test]
    async fn delete_client_record_cleans_runtime_and_allows_recreation() {
        let (_temp_dir, service) = create_test_service().await;

        service
            .set_client_settings("test.client", Some("hosted".to_string()), None, None)
            .await
            .expect("create active client state");

        sqlx::query(
            "INSERT OR REPLACE INTO client_template_runtime (identifier, payload_json, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)",
        )
        .bind("test.client")
        .bind("{}")
        .execute(service.db_pool.as_ref())
        .await
        .expect("insert runtime snapshot");

        let deleted = service
            .delete_client_record("test.client")
            .await
            .expect("delete client record");

        assert!(deleted);
        assert!(
            service
                .fetch_state("test.client")
                .await
                .expect("fetch state after delete")
                .is_none()
        );
        let runtime_payload: Option<String> =
            sqlx::query_scalar("SELECT payload_json FROM client_template_runtime WHERE identifier = ?")
                .bind("test.client")
                .fetch_optional(service.db_pool.as_ref())
                .await
                .expect("query runtime snapshot after delete");
        assert!(runtime_payload.is_none());

        let recreated = service
            .ensure_passive_observed_row("test.client", "Test Client", None)
            .await
            .expect("deleted client can be recreated passively");
        assert_eq!(recreated.identifier, "test.client");
    }
}
