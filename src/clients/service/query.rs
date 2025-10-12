use super::ClientConfigService;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::ServerTemplateInput;
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
