use crate::clients::TemplateEngine;
use crate::clients::detector::{ClientDetector, DetectedClient};
use crate::clients::engine::TemplateExecutionResult;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{
    BackupPolicySetting, ClientCapabilityConfig, ClientTemplate, ConfigMode, ServerTemplateInput, TemplateFormat,
};
use crate::clients::source::{ClientConfigSource, FileTemplateSource, TemplateRoot};
use crate::system::paths::PathService;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::fs as std_fs;
use std::sync::Arc;

// Generated at build time from the repository's config/client directory
include!(concat!(env!("OUT_DIR"), "/official_templates_generated.rs"));

fn seed_official_templates(dir: &std::path::Path) -> crate::clients::error::ConfigResult<()> {
    for (file_name, contents) in OFFICIAL_TEMPLATES {
        let path = dir.join(file_name);
        if let Some(parent) = path.parent() {
            std_fs::create_dir_all(parent).map_err(crate::clients::error::ConfigError::IoError)?;
        }

        let needs_write = match std_fs::read_to_string(&path) {
            Ok(existing) => existing != *contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => true,
        };

        if needs_write {
            std_fs::write(&path, contents).map_err(crate::clients::error::ConfigError::IoError)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow, Default)]
pub(super) struct ClientStateRow {
    pub(super) id: String,
    pub(super) identifier: String,
    pub(super) name: String,
    pub(super) managed: i64,
    pub(super) config_mode: String,
    pub(super) transport: Option<String>,
    pub(super) client_version: Option<String>,
    pub(super) backup_policy: Option<String>,
    pub(super) backup_limit: Option<i64>,
    pub(super) capability_source: Option<String>,
    pub(super) selected_profile_ids: Option<String>,
    pub(super) custom_profile_id: Option<String>,
}

impl ClientStateRow {
    pub(super) fn to_setting(&self) -> BackupPolicySetting {
        BackupPolicySetting::from_pair(
            self.backup_policy.as_deref(),
            self.backup_limit.map(|value| value.max(0) as u32),
        )
    }

    pub(super) fn managed(&self) -> bool {
        self.managed != 0
    }

    pub(super) fn capability_config(&self) -> ConfigResult<ClientCapabilityConfig> {
        ClientCapabilityConfig::from_parts(
            self.capability_source.as_deref(),
            self.selected_profile_ids.as_deref(),
            self.custom_profile_id.clone(),
        )
        .map_err(ConfigError::DataAccessError)
    }
}

/// Summarized view of a client template combined with detection and filesystem state
#[derive(Debug, Clone)]
pub struct ClientDescriptor {
    pub template: ClientTemplate,
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
    pub(super) template_source: Arc<FileTemplateSource>,
    pub(super) template_engine: Arc<TemplateEngine>,
    pub(super) detector: Arc<ClientDetector>,
    pub(super) db_pool: Arc<SqlitePool>,
}

impl ClientConfigService {
    /// Bootstrap service with default template root resolution
    pub async fn bootstrap(db_pool: Arc<SqlitePool>) -> crate::clients::error::ConfigResult<Self> {
        let template_root = TemplateRoot::resolve()?;
        template_root.ensure_base_dirs()?;
        let official_dir = template_root.official_dir();
        seed_official_templates(&official_dir)?;
        // Ensure keymap defaults file exists on first run
        let _ = crate::clients::keymap::reload();
        let source = Arc::new(FileTemplateSource::bootstrap(template_root).await?);
        Self::with_source(db_pool, source).await
    }

    /// Initialize service with pre-built template source (primarily for tests)
    pub async fn with_source(
        db_pool: Arc<SqlitePool>,
        template_source: Arc<FileTemplateSource>,
    ) -> crate::clients::error::ConfigResult<Self> {
        let source_dyn: Arc<dyn ClientConfigSource> = template_source.clone();
        let engine = TemplateEngine::with_defaults(source_dyn.clone());
        let detector = ClientDetector::new(source_dyn)?;

        Ok(Self {
            template_source,
            template_engine: Arc::new(engine),
            detector: Arc::new(detector),
            db_pool,
        })
    }

    /// Reload templates from disk, keeping previous index if reloading fails
    pub async fn reload_templates(&self) -> crate::clients::error::ConfigResult<()> {
        self.template_source.reload().await?;
        // Reload keymap registry alongside templates
        let _ = crate::clients::keymap::reload();
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
        let template = self.get_client_template(client_id).await?;
        let storage = self.template_engine.storage_for_template(&template)?;
        storage.read(&template).await
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
        let platform = PathService::get_current_platform();
        self.template_source.get_config_path(client_id, platform).await
    }
}
