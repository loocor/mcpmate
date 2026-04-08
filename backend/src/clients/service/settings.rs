use super::ClientConfigService;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{CapabilitySource, ClientCapabilityConfig};
use crate::clients::service::core::RuntimeClientMetadata;
use crate::common::profile::{ProfileRole, ProfileType};
use crate::config::models::Profile;
use serde_json::{Map, Value, json};

const VALID_TRANSPORTS: &[&str] = &["auto", "sse", "stdio", "streamable_http"];
const VALID_CONNECTION_MODES: &[&str] = &["local_config_detected", "remote_http", "manual"];
const VALID_SUPPORTED_TRANSPORTS: &[&str] = &["sse", "stdio", "streamable_http"];

#[derive(Debug, Clone, Default)]
pub struct ActiveClientSettingsUpdate {
    pub display_name: Option<String>,
    pub config_mode: Option<String>,
    pub transport: Option<String>,
    pub client_version: Option<String>,
    pub connection_mode: Option<String>,
    pub config_path: Option<String>,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub docs_url: Option<String>,
    pub support_url: Option<String>,
    pub logo_url: Option<String>,
    pub supported_transports: Option<Vec<String>>,
}

impl ClientConfigService {
    /// Update client settings (config_mode, transport, client_version)
    /// - config_mode: optional, only update if provided
    /// - transport: optional, only update if provided; must be one of: auto, sse, stdio, streamable_http
    /// - client_version: optional, only update if provided
    pub async fn set_client_settings(
        &self,
        identifier: &str,
        config_mode: Option<String>,
        transport: Option<String>,
        client_version: Option<String>,
    ) -> ConfigResult<()> {
        self.set_active_client_settings(
            identifier,
            ActiveClientSettingsUpdate {
                config_mode,
                transport,
                client_version,
                ..ActiveClientSettingsUpdate::default()
            },
        )
        .await
    }

    pub async fn set_active_client_settings(
        &self,
        identifier: &str,
        update: ActiveClientSettingsUpdate,
    ) -> ConfigResult<()> {
        tracing::info!(
            client = %identifier,
            config_mode = ?update.config_mode,
            transport = ?update.transport,
            client_version = ?update.client_version,
            connection_mode = ?update.connection_mode,
            config_path = ?update.config_path,
            "set_active_client_settings: entry"
        );

        if let Some(ref tr) = update.transport {
            if !VALID_TRANSPORTS.contains(&tr.as_str()) {
                let err = format!(
                    "Invalid transport value '{}', must be one of: {}",
                    tr,
                    VALID_TRANSPORTS.join(", ")
                );
                tracing::error!(client = %identifier, transport = %tr, "{}", err);
                return Err(ConfigError::DataAccessError(err));
            }
        }

        if let Some(ref mode) = update.connection_mode {
            if !VALID_CONNECTION_MODES.contains(&mode.as_str()) {
                let err = format!(
                    "Invalid connection_mode value '{}', must be one of: {}",
                    mode,
                    VALID_CONNECTION_MODES.join(", ")
                );
                tracing::error!(client = %identifier, connection_mode = %mode, "{}", err);
                return Err(ConfigError::DataAccessError(err));
            }
        }

        if let Some(ref supported_transports) = update.supported_transports {
            for transport in supported_transports {
                if !VALID_SUPPORTED_TRANSPORTS.contains(&transport.as_str()) {
                    let err = format!(
                        "Invalid supported transport '{}', must be one of: {}",
                        transport,
                        VALID_SUPPORTED_TRANSPORTS.join(", ")
                    );
                    tracing::error!(client = %identifier, supported_transport = %transport, "{}", err);
                    return Err(ConfigError::DataAccessError(err));
                }
            }
        }

        let name = update
            .display_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .unwrap_or(self.resolve_client_name(identifier).await?);
        let existing_state = self.fetch_state(identifier).await?;
        let should_persist_runtime_template = self
            .should_persist_runtime_active_template(identifier, existing_state.as_ref())
            .await?;
        self.ensure_active_state_row_with_name(identifier, &name, None, Some("approved"))
            .await?;

        self.update_client_name(identifier, &name).await?;
        self.update_display_name(identifier, &name).await?;

        if let Some(mode) = update.config_mode {
            self.update_config_mode(identifier, &mode).await?;
        }

        if let Some(tr) = update.transport {
            self.update_transport(identifier, &tr).await?;
        }

        if let Some(ver) = update.client_version {
            self.update_client_version(identifier, &ver).await?;
        }

        let resolved_connection_mode = update.connection_mode.or_else(|| {
            update.config_path.as_ref().map(|path| {
                if path.trim().is_empty() {
                    "manual".to_string()
                } else {
                    "local_config_detected".to_string()
                }
            })
        });

        if update.config_path.is_some() || resolved_connection_mode.is_some() {
            self.update_runtime_target(
                identifier,
                update.config_path.as_deref(),
                resolved_connection_mode.as_deref(),
            )
            .await?;
        }

        if update.description.is_some()
            || update.homepage_url.is_some()
            || update.docs_url.is_some()
            || update.support_url.is_some()
            || update.logo_url.is_some()
            || update.supported_transports.is_some()
        {
            let existing_metadata = existing_state
                .as_ref()
                .map(|state| state.runtime_client_metadata())
                .unwrap_or_default();
            let next_metadata = RuntimeClientMetadata {
                description: update.description.or(existing_metadata.description),
                homepage_url: update.homepage_url.or(existing_metadata.homepage_url),
                docs_url: update.docs_url.or(existing_metadata.docs_url),
                support_url: update.support_url.or(existing_metadata.support_url),
                logo_url: update.logo_url.or(existing_metadata.logo_url),
                category: existing_metadata.category,
                supported_transports: update
                    .supported_transports
                    .unwrap_or(existing_metadata.supported_transports),
            };
            self.update_runtime_client_metadata(identifier, &next_metadata)
                .await?;
        }

        if should_persist_runtime_template {
            self.persist_runtime_active_template(identifier).await?;
        }

        tracing::info!(client = %identifier, "set_active_client_settings: complete");
        Ok(())
    }

    /// Update client name
    async fn update_client_name(
        &self,
        identifier: &str,
        name: &str,
    ) -> ConfigResult<()> {
        tracing::debug!(client = %identifier, name = %name, "Updating client name");

        sqlx::query("UPDATE client SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
            .bind(name)
            .bind(identifier)
            .execute(&*self.db_pool)
            .await
            .map_err(|e| {
                tracing::error!(client = %identifier, error = %e, "Failed to update client name");
                ConfigError::DataAccessError(e.to_string())
            })?;

        Ok(())
    }

    async fn update_display_name(
        &self,
        identifier: &str,
        display_name: &str,
    ) -> ConfigResult<()> {
        sqlx::query("UPDATE client SET display_name = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
            .bind(display_name)
            .bind(identifier)
            .execute(&*self.db_pool)
            .await
            .map_err(|e| ConfigError::DataAccessError(e.to_string()))?;

        Ok(())
    }

    /// Update config_mode
    async fn update_config_mode(
        &self,
        identifier: &str,
        mode: &str,
    ) -> ConfigResult<()> {
        tracing::info!(client = %identifier, config_mode = %mode, "Updating config_mode");

        let result =
            sqlx::query("UPDATE client SET config_mode = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
                .bind(mode)
                .bind(identifier)
                .execute(&*self.db_pool)
                .await
                .map_err(|e| {
                    tracing::error!(client = %identifier, error = %e, "Failed to update config_mode");
                    ConfigError::DataAccessError(e.to_string())
                })?;

        tracing::info!(
            client = %identifier,
            rows_affected = %result.rows_affected(),
            "config_mode updated"
        );

        Ok(())
    }

    /// Update transport protocol
    async fn update_transport(
        &self,
        identifier: &str,
        transport: &str,
    ) -> ConfigResult<()> {
        tracing::info!(client = %identifier, transport = %transport, "Updating transport");

        let result =
            sqlx::query("UPDATE client SET transport = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
                .bind(transport)
                .bind(identifier)
                .execute(&*self.db_pool)
                .await
                .map_err(|e| {
                    tracing::error!(
                        client = %identifier,
                        transport = %transport,
                        error = %e,
                        "Failed to update transport"
                    );
                    ConfigError::DataAccessError(e.to_string())
                })?;

        tracing::info!(
            client = %identifier,
            transport = %transport,
            rows_affected = %result.rows_affected(),
            "transport updated"
        );

        Ok(())
    }

    /// Update client_version
    async fn update_client_version(
        &self,
        identifier: &str,
        version: &str,
    ) -> ConfigResult<()> {
        tracing::info!(client = %identifier, version = %version, "Updating client_version");

        let result =
            sqlx::query("UPDATE client SET client_version = ?, updated_at = CURRENT_TIMESTAMP WHERE identifier = ?")
                .bind(version)
                .bind(identifier)
                .execute(&*self.db_pool)
                .await
                .map_err(|e| {
                    tracing::error!(client = %identifier, error = %e, "Failed to update client_version");
                    ConfigError::DataAccessError(e.to_string())
                })?;

        tracing::info!(
            client = %identifier,
            rows_affected = %result.rows_affected(),
            "client_version updated"
        );

        Ok(())
    }

    async fn update_runtime_target(
        &self,
        identifier: &str,
        config_path: Option<&str>,
        connection_mode: Option<&str>,
    ) -> ConfigResult<()> {
        sqlx::query(
            r#"
            UPDATE client
            SET config_path = COALESCE(?, config_path),
                connection_mode = COALESCE(?, connection_mode),
                governance_kind = 'active',
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(config_path)
        .bind(connection_mode)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| ConfigError::DataAccessError(e.to_string()))?;

        Ok(())
    }

    async fn update_runtime_client_metadata(
        &self,
        identifier: &str,
        metadata: &RuntimeClientMetadata,
    ) -> ConfigResult<()> {
        let existing: Option<String> = sqlx::query_scalar("SELECT approval_metadata FROM client WHERE identifier = ?")
            .bind(identifier)
            .fetch_optional(&*self.db_pool)
            .await
            .map_err(|e| ConfigError::DataAccessError(e.to_string()))?;

        let mut payload = existing
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Map<String, Value>>(raw).ok())
            .unwrap_or_default();
        payload.insert("runtime_client".to_string(), json!(metadata));

        sqlx::query(
            r#"
            UPDATE client
            SET approval_metadata = ?, governance_kind = 'active', updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(serde_json::to_string(&payload).map_err(|e| ConfigError::DataAccessError(e.to_string()))?)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| ConfigError::DataAccessError(e.to_string()))?;

        Ok(())
    }

    /// Get client settings (config_mode, transport, client_version)
    /// Returns None if client state not found
    pub async fn get_client_settings(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<(Option<String>, String, Option<String>)>> {
        let state = self.fetch_state(identifier).await?;

        if state.is_none() {
            tracing::debug!(client = %identifier, "Client state not found");
            return Ok(None);
        }

        let state = state.unwrap();
        let transport = state.transport.unwrap_or_else(|| "auto".to_string());

        Ok(Some((state.config_mode, transport, state.client_version)))
    }

    pub async fn set_capability_config(
        &self,
        identifier: &str,
        capability_source: CapabilitySource,
        selected_profile_ids: Vec<String>,
    ) -> ConfigResult<ClientCapabilityConfig> {
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await?;

        let selected_profile_ids = self.normalize_selected_profile_ids(capability_source, selected_profile_ids)?;
        self.validate_selected_profile_ids(&selected_profile_ids).await?;

        let custom_profile_id = match capability_source {
            CapabilitySource::Activated | CapabilitySource::Profiles => None,
            CapabilitySource::Custom => Some(self.ensure_custom_profile(identifier).await?),
        };
        let selected_profile_ids_json = if selected_profile_ids.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&selected_profile_ids)
                    .map_err(|err| ConfigError::DataAccessError(err.to_string()))?,
            )
        };

        sqlx::query(
            r#"
            UPDATE client
            SET capability_source = ?,
                selected_profile_ids = ?,
                custom_profile_id = ?,
                governance_kind = 'active',
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(capability_source.as_str())
        .bind(selected_profile_ids_json)
        .bind(custom_profile_id.as_deref())
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        self.get_capability_config(identifier)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Failed to load capability config for {identifier}")))
    }

    pub async fn update_capability_config_and_invalidate(
        &self,
        identifier: &str,
        capability_source: CapabilitySource,
        selected_profile_ids: Vec<String>,
    ) -> ConfigResult<ClientCapabilityConfig> {
        let old_config = self.get_capability_config(identifier).await?;
        let old_fingerprint = old_config
            .as_ref()
            .map(crate::core::profile::visibility::compute_capability_fingerprint);

        let config = self
            .set_capability_config(identifier, capability_source, selected_profile_ids)
            .await?;

        if let Some(fingerprint) = old_fingerprint {
            if let Ok(cache_manager) = crate::core::cache::RedbCacheManager::global() {
                match cache_manager.invalidate_by_rules_fingerprint(&fingerprint).await {
                    Ok(count) => {
                        tracing::info!(
                            client = %identifier,
                            old_fingerprint = %fingerprint,
                            invalidated_count = count,
                            "Invalidated client-filtered cache entries after capability config update"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            client = %identifier,
                            error = %err,
                            "Failed to invalidate client-filtered cache entries"
                        );
                    }
                }
            }
        }

        crate::core::profile::visibility::invalidate_visibility_cache(identifier);
        Ok(config)
    }

    pub async fn get_capability_config(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<ClientCapabilityConfig>> {
        let state = self.fetch_state(identifier).await?;
        state.map(|row| row.capability_config()).transpose()
    }

    async fn ensure_custom_profile(
        &self,
        identifier: &str,
    ) -> ConfigResult<String> {
        let profile_name = format!("{}_custom", identifier);

        if let Some(profile) = crate::config::profile::get_profile_by_name(&self.db_pool, &profile_name)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
        {
            if profile.profile_type != ProfileType::HostApp {
                return Err(ConfigError::DataAccessError(format!(
                    "Profile '{}' already exists but is not host_app",
                    profile_name
                )));
            }

            return profile
                .id
                .ok_or_else(|| ConfigError::DataAccessError(format!("Profile '{}' is missing an id", profile_name)));
        }

        let profile = Profile {
            id: None,
            name: profile_name,
            description: Some(format!("Custom profile for {}", identifier)),
            profile_type: ProfileType::HostApp,
            role: ProfileRole::User,
            multi_select: false,
            priority: 0,
            is_active: false,
            is_default: false,
            created_at: None,
            updated_at: None,
        };

        crate::config::profile::upsert_profile(&self.db_pool, &profile)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))
    }

    async fn validate_selected_profile_ids(
        &self,
        selected_profile_ids: &[String],
    ) -> ConfigResult<()> {
        for profile_id in selected_profile_ids {
            let profile = crate::config::profile::get_profile(&self.db_pool, profile_id)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
                .ok_or_else(|| {
                    ConfigError::DataAccessError(format!("Selected profile '{}' does not exist", profile_id))
                })?;

            if profile.profile_type != ProfileType::Shared {
                return Err(ConfigError::DataAccessError(format!(
                    "Selected profile '{}' must be a shared profile",
                    profile_id
                )));
            }
        }

        Ok(())
    }

    fn normalize_selected_profile_ids(
        &self,
        capability_source: CapabilitySource,
        selected_profile_ids: Vec<String>,
    ) -> ConfigResult<Vec<String>> {
        match capability_source {
            CapabilitySource::Activated => Ok(Vec::new()),
            CapabilitySource::Profiles => {
                let mut normalized = selected_profile_ids
                    .into_iter()
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty())
                    .collect::<Vec<_>>();
                normalized.sort();
                normalized.dedup();

                if normalized.is_empty() {
                    return Err(ConfigError::DataAccessError(
                        "profiles capability source requires at least one selected profile".to_string(),
                    ));
                }

                Ok(normalized)
            }
            CapabilitySource::Custom => Ok(Vec::new()),
        }
    }
}
