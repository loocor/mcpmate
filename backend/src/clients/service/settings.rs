use super::ClientConfigService;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    CapabilitySource, ClientCapabilityConfig, ClientCapabilityConfigState, UnifyDirectExposureConfig,
    UnifyDirectExposureDiagnostics, UnifyDirectPromptSurface, UnifyDirectPromptSurfaceDiagnostic,
    UnifyDirectResourceSurface, UnifyDirectResourceSurfaceDiagnostic, UnifyDirectTemplateSurface,
    UnifyDirectTemplateSurfaceDiagnostic, UnifyDirectToolSurface, UnifyDirectToolSurfaceDiagnostic,
};
use crate::clients::service::core::RuntimeClientMetadata;
use crate::common::profile::{ProfileRole, ProfileType};
use crate::config::models::Profile;
use crate::system::paths::get_path_service;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use tokio::fs::OpenOptions;

const VALID_TRANSPORTS: &[&str] = &["auto", "sse", "stdio", "streamable_http"];
const VALID_CONNECTION_MODES: &[&str] = &["local_config_detected", "remote_http", "manual"];
const VALID_SUPPORTED_TRANSPORTS: &[&str] = &["sse", "stdio", "streamable_http"];

#[derive(Debug, Clone, Default)]
struct ResolvedUnifyDirectExposureState {
    config: UnifyDirectExposureConfig,
    diagnostics: UnifyDirectExposureDiagnostics,
}

#[derive(Debug, Clone, Default)]
struct UnifyDirectExposureInventory {
    tools: HashMap<String, HashSet<String>>,
    prompts: HashMap<String, HashSet<String>>,
    resources: HashMap<String, HashSet<String>>,
    templates: HashMap<String, HashSet<String>>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciledUnifyDirectExposure {
    pub identifier: String,
    pub unify_direct_exposure: UnifyDirectExposureConfig,
    pub visible_surface_changed: bool,
}

fn unify_direct_exposure_references_server(
    config: &UnifyDirectExposureConfig,
    server_id: &str,
) -> bool {
    config.selected_server_ids.iter().any(|id| id == server_id)
        || config
            .selected_tool_surfaces
            .iter()
            .any(|surface| surface.server_id == server_id)
        || config
            .selected_prompt_surfaces
            .iter()
            .any(|surface| surface.server_id == server_id)
        || config
            .selected_resource_surfaces
            .iter()
            .any(|surface| surface.server_id == server_id)
        || config
            .selected_template_surfaces
            .iter()
            .any(|surface| surface.server_id == server_id)
}

fn serialize_json_or_none<T: serde::Serialize>(value: &T) -> ConfigResult<Option<String>> {
    let serialized = serde_json::to_string(value)
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
    Ok(Some(serialized))
}

impl ClientConfigService {
    pub async fn get_effective_config_mode(
        &self,
        identifier: &str,
    ) -> ConfigResult<String> {
        let explicit = self
            .fetch_state(identifier)
            .await?
            .and_then(|state| state.config_mode)
            .filter(|mode| !mode.trim().is_empty());

        match explicit {
            Some(mode) => Ok(mode),
            None => crate::config::client::init::resolve_default_client_config_mode(&self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string())),
        }
    }

    async fn validate_runtime_target_input(
        &self,
        connection_mode: Option<&str>,
        config_path: Option<&str>,
    ) -> ConfigResult<()> {
        let normalized_path = config_path.map(str::trim).filter(|value| !value.is_empty());

        match connection_mode {
            Some("local_config_detected") => {
                let raw_path = normalized_path.ok_or_else(|| {
                    ConfigError::DataAccessError(
                        "Clients with a local config target must provide a valid MCP config file path.".to_string(),
                    )
                })?;
                self.validate_existing_config_target(raw_path).await?;
            }
            Some("manual") | Some("remote_http") => {
                if normalized_path.is_some() {
                    return Err(ConfigError::DataAccessError(
                        "Only clients with a local config target may store a config file path.".to_string(),
                    ));
                }
            }
            _ => {
                if let Some(raw_path) = normalized_path {
                    self.validate_existing_config_target(raw_path).await?;
                }
            }
        }

        Ok(())
    }

    async fn validate_existing_config_target(
        &self,
        raw_path: &str,
    ) -> ConfigResult<()> {
        let resolved_path = get_path_service()
            .resolve_user_path(raw_path)
            .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
        let metadata = tokio::fs::metadata(&resolved_path).await.map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                ConfigError::DataAccessError(format!("Configured MCP file does not exist: {}", raw_path))
            } else {
                ConfigError::FileOperationError(format!("Failed to inspect configured MCP file {}: {}", raw_path, err))
            }
        })?;

        if metadata.is_file() {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&resolved_path)
                .await
                .map_err(|_| ConfigError::PathNotWritable { path: resolved_path })?;
        } else if metadata.is_dir() {
            Self::validate_directory_target_writable(&resolved_path).await?;
        } else {
            return Err(ConfigError::DataAccessError(format!(
                "Configured MCP path is neither a file nor a directory: {}",
                raw_path
            )));
        }

        Ok(())
    }

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

        let raw_config_path = update.config_path.as_deref().map(str::trim);
        let normalized_config_path = raw_config_path
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let resolved_connection_mode = update.connection_mode.clone().or_else(|| match raw_config_path {
            Some("") => Some("manual".to_string()),
            Some(_) => Some("local_config_detected".to_string()),
            None => None,
        });

        self.validate_runtime_target_input(resolved_connection_mode.as_deref(), normalized_config_path.as_deref())
            .await?;

        let approval_status = existing_state
            .as_ref()
            .map(|state| state.approval_status().to_string())
            .unwrap_or_else(|| "approved".to_string());

        self.ensure_active_state_row_with_name(identifier, &name, None, Some(&approval_status))
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

        if update.config_path.is_some() || resolved_connection_mode.is_some() {
            self.update_runtime_target(
                identifier,
                normalized_config_path.as_deref(),
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
            self.update_runtime_client_metadata(identifier, &next_metadata).await?;
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
            SET config_path = CASE
                    WHEN ? IS NOT NULL THEN NULLIF(?, '')
                    WHEN ? IN ('manual', 'remote_http') THEN NULL
                    ELSE NULLIF(?, '')
                END,
                connection_mode = CASE
                    WHEN ? IS NULL THEN connection_mode
                    ELSE NULLIF(?, '')
                END,
                governance_kind = 'active',
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(config_path)
        .bind(config_path)
        .bind(connection_mode)
        .bind(config_path)
        .bind(connection_mode)
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
            serialize_json_or_none(&selected_profile_ids)?
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

    pub async fn get_capability_config_state(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<ClientCapabilityConfigState>> {
        let Some(state) = self.fetch_state(identifier).await? else {
            return Ok(None);
        };

        let capability_config = state.capability_config()?;
        let raw_unify_direct_exposure = state.unify_direct_exposure_config()?;
        let resolved = self
            .resolve_unify_direct_exposure_config(identifier, &capability_config, &raw_unify_direct_exposure)
            .await?;

        Ok(Some(ClientCapabilityConfigState {
            capability_config,
            unify_direct_exposure: resolved.config,
            unify_direct_exposure_diagnostics: resolved.diagnostics,
        }))
    }

    pub async fn get_unify_direct_exposure_config(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<UnifyDirectExposureConfig>> {
        Ok(self
            .get_capability_config_state(identifier)
            .await?
            .map(|state| state.unify_direct_exposure))
    }

    pub async fn update_capability_config_state_and_invalidate(
        &self,
        identifier: &str,
        capability_source: CapabilitySource,
        selected_profile_ids: Vec<String>,
        unify_direct_exposure_update: Option<UnifyDirectExposureConfig>,
    ) -> ConfigResult<(ClientCapabilityConfigState, bool)> {
        let old_state = self.get_capability_config_state(identifier).await?;
        let old_fingerprint = old_state
            .as_ref()
            .map(|state| crate::core::profile::visibility::compute_capability_fingerprint(&state.capability_config));
        let effective_mode = self.get_effective_config_mode(identifier).await?;
        let default_capability_config = ClientCapabilityConfig::default();
        let default_unify_direct_exposure = UnifyDirectExposureConfig::default();
        let old_visible_fingerprint = if effective_mode == "unify" {
            self.compute_unify_visible_tool_surface_fingerprint(
                identifier,
                old_state
                    .as_ref()
                    .map(|state| &state.capability_config)
                    .unwrap_or(&default_capability_config),
                old_state
                    .as_ref()
                    .map(|state| &state.unify_direct_exposure)
                    .unwrap_or(&default_unify_direct_exposure),
            )
            .await?
        } else {
            String::new()
        };

        let state = self
            .set_capability_config_state(
                identifier,
                capability_source,
                selected_profile_ids,
                unify_direct_exposure_update,
            )
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

        let visible_surface_changed = if effective_mode == "unify" {
            let new_visible_fingerprint = self
                .compute_unify_visible_tool_surface_fingerprint(
                    identifier,
                    &state.capability_config,
                    &state.unify_direct_exposure,
                )
                .await?;
            old_visible_fingerprint != new_visible_fingerprint
        } else {
            false
        };

        Ok((state, visible_surface_changed))
    }

    pub async fn update_capability_config_and_invalidate(
        &self,
        identifier: &str,
        capability_source: CapabilitySource,
        selected_profile_ids: Vec<String>,
    ) -> ConfigResult<ClientCapabilityConfig> {
        self.update_capability_config_state_and_invalidate(identifier, capability_source, selected_profile_ids, None)
            .await
            .map(|(state, _)| state.capability_config)
    }

    pub async fn reconcile_unify_direct_exposure_for_server(
        &self,
        server_id: &str,
    ) -> ConfigResult<Vec<ReconciledUnifyDirectExposure>> {
        let states = self.fetch_client_states().await?;
        let mut reconciled = Vec::new();

        for (identifier, row) in states {
            let raw_unify_direct_exposure = row.unify_direct_exposure_config()?;
            if !unify_direct_exposure_references_server(&raw_unify_direct_exposure, server_id) {
                continue;
            }

            if self.get_effective_config_mode(&identifier).await? != "unify" {
                continue;
            }

            let capability_config = row.capability_config()?;
            let (state, visible_surface_changed) = self
                .update_capability_config_state_and_invalidate(
                    &identifier,
                    capability_config.capability_source,
                    capability_config.selected_profile_ids,
                    None,
                )
                .await?;

            reconciled.push(ReconciledUnifyDirectExposure {
                identifier,
                unify_direct_exposure: state.unify_direct_exposure,
                visible_surface_changed,
            });
        }

        Ok(reconciled)
    }

    pub async fn get_capability_config(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<ClientCapabilityConfig>> {
        Ok(self
            .get_capability_config_state(identifier)
            .await?
            .map(|state| state.capability_config))
    }

    async fn set_capability_config_state(
        &self,
        identifier: &str,
        capability_source: CapabilitySource,
        selected_profile_ids: Vec<String>,
        unify_direct_exposure_update: Option<UnifyDirectExposureConfig>,
    ) -> ConfigResult<ClientCapabilityConfigState> {
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

        let existing_unify_direct_exposure = self
            .fetch_state(identifier)
            .await?
            .map(|row| row.unify_direct_exposure_config())
            .transpose()?
            .unwrap_or_default();
        let requested_unify_direct_exposure = unify_direct_exposure_update.unwrap_or(existing_unify_direct_exposure);
        let resolved_unify_direct_exposure = self
            .resolve_unify_direct_exposure_config(
                identifier,
                &ClientCapabilityConfig {
                    capability_source,
                    selected_profile_ids: selected_profile_ids.clone(),
                    custom_profile_id: custom_profile_id.clone(),
                },
                &requested_unify_direct_exposure,
            )
            .await?;
        let unify_selected_server_ids_json = if resolved_unify_direct_exposure.config.selected_server_ids.is_empty() {
            None
        } else {
            serialize_json_or_none(&resolved_unify_direct_exposure.config.selected_server_ids)?
        };
        let unify_selected_tool_surfaces_json =
            if resolved_unify_direct_exposure.config.selected_tool_surfaces.is_empty() {
                None
            } else {
                serialize_json_or_none(&resolved_unify_direct_exposure.config.selected_tool_surfaces)?
            };
        let unify_selected_prompt_surfaces_json =
            if resolved_unify_direct_exposure.config.selected_prompt_surfaces.is_empty() {
                None
            } else {
                serialize_json_or_none(&resolved_unify_direct_exposure.config.selected_prompt_surfaces)?
            };
        let unify_selected_resource_surfaces_json =
            if resolved_unify_direct_exposure.config.selected_resource_surfaces.is_empty() {
                None
            } else {
                serialize_json_or_none(&resolved_unify_direct_exposure.config.selected_resource_surfaces)?
            };
        let unify_selected_template_surfaces_json =
            if resolved_unify_direct_exposure.config.selected_template_surfaces.is_empty() {
                None
            } else {
                serialize_json_or_none(&resolved_unify_direct_exposure.config.selected_template_surfaces)?
            };

        sqlx::query(
            r#"
            UPDATE client
            SET capability_source = ?,
                selected_profile_ids = ?,
                custom_profile_id = ?,
                unify_route_mode = ?,
                unify_selected_server_ids = ?,
                unify_selected_tool_surfaces = ?,
                unify_selected_prompt_surfaces = ?,
                unify_selected_resource_surfaces = ?,
                unify_selected_template_surfaces = ?,
                governance_kind = 'active',
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(capability_source.as_str())
        .bind(selected_profile_ids_json)
        .bind(custom_profile_id.as_deref())
        .bind(resolved_unify_direct_exposure.config.route_mode.as_str())
        .bind(unify_selected_server_ids_json)
        .bind(unify_selected_tool_surfaces_json)
        .bind(unify_selected_prompt_surfaces_json)
        .bind(unify_selected_resource_surfaces_json)
        .bind(unify_selected_template_surfaces_json)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(ClientCapabilityConfigState {
            capability_config: ClientCapabilityConfig {
                capability_source,
                selected_profile_ids,
                custom_profile_id,
            },
            unify_direct_exposure: resolved_unify_direct_exposure.config,
            unify_direct_exposure_diagnostics: resolved_unify_direct_exposure.diagnostics,
        })
    }

    async fn resolve_unify_direct_exposure_config(
        &self,
        identifier: &str,
        capability_config: &ClientCapabilityConfig,
        config: &UnifyDirectExposureConfig,
    ) -> ConfigResult<ResolvedUnifyDirectExposureState> {
        let inventory = self.load_unify_direct_exposure_inventory().await?;
        let visible_server_ids = self
            .resolve_visible_server_ids_for_unify_direct_exposure(identifier, capability_config)
            .await?;
        let mut diagnostics = UnifyDirectExposureDiagnostics::default();

        let selected_server_ids = self.normalize_selected_server_ids_for_unify(config.selected_server_ids.clone());
        let selected_server_ids = selected_server_ids
            .into_iter()
            .filter_map(|server_id| {
                if !visible_server_ids.contains(&server_id) {
                    diagnostics.invalid_server_ids.push(server_id);
                    None
                } else if inventory.tools.contains_key(&server_id)
                    || inventory.prompts.contains_key(&server_id)
                    || inventory.resources.contains_key(&server_id)
                    || inventory.templates.contains_key(&server_id)
                {
                    Some(server_id)
                } else {
                    diagnostics.invalid_server_ids.push(server_id);
                    None
                }
            })
            .collect::<Vec<_>>();

        let selected_tool_surfaces = self.normalize_selected_tool_surfaces(config.selected_tool_surfaces.clone());
        let selected_tool_surfaces = selected_tool_surfaces
            .into_iter()
            .filter_map(|surface| {
                if !visible_server_ids.contains(&surface.server_id) {
                    diagnostics
                        .invalid_tool_surfaces
                        .push(UnifyDirectToolSurfaceDiagnostic {
                            server_id: surface.server_id,
                            tool_name: surface.tool_name,
                            reason: "server_not_visible".to_string(),
                        });
                    return None;
                }

                let Some(tool_names) = inventory.tools.get(&surface.server_id) else {
                    diagnostics
                        .invalid_tool_surfaces
                        .push(UnifyDirectToolSurfaceDiagnostic {
                            server_id: surface.server_id,
                            tool_name: surface.tool_name,
                            reason: "server_not_eligible_or_missing".to_string(),
                        });
                    return None;
                };

                if tool_names.contains(&surface.tool_name) {
                    Some(surface)
                } else {
                    diagnostics
                        .invalid_tool_surfaces
                        .push(UnifyDirectToolSurfaceDiagnostic {
                            server_id: surface.server_id,
                            tool_name: surface.tool_name,
                            reason: "tool_not_found".to_string(),
                        });
                    None
                }
            })
            .collect::<Vec<_>>();
        let selected_prompt_surfaces =
            self.normalize_selected_prompt_surfaces(config.selected_prompt_surfaces.clone());
        let selected_prompt_surfaces = selected_prompt_surfaces
            .into_iter()
            .filter_map(|surface| {
                if !visible_server_ids.contains(&surface.server_id) {
                    diagnostics
                        .invalid_prompt_surfaces
                        .push(UnifyDirectPromptSurfaceDiagnostic {
                            server_id: surface.server_id,
                            prompt_name: surface.prompt_name,
                            reason: "server_not_visible".to_string(),
                        });
                    return None;
                }
                let Some(prompt_names) = inventory.prompts.get(&surface.server_id) else {
                    diagnostics
                        .invalid_prompt_surfaces
                        .push(UnifyDirectPromptSurfaceDiagnostic {
                            server_id: surface.server_id,
                            prompt_name: surface.prompt_name,
                            reason: "server_not_eligible_or_missing".to_string(),
                        });
                    return None;
                };
                if prompt_names.contains(&surface.prompt_name) {
                    Some(surface)
                } else {
                    diagnostics
                        .invalid_prompt_surfaces
                        .push(UnifyDirectPromptSurfaceDiagnostic {
                            server_id: surface.server_id,
                            prompt_name: surface.prompt_name,
                            reason: "prompt_not_found".to_string(),
                        });
                    None
                }
            })
            .collect::<Vec<_>>();
        let selected_resource_surfaces =
            self.normalize_selected_resource_surfaces(config.selected_resource_surfaces.clone());
        let selected_resource_surfaces = selected_resource_surfaces
            .into_iter()
            .filter_map(|surface| {
                if !visible_server_ids.contains(&surface.server_id) {
                    diagnostics
                        .invalid_resource_surfaces
                        .push(UnifyDirectResourceSurfaceDiagnostic {
                            server_id: surface.server_id,
                            resource_uri: surface.resource_uri,
                            reason: "server_not_visible".to_string(),
                        });
                    return None;
                }
                let Some(resource_uris) = inventory.resources.get(&surface.server_id) else {
                    diagnostics
                        .invalid_resource_surfaces
                        .push(UnifyDirectResourceSurfaceDiagnostic {
                            server_id: surface.server_id,
                            resource_uri: surface.resource_uri,
                            reason: "server_not_eligible_or_missing".to_string(),
                        });
                    return None;
                };
                if resource_uris.contains(&surface.resource_uri) {
                    Some(surface)
                } else {
                    diagnostics
                        .invalid_resource_surfaces
                        .push(UnifyDirectResourceSurfaceDiagnostic {
                            server_id: surface.server_id,
                            resource_uri: surface.resource_uri,
                            reason: "resource_not_found".to_string(),
                        });
                    None
                }
            })
            .collect::<Vec<_>>();
        let selected_template_surfaces =
            self.normalize_selected_template_surfaces(config.selected_template_surfaces.clone());
        let selected_template_surfaces = selected_template_surfaces
            .into_iter()
            .filter_map(|surface| {
                if !visible_server_ids.contains(&surface.server_id) {
                    diagnostics
                        .invalid_template_surfaces
                        .push(UnifyDirectTemplateSurfaceDiagnostic {
                            server_id: surface.server_id,
                            uri_template: surface.uri_template,
                            reason: "server_not_visible".to_string(),
                        });
                    return None;
                }
                let Some(uri_templates) = inventory.templates.get(&surface.server_id) else {
                    diagnostics
                        .invalid_template_surfaces
                        .push(UnifyDirectTemplateSurfaceDiagnostic {
                            server_id: surface.server_id,
                            uri_template: surface.uri_template,
                            reason: "server_not_eligible_or_missing".to_string(),
                        });
                    return None;
                };
                if uri_templates.contains(&surface.uri_template) {
                    Some(surface)
                } else {
                    diagnostics
                        .invalid_template_surfaces
                        .push(UnifyDirectTemplateSurfaceDiagnostic {
                            server_id: surface.server_id,
                            uri_template: surface.uri_template,
                            reason: "template_not_found".to_string(),
                        });
                    None
                }
            })
            .collect::<Vec<_>>();

        diagnostics.invalid_server_ids.sort();
        diagnostics.invalid_server_ids.dedup();
        diagnostics.invalid_tool_surfaces.sort_by(|left, right| {
            left.server_id
                .cmp(&right.server_id)
                .then(left.tool_name.cmp(&right.tool_name))
                .then(left.reason.cmp(&right.reason))
        });
        diagnostics.invalid_tool_surfaces.dedup();
        diagnostics.invalid_prompt_surfaces.sort_by(|left, right| {
            left.server_id
                .cmp(&right.server_id)
                .then(left.prompt_name.cmp(&right.prompt_name))
                .then(left.reason.cmp(&right.reason))
        });
        diagnostics.invalid_prompt_surfaces.dedup();
        diagnostics.invalid_resource_surfaces.sort_by(|left, right| {
            left.server_id
                .cmp(&right.server_id)
                .then(left.resource_uri.cmp(&right.resource_uri))
                .then(left.reason.cmp(&right.reason))
        });
        diagnostics.invalid_resource_surfaces.dedup();
        diagnostics.invalid_template_surfaces.sort_by(|left, right| {
            left.server_id
                .cmp(&right.server_id)
                .then(left.uri_template.cmp(&right.uri_template))
                .then(left.reason.cmp(&right.reason))
        });
        diagnostics.invalid_template_surfaces.dedup();

        Ok(ResolvedUnifyDirectExposureState {
            config: UnifyDirectExposureConfig {
                route_mode: config.route_mode,
                selected_server_ids,
                selected_tool_surfaces,
                selected_prompt_surfaces,
                selected_resource_surfaces,
                selected_template_surfaces,
            },
            diagnostics,
        })
    }

    async fn load_unify_direct_exposure_inventory(&self) -> ConfigResult<UnifyDirectExposureInventory> {
        let tool_rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, st.tool_name
            FROM server_config sc
            LEFT JOIN server_tools st ON st.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, st.tool_name
            "#,
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        let prompt_rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, sp.prompt_name
            FROM server_config sc
            LEFT JOIN server_prompts sp ON sp.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, sp.prompt_name
            "#,
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        let resource_rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, sr.resource_uri
            FROM server_config sc
            LEFT JOIN server_resources sr ON sr.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, sr.resource_uri
            "#,
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        let template_rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, srt.uri_template
            FROM server_config sc
            LEFT JOIN server_resource_templates srt ON srt.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, srt.uri_template
            "#,
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        let mut inventory = UnifyDirectExposureInventory::default();
        for (server_id, tool_name) in tool_rows {
            let entry = inventory.tools.entry(server_id).or_default();
            if let Some(tool_name) = tool_name {
                entry.insert(tool_name);
            }
        }
        for (server_id, prompt_name) in prompt_rows {
            let entry = inventory.prompts.entry(server_id).or_default();
            if let Some(prompt_name) = prompt_name {
                entry.insert(prompt_name);
            }
        }
        for (server_id, resource_uri) in resource_rows {
            let entry = inventory.resources.entry(server_id).or_default();
            if let Some(resource_uri) = resource_uri {
                entry.insert(resource_uri);
            }
        }
        for (server_id, uri_template) in template_rows {
            let entry = inventory.templates.entry(server_id).or_default();
            if let Some(uri_template) = uri_template {
                entry.insert(uri_template);
            }
        }

        Ok(inventory)
    }

    async fn compute_unify_visible_tool_surface_fingerprint(
        &self,
        identifier: &str,
        capability_config: &ClientCapabilityConfig,
        unify_direct_exposure: &UnifyDirectExposureConfig,
    ) -> ConfigResult<String> {
        let visible_server_ids = self
            .resolve_visible_server_ids_for_unify_direct_exposure(identifier, capability_config)
            .await?;
        let inventory = self.load_unify_direct_exposure_inventory().await?;
        let mut visible_surfaces = Vec::new();

        match unify_direct_exposure.route_mode {
            crate::clients::models::UnifyRouteMode::BrokerOnly => {}
            crate::clients::models::UnifyRouteMode::ServerLive => {
                for server_id in &unify_direct_exposure.selected_server_ids {
                    if !visible_server_ids.contains(server_id) {
                        continue;
                    }
                    if let Some(tool_names) = inventory.tools.get(server_id) {
                        for tool_name in tool_names {
                            visible_surfaces.push(format!("tool\u{1f}{server_id}\u{1f}{tool_name}"));
                        }
                    }
                    if let Some(prompt_names) = inventory.prompts.get(server_id) {
                        for prompt_name in prompt_names {
                            visible_surfaces.push(format!("prompt\u{1f}{server_id}\u{1f}{prompt_name}"));
                        }
                    }
                    if let Some(resource_uris) = inventory.resources.get(server_id) {
                        for resource_uri in resource_uris {
                            visible_surfaces.push(format!("resource\u{1f}{server_id}\u{1f}{resource_uri}"));
                        }
                    }
                    if let Some(uri_templates) = inventory.templates.get(server_id) {
                        for uri_template in uri_templates {
                            visible_surfaces.push(format!("template\u{1f}{server_id}\u{1f}{uri_template}"));
                        }
                    }
                }
            }
            crate::clients::models::UnifyRouteMode::CapabilityLevel => {
                for surface in &unify_direct_exposure.selected_tool_surfaces {
                    if visible_server_ids.contains(&surface.server_id) {
                        visible_surfaces.push(format!(
                            "tool\u{1f}{}\u{1f}{}",
                            surface.server_id, surface.tool_name
                        ));
                    }
                }
                for surface in &unify_direct_exposure.selected_prompt_surfaces {
                    if visible_server_ids.contains(&surface.server_id) {
                        visible_surfaces.push(format!(
                            "prompt\u{1f}{}\u{1f}{}",
                            surface.server_id, surface.prompt_name
                        ));
                    }
                }
                for surface in &unify_direct_exposure.selected_resource_surfaces {
                    if visible_server_ids.contains(&surface.server_id) {
                        visible_surfaces.push(format!(
                            "resource\u{1f}{}\u{1f}{}",
                            surface.server_id, surface.resource_uri
                        ));
                    }
                }
                for surface in &unify_direct_exposure.selected_template_surfaces {
                    if visible_server_ids.contains(&surface.server_id) {
                        visible_surfaces.push(format!(
                            "template\u{1f}{}\u{1f}{}",
                            surface.server_id, surface.uri_template
                        ));
                    }
                }
            }
        }

        visible_surfaces.sort();
        visible_surfaces.dedup();
        Ok(visible_surfaces.join("\u{1e}"))
    }

    async fn resolve_visible_server_ids_for_capability_config(
        &self,
        identifier: &str,
        capability_config: &ClientCapabilityConfig,
    ) -> ConfigResult<HashSet<String>> {
        let profile_ids = self
            .resolve_profile_ids_for_capability_config(identifier, capability_config)
            .await?;
        let server_ids = if profile_ids.is_empty() {
            if capability_config.capability_source == CapabilitySource::Activated {
                sqlx::query_scalar::<_, String>(
                    r#"
                    SELECT id
                    FROM server_config
                    WHERE enabled = 1
                    ORDER BY name, id
                    "#,
                )
                .fetch_all(&*self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
            } else {
                Vec::new()
            }
        } else {
            let placeholders = vec!["?"; profile_ids.len()].join(", ");
            let sql = format!(
                r#"
                SELECT DISTINCT sc.id
                FROM server_config sc
                JOIN profile_server ps ON sc.id = ps.server_id
                WHERE ps.profile_id IN ({placeholders})
                  AND ps.enabled = 1
                  AND sc.enabled = 1
                ORDER BY sc.name, sc.id
                "#,
            );
            let mut query = sqlx::query_scalar::<_, String>(&sql);
            for profile_id in &profile_ids {
                query = query.bind(profile_id);
            }
            query
                .fetch_all(&*self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
        };

        Ok(server_ids.into_iter().collect())
    }

    async fn resolve_visible_server_ids_for_unify_direct_exposure(
        &self,
        identifier: &str,
        capability_config: &ClientCapabilityConfig,
    ) -> ConfigResult<HashSet<String>> {
        match capability_config.capability_source {
            CapabilitySource::Activated => {
                let server_ids = sqlx::query_scalar::<_, String>(
                    r#"
                    SELECT id
                    FROM server_config
                    WHERE enabled = 1
                    ORDER BY name, id
                    "#,
                )
                .fetch_all(&*self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

                Ok(server_ids.into_iter().collect())
            }
            CapabilitySource::Profiles | CapabilitySource::Custom => {
                self.resolve_visible_server_ids_for_capability_config(identifier, capability_config)
                    .await
            }
        }
    }

    async fn resolve_profile_ids_for_capability_config(
        &self,
        identifier: &str,
        capability_config: &ClientCapabilityConfig,
    ) -> ConfigResult<Vec<String>> {
        let mut profile_ids = match capability_config.capability_source {
            CapabilitySource::Activated => crate::config::profile::basic::get_active_profile(&self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
                .into_iter()
                .filter_map(|profile| profile.id)
                .collect::<Vec<_>>(),
            CapabilitySource::Profiles => capability_config.selected_profile_ids.clone(),
            CapabilitySource::Custom => vec![capability_config.custom_profile_id.clone().ok_or_else(|| {
                ConfigError::DataAccessError(format!(
                    "Custom capability source requires custom_profile_id for {}",
                    identifier
                ))
            })?],
        };

        profile_ids.sort();
        profile_ids.dedup();
        Ok(profile_ids)
    }

    fn normalize_selected_server_ids_for_unify(
        &self,
        selected_server_ids: Vec<String>,
    ) -> Vec<String> {
        let mut normalized = selected_server_ids
            .into_iter()
            .map(|server_id| server_id.trim().to_string())
            .filter(|server_id| !server_id.is_empty())
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    fn normalize_selected_tool_surfaces(
        &self,
        selected_tool_surfaces: Vec<UnifyDirectToolSurface>,
    ) -> Vec<UnifyDirectToolSurface> {
        let mut normalized = selected_tool_surfaces
            .into_iter()
            .map(|surface| UnifyDirectToolSurface {
                server_id: surface.server_id.trim().to_string(),
                tool_name: surface.tool_name.trim().to_string(),
            })
            .filter(|surface| !surface.server_id.is_empty() && !surface.tool_name.is_empty())
            .collect::<Vec<_>>();
        normalized.sort_by(|left, right| {
            left.server_id
                .cmp(&right.server_id)
                .then(left.tool_name.cmp(&right.tool_name))
        });
        normalized.dedup();
        normalized
    }

    fn normalize_selected_prompt_surfaces(
        &self,
        selected_prompt_surfaces: Vec<UnifyDirectPromptSurface>,
    ) -> Vec<UnifyDirectPromptSurface> {
        let mut normalized = selected_prompt_surfaces
            .into_iter()
            .map(|surface| UnifyDirectPromptSurface {
                server_id: surface.server_id.trim().to_string(),
                prompt_name: surface.prompt_name.trim().to_string(),
            })
            .filter(|surface| !surface.server_id.is_empty() && !surface.prompt_name.is_empty())
            .collect::<Vec<_>>();
        normalized.sort_by(|left, right| {
            left.server_id
                .cmp(&right.server_id)
                .then(left.prompt_name.cmp(&right.prompt_name))
        });
        normalized.dedup();
        normalized
    }

    fn normalize_selected_resource_surfaces(
        &self,
        selected_resource_surfaces: Vec<UnifyDirectResourceSurface>,
    ) -> Vec<UnifyDirectResourceSurface> {
        let mut normalized = selected_resource_surfaces
            .into_iter()
            .map(|surface| UnifyDirectResourceSurface {
                server_id: surface.server_id.trim().to_string(),
                resource_uri: surface.resource_uri.trim().to_string(),
            })
            .filter(|surface| !surface.server_id.is_empty() && !surface.resource_uri.is_empty())
            .collect::<Vec<_>>();
        normalized.sort_by(|left, right| {
            left.server_id
                .cmp(&right.server_id)
                .then(left.resource_uri.cmp(&right.resource_uri))
        });
        normalized.dedup();
        normalized
    }

    fn normalize_selected_template_surfaces(
        &self,
        selected_template_surfaces: Vec<UnifyDirectTemplateSurface>,
    ) -> Vec<UnifyDirectTemplateSurface> {
        let mut normalized = selected_template_surfaces
            .into_iter()
            .map(|surface| UnifyDirectTemplateSurface {
                server_id: surface.server_id.trim().to_string(),
                uri_template: surface.uri_template.trim().to_string(),
            })
            .filter(|surface| !surface.server_id.is_empty() && !surface.uri_template.is_empty())
            .collect::<Vec<_>>();
        normalized.sort_by(|left, right| {
            left.server_id
                .cmp(&right.server_id)
                .then(left.uri_template.cmp(&right.uri_template))
        });
        normalized.dedup();
        normalized
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
