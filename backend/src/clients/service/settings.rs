use super::ClientConfigService;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    CapabilitySource, ClientCapabilityConfig, ClientCapabilityConfigState, ClientConfigFileParse, ContainerType,
    FormatRule, UnifyDirectCapabilityIds, UnifyDirectExposureConfig, UnifyDirectExposureDiagnostics,
    UnifyDirectExposureIntent, UnifyDirectPromptSurface, UnifyDirectPromptSurfaceDiagnostic,
    UnifyDirectResourceSurface, UnifyDirectResourceSurfaceDiagnostic, UnifyDirectTemplateSurface,
    UnifyDirectTemplateSurfaceDiagnostic, UnifyDirectToolSurface, UnifyDirectToolSurfaceDiagnostic,
};
use crate::clients::service::core::{ClientStateRow, RuntimeClientMetadata};
use crate::common::profile::{ProfileRole, ProfileType};
use crate::config::database::Database;
use crate::config::models::Profile;
use crate::core::proxy::server::{ClientContext, ClientIdentitySource, ClientTransport};
use crate::system::paths::get_path_service;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;

const VALID_TRANSPORTS: &[&str] = &["auto", "sse", "stdio", "streamable_http"];
const VALID_CONNECTION_MODES: &[&str] = &["local_config_detected", "remote_http", "manual"];

fn canonical_record_transport(transport: &str) -> Option<&'static str> {
    match transport.trim() {
        "streamable_http" => Some("streamable_http"),
        "sse" => Some("sse"),
        "stdio" => Some("stdio"),
        _ => None,
    }
}

fn sanitize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone, Default)]
struct ResolvedUnifyDirectExposureState {
    intent: UnifyDirectExposureIntent,
    config: UnifyDirectExposureConfig,
    diagnostics: UnifyDirectExposureDiagnostics,
}

#[derive(Debug, Clone, Default)]
struct UnifyDirectExposureInventory {
    tools: HashMap<String, HashSet<String>>,
    prompts: HashMap<String, HashSet<String>>,
    resources: HashMap<String, HashSet<String>>,
    templates: HashMap<String, HashSet<String>>,
    tool_ids: HashMap<String, UnifyDirectToolSurface>,
    prompt_ids: HashMap<String, UnifyDirectPromptSurface>,
    resource_ids: HashMap<String, UnifyDirectResourceSurface>,
    template_ids: HashMap<String, UnifyDirectTemplateSurface>,
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
    pub config_file_parse: Option<ClientConfigFileParse>,
    pub clear_config_file_parse: bool,
    pub transports: Option<HashMap<String, FormatRule>>,
    pub clear_transports: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveClientSettingsResult {
    pub old_effective_mode: String,
    pub new_effective_mode: String,
    pub display_name_source: &'static str,
    pub approval_status_source: &'static str,
    pub connection_mode_source: &'static str,
}

impl ActiveClientSettingsResult {
    pub fn effective_mode_changed(&self) -> bool {
        self.old_effective_mode != self.new_effective_mode
    }
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

fn serialize_json<T: serde::Serialize>(value: &T) -> ConfigResult<String> {
    serde_json::to_string(value).map_err(|err| ConfigError::DataAccessError(err.to_string()))
}

fn retain_known_capability_ids<V>(
    ids: Vec<String>,
    valid_ids: &HashMap<String, V>,
) -> Vec<String> {
    ids.into_iter().filter(|id| valid_ids.contains_key(id)).collect()
}

fn can_apply_first_initialize_observation(state: &ClientStateRow) -> ConfigResult<bool> {
    if state.template_identifier().is_some() || state.governance_kind().as_str() != "passive" {
        return Ok(false);
    }

    if state
        .client_version
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        || state.transport.as_deref().is_some_and(|value| {
            let trimmed = value.trim();
            !trimmed.is_empty() && trimmed != "auto"
        })
        || !state.parsed_transports()?.is_empty()
    {
        return Ok(false);
    }

    let metadata = state.runtime_client_metadata();
    Ok(metadata.description.is_none()
        && metadata.homepage_url.is_none()
        && metadata.docs_url.is_none()
        && metadata.support_url.is_none()
        && metadata.logo_url.is_none()
        && metadata.category.is_none())
}

impl ClientConfigService {
    pub async fn persist_handshake_observation(
        &self,
        identifier: &str,
        observed_name: Option<&str>,
        client_version: Option<&str>,
        transport: Option<&str>,
        connection_mode: Option<&str>,
        description: Option<&str>,
        homepage_url: Option<&str>,
        logo_url: Option<&str>,
    ) -> ConfigResult<()> {
        let display_name = sanitize_optional(observed_name);
        let client_version = sanitize_optional(client_version);
        let transport = sanitize_optional(transport);
        let connection_mode = sanitize_optional(connection_mode);
        let description = sanitize_optional(description);
        let homepage_url = sanitize_optional(homepage_url);
        let logo_url = sanitize_optional(logo_url);

        if [
            display_name.as_deref(),
            client_version.as_deref(),
            transport.as_deref(),
            connection_mode.as_deref(),
            description.as_deref(),
            homepage_url.as_deref(),
            logo_url.as_deref(),
        ]
        .iter()
        .all(|value| value.is_none())
        {
            return Ok(());
        }

        let observed_name = display_name.as_deref().unwrap_or(identifier);
        let existing_state = if let Some(state) = self.fetch_state(identifier).await? {
            if !can_apply_first_initialize_observation(&state)? {
                return Ok(());
            }
            state
        } else {
            let platform = crate::system::paths::PathService::get_current_platform();
            if self.template_source.get_template(identifier, platform).await?.is_some() {
                return Ok(());
            }
            self.ensure_passive_observed_row(identifier, observed_name, None)
                .await?
        };

        if let Some(display_name) = display_name.as_deref() {
            self.update_client_names(identifier, display_name).await?;
        }

        if let Some(client_version) = client_version.as_deref() {
            self.update_client_version(identifier, client_version).await?;
        }

        if let Some(transport) = transport.as_deref() {
            self.update_transport(identifier, transport).await?;
        }

        if let Some(connection_mode) = connection_mode.as_deref() {
            self.update_runtime_target(identifier, None, Some(connection_mode), false)
                .await?;
        }

        if description.is_some() || homepage_url.is_some() || logo_url.is_some() {
            let existing_metadata = existing_state.runtime_client_metadata();
            let next_metadata = RuntimeClientMetadata {
                description: description.or(existing_metadata.description),
                homepage_url: homepage_url.or(existing_metadata.homepage_url),
                docs_url: existing_metadata.docs_url,
                support_url: existing_metadata.support_url,
                logo_url: logo_url.or(existing_metadata.logo_url),
                category: existing_metadata.category,
            };
            self.update_runtime_client_metadata(identifier, &next_metadata, false)
                .await?;
        }

        if let Some(observed_transport) = transport.as_deref() {
            self.upsert_observed_transport_support(identifier, observed_transport)
                .await?;
        }

        Ok(())
    }

    async fn upsert_observed_transport_support(
        &self,
        identifier: &str,
        observed_transport: &str,
    ) -> ConfigResult<()> {
        let Some(normalized_transport) = canonical_record_transport(observed_transport) else {
            if observed_transport.trim().is_empty() {
                return Ok(());
            }
            return Err(ConfigError::DataAccessError(format!(
                "Invalid observed transport '{}'; expected canonical transport key",
                observed_transport.trim()
            )));
        };

        let existing_raw: Option<String> = sqlx::query_scalar("SELECT transports FROM client WHERE identifier = ?")
            .bind(identifier)
            .fetch_optional(&*self.db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
            .flatten();

        let mut transports = existing_raw
            .as_deref()
            .map(serde_json::from_str::<HashMap<String, FormatRule>>)
            .transpose()
            .map_err(|err| ConfigError::DataAccessError(format!("Failed to parse transports: {}", err)))?
            .unwrap_or_default();

        if transports.contains_key(normalized_transport) {
            return Ok(());
        }

        transports.insert(
            normalized_transport.to_string(),
            FormatRule {
                selected: Some(true),
                ..FormatRule::default()
            },
        );

        self.update_transports(identifier, Some(&transports), false).await
    }

    async fn resolve_effective_mode_from_explicit(
        &self,
        explicit_mode: Option<&str>,
    ) -> ConfigResult<String> {
        match explicit_mode.map(str::trim).filter(|mode| !mode.is_empty()) {
            Some(mode) => Ok(mode.to_string()),
            None => crate::config::client::init::resolve_default_client_config_mode(&self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string())),
        }
    }

    pub async fn get_effective_config_mode(
        &self,
        identifier: &str,
    ) -> ConfigResult<String> {
        let explicit = self.fetch_state(identifier).await?.and_then(|state| state.config_mode);
        self.resolve_effective_mode_from_explicit(explicit.as_deref()).await
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
        .map(|_| ())
    }

    pub async fn set_active_client_settings(
        &self,
        identifier: &str,
        update: ActiveClientSettingsUpdate,
    ) -> ConfigResult<ActiveClientSettingsResult> {
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

        let trimmed_display_name = update
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let (name, display_name_source): (String, &'static str) = match trimmed_display_name {
            Some(value) => (value.to_string(), "provided"),
            None => (self.resolve_client_name(identifier).await?, "stored"),
        };
        let existing_state = self.fetch_state(identifier).await?;
        let old_effective_mode = self
            .resolve_effective_mode_from_explicit(
                existing_state.as_ref().and_then(|state| state.config_mode.as_deref()),
            )
            .await?;
        let requested_config_mode = update.config_mode.clone();

        let raw_config_path = update.config_path.as_deref().map(str::trim);
        let normalized_config_path = raw_config_path.filter(|value| !value.is_empty()).map(str::to_string);

        let (resolved_connection_mode, connection_mode_source): (Option<String>, &'static str) =
            if let Some(mode) = update.connection_mode.clone() {
                (Some(mode), "provided")
            } else {
                match raw_config_path {
                    Some("") => (Some("manual".to_string()), "derived"),
                    Some(_) => (Some("local_config_detected".to_string()), "derived"),
                    None => (None, "stored"),
                }
            };

        self.validate_runtime_target_input(resolved_connection_mode.as_deref(), normalized_config_path.as_deref())
            .await?;

        let effective_parse_for_validation = if let Some(parse) = update.config_file_parse.clone() {
            Some(parse)
        } else if update.clear_config_file_parse {
            existing_state
                .as_ref()
                .and_then(|state| state.legacy_config_file_parse().ok().flatten())
        } else {
            existing_state
                .as_ref()
                .and_then(|state| state.config_file_parse_override().ok().flatten())
                .or_else(|| {
                    existing_state
                        .as_ref()
                        .and_then(|state| state.legacy_config_file_parse().ok().flatten())
                })
        };
        let validation_path = normalized_config_path
            .as_deref()
            .or_else(|| existing_state.as_ref().and_then(|state| state.config_path()));

        if matches!(resolved_connection_mode.as_deref(), Some("local_config_detected")) {
            if let (Some(path), Some(parse)) = (validation_path, effective_parse_for_validation.as_ref()) {
                self.validate_config_file_parse_rule(path, parse).await?;
            }
        }

        let (approval_status, approval_status_source): (String, &'static str) = existing_state
            .as_ref()
            .map(|state| (state.approval_status().to_string(), "stored"))
            .unwrap_or_else(|| ("approved".to_string(), "default"));

        self.ensure_active_state_row_with_name(identifier, &name, Some(&approval_status))
            .await?;

        self.update_client_names(identifier, &name).await?;

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
                true,
            )
            .await?;
        }

        if update.description.is_some()
            || update.homepage_url.is_some()
            || update.docs_url.is_some()
            || update.support_url.is_some()
            || update.logo_url.is_some()
        {
            let existing_metadata = existing_state
                .as_ref()
                .map(|state| state.runtime_client_metadata())
                .unwrap_or_default();

            tracing::debug!(
                client = %identifier,
                update_logo_url = ?update.logo_url,
                existing_logo_url = ?existing_metadata.logo_url,
                "Merging runtime metadata"
            );

            let next_metadata = RuntimeClientMetadata {
                description: update.description.or(existing_metadata.description),
                homepage_url: update.homepage_url.or(existing_metadata.homepage_url),
                docs_url: update.docs_url.or(existing_metadata.docs_url),
                support_url: update.support_url.or(existing_metadata.support_url),
                logo_url: update.logo_url.or(existing_metadata.logo_url),
                category: existing_metadata.category,
            };

            tracing::debug!(
                client = %identifier,
                merged_logo_url = ?next_metadata.logo_url,
                "Merged runtime metadata, calling update_runtime_client_metadata"
            );

            self.update_runtime_client_metadata(identifier, &next_metadata, true)
                .await?;
        }

        if update.clear_config_file_parse || update.config_file_parse.is_some() {
            self.update_config_file_parse(
                identifier,
                update.config_file_parse.as_ref(),
                update.clear_config_file_parse,
            )
            .await?;
        }

        if update.clear_transports || update.transports.is_some() {
            self.update_transports(identifier, update.transports.as_ref(), update.clear_transports)
                .await?;
        }

        let new_effective_mode = self
            .resolve_effective_mode_from_explicit(
                requested_config_mode.as_deref().or(Some(old_effective_mode.as_str())),
            )
            .await?;

        tracing::info!(client = %identifier, "set_active_client_settings: complete");
        Ok(ActiveClientSettingsResult {
            old_effective_mode,
            new_effective_mode,
            display_name_source,
            approval_status_source,
            connection_mode_source,
        })
    }

    async fn update_client_names(
        &self,
        identifier: &str,
        name: &str,
    ) -> ConfigResult<()> {
        tracing::debug!(client = %identifier, name = %name, "Updating client name and display name");

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?,
                display_name = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(name)
        .bind(name)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

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
        promote_active: bool,
    ) -> ConfigResult<()> {
        let governance_kind = if promote_active { Some("active") } else { None };
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
                attachment_state = CASE
                    WHEN ? IS NOT NULL AND NULLIF(?, '') IS NOT NULL THEN 'detached'
                    WHEN ? IN ('manual', 'remote_http') THEN 'not_applicable'
                    WHEN ? = 'local_config_detected' AND NULLIF(?, '') IS NOT NULL THEN 'detached'
                    ELSE attachment_state
                END,
                governance_kind = COALESCE(?, governance_kind),
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
        .bind(config_path)
        .bind(config_path)
        .bind(connection_mode)
        .bind(connection_mode)
        .bind(config_path)
        .bind(governance_kind)
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
        promote_active: bool,
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

        let governance_kind = if promote_active { Some("active") } else { None };

        sqlx::query(
            r#"
            UPDATE client
            SET approval_metadata = ?,
                governance_kind = COALESCE(?, governance_kind),
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(serde_json::to_string(&payload).map_err(|e| ConfigError::DataAccessError(e.to_string()))?)
        .bind(governance_kind)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| ConfigError::DataAccessError(e.to_string()))?;

        Ok(())
    }

    async fn update_config_file_parse(
        &self,
        identifier: &str,
        config_file_parse: Option<&ClientConfigFileParse>,
        clear_override: bool,
    ) -> ConfigResult<()> {
        let existing_state = self.fetch_state(identifier).await?;
        let serialized_override = if clear_override {
            None
        } else {
            config_file_parse
                .map(|value| serde_json::to_string(value).map_err(|err| ConfigError::DataAccessError(err.to_string())))
                .transpose()?
        };

        let effective_parse = if let Some(value) = config_file_parse {
            Some(value.clone())
        } else if clear_override {
            existing_state
                .as_ref()
                .and_then(|state| state.legacy_config_file_parse().ok().flatten())
        } else {
            None
        };

        let config_format = effective_parse.as_ref().map(|value| value.format.as_str().to_string());
        let container_type = effective_parse.as_ref().map(|value| match value.container_type {
            ContainerType::Array => "array".to_string(),
            ContainerType::ObjectMap => "object".to_string(),
        });
        let container_keys = effective_parse
            .as_ref()
            .map(|value| {
                serde_json::to_string(&value.container_keys)
                    .map_err(|err| ConfigError::DataAccessError(err.to_string()))
            })
            .transpose()?;

        sqlx::query(
            r#"
            UPDATE client
            SET config_file_parse = ?,
                config_format = COALESCE(?, config_format),
                container_type = COALESCE(?, container_type),
                container_keys = COALESCE(?, container_keys),
                governance_kind = 'active',
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(serialized_override)
        .bind(config_format)
        .bind(container_type)
        .bind(container_keys)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(())
    }

    async fn update_transports(
        &self,
        identifier: &str,
        transports: Option<&HashMap<String, FormatRule>>,
        clear_override: bool,
    ) -> ConfigResult<()> {
        if clear_override {
            sqlx::query(
                r#"
                UPDATE client
                SET transports = NULL,
                    updated_at = CURRENT_TIMESTAMP
                WHERE identifier = ?
                "#,
            )
            .bind(identifier)
            .execute(&*self.db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
            return Ok(());
        }

        let Some(rules) = transports else {
            return Ok(());
        };

        for transport in rules.keys() {
            if canonical_record_transport(transport).is_none() {
                return Err(ConfigError::DataAccessError(format!(
                    "Invalid transport key '{}'; expected canonical transport key",
                    transport
                )));
            }
        }

        let serialized = serde_json::to_string(rules).map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        sqlx::query(
            r#"
            UPDATE client
            SET transports = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(serialized)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

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
            Some(serialize_json(&selected_profile_ids)?)
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
        let custom_profile_missing = self
            .resolve_custom_profile_missing(
                capability_config.capability_source,
                capability_config.custom_profile_id.as_deref(),
            )
            .await?;
        let raw_unify_direct_exposure = state.unify_direct_exposure_intent()?;
        let resolved = self
            .resolve_unify_direct_exposure_intent(identifier, &capability_config, &raw_unify_direct_exposure)
            .await?;

        Ok(Some(ClientCapabilityConfigState {
            capability_config,
            custom_profile_missing,
            unify_direct_exposure_intent: resolved.intent,
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
        unify_direct_exposure_update: Option<UnifyDirectExposureIntent>,
    ) -> ConfigResult<(ClientCapabilityConfigState, bool)> {
        let visibility_service = crate::core::profile::visibility::ProfileVisibilityService::new(
            Some(Arc::new(Database {
                pool: self.db_pool.as_ref().clone(),
                path: PathBuf::new(),
            })),
            None,
        );

        let build_client_context =
            |config_mode: &str, unify_direct_exposure: Option<UnifyDirectExposureConfig>| ClientContext {
                client_id: identifier.to_string(),
                session_id: None,
                profile_id: None,
                config_mode: Some(config_mode.to_string()),
                unify_workspace: unify_direct_exposure,
                surface_fingerprint: None,
                transport: ClientTransport::Other,
                source: ClientIdentitySource::ManagedQuery,
                observed_client_info: None,
            };

        let old_state = self.get_capability_config_state(identifier).await?;
        let effective_mode = self.get_effective_config_mode(identifier).await?;
        let resolve_unify_workspace = |state: &ClientCapabilityConfigState| {
            (effective_mode == "unify").then(|| state.unify_direct_exposure.clone())
        };
        let old_fingerprint = if let Some(state) = old_state.as_ref() {
            let context = build_client_context(&effective_mode, resolve_unify_workspace(state));
            Some(
                visibility_service
                    .resolve_snapshot_for_client(&context)
                    .await
                    .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
                    .surface_fingerprint,
            )
        } else {
            None
        };

        let state = self
            .set_capability_config_state(
                identifier,
                capability_source,
                selected_profile_ids,
                unify_direct_exposure_update,
            )
            .await?;

        let new_context = build_client_context(&effective_mode, resolve_unify_workspace(&state));
        let new_fingerprint = visibility_service
            .resolve_snapshot_for_client(&new_context)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
            .surface_fingerprint;

        if let Some(ref fingerprint) = old_fingerprint {
            if let Ok(cache_manager) = crate::core::cache::RedbCacheManager::global() {
                match cache_manager.invalidate_by_surface_fingerprint(fingerprint).await {
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

        let has_visible_direct_surface = !state.unify_direct_exposure.selected_tool_surfaces.is_empty()
            || !state.unify_direct_exposure.selected_prompt_surfaces.is_empty()
            || !state.unify_direct_exposure.selected_resource_surfaces.is_empty()
            || !state.unify_direct_exposure.selected_template_surfaces.is_empty();
        let visible_surface_changed = old_fingerprint
            .as_ref()
            .map(|fingerprint| fingerprint != &new_fingerprint)
            .unwrap_or(has_visible_direct_surface);

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
            if self.get_effective_config_mode(&identifier).await? != "unify" {
                continue;
            }

            let capability_config = row.capability_config()?;
            let raw_unify_direct_exposure = row.unify_direct_exposure_intent()?;
            let resolved = self
                .resolve_unify_direct_exposure_intent(&identifier, &capability_config, &raw_unify_direct_exposure)
                .await?;
            if !unify_direct_exposure_references_server(&resolved.config, server_id) {
                continue;
            }

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
        unify_direct_exposure_update: Option<UnifyDirectExposureIntent>,
    ) -> ConfigResult<ClientCapabilityConfigState> {
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await?;

        let selected_profile_ids = self.normalize_selected_profile_ids(capability_source, selected_profile_ids)?;
        self.validate_selected_profile_ids(&selected_profile_ids).await?;

        let custom_profile_id = match capability_source {
            CapabilitySource::Activated | CapabilitySource::Profiles => None,
            CapabilitySource::Custom => Some(self.ensure_custom_profile(identifier).await?),
        };
        let custom_profile_missing = self
            .resolve_custom_profile_missing(capability_source, custom_profile_id.as_deref())
            .await?;
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
            .map(|row| row.unify_direct_exposure_intent())
            .transpose()?
            .unwrap_or_default();
        let requested_unify_direct_exposure = unify_direct_exposure_update.unwrap_or(existing_unify_direct_exposure);
        let resolved_unify_direct_exposure = self
            .resolve_unify_direct_exposure_intent(
                identifier,
                &ClientCapabilityConfig {
                    capability_source,
                    selected_profile_ids: selected_profile_ids.clone(),
                    custom_profile_id: custom_profile_id.clone(),
                },
                &requested_unify_direct_exposure,
            )
            .await?;
        let unify_direct_exposure_intent_json =
            if resolved_unify_direct_exposure.intent == UnifyDirectExposureIntent::default() {
                None
            } else {
                Some(serialize_json(&resolved_unify_direct_exposure.intent)?)
            };

        sqlx::query(
            r#"
            UPDATE client
            SET capability_source = ?,
                selected_profile_ids = ?,
                custom_profile_id = ?,
                unify_direct_exposure_intent = ?,
                governance_kind = 'active',
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(capability_source.as_str())
        .bind(selected_profile_ids_json)
        .bind(custom_profile_id.as_deref())
        .bind(unify_direct_exposure_intent_json)
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
            custom_profile_missing,
            unify_direct_exposure_intent: resolved_unify_direct_exposure.intent,
            unify_direct_exposure: resolved_unify_direct_exposure.config,
            unify_direct_exposure_diagnostics: resolved_unify_direct_exposure.diagnostics,
        })
    }

    async fn resolve_custom_profile_missing(
        &self,
        capability_source: CapabilitySource,
        custom_profile_id: Option<&str>,
    ) -> ConfigResult<bool> {
        if capability_source != CapabilitySource::Custom {
            return Ok(false);
        }

        let Some(profile_id) = custom_profile_id.filter(|value| !value.trim().is_empty()) else {
            return Ok(true);
        };

        Ok(crate::config::profile::get_profile(self.db_pool.as_ref(), profile_id)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?
            .is_none())
    }

    async fn resolve_unify_direct_exposure_intent(
        &self,
        identifier: &str,
        capability_config: &ClientCapabilityConfig,
        intent: &UnifyDirectExposureIntent,
    ) -> ConfigResult<ResolvedUnifyDirectExposureState> {
        let inventory = self.load_unify_direct_exposure_inventory().await?;
        let visible_server_ids = self
            .resolve_visible_server_ids_for_unify_direct_exposure(identifier, capability_config)
            .await?;
        let mut diagnostics = UnifyDirectExposureDiagnostics::default();

        let requested_server_ids = self.normalize_selected_server_ids_for_unify(intent.server_ids.clone());
        let selected_server_ids = requested_server_ids
            .iter()
            .cloned()
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

        let server_level_tool_surfaces = if intent.route_mode == crate::clients::models::UnifyRouteMode::ServerLevel {
            self.materialize_tool_surfaces_for_servers(&selected_server_ids, &inventory)
        } else {
            Vec::new()
        };
        let server_level_prompt_surfaces = if intent.route_mode == crate::clients::models::UnifyRouteMode::ServerLevel {
            self.materialize_prompt_surfaces_for_servers(&selected_server_ids, &inventory)
        } else {
            Vec::new()
        };
        let server_level_resource_surfaces = if intent.route_mode == crate::clients::models::UnifyRouteMode::ServerLevel
        {
            self.materialize_resource_surfaces_for_servers(&selected_server_ids, &inventory)
        } else {
            Vec::new()
        };
        let server_level_template_surfaces = if intent.route_mode == crate::clients::models::UnifyRouteMode::ServerLevel
        {
            self.materialize_template_surfaces_for_servers(&selected_server_ids, &inventory)
        } else {
            Vec::new()
        };

        let capability_level_tool_surfaces = if intent.route_mode
            == crate::clients::models::UnifyRouteMode::CapabilityLevel
        {
            self.resolve_tool_surfaces_for_capability_ids(&intent.capability_ids.tool_ids, &inventory, &mut diagnostics)
        } else {
            Vec::new()
        };
        let capability_level_prompt_surfaces =
            if intent.route_mode == crate::clients::models::UnifyRouteMode::CapabilityLevel {
                self.resolve_prompt_surfaces_for_capability_ids(
                    &intent.capability_ids.prompt_ids,
                    &inventory,
                    &mut diagnostics,
                )
            } else {
                Vec::new()
            };
        let capability_level_resource_surfaces =
            if intent.route_mode == crate::clients::models::UnifyRouteMode::CapabilityLevel {
                self.resolve_resource_surfaces_for_capability_ids(
                    &intent.capability_ids.resource_ids,
                    &inventory,
                    &mut diagnostics,
                )
            } else {
                Vec::new()
            };
        let capability_level_template_surfaces =
            if intent.route_mode == crate::clients::models::UnifyRouteMode::CapabilityLevel {
                self.resolve_template_surfaces_for_capability_ids(
                    &intent.capability_ids.template_ids,
                    &inventory,
                    &mut diagnostics,
                )
            } else {
                Vec::new()
            };

        let selected_tool_surfaces = self.normalize_selected_tool_surfaces(
            capability_level_tool_surfaces
                .into_iter()
                .chain(server_level_tool_surfaces)
                .collect(),
        );
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
        let selected_prompt_surfaces = self.normalize_selected_prompt_surfaces(
            capability_level_prompt_surfaces
                .into_iter()
                .chain(server_level_prompt_surfaces)
                .collect(),
        );
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
        let selected_resource_surfaces = self.normalize_selected_resource_surfaces(
            capability_level_resource_surfaces
                .into_iter()
                .chain(server_level_resource_surfaces)
                .collect(),
        );
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
        let selected_template_surfaces = self.normalize_selected_template_surfaces(
            capability_level_template_surfaces
                .into_iter()
                .chain(server_level_template_surfaces)
                .collect(),
        );
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
        diagnostics.invalid_capability_ids.sort();
        diagnostics.invalid_capability_ids.dedup();

        let resolved_intent = UnifyDirectExposureIntent {
            route_mode: intent.route_mode,
            server_ids: if intent.route_mode == crate::clients::models::UnifyRouteMode::ServerLevel {
                requested_server_ids
            } else {
                Vec::new()
            },
            capability_ids: if intent.route_mode == crate::clients::models::UnifyRouteMode::CapabilityLevel {
                self.resolve_valid_unify_direct_capability_ids(&intent.capability_ids, &inventory)
            } else {
                UnifyDirectCapabilityIds::default()
            },
        };

        Ok(ResolvedUnifyDirectExposureState {
            intent: resolved_intent,
            config: UnifyDirectExposureConfig {
                route_mode: intent.route_mode,
                selected_server_ids: if intent.route_mode == crate::clients::models::UnifyRouteMode::ServerLevel {
                    selected_server_ids
                } else {
                    Vec::new()
                },
                selected_tool_surfaces,
                selected_prompt_surfaces,
                selected_resource_surfaces,
                selected_template_surfaces,
            },
            diagnostics,
        })
    }

    async fn load_unify_direct_exposure_inventory(&self) -> ConfigResult<UnifyDirectExposureInventory> {
        let tool_rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, st.tool_name, st.unique_name
            FROM server_config sc
            LEFT JOIN server_tools st ON st.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, st.tool_name
            "#,
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        let prompt_rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, sp.prompt_name, sp.unique_name
            FROM server_config sc
            LEFT JOIN server_prompts sp ON sp.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, sp.prompt_name
            "#,
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        let resource_rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, sr.resource_uri, sr.unique_uri
            FROM server_config sc
            LEFT JOIN server_resources sr ON sr.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, sr.resource_uri
            "#,
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        let template_rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, srt.uri_template, srt.unique_name
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
        for (server_id, tool_name, unique_name) in tool_rows {
            let entry = inventory.tools.entry(server_id.clone()).or_default();
            if let Some(tool_name) = tool_name {
                entry.insert(tool_name.clone());
                if let Some(unique_name) = unique_name {
                    inventory
                        .tool_ids
                        .insert(unique_name, UnifyDirectToolSurface { server_id, tool_name });
                }
            }
        }
        for (server_id, prompt_name, unique_name) in prompt_rows {
            let entry = inventory.prompts.entry(server_id.clone()).or_default();
            if let Some(prompt_name) = prompt_name {
                entry.insert(prompt_name.clone());
                if let Some(unique_name) = unique_name {
                    inventory
                        .prompt_ids
                        .insert(unique_name, UnifyDirectPromptSurface { server_id, prompt_name });
                }
            }
        }
        for (server_id, resource_uri, unique_uri) in resource_rows {
            let entry = inventory.resources.entry(server_id.clone()).or_default();
            if let Some(resource_uri) = resource_uri {
                entry.insert(resource_uri.clone());
                if let Some(unique_uri) = unique_uri {
                    inventory.resource_ids.insert(
                        unique_uri,
                        UnifyDirectResourceSurface {
                            server_id,
                            resource_uri,
                        },
                    );
                }
            }
        }
        for (server_id, uri_template, unique_name) in template_rows {
            let entry = inventory.templates.entry(server_id.clone()).or_default();
            if let Some(uri_template) = uri_template {
                entry.insert(uri_template.clone());
                if let Some(unique_name) = unique_name {
                    inventory.template_ids.insert(
                        unique_name,
                        UnifyDirectTemplateSurface {
                            server_id,
                            uri_template,
                        },
                    );
                }
            }
        }

        Ok(inventory)
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

    fn resolve_tool_surfaces_for_capability_ids(
        &self,
        capability_ids: &[String],
        inventory: &UnifyDirectExposureInventory,
        diagnostics: &mut UnifyDirectExposureDiagnostics,
    ) -> Vec<UnifyDirectToolSurface> {
        self.normalize_unify_direct_ids(capability_ids.to_vec())
            .into_iter()
            .filter_map(|capability_id| match inventory.tool_ids.get(&capability_id) {
                Some(surface) => Some(surface.clone()),
                None => {
                    diagnostics.invalid_capability_ids.push(capability_id);
                    None
                }
            })
            .collect()
    }

    fn resolve_prompt_surfaces_for_capability_ids(
        &self,
        capability_ids: &[String],
        inventory: &UnifyDirectExposureInventory,
        diagnostics: &mut UnifyDirectExposureDiagnostics,
    ) -> Vec<UnifyDirectPromptSurface> {
        self.normalize_unify_direct_ids(capability_ids.to_vec())
            .into_iter()
            .filter_map(|capability_id| match inventory.prompt_ids.get(&capability_id) {
                Some(surface) => Some(surface.clone()),
                None => {
                    diagnostics.invalid_capability_ids.push(capability_id);
                    None
                }
            })
            .collect()
    }

    fn resolve_resource_surfaces_for_capability_ids(
        &self,
        capability_ids: &[String],
        inventory: &UnifyDirectExposureInventory,
        diagnostics: &mut UnifyDirectExposureDiagnostics,
    ) -> Vec<UnifyDirectResourceSurface> {
        self.normalize_unify_direct_ids(capability_ids.to_vec())
            .into_iter()
            .filter_map(|capability_id| match inventory.resource_ids.get(&capability_id) {
                Some(surface) => Some(surface.clone()),
                None => {
                    diagnostics.invalid_capability_ids.push(capability_id);
                    None
                }
            })
            .collect()
    }

    fn resolve_template_surfaces_for_capability_ids(
        &self,
        capability_ids: &[String],
        inventory: &UnifyDirectExposureInventory,
        diagnostics: &mut UnifyDirectExposureDiagnostics,
    ) -> Vec<UnifyDirectTemplateSurface> {
        self.normalize_unify_direct_ids(capability_ids.to_vec())
            .into_iter()
            .filter_map(|capability_id| match inventory.template_ids.get(&capability_id) {
                Some(surface) => Some(surface.clone()),
                None => {
                    diagnostics.invalid_capability_ids.push(capability_id);
                    None
                }
            })
            .collect()
    }

    fn materialize_tool_surfaces_for_servers(
        &self,
        server_ids: &[String],
        inventory: &UnifyDirectExposureInventory,
    ) -> Vec<UnifyDirectToolSurface> {
        server_ids
            .iter()
            .flat_map(|server_id| {
                inventory.tools.get(server_id).into_iter().flat_map(|tool_names| {
                    tool_names.iter().map(|tool_name| UnifyDirectToolSurface {
                        server_id: server_id.clone(),
                        tool_name: tool_name.clone(),
                    })
                })
            })
            .collect()
    }

    fn materialize_prompt_surfaces_for_servers(
        &self,
        server_ids: &[String],
        inventory: &UnifyDirectExposureInventory,
    ) -> Vec<UnifyDirectPromptSurface> {
        server_ids
            .iter()
            .flat_map(|server_id| {
                inventory.prompts.get(server_id).into_iter().flat_map(|prompt_names| {
                    prompt_names.iter().map(|prompt_name| UnifyDirectPromptSurface {
                        server_id: server_id.clone(),
                        prompt_name: prompt_name.clone(),
                    })
                })
            })
            .collect()
    }

    fn materialize_resource_surfaces_for_servers(
        &self,
        server_ids: &[String],
        inventory: &UnifyDirectExposureInventory,
    ) -> Vec<UnifyDirectResourceSurface> {
        server_ids
            .iter()
            .flat_map(|server_id| {
                inventory
                    .resources
                    .get(server_id)
                    .into_iter()
                    .flat_map(|resource_uris| {
                        resource_uris.iter().map(|resource_uri| UnifyDirectResourceSurface {
                            server_id: server_id.clone(),
                            resource_uri: resource_uri.clone(),
                        })
                    })
            })
            .collect()
    }

    fn materialize_template_surfaces_for_servers(
        &self,
        server_ids: &[String],
        inventory: &UnifyDirectExposureInventory,
    ) -> Vec<UnifyDirectTemplateSurface> {
        server_ids
            .iter()
            .flat_map(|server_id| {
                inventory
                    .templates
                    .get(server_id)
                    .into_iter()
                    .flat_map(|uri_templates| {
                        uri_templates.iter().map(|uri_template| UnifyDirectTemplateSurface {
                            server_id: server_id.clone(),
                            uri_template: uri_template.clone(),
                        })
                    })
            })
            .collect()
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

    fn normalize_unify_direct_ids(
        &self,
        ids: Vec<String>,
    ) -> Vec<String> {
        let mut normalized = ids
            .into_iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    fn normalize_unify_direct_capability_ids(
        &self,
        capability_ids: UnifyDirectCapabilityIds,
    ) -> UnifyDirectCapabilityIds {
        UnifyDirectCapabilityIds {
            tool_ids: self.normalize_unify_direct_ids(capability_ids.tool_ids),
            prompt_ids: self.normalize_unify_direct_ids(capability_ids.prompt_ids),
            resource_ids: self.normalize_unify_direct_ids(capability_ids.resource_ids),
            template_ids: self.normalize_unify_direct_ids(capability_ids.template_ids),
        }
    }

    fn resolve_valid_unify_direct_capability_ids(
        &self,
        capability_ids: &UnifyDirectCapabilityIds,
        inventory: &UnifyDirectExposureInventory,
    ) -> UnifyDirectCapabilityIds {
        let capability_ids = self.normalize_unify_direct_capability_ids(capability_ids.clone());
        UnifyDirectCapabilityIds {
            tool_ids: retain_known_capability_ids(capability_ids.tool_ids, &inventory.tool_ids),
            prompt_ids: retain_known_capability_ids(capability_ids.prompt_ids, &inventory.prompt_ids),
            resource_ids: retain_known_capability_ids(capability_ids.resource_ids, &inventory.resource_ids),
            template_ids: retain_known_capability_ids(capability_ids.template_ids, &inventory.template_ids),
        }
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
