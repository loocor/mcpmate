use super::ClientConfigService;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{CapabilitySource, ServerTemplateInput};
use crate::common::constants::defaults;
use crate::config::profile::basic::get_active_profile;
use crate::config::server::{args::get_server_args, env::get_server_env};
use serde_json::json;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ServerRow {
    pub id: String,
    pub name: String,
    pub command: Option<String>,
    pub url: Option<String>,
    pub server_type: String,
}

#[derive(Debug)]
pub enum ServerSelection {
    AllEnabled,
    Profile(String),
    Profiles(Vec<String>),
    Explicit(Vec<String>),
}

impl ClientConfigService {
    pub async fn prepare_servers(
        &self,
        options: &super::ClientRenderOptions,
    ) -> ConfigResult<Vec<ServerTemplateInput>> {
        let selection = self.resolve_server_selection(options).await?;
        let rows = self.fetch_servers(selection).await?;

        let mut servers = Vec::with_capacity(rows.len());
        for row in rows {
            servers.push(self.map_server_row(row).await?);
        }

        Ok(servers)
    }

    async fn resolve_server_selection(
        &self,
        options: &super::ClientRenderOptions,
    ) -> ConfigResult<ServerSelection> {
        if matches!(options.mode, crate::clients::models::ConfigMode::Native) {
            if let Some(ids) = &options.server_ids {
                if !ids.is_empty() {
                    return Ok(ServerSelection::Explicit(ids.clone()));
                }
            }
        }

        if let Some(profile_id) = &options.profile_id {
            return Ok(ServerSelection::Profile(profile_id.clone()));
        }

        if let Some(state) = self.fetch_state(&options.client_id).await? {
            let capability_config = state.capability_config()?;
            match capability_config.capability_source {
                CapabilitySource::Activated => {}
                CapabilitySource::Profiles => {
                    let profile_ids = capability_config.selected_profile_ids;
                    if profile_ids.len() == 1 {
                        return Ok(ServerSelection::Profile(profile_ids[0].clone()));
                    }
                    if !profile_ids.is_empty() {
                        return Ok(ServerSelection::Profiles(profile_ids));
                    }
                }
                CapabilitySource::Custom => {
                    if let Some(profile_id) = capability_config.custom_profile_id {
                        return Ok(ServerSelection::Profile(profile_id));
                    }
                }
            }
        }

        let active_profiles = get_active_profile(&self.db_pool)
            .await
            .map_err(|err| crate::clients::ConfigError::DataAccessError(err.to_string()))?;

        let mut active_ids: Vec<String> = active_profiles.into_iter().filter_map(|p| p.id).collect();
        active_ids.sort();
        active_ids.dedup();

        if active_ids.is_empty() {
            return Ok(ServerSelection::AllEnabled);
        }
        if active_ids.len() == 1 {
            return Ok(ServerSelection::Profile(active_ids.remove(0)));
        }
        Ok(ServerSelection::Profiles(active_ids))
    }

    async fn fetch_servers(
        &self,
        selection: ServerSelection,
    ) -> ConfigResult<Vec<ServerRow>> {
        match selection {
            ServerSelection::AllEnabled => {
                let rows = sqlx::query_as::<_, ServerRow>(
                    r#"
                    SELECT id, name, command, url, server_type
                    FROM server_config
                    WHERE enabled = 1
                    ORDER BY name
                    "#,
                )
                .fetch_all(&*self.db_pool)
                .await
                .map_err(|err| crate::clients::ConfigError::DataAccessError(err.to_string()))?;
                Ok(rows)
            }
            ServerSelection::Profile(profile_id) => {
                let rows = sqlx::query_as::<_, ServerRow>(
                    r#"
                    SELECT sc.id, sc.name, sc.command, sc.url, sc.server_type
                    FROM server_config sc
                    JOIN profile_server ps ON sc.id = ps.server_id
                    WHERE ps.profile_id = ?
                        AND ps.enabled = 1
                        AND sc.enabled = 1
                    ORDER BY ps.server_name
                    "#,
                )
                .bind(profile_id)
                .fetch_all(&*self.db_pool)
                .await
                .map_err(|err| crate::clients::ConfigError::DataAccessError(err.to_string()))?;
                Ok(rows)
            }
            ServerSelection::Profiles(profile_ids) => {
                if profile_ids.is_empty() {
                    return Ok(Vec::new());
                }
                let placeholders = vec!["?"; profile_ids.len()].join(", ");
                let sql = format!(
                    r#"
                    SELECT DISTINCT sc.id, sc.name, sc.command, sc.url, sc.server_type
                    FROM server_config sc
                    JOIN profile_server ps ON sc.id = ps.server_id
                    WHERE ps.profile_id IN ({})
                        AND ps.enabled = 1
                        AND sc.enabled = 1
                    ORDER BY sc.name
                    "#,
                    placeholders
                );
                let mut query = sqlx::query_as::<_, ServerRow>(&sql);
                for id in profile_ids {
                    query = query.bind(id);
                }
                query
                    .fetch_all(&*self.db_pool)
                    .await
                    .map_err(|err| crate::clients::ConfigError::DataAccessError(err.to_string()))
            }
            ServerSelection::Explicit(ids) => {
                if ids.is_empty() {
                    return Ok(Vec::new());
                }
                let placeholders = vec!["?"; ids.len()].join(", ");
                let sql = format!(
                    r#"
                    SELECT id, name, command, url, server_type
                    FROM server_config
                    WHERE id IN ({}) AND enabled = 1
                    ORDER BY name
                    "#,
                    placeholders
                );
                let mut query = sqlx::query_as::<_, ServerRow>(&sql);
                for id in ids {
                    query = query.bind(id);
                }
                query
                    .fetch_all(&*self.db_pool)
                    .await
                    .map_err(|err| crate::clients::ConfigError::DataAccessError(err.to_string()))
            }
        }
    }
}

impl ClientConfigService {
    pub(super) async fn map_server_row(
        &self,
        row: crate::clients::service::query::ServerRow,
    ) -> ConfigResult<ServerTemplateInput> {
        let args = get_server_args(&self.db_pool, &row.id)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
            .into_iter()
            .map(|arg| arg.arg_value)
            .collect::<Vec<_>>();

        let env = get_server_env(&self.db_pool, &row.id)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        let transport = row.server_type.as_str().to_string();

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("server_id".to_string(), json!(row.id));
        metadata.insert("runtime".to_string(), json!(defaults::RUNTIME));

        Ok(ServerTemplateInput {
            name: sanitize_server_name(&row.name),
            display_name: Some(row.name),
            transport,
            command: row.command,
            args,
            env,
            url: row.url,
            headers: std::collections::HashMap::new(),
            metadata,
        })
    }
}

fn sanitize_server_name(name: &str) -> String {
    name.replace(' ', "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::models::ConfigMode;
    use crate::clients::source::{FileTemplateSource, TemplateRoot};
    use crate::common::profile::ProfileType;
    use crate::config::{
        client::init::{initialize_client_table, initialize_system_settings_table},
        models::Profile,
        profile::{self, init::initialize_profile_tables},
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
        let service = ClientConfigService::with_source(pool, source)
            .await
            .expect("client config service");

        (temp_dir, service)
    }

    async fn insert_profile(
        service: &ClientConfigService,
        name: &str,
        profile_type: ProfileType,
        is_active: bool,
    ) -> String {
        let mut profile = Profile::new(name.to_string(), profile_type);
        profile.is_active = is_active;
        profile::upsert_profile(service.db_pool.as_ref(), &profile)
            .await
            .expect("upsert profile")
    }

    #[tokio::test]
    async fn set_capability_config_normalizes_selected_profiles() {
        let (_temp_dir, service) = create_test_service().await;
        let profile_a = insert_profile(&service, "profile-a", ProfileType::Shared, false).await;
        let profile_b = insert_profile(&service, "profile-b", ProfileType::Shared, false).await;

        let config = service
            .set_capability_config(
                "client-a",
                CapabilitySource::Profiles,
                vec![format!("  {}  ", profile_b), profile_a.clone(), profile_b.clone()],
            )
            .await
            .expect("set capability config");

        let mut expected = vec![profile_a, profile_b];
        expected.sort();

        assert_eq!(config.capability_source, CapabilitySource::Profiles);
        assert_eq!(config.selected_profile_ids, expected);
        assert!(config.custom_profile_id.is_none());
        assert_eq!(
            service
                .get_capability_config("client-a")
                .await
                .expect("get capability config")
                .expect("stored config"),
            config
        );
    }

    #[tokio::test]
    async fn set_capability_config_custom_creates_host_app_profile() {
        let (_temp_dir, service) = create_test_service().await;

        let config = service
            .set_capability_config("client-a", CapabilitySource::Custom, vec!["ignored".to_string()])
            .await
            .expect("set custom capability config");

        let custom_profile_id = config.custom_profile_id.clone().expect("custom profile id");
        let profile = profile::get_profile(service.db_pool.as_ref(), &custom_profile_id)
            .await
            .expect("load custom profile")
            .expect("custom profile exists");

        assert_eq!(config.capability_source, CapabilitySource::Custom);
        assert!(config.selected_profile_ids.is_empty());
        assert_eq!(profile.profile_type, ProfileType::HostApp);
        assert_eq!(profile.name, "client-a_custom");
    }

    #[tokio::test]
    async fn update_capability_config_and_invalidate_rejects_empty_profiles_selection() {
        let (_temp_dir, service) = create_test_service().await;

        let error = service
            .update_capability_config_and_invalidate("client-a", CapabilitySource::Profiles, Vec::new())
            .await
            .expect_err("empty profiles selection should fail");

        assert!(
            error
                .to_string()
                .contains("profiles capability source requires at least one selected profile")
        );
    }

    #[tokio::test]
    async fn resolve_server_selection_prefers_client_profiles_over_active_profiles() {
        let (_temp_dir, service) = create_test_service().await;
        let active_profile_id = insert_profile(&service, "active-profile", ProfileType::Shared, true).await;
        let selected_profile_id = insert_profile(&service, "selected-profile", ProfileType::Shared, false).await;

        service
            .set_capability_config(
                "client-a",
                CapabilitySource::Profiles,
                vec![selected_profile_id.clone()],
            )
            .await
            .expect("set profile capability config");

        let selection = service
            .resolve_server_selection(&crate::clients::ClientRenderOptions {
                client_id: "client-a".to_string(),
                mode: ConfigMode::Managed,
                profile_id: None,
                server_ids: None,
                dry_run: true,
            })
            .await
            .expect("resolve selection");

        match selection {
            ServerSelection::Profile(profile_id) => assert_eq!(profile_id, selected_profile_id),
            other => panic!("expected selected profile, got {other:?}"),
        }

        assert_ne!(selected_profile_id, active_profile_id);
    }

    #[tokio::test]
    async fn resolve_server_selection_uses_custom_profile() {
        let (_temp_dir, service) = create_test_service().await;

        let config = service
            .set_capability_config("client-a", CapabilitySource::Custom, Vec::new())
            .await
            .expect("set custom capability config");

        let selection = service
            .resolve_server_selection(&crate::clients::ClientRenderOptions {
                client_id: "client-a".to_string(),
                mode: ConfigMode::Managed,
                profile_id: None,
                server_ids: None,
                dry_run: true,
            })
            .await
            .expect("resolve selection");

        match selection {
            ServerSelection::Profile(profile_id) => {
                assert_eq!(Some(profile_id), config.custom_profile_id)
            }
            other => panic!("expected custom profile, got {other:?}"),
        }
    }
}
