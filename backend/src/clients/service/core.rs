use crate::clients::TemplateEngine;
use crate::clients::detector::{ClientDetector, DetectedClient};
use crate::clients::engine::TemplateExecutionResult;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    AttachmentState, BackupPolicySetting, ClientCapabilityConfig, ClientConfigFileParse, ClientConnectionMode,
    ClientGovernanceKind, ClientRenderDefinition, ClientTemplate, ConfigMapping, ConfigMode, FormatRule,
    ManagedEndpointConfig, MergeStrategy, ServerTemplateInput, StorageConfig, StorageKind, TemplateFormat,
};
#[cfg(test)]
use crate::clients::source::FileTemplateSource;
use crate::clients::source::{ClientConfigSource, DbTemplateSource};
use crate::clients::utils::get_nested_value_mut;
use crate::common::constants::{client_headers, profile_keys};
use crate::system::paths::{PathService, get_path_service};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::OpenOptions;

// Generated at build time from the repository's config/client directory
include!(concat!(env!("OUT_DIR"), "/official_templates_generated.rs"));

fn parse_embedded_template(
    file_name: &str,
    contents: &str,
) -> ConfigResult<ClientTemplate> {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| ConfigError::TemplateParseError(format!("Unsupported embedded template: {}", file_name)))?;

    let mut template: ClientTemplate = match ext.as_str() {
        "json" => serde_json::from_str(contents).map_err(|err| {
            ConfigError::TemplateParseError(format!("Failed to parse embedded template {}: {}", file_name, err))
        })?,
        "json5" => json5::from_str(contents).map_err(|err| {
            ConfigError::TemplateParseError(format!("Failed to parse embedded template {}: {}", file_name, err))
        })?,
        "yaml" | "yml" => serde_yaml::from_str(contents).map_err(|err| {
            ConfigError::TemplateParseError(format!("Failed to parse embedded template {}: {}", file_name, err))
        })?,
        "toml" => toml::from_str(contents).map_err(|err| {
            ConfigError::TemplateParseError(format!("Failed to parse embedded template {}: {}", file_name, err))
        })?,
        _ => {
            return Err(ConfigError::TemplateParseError(format!(
                "Unsupported embedded template extension {} ({})",
                ext, file_name
            )));
        }
    };

    if matches!(template.format, TemplateFormat::Json) {
        match ext.as_str() {
            "json5" => template.format = TemplateFormat::Json5,
            "yaml" | "yml" => template.format = TemplateFormat::Yaml,
            "toml" => template.format = TemplateFormat::Toml,
            _ => {}
        }
    }

    if template.identifier.trim().is_empty() {
        return Err(ConfigError::TemplateParseError(format!(
            "Embedded template missing identifier field: {}",
            file_name
        )));
    }

    if template.config_mapping.container_keys.is_empty() {
        return Err(ConfigError::TemplateParseError(format!(
            "Embedded template {} missing config_mapping.container_keys",
            template.identifier
        )));
    }

    if template.detection.is_empty() {
        return Err(ConfigError::TemplateParseError(format!(
            "Embedded template {} missing detection rules",
            template.identifier
        )));
    }

    Ok(template)
}

fn embedded_official_templates() -> ConfigResult<Vec<ClientTemplate>> {
    OFFICIAL_TEMPLATES
        .iter()
        .map(|(file_name, contents)| parse_embedded_template(file_name, contents))
        .collect()
}

#[derive(Debug, Clone, sqlx::FromRow, Default)]
pub struct ClientStateRow {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) id: String,
    pub(super) identifier: String,
    pub(super) name: String,
    pub(super) display_name: Option<String>,
    pub(super) config_path: Option<String>,
    pub(super) config_mode: Option<String>,
    pub(super) transport: Option<String>,
    pub(super) client_version: Option<String>,
    pub(super) backup_policy: Option<String>,
    pub(super) backup_limit: Option<i64>,
    pub(super) capability_source: Option<String>,
    pub(super) governance_kind: Option<String>,
    pub(super) connection_mode: Option<String>,
    pub(super) template_identifier: Option<String>,
    pub(super) selected_profile_ids: Option<String>,
    pub(super) custom_profile_id: Option<String>,
    pub(super) unify_direct_exposure_intent: Option<String>,
    pub(super) approval_status: Option<String>,
    pub(super) attachment_state: Option<String>,
    #[allow(dead_code)]
    pub(super) template_id: Option<String>,
    #[allow(dead_code)]
    pub(super) template_version: Option<String>,
    #[allow(dead_code)]
    pub(super) approval_metadata: Option<String>,
    // Template configuration fields (persisted from template at initialization)
    pub(super) config_format: Option<String>,
    pub(super) protocol_revision: Option<String>,
    pub(super) container_type: Option<String>,
    pub(super) container_keys: Option<String>,
    pub(super) storage_kind: Option<String>,
    pub(super) storage_adapter: Option<String>,
    pub(super) storage_path_strategy: Option<String>,
    pub(super) merge_strategy: Option<String>,
    pub(super) keep_original_config: Option<i64>,
    pub(super) managed_source: Option<String>,
    pub(super) transports: Option<String>,
    pub(super) config_file_parse: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeClientMetadata {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage_url: Option<String>,
    #[serde(default)]
    pub docs_url: Option<String>,
    #[serde(default)]
    pub support_url: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PersistedTemplateConfig {
    pub(super) config_format: Option<String>,
    pub(super) protocol_revision: Option<String>,
    pub(super) container_type: Option<String>,
    pub(super) container_keys: Option<String>,
    pub(super) storage_kind: Option<String>,
    pub(super) storage_adapter: Option<String>,
    pub(super) storage_path_strategy: Option<String>,
    pub(super) merge_strategy: Option<String>,
    pub(super) keep_original_config: Option<i64>,
    pub(super) managed_source: Option<String>,
    pub(super) transports: Option<String>,
    pub(super) config_file_parse: Option<String>,
}

impl PersistedTemplateConfig {
    pub(super) fn from_template(template: &ClientTemplate) -> Self {
        Self {
            config_format: Some(template.format.as_str().to_string()),
            protocol_revision: template.protocol_revision.clone(),
            container_type: Some(
                match template.config_mapping.container_type {
                    crate::clients::models::ContainerType::ObjectMap => "object",
                    crate::clients::models::ContainerType::Array => "array",
                }
                .to_string(),
            ),
            container_keys: serde_json::to_string(&template.config_mapping.container_keys).ok(),
            storage_kind: Some(
                match template.storage.kind {
                    crate::clients::models::StorageKind::File => "file",
                    crate::clients::models::StorageKind::Kv => "kv",
                    crate::clients::models::StorageKind::Custom => "custom",
                }
                .to_string(),
            ),
            storage_adapter: template.storage.adapter.clone(),
            storage_path_strategy: template.storage.path_strategy.clone(),
            merge_strategy: Some(
                match template.config_mapping.merge_strategy {
                    crate::clients::models::MergeStrategy::Replace => "replace",
                    crate::clients::models::MergeStrategy::DeepMerge => "deep_merge",
                }
                .to_string(),
            ),
            keep_original_config: Some(template.config_mapping.keep_original_config as i64),
            managed_source: template.config_mapping.managed_source.clone(),
            transports: if template.config_mapping.format_rules.is_empty() {
                None
            } else {
                serde_json::to_string(&template.config_mapping.format_rules).ok()
            },
            config_file_parse: None,
        }
    }
}

impl ClientStateRow {
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub(super) fn to_setting(&self) -> BackupPolicySetting {
        BackupPolicySetting::from_pair(
            self.backup_policy.as_deref(),
            self.backup_limit.map(|value| value.max(0) as u32),
        )
    }

    #[allow(dead_code)]
    pub fn is_approved(&self) -> bool {
        matches!(self.approval_status.as_deref(), Some("approved") | None)
    }

    pub fn approval_status(&self) -> &str {
        self.approval_status.as_deref().unwrap_or("approved")
    }

    pub fn attachment_state(&self) -> AttachmentState {
        if !self.has_local_config_target() {
            return AttachmentState::NotApplicable;
        }

        self.attachment_state
            .as_deref()
            .and_then(|value| value.parse::<AttachmentState>().ok())
            .unwrap_or_default()
    }

    pub fn connection_mode(&self) -> ClientConnectionMode {
        self.connection_mode
            .as_deref()
            .and_then(|value| value.parse::<ClientConnectionMode>().ok())
            .unwrap_or_default()
    }

    pub fn governance_kind(&self) -> ClientGovernanceKind {
        self.governance_kind
            .as_deref()
            .and_then(|value| value.parse::<ClientGovernanceKind>().ok())
            .unwrap_or_default()
    }

    pub fn display_name(&self) -> &str {
        self.display_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&self.name)
    }

    pub fn config_path(&self) -> Option<&str> {
        self.config_path.as_deref().filter(|value| !value.trim().is_empty())
    }

    pub fn governed_by_default_policy(&self) -> bool {
        self.governance_kind() == ClientGovernanceKind::Passive
    }

    pub fn has_local_config_target(&self) -> bool {
        self.connection_mode() == ClientConnectionMode::LocalConfigDetected && self.config_path().is_some()
    }

    #[allow(dead_code)]
    pub fn is_pending_approval(&self) -> bool {
        self.approval_status.as_deref() == Some("pending")
    }

    #[allow(dead_code)]
    pub fn template_id(&self) -> Option<&str> {
        self.template_id.as_deref()
    }

    /// Returns the template identifier (init-time seed; NOT for runtime inference).
    pub fn template_identifier(&self) -> Option<&str> {
        self.template_identifier
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }

    pub(super) fn capability_config(&self) -> ConfigResult<ClientCapabilityConfig> {
        ClientCapabilityConfig::from_parts(
            self.capability_source.as_deref(),
            self.selected_profile_ids.as_deref(),
            self.custom_profile_id.clone(),
        )
        .map_err(ConfigError::DataAccessError)
    }

    pub(super) fn unify_direct_exposure_intent(
        &self
    ) -> ConfigResult<crate::clients::models::UnifyDirectExposureIntent> {
        crate::clients::models::UnifyDirectExposureIntent::from_parts(self.unify_direct_exposure_intent.as_deref())
            .map_err(ConfigError::DataAccessError)
    }

    pub fn runtime_client_metadata(&self) -> RuntimeClientMetadata {
        let Some(raw) = self.approval_metadata.as_deref() else {
            return RuntimeClientMetadata::default();
        };

        let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
            return RuntimeClientMetadata::default();
        };

        value
            .get("runtime_client")
            .cloned()
            .and_then(|entry| serde_json::from_value::<RuntimeClientMetadata>(entry).ok())
            .unwrap_or_default()
    }

    // Template configuration accessors (persisted from template at initialization)

    pub fn config_format(&self) -> Option<&str> {
        self.config_format.as_deref().filter(|v| !v.trim().is_empty())
    }

    pub fn protocol_revision(&self) -> Option<&str> {
        self.protocol_revision.as_deref().filter(|v| !v.trim().is_empty())
    }

    pub fn container_type(&self) -> Option<&str> {
        self.container_type.as_deref().filter(|v| !v.trim().is_empty())
    }

    pub fn container_keys(&self) -> ConfigResult<Vec<String>> {
        let Some(raw) = self.container_keys.as_deref() else {
            return Ok(Vec::new());
        };

        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str::<Vec<String>>(raw)
            .map_err(|e| ConfigError::DataAccessError(format!("Failed to parse container_keys: {}", e)))
    }

    pub fn storage_kind(&self) -> Option<&str> {
        self.storage_kind.as_deref().filter(|v| !v.trim().is_empty())
    }

    pub fn storage_adapter(&self) -> Option<&str> {
        self.storage_adapter.as_deref().filter(|v| !v.trim().is_empty())
    }

    pub fn storage_path_strategy(&self) -> Option<&str> {
        self.storage_path_strategy.as_deref().filter(|v| !v.trim().is_empty())
    }

    pub fn merge_strategy(&self) -> Option<&str> {
        self.merge_strategy.as_deref().filter(|v| !v.trim().is_empty())
    }

    pub fn keep_original_config(&self) -> bool {
        self.keep_original_config.map(|v| v != 0).unwrap_or(false)
    }

    pub fn managed_source(&self) -> Option<&str> {
        self.managed_source.as_deref().filter(|v| !v.trim().is_empty())
    }

    pub fn transports(&self) -> ConfigResult<Option<serde_json::Value>> {
        let Some(raw) = self.transports.as_deref() else {
            return Ok(None);
        };

        if raw.trim().is_empty() {
            return Ok(None);
        }

        serde_json::from_str::<serde_json::Value>(raw)
            .map(Some)
            .map_err(|e| ConfigError::DataAccessError(format!("Failed to parse transports: {}", e)))
    }

    pub fn parsed_transports(&self) -> ConfigResult<std::collections::HashMap<String, FormatRule>> {
        let Some(value) = self.transports()? else {
            return Ok(std::collections::HashMap::new());
        };

        serde_json::from_value::<std::collections::HashMap<String, FormatRule>>(value)
            .map_err(|e| ConfigError::DataAccessError(format!("Failed to decode transports: {}", e)))
    }

    pub fn config_file_parse_override(&self) -> ConfigResult<Option<ClientConfigFileParse>> {
        let Some(raw) = self.config_file_parse.as_deref() else {
            return Ok(None);
        };

        if raw.trim().is_empty() {
            return Ok(None);
        }

        serde_json::from_str::<ClientConfigFileParse>(raw)
            .map(Some)
            .map_err(|e| ConfigError::DataAccessError(format!("Failed to parse config_file_parse: {}", e)))
    }

    pub fn legacy_config_file_parse(&self) -> ConfigResult<Option<ClientConfigFileParse>> {
        let format = match self.config_format() {
            Some("json") => TemplateFormat::Json,
            Some("json5") => TemplateFormat::Json5,
            Some("toml") => TemplateFormat::Toml,
            Some("yaml") => TemplateFormat::Yaml,
            Some(_) | None => return Ok(None),
        };

        let container_keys = self.container_keys()?;
        if container_keys.is_empty() {
            return Ok(None);
        }

        let container_type = match self.container_type() {
            Some("array") => crate::clients::models::ContainerType::Array,
            _ => crate::clients::models::ContainerType::ObjectMap,
        };

        Ok(Some(ClientConfigFileParse {
            format,
            container_type,
            container_keys,
        }))
    }
}

fn effective_transports_for_state(state: &ClientStateRow) -> ConfigResult<HashMap<String, FormatRule>> {
    Ok(state
        .parsed_transports()?
        .into_iter()
        .map(|(transport, rule)| (transport, rule.normalized()))
        .collect())
}

pub(crate) fn supported_transports_from_transports(transports: &HashMap<String, FormatRule>) -> Vec<String> {
    ["streamable_http", "sse", "stdio"]
        .into_iter()
        .filter(|transport| transports.contains_key(*transport))
        .map(str::to_string)
        .collect()
}

/// Summarized view of a client template combined with detection and filesystem state
#[derive(Debug, Clone)]
pub struct ClientDescriptor {
    pub template: Option<ClientTemplate>,
    pub state: ClientStateRow,
    pub detection: Option<DetectedClient>,
    pub config_path: Option<String>,
    pub config_exists: bool,
    pub detected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ClientBackupRecord {
    pub identifier: String,
    pub backup: String,
    pub path: String,
    pub size: u64,
    pub created_at: Option<DateTime<Utc>>,
}

/// Parameters for rendering/applying a client configuration
#[derive(Debug, Clone)]
pub struct ClientRenderOptions {
    pub client_id: String,
    pub mode: ConfigMode,
    pub profile_id: Option<String>,
    pub server_ids: Option<Vec<String>>,
    pub dry_run: bool,
}

/// Result of a configuration execution
#[derive(Debug)]
pub struct ClientRenderResult {
    pub execution: TemplateExecutionResult,
    pub target_path: Option<String>,
    pub servers: Vec<ServerTemplateInput>,
    pub warnings: Vec<String>,
    pub chosen_transport: Option<String>,
    pub auto_selected: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PreviewOutcome {
    pub format: TemplateFormat,
    pub before: Option<String>,
    pub after: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    pub preview: PreviewOutcome,
    pub applied: bool,
    pub backup_path: Option<String>,
    pub scheduled: bool,
    pub scheduled_reason: Option<String>,
    pub warnings: Vec<String>,
}

/// High-level service wiring templates, detection, and storage backends
pub struct ClientConfigService {
    pub(super) template_source: Arc<dyn ClientConfigSource>,
    pub(super) template_engine: Arc<TemplateEngine>,
    pub(super) detector: Arc<ClientDetector>,
    pub(super) db_pool: Arc<SqlitePool>,
}

impl ClientConfigService {
    /// Bootstrap service with default template root resolution
    pub async fn bootstrap(db_pool: Arc<SqlitePool>) -> crate::clients::error::ConfigResult<Self> {
        let templates = embedded_official_templates()?;
        Self::seed_runtime_template_snapshots_from_templates(db_pool.as_ref(), &templates).await?;
        Self::seed_client_runtime_rows_from_templates(db_pool.as_ref(), &templates).await?;
        let runtime_source: Arc<dyn ClientConfigSource> = Arc::new(DbTemplateSource::new(db_pool.clone())?);
        Self::with_source(db_pool, runtime_source).await
    }

    /// Initialize service with pre-built template source (primarily for tests)
    pub async fn with_source(
        db_pool: Arc<SqlitePool>,
        template_source: Arc<dyn ClientConfigSource>,
    ) -> crate::clients::error::ConfigResult<Self> {
        let engine = TemplateEngine::with_defaults(template_source.clone());
        let detector = ClientDetector::new(template_source.clone())?;

        Ok(Self {
            template_source,
            template_engine: Arc::new(engine),
            detector: Arc::new(detector),
            db_pool,
        })
    }

    /// Reload templates from disk, keeping previous index if reloading fails
    pub async fn reload_templates(&self) -> crate::clients::error::ConfigResult<()> {
        Ok(())
    }

    /// Get template for client identifier on current platform
    pub async fn get_client_template(
        &self,
        client_id: &str,
    ) -> crate::clients::error::ConfigResult<ClientTemplate> {
        self.template_source
            .get_template(client_id, PathService::get_current_platform())
            .await?
            .ok_or_else(|| {
                crate::clients::error::ConfigError::TemplateIndexError(format!(
                    "Client template not found for {}",
                    client_id
                ))
            })
    }

    pub fn build_render_definition_from_state(state: &ClientStateRow) -> ConfigResult<ClientRenderDefinition> {
        let parse = state
            .config_file_parse_override()?
            .or_else(|| state.legacy_config_file_parse().ok().flatten())
            .ok_or_else(|| {
                ConfigError::DataAccessError(format!(
                    "Client '{}' is missing persisted config_file_parse; cannot render configuration",
                    state.identifier()
                ))
            })?;

        if parse.container_keys.is_empty() {
            return Err(ConfigError::DataAccessError(format!(
                "Client '{}' is missing config_file_parse.container_keys; cannot render configuration",
                state.identifier()
            )));
        }

        let transports = effective_transports_for_state(state)?;
        let supported_transports = supported_transports_from_transports(&transports);
        if supported_transports.is_empty() {
            return Err(ConfigError::DataAccessError(format!(
                "Client '{}' is missing persisted transports; cannot render configuration",
                state.identifier()
            )));
        }

        for transport in supported_transports {
            if let Some(rule) = transports.get(&transport) {
                rule.validate_for_transport(&transport)
                    .map_err(ConfigError::DataAccessError)?;
            } else {
                return Err(ConfigError::DataAccessError(format!(
                    "Client '{}' is missing persisted format rule for supported transport '{}'",
                    state.identifier(),
                    transport
                )));
            }
        }

        let config_mapping = ConfigMapping {
            container_keys: parse.container_keys.clone(),
            container_type: parse.container_type,
            merge_strategy: match state.merge_strategy() {
                Some("deep_merge") => MergeStrategy::DeepMerge,
                _ => MergeStrategy::Replace,
            },
            keep_original_config: state.keep_original_config(),
            managed_endpoint: Some(ManagedEndpointConfig {
                source: state.managed_source().map(str::to_string),
            }),
            managed_source: state.managed_source().map(str::to_string),
            parse: Some(parse.clone()),
            format_rules: transports,
        };

        let storage = StorageConfig {
            kind: match state.storage_kind() {
                Some("kv") => StorageKind::Kv,
                Some("custom") => StorageKind::Custom,
                _ => StorageKind::File,
            },
            path_strategy: state.storage_path_strategy().map(str::to_string),
            adapter: state.storage_adapter().map(str::to_string),
        };

        Ok(ClientRenderDefinition {
            identifier: state.identifier().to_string(),
            format: parse.format,
            storage,
            config_mapping,
        })
    }

    /// Read current configuration file content for a client
    pub async fn read_current_config(
        &self,
        client_id: &str,
    ) -> crate::clients::error::ConfigResult<Option<String>> {
        let state = self
            .fetch_state(client_id)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} not found", client_id)))?;
        let config_path = state
            .config_path()
            .ok_or_else(|| ConfigError::PathResolutionError(format!("No config_path for client {}", client_id)))?;
        let storage = self.template_engine.storage_for_client(&state)?;
        storage.read(config_path).await
    }

    fn parse_detach_config(
        raw_content: &str,
        format: &str,
    ) -> ConfigResult<serde_json::Value> {
        match format {
            "json5" => json5::from_str(raw_content)
                .map_err(|err| ConfigError::DataAccessError(format!("Failed to parse config for detach: {err}"))),
            "yaml" => serde_yaml::from_str(raw_content)
                .map_err(|err| ConfigError::DataAccessError(format!("Failed to parse config for detach: {err}"))),
            "toml" => {
                let value: toml::Value = toml::from_str(raw_content)
                    .map_err(|err| ConfigError::DataAccessError(format!("Failed to parse config for detach: {err}")))?;
                serde_json::to_value(value)
                    .map_err(|err| ConfigError::DataAccessError(format!("Failed to convert TOML for detach: {err}")))
            }
            _ => serde_json::from_str(raw_content)
                .map_err(|err| ConfigError::DataAccessError(format!("Failed to parse config for detach: {err}"))),
        }
    }

    fn serialize_detach_config(
        value: &serde_json::Value,
        format: &str,
    ) -> ConfigResult<String> {
        match format {
            "json5" => json5::to_string(value)
                .map_err(|err| ConfigError::DataAccessError(format!("Failed to serialize detached config: {err}"))),
            "yaml" => serde_yaml::to_string(value)
                .map_err(|err| ConfigError::DataAccessError(format!("Failed to serialize detached config: {err}"))),
            "toml" => toml::to_string(value)
                .map_err(|err| ConfigError::DataAccessError(format!("Failed to serialize detached config: {err}"))),
            _ => serde_json::to_string_pretty(value)
                .map(|content| content.replace("\\/", "/"))
                .map_err(|err| ConfigError::DataAccessError(format!("Failed to serialize detached config: {err}"))),
        }
    }

    /// Detach MCPMate from a client's external configuration while preserving MCPMate-side settings.
    pub async fn detach_client(
        &self,
        client_id: &str,
    ) -> ConfigResult<bool> {
        let state = self
            .fetch_state(client_id)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} not found", client_id)))?;
        let config_path = state
            .config_path()
            .ok_or_else(|| ConfigError::PathResolutionError(format!("No config_path for client {}", client_id)))?;
        if !state.has_local_config_target() {
            return Err(ConfigError::DataAccessError(format!(
                "Client {} does not have an attachable local config target",
                client_id
            )));
        }
        let raw_content = self.read_current_config(client_id).await?.ok_or_else(|| {
            ConfigError::FileOperationError(format!("Config file not found for client {}", client_id))
        })?;

        let format = state.config_format().unwrap_or("json");
        let parsed = Self::parse_detach_config(&raw_content, format)?;
        let container_keys = state.container_keys().unwrap_or_default();
        let is_array = state.container_type() == Some("array");
        let (filtered, changed) = filter_mcp_mate_entries(parsed, &container_keys, is_array);

        if changed {
            let output = Self::serialize_detach_config(&filtered, format)?;
            let storage = self.template_engine.storage_for_client(&state)?;
            storage
                .write_atomic(client_id, config_path, &output, &BackupPolicySetting::default())
                .await?;
        }

        self.mark_client_detached(client_id).await?;
        Ok(changed)
    }

    /// Get resolved configuration path for a client on current platform
    pub async fn config_path(
        &self,
        client_id: &str,
    ) -> crate::clients::error::ConfigResult<Option<String>> {
        self.resolved_config_path(client_id).await
    }

    pub(super) async fn resolved_config_path(
        &self,
        client_id: &str,
    ) -> crate::clients::error::ConfigResult<Option<String>> {
        if let Some(state) = self.fetch_state(client_id).await? {
            if let Some(config_path) = state.config_path.filter(|value| !value.trim().is_empty()) {
                let resolved = get_path_service()
                    .resolve_user_path(&config_path)
                    .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
                return Ok(Some(resolved.to_string_lossy().to_string()));
            }
        }

        Ok(None)
    }

    pub(super) async fn verified_local_config_target(
        &self,
        client_id: &str,
    ) -> ConfigResult<Option<String>> {
        let Some(config_path) = self.resolved_config_path(client_id).await? else {
            return Ok(None);
        };

        let resolved_path = std::path::PathBuf::from(&config_path);
        let metadata = tokio::fs::metadata(&resolved_path).await.map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                ConfigError::DataAccessError(format!("Client config target does not exist: {}", config_path))
            } else {
                ConfigError::FileOperationError(format!(
                    "Failed to inspect client config target {}: {}",
                    config_path, err
                ))
            }
        })?;

        if metadata.is_file() {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&resolved_path)
                .await
                .map_err(|_| ConfigError::PathNotWritable {
                    path: resolved_path.clone(),
                })?;
        } else if metadata.is_dir() {
            Self::validate_directory_target_writable(&resolved_path).await?;
        } else {
            return Err(ConfigError::DataAccessError(format!(
                "Client config target is neither a file nor a directory: {}",
                config_path
            )));
        }

        Ok(Some(config_path))
    }

    pub(super) async fn validate_directory_target_writable(directory_path: &std::path::Path) -> ConfigResult<()> {
        let probe_name = format!(
            ".mcpmate-write-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        );
        let probe_path = directory_path.join(probe_name);

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe_path)
            .await
            .map_err(|_| ConfigError::PathNotWritable {
                path: directory_path.to_path_buf(),
            })?;

        tokio::fs::remove_file(&probe_path)
            .await
            .map_err(|_| ConfigError::PathNotWritable {
                path: directory_path.to_path_buf(),
            })?;

        Ok(())
    }

    pub async fn has_verified_local_config_target(
        &self,
        client_id: &str,
    ) -> ConfigResult<bool> {
        Ok(self.verified_local_config_target(client_id).await?.is_some())
    }

    #[cfg(test)]
    pub(crate) async fn seed_runtime_template_snapshots(
        db_pool: &SqlitePool,
        file_source: &FileTemplateSource,
    ) -> crate::clients::error::ConfigResult<()> {
        let templates = file_source.list_client().await?;
        Self::seed_runtime_template_snapshots_from_templates(db_pool, &templates).await
    }

    async fn seed_runtime_template_snapshots_from_templates(
        db_pool: &SqlitePool,
        templates: &[ClientTemplate],
    ) -> crate::clients::error::ConfigResult<()> {
        for template in templates {
            let payload_json = serde_json::to_string(&template).map_err(|err| {
                ConfigError::TemplateParseError(format!("Failed to serialize runtime template payload: {}", err))
            })?;
            sqlx::query(
                r#"
                INSERT INTO client_template_runtime (identifier, payload_json)
                VALUES (?, ?)
                ON CONFLICT(identifier) DO UPDATE SET
                    payload_json = excluded.payload_json,
                    updated_at = CURRENT_TIMESTAMP
                "#,
            )
            .bind(&template.identifier)
            .bind(payload_json)
            .execute(db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) async fn seed_client_runtime_rows(
        db_pool: &SqlitePool,
        file_source: &FileTemplateSource,
    ) -> crate::clients::error::ConfigResult<()> {
        let templates = file_source.list_client().await?;
        Self::seed_client_runtime_rows_from_templates(db_pool, &templates).await
    }

    async fn seed_client_runtime_rows_from_templates(
        db_pool: &SqlitePool,
        templates: &[ClientTemplate],
    ) -> crate::clients::error::ConfigResult<()> {
        for template in templates {
            let display_name = template.display_name.as_deref().unwrap_or(&template.identifier);
            let config_path = Self::extract_runtime_config_path_from_template(template);
            let id = crate::generate_id!("clnt");
            let persisted_config = PersistedTemplateConfig::from_template(template);

            let runtime_metadata = serde_json::json!({
                "runtime_client": {
                    "description": template.metadata.get("description").and_then(|v| v.as_str()),
                    "homepage_url": template.metadata.get("homepage_url").and_then(|v| v.as_str()),
                    "docs_url": template.metadata.get("docs_url").and_then(|v| v.as_str()),
                    "support_url": template.metadata.get("support_url").and_then(|v| v.as_str()),
                    "logo_url": template.metadata.get("logo_url").and_then(|v| v.as_str()),
                    "category": template.metadata.get("category").and_then(|v| v.as_str())
                }
            });
            let approval_metadata = serde_json::to_string(&runtime_metadata).ok();

            sqlx::query(
                r#"
                INSERT INTO client (
                    id, name, display_name, identifier, config_path, backup_policy, backup_limit,
                    capability_source, governance_kind, connection_mode, approval_status, template_identifier,
                    config_format, protocol_revision, container_type, container_keys,
                    storage_kind, storage_adapter, storage_path_strategy,
                    merge_strategy, keep_original_config, managed_source, transports, config_file_parse,
                    approval_metadata, attachment_state
                )
                VALUES (?, ?, ?, ?, ?, 'keep_n', 5, 'activated', 'passive', 'local_config_detected', 'approved', ?,
                        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'attached')
                ON CONFLICT(identifier) DO UPDATE SET
                    display_name = COALESCE(NULLIF(client.display_name, ''), excluded.display_name),
                    config_path = COALESCE(NULLIF(client.config_path, ''), excluded.config_path),
                    template_identifier = COALESCE(NULLIF(client.template_identifier, ''), excluded.template_identifier),
                    config_format = excluded.config_format,
                    protocol_revision = excluded.protocol_revision,
                    container_type = excluded.container_type,
                    container_keys = excluded.container_keys,
                    storage_kind = excluded.storage_kind,
                    storage_adapter = excluded.storage_adapter,
                    storage_path_strategy = excluded.storage_path_strategy,
                    merge_strategy = excluded.merge_strategy,
                    keep_original_config = excluded.keep_original_config,
                    managed_source = excluded.managed_source,
                    transports = COALESCE(client.transports, excluded.transports),
                    config_file_parse = COALESCE(client.config_file_parse, excluded.config_file_parse),
                    approval_metadata = COALESCE(client.approval_metadata, excluded.approval_metadata),
                    updated_at = CURRENT_TIMESTAMP
                "#,
            )
            .bind(&id)
            .bind(display_name)
            .bind(display_name)
            .bind(&template.identifier)
            .bind(config_path)
            .bind(&template.identifier)
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
            .bind(approval_metadata)
            .execute(db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;
        }

        Ok(())
    }

    pub(crate) fn extract_runtime_config_path_from_template(template: &ClientTemplate) -> Option<String> {
        let platform = PathService::get_current_platform();
        let rules = template.platform_rules(platform)?;
        let rule = rules.first()?;
        let candidate = rule.config_path.as_ref().or(Some(&rule.value))?;
        PathService::new()
            .ok()?
            .resolve_user_path(candidate)
            .ok()
            .map(|value| value.to_string_lossy().to_string())
    }
}

fn filter_mcp_mate_entries(
    mut value: serde_json::Value,
    container_keys: &[String],
    is_array: bool,
) -> (serde_json::Value, bool) {
    let mut changed = false;
    for key in container_keys {
        if let Some(container) = get_nested_value_mut(&mut value, key) {
            if is_array {
                if let Some(entries) = container.as_array_mut() {
                    let before_len = entries.len();
                    entries.retain(|entry| !is_attached_server_entry(entry));
                    changed |= entries.len() != before_len;
                }
            } else if let Some(entries) = container.as_object_mut() {
                let before_len = entries.len();
                entries.retain(|name, entry| !is_attached_server_name(name) && !is_attached_server_entry(entry));
                changed |= entries.len() != before_len;
            }
        }
    }
    (value, changed)
}

fn is_attached_server_name(name: &str) -> bool {
    name.eq_ignore_ascii_case(profile_keys::MCPMATE)
}

fn is_attached_server_entry(entry: &serde_json::Value) -> bool {
    let Some(object) = entry.as_object() else {
        return false;
    };

    if object
        .get("name")
        .and_then(|name| name.as_str())
        .map(is_attached_server_name)
        .unwrap_or(false)
    {
        return true;
    }

    object
        .get("headers")
        .and_then(|headers| headers.as_object())
        .map(|headers| {
            headers.contains_key(client_headers::MCPMATE_CLIENT_ID)
                || headers.contains_key(client_headers::MCPMATE_PROFILE_ID)
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod render_definition_tests {
    use super::*;

    #[test]
    fn build_render_definition_ignores_metadata_supported_transports() {
        let state = ClientStateRow {
            identifier: "zed".to_string(),
            config_path: Some("~/.config/zed/settings.json".to_string()),
            connection_mode: Some("local_config_detected".to_string()),
            template_identifier: Some("zed".to_string()),
            config_format: Some("json".to_string()),
            container_type: Some("object".to_string()),
            container_keys: Some("[\"context_servers\"]".to_string()),
            transports: Some(
                serde_json::json!({
                    "stdio": {
                        "template": {
                            "type": "stdio",
                            "command": "{{{command}}}"
                        },
                    "include_type": false
                    }
                })
                .to_string(),
            ),
            approval_metadata: Some(
                serde_json::json!({
                    "runtime_client": {
                        "supported_transports": ["streamable_http"]
                    }
                })
                .to_string(),
            ),
            ..ClientStateRow::default()
        };

        let definition = ClientConfigService::build_render_definition_from_state(&state)
            .expect("metadata transports should not affect render definition");

        assert!(definition.config_mapping.format_rules.contains_key("stdio"));
        assert!(!definition.config_mapping.format_rules.contains_key("streamable_http"));
    }

    #[test]
    fn build_render_definition_derives_supported_transports_from_transports() {
        let state = ClientStateRow {
            identifier: "cursor".to_string(),
            config_path: Some("~/.cursor/mcp.json".to_string()),
            connection_mode: Some("local_config_detected".to_string()),
            template_identifier: Some("cursor".to_string()),
            config_format: Some("json".to_string()),
            container_type: Some("object".to_string()),
            container_keys: Some("[\"mcpServers\"]".to_string()),
            storage_kind: Some("file".to_string()),
            storage_path_strategy: Some("config_path".to_string()),
            merge_strategy: Some("replace".to_string()),
            managed_source: Some("profile".to_string()),
            transports: Some(
                serde_json::json!({
                    "streamable_http": {
                        "template": {
                            "type": "streamable_http",
                            "url": "{{{url}}}"
                        },
                        "include_type": false
                    }
                })
                .to_string(),
            ),
            approval_metadata: None,
            ..ClientStateRow::default()
        };

        let definition = ClientConfigService::build_render_definition_from_state(&state)
            .expect("render definition should derive transports from format rules");

        assert!(definition.config_mapping.format_rules.contains_key("streamable_http"));
    }

    #[test]
    fn build_render_definition_requires_canonical_transport_keys() {
        let state = ClientStateRow {
            identifier: "cursor".to_string(),
            config_path: Some("~/.cursor/mcp.json".to_string()),
            connection_mode: Some("local_config_detected".to_string()),
            template_identifier: Some("cursor".to_string()),
            config_format: Some("json".to_string()),
            container_type: Some("object".to_string()),
            container_keys: Some("[\"mcpServers\"]".to_string()),
            storage_kind: Some("file".to_string()),
            storage_path_strategy: Some("config_path".to_string()),
            merge_strategy: Some("replace".to_string()),
            managed_source: Some("profile".to_string()),
            transports: Some(
                serde_json::json!({
                    "http": {
                        "template": {
                            "type": "streamable_http",
                            "url": "{{{url}}}"
                        },
                        "include_type": false
                    }
                })
                .to_string(),
            ),
            approval_metadata: None,
            ..ClientStateRow::default()
        };

        let error = ClientConfigService::build_render_definition_from_state(&state)
            .expect_err("alias transport keys should be rejected");

        assert!(error.to_string().contains("missing persisted transports"));
    }

    #[test]
    fn filter_mcp_mate_entries_removes_attached_object_entry_from_recorded_container() {
        let config = serde_json::json!({
            "servers": {
                "MCPMate": {
                    "type": "streamable_http",
                    "url": "http://127.0.0.1:8000/mcp?client_id=client"
                },
                "other": {
                    "type": "stdio",
                    "command": "other"
                }
            }
        });

        let (filtered, changed) = filter_mcp_mate_entries(config, &["servers".to_string()], false);

        assert!(changed);
        assert!(filtered["servers"].get("MCPMate").is_none());
        assert!(filtered["servers"].get("other").is_some());
    }

    #[test]
    fn filter_mcp_mate_entries_removes_attached_array_entry_from_recorded_nested_container() {
        let config = serde_json::json!({
            "mcp": {
                "servers": [
                    { "name": "MCPMate", "type": "stdio" },
                    { "name": "other", "type": "stdio" }
                ]
            }
        });

        let (filtered, changed) = filter_mcp_mate_entries(config, &["mcp.servers".to_string()], true);

        assert!(changed);
        let servers = filtered["mcp"]["servers"].as_array().expect("servers array");
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0]["name"], "other");
    }

    #[tokio::test]
    async fn seed_client_runtime_rows_persists_transports_without_transport_metadata() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .expect("init client table");
        let mut transports = HashMap::new();
        transports.insert(
            "streamable_http".to_string(),
            FormatRule {
                template: serde_json::json!({
                    "type": "streamable_http",
                    "url": "{{{url}}}"
                }),
                include_type: false,
                ..Default::default()
            },
        );
        let template = ClientTemplate {
            identifier: "cursor".to_string(),
            display_name: Some("Cursor".to_string()),
            format: TemplateFormat::Json,
            storage: StorageConfig {
                kind: StorageKind::File,
                path_strategy: Some("config_path".to_string()),
                adapter: None,
            },
            config_mapping: ConfigMapping {
                container_keys: vec!["mcpServers".to_string()],
                container_type: crate::clients::models::ContainerType::ObjectMap,
                merge_strategy: MergeStrategy::Replace,
                keep_original_config: false,
                managed_endpoint: None,
                managed_source: Some("profile".to_string()),
                parse: None,
                format_rules: transports,
            },
            ..Default::default()
        };

        ClientConfigService::seed_client_runtime_rows_from_templates(&pool, &[template])
            .await
            .expect("seed client runtime row");
        let (approval_metadata, persisted_transports): (String, String) =
            sqlx::query_as("SELECT approval_metadata, transports FROM client WHERE identifier = ?")
                .bind("cursor")
                .fetch_one(&pool)
                .await
                .expect("load client row");
        let value: serde_json::Value = serde_json::from_str(&approval_metadata).expect("approval metadata json");
        let persisted_rules: HashMap<String, FormatRule> =
            serde_json::from_str(&persisted_transports).expect("persisted transports json");

        assert!(value["runtime_client"].get("supported_transports").is_none());
        assert_eq!(
            supported_transports_from_transports(&persisted_rules),
            vec!["streamable_http".to_string()]
        );
    }
}
