use super::core::{ClientConfigService, ClientStateRow};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    BackupPolicySetting, ClientConnectionMode, ClientGovernanceKind, FirstContactBehavior,
};
use std::collections::HashMap;

impl ClientConfigService {
    pub(super) async fn fetch_client_states(&self) -> ConfigResult<HashMap<String, ClientStateRow>> {
        let rows = sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, display_name, config_path, managed, config_mode, transport, client_version, backup_policy, backup_limit, capability_source, governance_kind, connection_mode, template_identifier, selected_profile_ids, custom_profile_id, unify_route_mode, unify_selected_server_ids, unify_selected_tool_surfaces, unify_selected_prompt_surfaces, unify_selected_resource_surfaces, unify_selected_template_surfaces, approval_status, template_id, template_version, approval_metadata, config_format, protocol_revision, container_type, container_keys, storage_kind, storage_adapter, storage_path_strategy, merge_strategy, keep_original_config, managed_source, format_rules, config_file_parse FROM client",
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
        let (managed, approval_status) = match first_contact_behavior {
            FirstContactBehavior::Deny => (0_i64, "rejected"),
            FirstContactBehavior::Review => (0_i64, "pending"),
            FirstContactBehavior::Allow => (1_i64, "approved"),
        };

        self.create_state_row(
            identifier,
            name,
            ClientGovernanceKind::Passive,
            managed,
            approval_status,
            None,
        )
        .await
    }

    pub async fn fetch_state(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<ClientStateRow>> {
        sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, display_name, config_path, managed, config_mode, transport, client_version, backup_policy, backup_limit, capability_source, governance_kind, connection_mode, template_identifier, selected_profile_ids, custom_profile_id, unify_route_mode, unify_selected_server_ids, unify_selected_tool_surfaces, unify_selected_prompt_surfaces, unify_selected_resource_surfaces, unify_selected_template_surfaces, approval_status, template_id, template_version, approval_metadata, config_format, protocol_revision, container_type, container_keys, storage_kind, storage_adapter, storage_path_strategy, merge_strategy, keep_original_config, managed_source, format_rules, config_file_parse FROM client WHERE identifier = ?",
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

        crate::core::profile::visibility::invalidate_visibility_cache(identifier);

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

    pub async fn set_client_managed(
        &self,
        identifier: &str,
        managed: bool,
    ) -> ConfigResult<bool> {
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_active_state_row_with_name(identifier, &name, Some(managed), Some("approved"))
            .await?;

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?, managed = ?, governance_kind = 'active', updated_at = CURRENT_TIMESTAMP
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
        self.ensure_active_state_row_with_name(identifier, &name, None, Some("approved"))
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
    ) -> ConfigResult<(String, bool)> {
        let name = self.resolve_client_name(identifier).await?;
        let row = self
            .ensure_active_state_row_with_name(identifier, &name, Some(true), Some("approved"))
            .await?;

        if row.approval_status() == "approved" {
            return Ok((row.approval_status().to_string(), row.managed()));
        }

        sqlx::query(
            r#"
            UPDATE client
            SET managed = 1, approval_status = 'approved', governance_kind = 'active', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(("approved".to_string(), true))
    }

    pub async fn reject_client(
        &self,
        identifier: &str,
    ) -> ConfigResult<(String, bool)> {
        let name = self.resolve_client_name(identifier).await?;
        let row = self
            .ensure_active_state_row_with_name(identifier, &name, Some(false), Some("rejected"))
            .await?;

        if row.approval_status() == "rejected" {
            return Ok((row.approval_status().to_string(), row.managed()));
        }

        sqlx::query(
            r#"
            UPDATE client
            SET approval_status = 'rejected', governance_kind = 'active', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        let updated_row = self.fetch_state(identifier).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Failed to fetch client {} after rejection", identifier))
        })?;

        Ok((updated_row.approval_status().to_string(), updated_row.managed()))
    }

    pub async fn suspend_client(
        &self,
        identifier: &str,
    ) -> ConfigResult<(String, bool)> {
        let name = self.resolve_client_name(identifier).await?;
        let row = self
            .ensure_active_state_row_with_name(identifier, &name, Some(false), Some("suspended"))
            .await?;

        if row.approval_status() == "suspended" {
            return Ok((row.approval_status().to_string(), row.managed()));
        }

        sqlx::query(
            r#"
            UPDATE client
            SET managed = 0, approval_status = 'suspended', governance_kind = 'active', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(("suspended".to_string(), false))
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
        let (managed, approval_status) = match first_contact_behavior {
            FirstContactBehavior::Deny => (0_i64, "rejected"),
            FirstContactBehavior::Review => (0_i64, "pending"),
            FirstContactBehavior::Allow => (1_i64, "approved"),
        };

        self.create_state_row(
            identifier,
            name,
            ClientGovernanceKind::Passive,
            managed,
            approval_status,
            config_path,
        )
        .await
    }

    pub(crate) async fn ensure_active_state_row_with_name(
        &self,
        identifier: &str,
        name: &str,
        managed: Option<bool>,
        approval_status: Option<&str>,
    ) -> ConfigResult<ClientStateRow> {
        if let Some(existing) = self.fetch_state(identifier).await? {
            self.promote_existing_state(identifier, name, managed, approval_status, existing)
                .await
        } else {
            self.create_state_row(
                identifier,
                name,
                ClientGovernanceKind::Active,
                i64::from(managed.unwrap_or(true)),
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
        managed: Option<bool>,
        approval_status: Option<&str>,
        existing: ClientStateRow,
    ) -> ConfigResult<ClientStateRow> {
        let managed = managed.unwrap_or(existing.managed()) as i64;
        let approval_status = approval_status.unwrap_or(existing.approval_status());

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?,
                governance_kind = 'active',
                managed = ?,
                approval_status = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(name)
        .bind(managed)
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
        managed: i64,
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
        let template_identifier = template.as_ref().map(|entry| entry.identifier.clone());
        let generated_id = crate::generate_id!("clnt");

        // Extract template configuration fields for persistence
        let (
            config_format,
            protocol_revision,
            container_type,
            container_keys,
            storage_kind,
            storage_adapter,
            storage_path_strategy,
            merge_strategy,
            keep_original_config,
            managed_source,
            format_rules,
            config_file_parse,
        ) = if let Some(ref tmpl) = template {
            let config_format = Some(tmpl.format.as_str().to_string());
            let protocol_revision = tmpl.protocol_revision.clone();
            let container_type = Some(
                match tmpl.config_mapping.container_type {
                    crate::clients::models::ContainerType::ObjectMap => "object",
                    crate::clients::models::ContainerType::Array => "array",
                }
                .to_string(),
            );
            let container_keys = serde_json::to_string(&tmpl.config_mapping.container_keys).ok();
            let storage_kind = Some(
                match tmpl.storage.kind {
                    crate::clients::models::StorageKind::File => "file",
                    crate::clients::models::StorageKind::Kv => "kv",
                    crate::clients::models::StorageKind::Custom => "custom",
                }
                .to_string(),
            );
            let storage_adapter = tmpl.storage.adapter.clone();
            let storage_path_strategy = tmpl.storage.path_strategy.clone();
            let merge_strategy = Some(
                match tmpl.config_mapping.merge_strategy {
                    crate::clients::models::MergeStrategy::Replace => "replace",
                    crate::clients::models::MergeStrategy::DeepMerge => "deep_merge",
                }
                .to_string(),
            );
            let keep_original_config = Some(if tmpl.config_mapping.keep_original_config {
                1_i64
            } else {
                0_i64
            });
            let managed_source = tmpl.config_mapping.managed_source.clone();
            let format_rules = if tmpl.config_mapping.format_rules.is_empty() {
                None
            } else {
                serde_json::to_string(&tmpl.config_mapping.format_rules).ok()
            };

            (
                config_format,
                protocol_revision,
                container_type,
                container_keys,
                storage_kind,
                storage_adapter,
                storage_path_strategy,
                merge_strategy,
                keep_original_config,
                managed_source,
                format_rules,
                None::<String>,
            )
        } else {
            (None, None, None, None, None, None, None, None, None, None, None, None)
        };

        let insert_result = sqlx::query(
            r#"
            INSERT INTO client (
                id, name, display_name, identifier, config_path, managed, backup_policy, backup_limit,
                approval_status, governance_kind, connection_mode, template_identifier,
                config_format, protocol_revision, container_type, container_keys,
                storage_kind, storage_adapter, storage_path_strategy,
                merge_strategy, keep_original_config, managed_source, format_rules, config_file_parse
            )
            VALUES (?, ?, ?, ?, ?, ?, 'keep_n', 5, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&generated_id)
        .bind(name)
        .bind(display_name)
        .bind(identifier)
        .bind(config_path)
        .bind(managed)
        .bind(approval_status)
        .bind(governance_kind.as_str())
        .bind(connection_mode)
        .bind(template_identifier)
        .bind(config_format)
        .bind(protocol_revision)
        .bind(container_type)
        .bind(container_keys)
        .bind(storage_kind)
        .bind(storage_adapter)
        .bind(storage_path_strategy)
        .bind(merge_strategy)
        .bind(keep_original_config)
        .bind(managed_source)
        .bind(format_rules)
        .bind(config_file_parse)
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
        assert_eq!(state.governance_kind().as_str(), "passive");
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
        assert_eq!(state.governance_kind().as_str(), "passive");
    }

    #[tokio::test]
    async fn manual_policy_creates_rejected_client() {
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
        assert_eq!(state.managed, 0);
        assert_eq!(state.approval_status.as_deref(), Some("rejected"));
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
