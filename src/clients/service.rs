use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{FromRow, SqlitePool};
use std::fs as std_fs;

use crate::clients::detector::{ClientDetector, DetectedClient};
use crate::clients::engine::{RenderRequest, TemplateExecutionResult};
use crate::clients::error::ConfigResult;
use crate::clients::models::{BackupPolicySetting, ClientTemplate, ConfigMode, ServerTemplateInput, TemplateFormat};
use crate::clients::source::{ClientConfigSource, FileTemplateSource, TemplateRoot};
use crate::clients::storage::BackupFile;
use crate::clients::{ConfigError, TemplateEngine};
use crate::common::constants::defaults;
use crate::common::constants::timeouts;
use crate::config::profile::basic::get_active_profile;
use crate::config::server::{args::get_server_args, env::get_server_env};
use crate::generate_id;
use crate::system::paths::{PathService, get_path_service};

// Generated at build time from the repository's config/client directory
include!(concat!(env!("OUT_DIR"), "/official_templates_generated.rs"));

fn seed_official_templates(dir: &std::path::Path) -> ConfigResult<()> {
    for (file_name, contents) in OFFICIAL_TEMPLATES {
        let path = dir.join(file_name);
        if let Some(parent) = path.parent() {
            std_fs::create_dir_all(parent).map_err(ConfigError::IoError)?;
        }

        let needs_write = match std_fs::read_to_string(&path) {
            Ok(existing) => existing != *contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => true,
        };

        if needs_write {
            std_fs::write(&path, contents).map_err(ConfigError::IoError)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, FromRow, Default)]
struct ClientStateRow {
    id: String,
    identifier: String,
    name: String,
    managed: i64,
    backup_policy: Option<String>,
    backup_limit: Option<i64>,
}

impl ClientStateRow {
    fn to_setting(&self) -> BackupPolicySetting {
        BackupPolicySetting::from_pair(
            self.backup_policy.as_deref(),
            self.backup_limit.map(|value| value.max(0) as u32),
        )
    }

    fn managed(&self) -> bool {
        self.managed != 0
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
    template_source: Arc<FileTemplateSource>,
    template_engine: Arc<TemplateEngine>,
    detector: Arc<ClientDetector>,
    db_pool: Arc<SqlitePool>,
}

impl ClientConfigService {
    /// Bootstrap service with default template root resolution
    pub async fn bootstrap(db_pool: Arc<SqlitePool>) -> ConfigResult<Self> {
        let template_root = TemplateRoot::resolve()?;
        template_root.ensure_base_dirs()?;
        let official_dir = template_root.official_dir();
        seed_official_templates(&official_dir)?;
        let source = Arc::new(FileTemplateSource::bootstrap(template_root).await?);
        Self::with_source(db_pool, source).await
    }

    /// Initialize service with pre-built template source (primarily for tests)
    pub async fn with_source(
        db_pool: Arc<SqlitePool>,
        template_source: Arc<FileTemplateSource>,
    ) -> ConfigResult<Self> {
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
    pub async fn reload_templates(&self) -> ConfigResult<()> {
        self.template_source.reload().await
    }

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

        Ok(results)
    }

    /// Get template for client identifier on current platform
    pub async fn get_client_template(
        &self,
        client_id: &str,
    ) -> ConfigResult<ClientTemplate> {
        self.template_source
            .get_template(client_id, PathService::get_current_platform())
            .await?
            .ok_or_else(|| ConfigError::TemplateIndexError(format!("未找到客户端 {} 的模板", client_id)))
    }

    /// Read current configuration file content for a client
    pub async fn read_current_config(
        &self,
        client_id: &str,
    ) -> ConfigResult<Option<String>> {
        let template = self.get_client_template(client_id).await?;
        let storage = self.template_engine.storage_for_template(&template)?;
        storage.read(&template).await
    }

    /// Get resolved configuration path for a client on current platform
    pub async fn config_path(
        &self,
        client_id: &str,
    ) -> ConfigResult<Option<String>> {
        self.resolved_config_path(client_id).await
    }

    /// Execute configuration generation (dry-run or apply)
    pub async fn execute_render(
        &self,
        options: ClientRenderOptions,
    ) -> ConfigResult<ClientRenderResult> {
        if !self.is_client_managed(&options.client_id).await? {
            return Err(ConfigError::ClientDisabled {
                identifier: options.client_id.clone(),
            });
        }

        let template = self.get_client_template(&options.client_id).await?;
        let mut servers = self.prepare_servers(&options).await?;
        let backup_policy = self.get_backup_policy(&options.client_id).await?;

        if matches!(options.mode, ConfigMode::Native) {
            let supported: HashSet<String> = template.config_mapping.format_rules.keys().cloned().collect();
            let before = servers.len();
            servers.retain(|server| supported.contains(&server.transport));
            if before != servers.len() {
                tracing::debug!(
                    "Filtered unsupported servers for client {} (native mode): {} -> {}",
                    options.client_id,
                    before,
                    servers.len()
                );
            }
        }

        let request = RenderRequest {
            client_id: &options.client_id,
            servers: &servers,
            mode: options.mode.clone(),
            profile_id: options.profile_id.as_deref(),
            dry_run: options.dry_run,
            backup_policy: &backup_policy,
        };

        let execution = self.template_engine.render_config(request).await?;
        let target_path = self.resolved_config_path(&options.client_id).await?;

        Ok(ClientRenderResult {
            execution,
            target_path,
            servers,
        })
    }

    async fn resolved_config_path(
        &self,
        client_id: &str,
    ) -> ConfigResult<Option<String>> {
        let platform = PathService::get_current_platform();
        let resolved = self.template_source.get_config_path(client_id, platform).await?;

        let Some(raw_path) = resolved else {
            return Ok(None);
        };

        let path_service = get_path_service();
        let expanded = path_service
            .resolve_user_path(&raw_path)
            .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;

        Ok(Some(expanded.to_string_lossy().to_string()))
    }

    async fn prepare_servers(
        &self,
        options: &ClientRenderOptions,
    ) -> ConfigResult<Vec<ServerTemplateInput>> {
        let selection = self.resolve_server_selection(options).await?;
        let rows = self.fetch_servers(selection).await?;

        let mut servers = Vec::with_capacity(rows.len());
        for row in rows {
            servers.push(self.map_server_row(row).await?);
        }

        Ok(servers)
    }

    fn preview_from_execution(exec: &TemplateExecutionResult) -> PreviewOutcome {
        match exec {
            TemplateExecutionResult::DryRun { diff, .. } => PreviewOutcome {
                format: diff.format,
                before: diff.before.clone(),
                after: diff.after.clone(),
                summary: diff.summary.clone(),
            },
            _ => PreviewOutcome {
                format: TemplateFormat::Json,
                before: None,
                after: None,
                summary: None,
            },
        }
    }

    pub async fn apply_or_preview(
        &self,
        options: ClientRenderOptions,
    ) -> ConfigResult<ApplyOutcome> {
        let result = self.execute_render(options.clone()).await?;
        let mut outcome = ApplyOutcome::default();
        outcome.preview = Self::preview_from_execution(&result.execution);
        match result.execution {
            TemplateExecutionResult::Applied { backup_path, .. } => {
                outcome.applied = true;
                outcome.backup_path = backup_path;
            }
            _ => {}
        }
        Ok(outcome)
    }

    pub async fn apply_with_deferred(
        &self,
        options: ClientRenderOptions,
    ) -> ConfigResult<ApplyOutcome> {
        // Always compute a preview via dry-run first for stable diff/preview fields.
        let mut preview_opts = options.clone();
        preview_opts.dry_run = true;
        let preview_outcome = self.apply_or_preview(preview_opts).await?;

        // If the original request is a preview, return directly
        if options.dry_run {
            return Ok(preview_outcome);
        }

        // If not a preview: try to write
        match self.execute_render(options.clone()).await {
            Ok(exec) => {
                let mut out = preview_outcome;
                if let TemplateExecutionResult::Applied { backup_path, .. } = exec.execution {
                    out.applied = true;
                    out.backup_path = backup_path;
                }
                Ok(out)
            }
            Err(ConfigError::FileOperationError(msg)) if msg.to_ascii_lowercase().contains("locked") => {
                // Only Cherry triggers delayed write
                let template = self.get_client_template(&options.client_id).await?;
                let is_cherry = template.storage.kind == crate::clients::models::StorageKind::Kv
                    && template.storage.adapter.as_deref() == Some("cherry_kv");
                if !is_cherry {
                    return Err(ConfigError::FileOperationError(msg));
                }

                let merged_after = preview_outcome.preview.after.clone().unwrap_or_default();
                let policy = self.get_backup_policy(&options.client_id).await?;
                self.schedule_write_after_unlock(template, merged_after, policy)?;

                let mut out = preview_outcome;
                out.scheduled = true;
                out.scheduled_reason = Some("db_locked".into());
                Ok(out)
            }
            Err(e) => Err(e),
        }
    }

    async fn resolve_server_selection(
        &self,
        options: &ClientRenderOptions,
    ) -> ConfigResult<ServerSelection> {
        if matches!(options.mode, ConfigMode::Native) {
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
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        let mut active_ids: Vec<String> = active_profiles.into_iter().filter_map(|profile| profile.id).collect();
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
                    SELECT id, name, command, url, transport_type, server_type
                    FROM server_config
                    WHERE enabled = 1
                    ORDER BY name
                    "#,
                )
                .fetch_all(&*self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

                Ok(rows)
            }
            ServerSelection::Profile(profile_id) => {
                let rows = sqlx::query_as::<_, ServerRow>(
                    r#"
                    SELECT sc.id, sc.name, sc.command, sc.url, sc.transport_type, sc.server_type
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
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

                Ok(rows)
            }
            ServerSelection::Profiles(profile_ids) => {
                if profile_ids.is_empty() {
                    return Ok(Vec::new());
                }

                let placeholders = vec!["?"; profile_ids.len()].join(", ");
                let sql = format!(
                    r#"
                    SELECT DISTINCT
                        sc.id,
                        sc.name,
                        sc.command,
                        sc.url,
                        sc.transport_type,
                        sc.server_type
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
                    .map_err(|err| ConfigError::DataAccessError(err.to_string()))
            }
            ServerSelection::Explicit(ids) => {
                if ids.is_empty() {
                    return Ok(Vec::new());
                }

                let placeholders = vec!["?"; ids.len()].join(", ");
                let sql = format!(
                    r#"
                    SELECT id, name, command, url, transport_type, server_type
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
                    .map_err(|err| ConfigError::DataAccessError(err.to_string()))
            }
        }
    }

    async fn fetch_client_states(&self) -> ConfigResult<HashMap<String, ClientStateRow>> {
        let rows = sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, managed, backup_policy, backup_limit FROM client",
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(rows.into_iter().map(|row| (row.identifier.clone(), row)).collect())
    }

    async fn ensure_state_row(
        &self,
        identifier: &str,
    ) -> ConfigResult<ClientStateRow> {
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await
    }

    async fn ensure_state_row_with_name(
        &self,
        identifier: &str,
        name: &str,
    ) -> ConfigResult<ClientStateRow> {
        if let Some(existing) = self.fetch_state(identifier).await? {
            if existing.name != name {
                sqlx::query(
                    r#"
                    UPDATE client
                    SET name = ?, updated_at = CURRENT_TIMESTAMP
                    WHERE identifier = ?
                    "#,
                )
                .bind(name)
                .bind(identifier)
                .execute(&*self.db_pool)
                .await
                .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

                return self
                    .fetch_state(identifier)
                    .await?
                    .ok_or_else(|| ConfigError::DataAccessError(format!("未能更新客户端 {} 的管理状态", identifier)));
            }

            return Ok(existing);
        }

        let generated_id = generate_id!("clnt");
        sqlx::query(
            r#"
            INSERT INTO client (id, name, identifier, managed, backup_policy, backup_limit)
            VALUES (?, ?, ?, 1, 'keep_n', 30)
            "#,
        )
        .bind(&generated_id)
        .bind(name)
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        self.fetch_state(identifier)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("未能创建客户端 {} 的管理状态", identifier)))
    }

    async fn fetch_state(
        &self,
        identifier: &str,
    ) -> ConfigResult<Option<ClientStateRow>> {
        sqlx::query_as::<_, ClientStateRow>(
            "SELECT id, identifier, name, managed, backup_policy, backup_limit FROM client WHERE identifier = ?",
        )
        .bind(identifier)
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))
    }

    async fn resolve_client_name(
        &self,
        identifier: &str,
    ) -> ConfigResult<String> {
        let platform = PathService::get_current_platform();
        if let Some(template) = self.template_source.get_template(identifier, platform).await? {
            if let Some(display_name) = template.display_name {
                return Ok(display_name);
            }
        }

        Ok(identifier.to_string())
    }

    pub async fn set_client_managed(
        &self,
        identifier: &str,
        managed: bool,
    ) -> ConfigResult<bool> {
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await?;

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?, managed = ?, updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(name)
        .bind(if managed { 1 } else { 0 })
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(managed)
    }

    pub async fn is_client_managed(
        &self,
        identifier: &str,
    ) -> ConfigResult<bool> {
        let row = sqlx::query_scalar::<_, i64>("SELECT managed FROM client WHERE identifier = ?")
            .bind(identifier)
            .fetch_optional(&*self.db_pool)
            .await
            .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        match row {
            Some(value) => Ok(value != 0),
            None => {
                let state = self.ensure_state_row(identifier).await?;
                Ok(state.managed())
            }
        }
    }

    pub async fn get_backup_policy(
        &self,
        identifier: &str,
    ) -> ConfigResult<BackupPolicySetting> {
        let state = self.ensure_state_row(identifier).await?;
        Ok(state.to_setting())
    }

    pub async fn set_backup_policy(
        &self,
        identifier: &str,
        policy: BackupPolicySetting,
    ) -> ConfigResult<BackupPolicySetting> {
        let (policy_label, limit) = policy.as_db_pair();
        let name = self.resolve_client_name(identifier).await?;
        self.ensure_state_row_with_name(identifier, &name).await?;

        sqlx::query(
            r#"
            UPDATE client
            SET name = ?,
                backup_policy = ?,
                backup_limit = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE identifier = ?
            "#,
        )
        .bind(name)
        .bind(policy_label)
        .bind(limit.map(|v| v as i64))
        .bind(identifier)
        .execute(&*self.db_pool)
        .await
        .map_err(|err| ConfigError::DataAccessError(err.to_string()))?;

        Ok(policy)
    }

    pub async fn list_backups(
        &self,
        identifier: Option<&str>,
    ) -> ConfigResult<Vec<ClientBackupRecord>> {
        let templates = if let Some(id) = identifier {
            vec![self.get_client_template(id).await?]
        } else {
            self.template_source.list_client().await?
        };

        let mut records = Vec::new();
        for template in templates {
            let storage = self.template_engine.storage_for_template(&template)?;
            let backups = storage.list_backups(&template).await?;
            for backup in backups {
                records.push(Self::map_backup_record(&template.identifier, backup));
            }
        }

        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(records)
    }

    pub async fn delete_backup(
        &self,
        identifier: &str,
        backup: &str,
    ) -> ConfigResult<()> {
        let template = self.get_client_template(identifier).await?;
        let storage = self.template_engine.storage_for_template(&template)?;
        storage.delete_backup(&template, backup).await
    }

    pub async fn restore_backup(
        &self,
        identifier: &str,
        backup: &str,
    ) -> ConfigResult<Option<String>> {
        let template = self.get_client_template(identifier).await?;
        let storage = self.template_engine.storage_for_template(&template)?;
        let policy = self.get_backup_policy(identifier).await?;
        storage.restore_backup(&template, backup, &policy).await
    }

    /// Schedule a background write when the target storage is temporarily locked.
    pub fn schedule_write_after_unlock(
        &self,
        template: ClientTemplate,
        content: String,
        policy: BackupPolicySetting,
    ) -> ConfigResult<()> {
        let storage = self.template_engine.storage_for_template(&template)?;
        let identifier = template.identifier.clone();
        let retry_window = std::time::Duration::from_secs(timeouts::CHERRY_LOCK_RETRY_WINDOW_SEC);
        let interval = std::time::Duration::from_millis(timeouts::CHERRY_LOCK_RETRY_INTERVAL_MS);

        tokio::spawn(async move {
            use std::time::Instant;
            let deadline = Instant::now() + retry_window;
            loop {
                match storage.write_atomic(&template, &content, &policy).await {
                    Ok(_) => {
                        tracing::info!(
                            client = %identifier,
                            "Deferred Cherry config write applied after unlock"
                        );
                        break;
                    }
                    Err(ConfigError::FileOperationError(msg)) if msg.to_ascii_lowercase().contains("locked") => {
                        if Instant::now() >= deadline {
                            tracing::error!(
                                client = %identifier,
                                "Deferred write window expired; database remained locked"
                            );
                            break;
                        }
                        tokio::time::sleep(interval).await;
                    }
                    Err(err) => {
                        tracing::error!(
                            client = %identifier,
                            error = %err,
                            "Deferred write aborted due to non-lock error"
                        );
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    fn map_backup_record(
        identifier: &str,
        backup: BackupFile,
    ) -> ClientBackupRecord {
        ClientBackupRecord {
            identifier: identifier.to_string(),
            backup: backup.name,
            path: backup.path.to_string_lossy().to_string(),
            size: backup.size,
            created_at: backup.modified_at,
        }
    }

    async fn map_server_row(
        &self,
        row: ServerRow,
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

        let transport = row
            .transport_type
            .as_deref()
            .unwrap_or_else(|| row.server_type.as_str())
            .to_string();

        let mut metadata = HashMap::new();
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
            headers: HashMap::new(),
            metadata,
        })
    }
}

fn sanitize_server_name(name: &str) -> String {
    name.replace(' ', "_")
}

#[derive(Debug, Clone)]
enum ServerSelection {
    AllEnabled,
    Profile(String),
    Profiles(Vec<String>),
    Explicit(Vec<String>),
}

#[derive(Debug, Clone, FromRow)]
struct ServerRow {
    id: String,
    name: String,
    command: Option<String>,
    url: Option<String>,
    transport_type: Option<String>,
    server_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
    use std::fs;
    use tempfile::tempdir;

    async fn memory_pool() -> SqlitePool {
        // In-memory SQLite pool for tests. Ensure schema is initialized
        // so tests that touch DB-backed client state (e.g., `client` table)
        // do not fail with "no such table" errors.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");

        // Run the same initialization as production startup to create tables
        // (server, client, profile, linking tables, etc.).
        crate::config::initialization::run_initialization(&pool)
            .await
            .expect("initialize in-memory schema");

        pool
    }

    async fn bootstrap_service_in(dir: &std::path::Path) -> ClientConfigService {
        let template_root = TemplateRoot::new(dir.to_path_buf());
        template_root.ensure_base_dirs().expect("ensure dirs");
        seed_official_templates(&template_root.official_dir()).expect("seed templates");
        let source = Arc::new(
            FileTemplateSource::bootstrap(template_root)
                .await
                .expect("bootstrap source"),
        );
        ClientConfigService::with_source(Arc::new(memory_pool().await), source)
            .await
            .expect("service")
    }

    #[tokio::test]
    async fn seeds_official_templates_on_bootstrap() {
        let temp_dir = tempdir().expect("temp dir");
        let service = bootstrap_service_in(temp_dir.path()).await;
        let templates = service.template_source.list_client().await.expect("list templates");
        let ids: Vec<_> = templates.iter().map(|tpl| tpl.identifier.as_str()).collect();
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"zed"));
        assert!(ids.contains(&"codex"));
        assert!(temp_dir.path().join("official/cursor.json5").exists());
    }

    #[tokio::test]
    async fn cursor_native_dry_run_renders_stdio_server() {
        let temp_dir = tempdir().expect("temp dir");
        let service = bootstrap_service_in(temp_dir.path()).await;
        let source = service.template_source.clone() as Arc<dyn ClientConfigSource>;
        let engine = TemplateEngine::with_defaults(source);
        let server = ServerTemplateInput {
            name: "sample-server".to_string(),
            display_name: Some("Sample Server".to_string()),
            transport: "stdio".to_string(),
            command: Some("uvx".to_string()),
            args: vec!["run".to_string(), "tool".to_string()],
            env: HashMap::from([("KEY".to_string(), "VALUE".to_string())]),
            url: None,
            headers: HashMap::new(),
            metadata: HashMap::new(),
        };
        let policy = BackupPolicySetting::default();
        let request = RenderRequest {
            client_id: "cursor",
            servers: &[server.clone()],
            mode: ConfigMode::Native,
            profile_id: None,
            dry_run: true,
            backup_policy: &policy,
        };
        let result = engine.render_config(request).await.expect("render cursor");
        match result {
            TemplateExecutionResult::DryRun { content, .. } => {
                assert!(content.contains("sample-server"));
                assert!(content.contains("\"command\""));
            }
            other => panic!("expected dry-run, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn cursor_managed_dry_run_includes_proxy_endpoint() {
        let temp_dir = tempdir().expect("temp dir");
        let service = bootstrap_service_in(temp_dir.path()).await;
        let source = service.template_source.clone() as Arc<dyn ClientConfigSource>;
        let engine = TemplateEngine::with_defaults(source);
        let server = ServerTemplateInput {
            name: "upstream".to_string(),
            display_name: None,
            transport: "streamable_http".to_string(),
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: Some("https://example.com".to_string()),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        };
        let policy = BackupPolicySetting::default();
        let request = RenderRequest {
            client_id: "cursor",
            servers: &[server],
            mode: ConfigMode::Managed,
            profile_id: Some("profile-demo"),
            dry_run: true,
            backup_policy: &policy,
        };
        let result = engine.render_config(request).await.expect("render managed");
        match result {
            TemplateExecutionResult::DryRun { content, .. } => {
                let json: serde_json::Value = serde_json::from_str(&content).expect("managed json");
                let server = &json["mcpServers"]["mcpmate"];
                assert_eq!(server["url"], "http://localhost:8000/mcp");
            }
            other => panic!("expected dry-run, got {:?}", other),
        }
    }

    #[test]
    fn updates_official_templates_when_changed() {
        let temp_dir = tempdir().expect("temp dir");
        let official_dir = temp_dir.path().join("official");
        fs::create_dir_all(&official_dir).expect("create official dir");

        let path = official_dir.join("zed.json5");
        fs::write(&path, "outdated").expect("write outdated template");

        seed_official_templates(&official_dir).expect("seed templates");

        let updated = fs::read_to_string(path).expect("read updated template");
        assert_eq!(updated, include_str!("../../config/client/zed.json5"));
    }

    #[tokio::test]
    async fn toggles_client_management_state() {
        let temp_dir = tempdir().expect("temp dir");
        let service = bootstrap_service_in(temp_dir.path()).await;

        assert!(service.is_client_managed("cursor").await.expect("managed default"));

        service
            .set_client_managed("cursor", false)
            .await
            .expect("disable cursor");

        assert!(!service.is_client_managed("cursor").await.expect("cursor disabled"));

        service.set_client_managed("cursor", true).await.expect("enable cursor");

        assert!(service.is_client_managed("cursor").await.expect("cursor enabled"));
    }
}
