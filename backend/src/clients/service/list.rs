use super::core::{ClientConfigService, ClientDescriptor};
use crate::clients::detector::DetectedClient;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::system::paths::get_path_service;
use std::collections::HashMap;
use std::path::PathBuf;

impl ClientConfigService {
    /// List known clients enriched with detection and filesystem information
    pub async fn list_clients(
        &self,
        _force_detect: bool,
    ) -> ConfigResult<Vec<ClientDescriptor>> {
        let detected = self.detector.detect_installed_client().await?;
        let states = self.fetch_client_states().await?;
        let templates = self.template_source.list_client().await?;
        let mut template_map = templates
            .into_iter()
            .map(|template| (template.identifier.clone(), template))
            .collect::<HashMap<_, _>>();

        let mut detected_map: HashMap<String, DetectedClient> = HashMap::new();
        for entry in detected {
            detected_map.insert(entry.identifier.clone(), entry);
        }

        let mut results = Vec::with_capacity(states.len() + detected_map.len());
        for (identifier, state) in &states {
            let detection = detected_map.remove(identifier);
            let template = template_map.remove(identifier);

            let resolved_path = self.resolved_config_path(identifier).await?;
            let config_exists = if let Some(path_str) = &resolved_path {
                let path = PathBuf::from(path_str);
                get_path_service()
                    .validate_path_exists(&path)
                    .await
                    .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?
            } else {
                false
            };

            let detected_at = detection.as_ref().map(|entry| entry.detected_at);
            results.push(ClientDescriptor {
                state: state.clone(),
                detection,
                template,
                config_path: resolved_path,
                config_exists,
                detected_at,
                managed: state.managed(),
            });
        }

        for (identifier, detected) in detected_map {
            let display_name = detected.display_name.as_deref().unwrap_or(&identifier);

            match self
                .ensure_passive_observed_row(&identifier, display_name, detected.config_path.as_deref())
                .await
            {
                Ok(state_row) => {
                    tracing::info!(
                        identifier = %identifier,
                        approval_status = %state_row.approval_status(),
                        governance_kind = %state_row.governance_kind().as_str(),
                        "Created passive observed client row"
                    );

                    results.push(ClientDescriptor {
                        state: state_row.clone(),
                        detection: Some(detected.clone()),
                        template: None,
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
    async fn passive_observed_row_creation_workflow() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::RequireApproval)
            .await
            .expect("set policy");

        let state = service
            .ensure_passive_observed_row("test.unknown", "Test Unknown", None)
            .await
            .expect("ensure passive observed row");

        assert_eq!(state.identifier, "test.unknown");
        assert_eq!(state.name, "Test Unknown");
        assert_eq!(state.managed, 0);
        assert_eq!(state.approval_status.as_deref(), Some("pending"));
        assert!(state.template_id.is_none());
        assert!(state.is_pending_unknown());
        assert_eq!(state.governance_kind().as_str(), "passive");

        let fetched_state = service
            .fetch_state("test.unknown")
            .await
            .expect("fetch state")
            .expect("state should exist");

        assert_eq!(fetched_state.identifier, "test.unknown");
        assert_eq!(fetched_state.approval_status.as_deref(), Some("pending"));
    }

    #[tokio::test]
    async fn ensure_passive_observed_row_is_idempotent() {
        let (_temp_dir, service) = create_test_service().await;

        set_onboarding_policy(&service, OnboardingPolicy::RequireApproval)
            .await
            .expect("set policy");

        let state1 = service
            .ensure_passive_observed_row("test.idempotent", "Test Idempotent", None)
            .await
            .expect("first ensure");

        let state2 = service
            .ensure_passive_observed_row("test.idempotent", "Different Name", None)
            .await
            .expect("second ensure");

        assert_eq!(state1.id, state2.id);
        assert_eq!(state1.identifier, state2.identifier);
        assert_eq!(state2.name, "Different Name");
    }

    #[tokio::test]
    async fn list_clients_includes_active_runtime_only_records() {
        let (_temp_dir, service) = create_test_service().await;

        service
            .set_client_settings("custom.runtime", Some("hosted".to_string()), None, None)
            .await
            .expect("create active runtime-only client");

        let descriptors = service.list_clients(false).await.expect("list clients");
        let descriptor = descriptors
            .into_iter()
            .find(|entry| entry.state.identifier() == "custom.runtime")
            .expect("runtime-only descriptor should exist");

        let template = descriptor.template.expect("runtime-only template should be persisted");
        assert_eq!(template.identifier, "custom.runtime");
        assert_eq!(
            template.config_mapping.managed_source.as_deref(),
            Some("runtime_active_client")
        );
        assert_eq!(descriptor.state.display_name(), "custom.runtime");
        assert_eq!(descriptor.state.record_kind().as_str(), "template_known");
        assert!(descriptor.managed);
    }
}
