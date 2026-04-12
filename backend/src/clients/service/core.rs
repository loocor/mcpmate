use crate::clients::TemplateEngine;
use crate::clients::detector::{ClientDetector, DetectedClient};
use crate::clients::engine::TemplateExecutionResult;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    BackupPolicySetting, ClientCapabilityConfig, ClientConnectionMode, ClientGovernanceKind, ClientRecordKind,
    ClientTemplate, ConfigMapping, ConfigMode, DetectionMethod, DetectionRule, FormatRule, ManagedEndpointConfig,
    MergeStrategy, ServerTemplateInput, StorageConfig, StorageKind, TemplateFormat, UnifyDirectExposureConfig,
};
#[cfg(test)]
use crate::clients::source::FileTemplateSource;
use crate::clients::source::{ClientConfigSource, DbTemplateSource};
use crate::system::paths::{PathService, get_path_service};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::OpenOptions;

// Generated at build time from the repository's config/client directory
include!(concat!(env!("OUT_DIR"), "/official_templates_generated.rs"));

const RUNTIME_ACTIVE_TEMPLATE_SOURCE: &str = "runtime_active_client";

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
    pub(super) managed: i64,
    pub(super) config_mode: Option<String>,
    pub(super) transport: Option<String>,
    pub(super) client_version: Option<String>,
    pub(super) backup_policy: Option<String>,
    pub(super) backup_limit: Option<i64>,
    pub(super) capability_source: Option<String>,
    pub(super) governance_kind: Option<String>,
    pub(super) connection_mode: Option<String>,
    pub(super) record_kind: Option<String>,
    pub(super) template_identifier: Option<String>,
    pub(super) selected_profile_ids: Option<String>,
    pub(super) custom_profile_id: Option<String>,
    pub(super) unify_route_mode: Option<String>,
    pub(super) unify_selected_server_ids: Option<String>,
    pub(super) unify_selected_tool_surfaces: Option<String>,
    pub(super) unify_selected_prompt_surfaces: Option<String>,
    pub(super) unify_selected_resource_surfaces: Option<String>,
    pub(super) unify_selected_template_surfaces: Option<String>,
    pub(super) approval_status: Option<String>,
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
    pub(super) format_rules: Option<String>,
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
    #[serde(default)]
    pub supported_transports: Vec<String>,
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

    pub fn managed(&self) -> bool {
        self.managed != 0
    }

    #[allow(dead_code)]
    pub(super) fn is_approved(&self) -> bool {
        matches!(self.approval_status.as_deref(), Some("approved") | None)
    }

    pub fn approval_status(&self) -> &str {
        self.approval_status.as_deref().unwrap_or("approved")
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

    pub fn record_kind(&self) -> ClientRecordKind {
        self.record_kind
            .as_deref()
            .and_then(|value| value.parse::<ClientRecordKind>().ok())
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
        self.connection_mode() != ClientConnectionMode::RemoteHttp && self.config_path().is_some()
    }

    #[allow(dead_code)]
    pub fn is_template_known(&self) -> bool {
        self.record_kind() == ClientRecordKind::TemplateKnown
    }

    #[allow(dead_code)]
    pub fn is_pending_unknown(&self) -> bool {
        self.approval_status.as_deref() == Some("pending") && self.record_kind() == ClientRecordKind::ObservedUnknown
    }

    pub fn template_id(&self) -> Option<&str> {
        self.template_id.as_deref()
    }

    pub fn template_identifier(&self) -> Option<&str> {
        self.template_identifier
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| self.template_id())
    }

    pub(super) fn capability_config(&self) -> ConfigResult<ClientCapabilityConfig> {
        ClientCapabilityConfig::from_parts(
            self.capability_source.as_deref(),
            self.selected_profile_ids.as_deref(),
            self.custom_profile_id.clone(),
        )
        .map_err(ConfigError::DataAccessError)
    }

    pub(super) fn unify_direct_exposure_config(&self) -> ConfigResult<UnifyDirectExposureConfig> {
        UnifyDirectExposureConfig::from_parts(
            self.unify_route_mode.as_deref(),
            self.unify_selected_server_ids.as_deref(),
            self.unify_selected_tool_surfaces.as_deref(),
            self.unify_selected_prompt_surfaces.as_deref(),
            self.unify_selected_resource_surfaces.as_deref(),
            self.unify_selected_template_surfaces.as_deref(),
        )
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

    pub fn format_rules(&self) -> ConfigResult<Option<serde_json::Value>> {
        let Some(raw) = self.format_rules.as_deref() else {
            return Ok(None);
        };

        if raw.trim().is_empty() {
            return Ok(None);
        }

        serde_json::from_str::<serde_json::Value>(raw)
            .map(Some)
            .map_err(|e| ConfigError::DataAccessError(format!("Failed to parse format_rules: {}", e)))
    }
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
    pub managed: bool,
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

    /// Read current configuration file content for a client
    pub async fn read_current_config(
        &self,
        client_id: &str,
    ) -> crate::clients::error::ConfigResult<Option<String>> {
        let state = self.fetch_state(client_id).await?.ok_or_else(|| {
            ConfigError::DataAccessError(format!("Client {} not found", client_id))
        })?;
        let config_path = state.config_path().ok_or_else(|| {
            ConfigError::PathResolutionError(format!("No config_path for client {}", client_id))
        })?;
        let storage = self.template_engine.storage_for_client(&state)?;
        storage.read(config_path).await
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

    pub(super) async fn should_persist_runtime_active_template(
        &self,
        identifier: &str,
        existing_state: Option<&ClientStateRow>,
    ) -> ConfigResult<bool> {
        if existing_state
            .map(|state| state.record_kind() == ClientRecordKind::ObservedUnknown)
            .unwrap_or(true)
        {
            return Ok(true);
        }

        let payload_json = sqlx::query_scalar::<_, String>(
            r#"
            SELECT payload_json
            FROM client_template_runtime
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        let Some(payload_json) = payload_json else {
            return Ok(true);
        };

        let template = serde_json::from_str::<ClientTemplate>(&payload_json).map_err(|err| {
            ConfigError::TemplateParseError(format!("Failed to parse runtime template payload: {}", err))
        })?;

        Ok(template.config_mapping.managed_source.as_deref() == Some(RUNTIME_ACTIVE_TEMPLATE_SOURCE))
    }

    pub(super) async fn persist_runtime_active_template(
        &self,
        identifier: &str,
    ) -> ConfigResult<()> {
        let state = self
            .fetch_state(identifier)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Missing client state for {}", identifier)))?;
        let template = Self::build_runtime_active_template(&state);
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
        .bind(identifier)
        .bind(payload_json)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        sqlx::query(
            r#"
            UPDATE client
            SET record_kind = 'template_known',
                template_identifier = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(identifier)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(())
    }

    fn build_runtime_active_template(state: &ClientStateRow) -> ClientTemplate {
        let config_path = state.config_path().map(str::to_string);
        let runtime_metadata = state.runtime_client_metadata();
        let format = Self::infer_runtime_template_format(config_path.as_deref());
        let mut detection = HashMap::new();
        if let Some(config_path) = config_path.clone().filter(|value| !value.trim().is_empty()) {
            detection.insert(
                PathService::get_current_platform().to_string(),
                vec![DetectionRule {
                    method: DetectionMethod::ConfigPath,
                    value: config_path.clone(),
                    config_path: Some(config_path),
                    priority: Some(0),
                }],
            );
        }

        let mut metadata = HashMap::new();
        if let Some(description) = runtime_metadata.description.clone() {
            metadata.insert("description".to_string(), json!(description));
        }
        if let Some(homepage_url) = runtime_metadata.homepage_url.clone() {
            metadata.insert("homepage_url".to_string(), json!(homepage_url));
        }
        if let Some(docs_url) = runtime_metadata.docs_url.clone() {
            metadata.insert("docs_url".to_string(), json!(docs_url));
        }
        if let Some(support_url) = runtime_metadata.support_url.clone() {
            metadata.insert("support_url".to_string(), json!(support_url));
        }
        if let Some(logo_url) = runtime_metadata.logo_url.clone() {
            metadata.insert("logo_url".to_string(), json!(logo_url));
        }
        if let Some(category) = runtime_metadata.category.clone() {
            metadata.insert("category".to_string(), json!(category));
        }

        ClientTemplate {
            identifier: state.identifier().to_string(),
            display_name: Some(state.display_name().to_string()),
            version: state.client_version.clone(),
            format,
            protocol_revision: None,
            storage: StorageConfig {
                kind: StorageKind::File,
                path_strategy: Some("config_path".to_string()),
                adapter: None,
            },
            detection,
            config_mapping: ConfigMapping {
                container_keys: vec!["mcpServers".to_string()],
                container_type: crate::clients::models::ContainerType::ObjectMap,
                merge_strategy: MergeStrategy::DeepMerge,
                keep_original_config: true,
                managed_endpoint: Some(ManagedEndpointConfig {
                    source: Some("profile".to_string()),
                }),
                managed_source: Some(RUNTIME_ACTIVE_TEMPLATE_SOURCE.to_string()),
                format_rules: Self::build_runtime_active_format_rules(
                    &runtime_metadata.supported_transports,
                    state.transport.as_deref(),
                ),
            },
            metadata,
        }
    }

    fn infer_runtime_template_format(config_path: Option<&str>) -> TemplateFormat {
        let Some(path) = config_path else {
            return TemplateFormat::Json;
        };

        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());

        match extension.as_deref() {
            Some("json5") => TemplateFormat::Json5,
            Some("toml") => TemplateFormat::Toml,
            Some("yaml") | Some("yml") => TemplateFormat::Yaml,
            _ => TemplateFormat::Json,
        }
    }

    fn build_runtime_active_format_rules(
        supported_transports: &[String],
        preferred_transport: Option<&str>,
    ) -> HashMap<String, FormatRule> {
        let mut normalized = supported_transports
            .iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .map(|value| match value.as_str() {
                "sse" | "http" | "streamablehttp" => "streamable_http".to_string(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>();

        if normalized.is_empty() {
            if let Some(preferred_transport) = preferred_transport {
                let preferred = preferred_transport.trim().to_ascii_lowercase();
                if matches!(preferred.as_str(), "streamable_http" | "sse" | "http") {
                    normalized.push("streamable_http".to_string());
                } else if preferred == "stdio" {
                    normalized.push("stdio".to_string());
                }
            }
        }

        if normalized.is_empty() {
            normalized.extend(["streamable_http".to_string(), "stdio".to_string()]);
        }

        let mut format_rules = HashMap::new();

        if normalized.iter().any(|value| value == "streamable_http") {
            let http_rule = FormatRule {
                template: json!({
                    "type": "streamable_http",
                    "url": "{{url}}",
                    "headers": "{{{json headers}}}"
                }),
                requires_type_field: false,
            };
            format_rules.insert("streamable_http".to_string(), http_rule.clone());
            format_rules.insert("sse".to_string(), http_rule);
        }

        if normalized.iter().any(|value| value == "stdio") {
            format_rules.insert(
                "stdio".to_string(),
                FormatRule {
                    template: json!({
                        "type": "stdio",
                        "command": "{{command}}",
                        "args": "{{{json args}}}",
                        "env": "{{{json env}}}"
                    }),
                    requires_type_field: false,
                },
            );
        }

        format_rules
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

            // Extract template configuration fields for persistence (same as create_state_row)
            let config_format = template.format.as_str().to_string();
            let protocol_revision = template.protocol_revision.clone();
            let container_type = match template.config_mapping.container_type {
                crate::clients::models::ContainerType::ObjectMap => "object",
                crate::clients::models::ContainerType::Array => "array",
            };
            let container_keys = serde_json::to_string(&template.config_mapping.container_keys).ok();
            let storage_kind = match template.storage.kind {
                crate::clients::models::StorageKind::File => "file",
                crate::clients::models::StorageKind::Kv => "kv",
                crate::clients::models::StorageKind::Custom => "custom",
            };
            let storage_adapter = template.storage.adapter.clone();
            let storage_path_strategy = template.storage.path_strategy.clone();
            let merge_strategy = match template.config_mapping.merge_strategy {
                crate::clients::models::MergeStrategy::Replace => "replace",
                crate::clients::models::MergeStrategy::DeepMerge => "deep_merge",
            };
            let keep_original_config = if template.config_mapping.keep_original_config { 1_i64 } else { 0_i64 };
            let managed_source = template.config_mapping.managed_source.clone();
            let format_rules = if template.config_mapping.format_rules.is_empty() {
                None
            } else {
                serde_json::to_string(&template.config_mapping.format_rules).ok()
            };

            // Extract Meta information from template
            let runtime_metadata = serde_json::json!({
                "runtime_client": {
                    "description": template.metadata.get("description").and_then(|v| v.as_str()),
                    "homepage_url": template.metadata.get("homepage_url").and_then(|v| v.as_str()),
                    "docs_url": template.metadata.get("docs_url").and_then(|v| v.as_str()),
                    "support_url": template.metadata.get("support_url").and_then(|v| v.as_str()),
                    "logo_url": template.metadata.get("logo_url").and_then(|v| v.as_str()),
                    "category": template.metadata.get("category").and_then(|v| v.as_str()),
                    "supported_transports": template.metadata.get("supported_transports")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
                        .unwrap_or_default()
                }
            });
            let approval_metadata = serde_json::to_string(&runtime_metadata).ok();

            sqlx::query(
                r#"
                INSERT INTO client (
                    id, name, display_name, identifier, config_path, managed, backup_policy, backup_limit,
                    capability_source, governance_kind, connection_mode, approval_status, record_kind, template_identifier,
                    config_format, protocol_revision, container_type, container_keys,
                    storage_kind, storage_adapter, storage_path_strategy,
                    merge_strategy, keep_original_config, managed_source, format_rules, approval_metadata
                )
                VALUES (?, ?, ?, ?, ?, 1, 'keep_n', 5, 'activated', 'passive', 'local_config_detected', 'approved', 'template_known', ?,
                        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(identifier) DO UPDATE SET
                    display_name = COALESCE(NULLIF(client.display_name, ''), excluded.display_name),
                    config_path = COALESCE(NULLIF(client.config_path, ''), excluded.config_path),
                    record_kind = COALESCE(NULLIF(client.record_kind, ''), excluded.record_kind),
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
                    format_rules = excluded.format_rules,
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
