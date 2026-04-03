use super::core::{ClientConfigService, ClientDescriptor};
use crate::clients::detector::DetectedClient;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::ClientTemplate;
use crate::clients::source::ClientConfigSource;
use crate::system::paths::get_path_service;
use std::collections::HashMap;
use std::path::PathBuf;

impl ClientConfigService {
    /// List known clients enriched with detection and filesystem information
    pub async fn list_clients(
        &self,
        _force_detect: bool,
    ) -> ConfigResult<Vec<ClientDescriptor>> {
        let templates = self.template_source.list_client().await?;
        let detected = self.detector.detect_installed_client().await?;
        let states = self.fetch_client_states().await?;

        let mut detected_map: HashMap<String, DetectedClient> = HashMap::new();
        for entry in detected {
            detected_map.insert(entry.identifier.clone(), entry);
        }

        let mut results = Vec::with_capacity(templates.len());
        for mut template in templates {
            let identifier = template.identifier.clone();
            let state_entry = states.get(&identifier);

            if let Some(state) = state_entry {
                tracing::trace!(
                    client_state_id = %state.id,
                    client_identifier = %identifier,
                    "Loaded client state metadata"
                );
            }

            if template.display_name.is_none() {
                if let Some(state) = state_entry {
                    if !state.name.is_empty() {
                        template.display_name = Some(state.name.clone());
                    }
                }
            }

            let resolved_path = self.resolved_config_path(&identifier).await?;
            let config_exists = if let Some(path_str) = &resolved_path {
                let path = PathBuf::from(path_str);
                get_path_service()
                    .validate_path_exists(&path)
                    .await
                    .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?
            } else {
                false
            };

            let detection = detected_map.remove(&identifier);
            let detected_at = detection.as_ref().map(|entry| entry.detected_at);
            let managed = state_entry.map(|state| state.managed()).unwrap_or(true);
            results.push(ClientDescriptor {
                detection,
                template,
                config_path: resolved_path,
                config_exists,
                detected_at,
                managed,
            });
        }

        // Process remaining detected clients without templates (pending unknowns)
        for (identifier, detected) in detected_map {
            // Ensure pending row exists
            let display_name = detected
                .display_name
                .as_deref()
                .unwrap_or(&identifier);
            
            match self.ensure_pending_unknown_row(&identifier, display_name).await {
                Ok(state_row) => {
                    tracing::info!(
                        identifier = %identifier,
                        approval_status = %state_row.approval_status(),
                        "Created or found pending unknown client"
                    );
                    
                    // Create synthetic template for this unknown client
                    let synthetic_template = ClientTemplate {
                        identifier: identifier.clone(),
                        display_name: detected.display_name.clone(),
                        version: None,
                        format: crate::clients::models::TemplateFormat::Json,
                        protocol_revision: None,
                        storage: crate::clients::models::StorageConfig {
                            kind: crate::clients::models::StorageKind::File,
                            path_strategy: None,
                            adapter: None,
                        },
                        detection: std::collections::HashMap::new(),
                        config_mapping: crate::clients::models::ConfigMapping {
                            container_keys: vec![],
                            container_type: crate::clients::models::ContainerType::ObjectMap,
                            merge_strategy: crate::clients::models::MergeStrategy::Replace,
                            keep_original_config: false,
                            managed_endpoint: None,
                            managed_source: None,
                            format_rules: std::collections::HashMap::new(),
                        },
                        metadata: std::collections::HashMap::new(),
                    };
                    
                    results.push(ClientDescriptor {
                        detection: Some(detected.clone()),
                        template: synthetic_template,
                        config_path: detected.config_path.clone(),
                        config_exists: false,
                        detected_at: Some(detected.detected_at),
                        managed: state_row.managed(),
                    });
                }
                Err(err) => {
                    tracing::warn!(
                        identifier = %identifier,
                        error = %err,
                        "Failed to ensure pending unknown row for detected client"
                    );
                }
            }
        }

        Ok(results)
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
    async fn pending_unknown_row_creation_workflow() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::RequireApproval)
            .await
            .expect("set policy");

        let state = service
            .ensure_pending_unknown_row("test.unknown", "Test Unknown")
            .await
            .expect("ensure pending row");

        assert_eq!(state.identifier, "test.unknown");
        assert_eq!(state.name, "Test Unknown");
        assert_eq!(state.managed, 0);
        assert_eq!(state.approval_status.as_deref(), Some("pending"));
        assert!(state.template_id.is_none());
        assert!(state.is_pending_unknown());

        let fetched_state = service
            .fetch_state("test.unknown")
            .await
            .expect("fetch state")
            .expect("state should exist");

        assert_eq!(fetched_state.identifier, "test.unknown");
        assert_eq!(fetched_state.approval_status.as_deref(), Some("pending"));
    }

    #[tokio::test]
    async fn ensure_pending_unknown_row_is_idempotent() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::RequireApproval)
            .await
            .expect("set policy");

        let state1 = service
            .ensure_pending_unknown_row("test.idempotent", "Test Idempotent")
            .await
            .expect("first ensure");

        let state2 = service
            .ensure_pending_unknown_row("test.idempotent", "Different Name")
            .await
            .expect("second ensure");

        assert_eq!(state1.id, state2.id);
        assert_eq!(state1.identifier, state2.identifier);
        assert_eq!(state2.name, "Test Idempotent");
    }
}
