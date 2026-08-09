use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::fs;
use uuid::Uuid;

use crate::clients::ClientConfigService;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{FirstContactBehavior, OnboardingPolicy};
use crate::common::MCPMatePaths;
use crate::common::constants::ports;
use crate::common::paths::global_paths;
use crate::config::client::init::DEFAULT_CONFIG_MODE;
use crate::system::config::init_port_config;
use crate::system::paths::PathService;

pub const DEFAULT_INSPECTOR_TIMEOUT_MS: u64 = 8_000;
pub const DEFAULT_CLIENT_DISCOVERY_SNAPSHOT_TTL_SECONDS: i64 = 21_600;
const SETTINGS_BACKUP_LIMIT: usize = 5;

const fn default_client_discovery_snapshot_ttl_seconds() -> i64 {
    DEFAULT_CLIENT_DISCOVERY_SNAPSHOT_TTL_SECONDS
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemSettings {
    pub api_port: u16,
    pub mcp_port: u16,
    pub first_contact_behavior: FirstContactBehavior,
    pub inspector_timeout_ms: u64,
    pub default_config_mode: String,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default = "default_client_discovery_snapshot_ttl_seconds")]
    pub client_discovery_snapshot_ttl_seconds: i64,
    #[serde(default)]
    pub client_discovery_snapshot_last_success_at: Option<String>,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            api_port: ports::API_PORT,
            mcp_port: ports::MCP_PORT,
            first_contact_behavior: FirstContactBehavior::default(),
            inspector_timeout_ms: DEFAULT_INSPECTOR_TIMEOUT_MS,
            default_config_mode: DEFAULT_CONFIG_MODE.to_string(),
            onboarding_completed: false,
            client_discovery_snapshot_ttl_seconds: DEFAULT_CLIENT_DISCOVERY_SNAPSHOT_TTL_SECONDS,
            client_discovery_snapshot_last_success_at: None,
        }
    }
}

impl SystemSettings {
    pub fn onboarding_policy(&self) -> OnboardingPolicy {
        onboarding_policy_from_behavior(self.first_contact_behavior)
    }

    fn validate(&self) -> ConfigResult<()> {
        if self.api_port == 0 {
            return Err(ConfigError::DataAccessError("invalid api port: 0".to_string()));
        }

        if self.mcp_port == 0 {
            return Err(ConfigError::DataAccessError("invalid mcp port: 0".to_string()));
        }

        if self.api_port == self.mcp_port {
            return Err(ConfigError::DataAccessError(
                "api port and mcp port cannot be the same".to_string(),
            ));
        }

        if !matches!(self.default_config_mode.as_str(), "unify" | "hosted" | "transparent") {
            return Err(ConfigError::DataAccessError(format!(
                "invalid default client config mode: {}",
                self.default_config_mode
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SystemSettingsUpdate {
    pub api_port: Option<u16>,
    pub mcp_port: Option<u16>,
    pub first_contact_behavior: Option<FirstContactBehavior>,
    pub inspector_timeout_ms: Option<u64>,
    pub default_config_mode: Option<String>,
}

impl SystemSettingsUpdate {
    pub fn has_changes(&self) -> bool {
        self.api_port.is_some()
            || self.mcp_port.is_some()
            || self.first_contact_behavior.is_some()
            || self.inspector_timeout_ms.is_some()
            || self.default_config_mode.is_some()
    }

    fn apply_to(
        &self,
        settings: &mut SystemSettings,
    ) {
        if let Some(api_port) = self.api_port {
            settings.api_port = api_port;
        }
        if let Some(mcp_port) = self.mcp_port {
            settings.mcp_port = mcp_port;
        }
        if let Some(first_contact_behavior) = self.first_contact_behavior {
            settings.first_contact_behavior = first_contact_behavior;
        }
        if let Some(inspector_timeout_ms) = self.inspector_timeout_ms {
            settings.inspector_timeout_ms = inspector_timeout_ms;
        }
        if let Some(default_config_mode) = &self.default_config_mode {
            settings.default_config_mode = default_config_mode.clone();
        }
    }
}

pub fn onboarding_policy_from_behavior(behavior: FirstContactBehavior) -> OnboardingPolicy {
    match behavior {
        FirstContactBehavior::Allow => OnboardingPolicy::AutoManage,
        FirstContactBehavior::Review => OnboardingPolicy::RequireApproval,
        FirstContactBehavior::Deny => OnboardingPolicy::Manual,
    }
}

pub fn behavior_from_onboarding_policy(policy: OnboardingPolicy) -> FirstContactBehavior {
    match policy {
        OnboardingPolicy::AutoManage => FirstContactBehavior::Allow,
        OnboardingPolicy::RequireApproval => FirstContactBehavior::Review,
        OnboardingPolicy::Manual => FirstContactBehavior::Deny,
    }
}

pub async fn initialize_settings_file(pool: &SqlitePool) -> ConfigResult<()> {
    let path = settings_path(pool);

    match fs::metadata(&path).await {
        Ok(_) => {
            read_settings_async(&path).await?;
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            write_settings(pool, &SystemSettings::default()).await
        }
        Err(err) => Err(ConfigError::IoError(err)),
    }
}

pub async fn get_settings(pool: &SqlitePool) -> ConfigResult<SystemSettings> {
    read_settings_async(&settings_path(pool)).await
}

pub async fn set_settings(
    pool: &SqlitePool,
    settings: &SystemSettings,
) -> ConfigResult<SystemSettings> {
    let settings = settings.clone();
    let (settings, ()) = mutate_settings_at_path(&settings_path(pool), move |current| {
        *current = settings;
        Ok(())
    })
    .await?;
    Ok(settings)
}

/// Outcome of [`apply_settings_with_effects`].
pub struct SystemSettingsApplyResult {
    /// `api_port` changed between the lock-held current settings and the applied settings.
    pub api_port_changed: bool,
    /// `mcp_port` changed between the lock-held current settings and the applied settings.
    pub mcp_port_changed: bool,
    /// Full applied settings snapshot.
    pub settings: SystemSettings,
    /// Background re-apply task started for an MCP-port change.
    pub client_reapply_task: Option<tokio::task::JoinHandle<ConfigResult<crate::clients::HostedClientReapplySummary>>>,
}

/// Apply system settings with all required side effects.
///
/// Converges settings persistence, port-config refresh, and hosted/managed client re-apply
/// into a single entry point used by both the REST API and the Tauri shell.
///
/// - `previous -> next` expresses only the fields this caller explicitly intends to change.
///   The function reloads the current settings under the path lock, applies those explicit changes,
///   and derives port side effects from the lock-held current settings and applied settings.
/// - `client_service` is optional — if absent and `mcp_port_changed` is true, a temporary
///   service is bootstrapped in the background for the re-apply side effect.
pub async fn apply_settings_with_effects(
    pool: &SqlitePool,
    previous: &SystemSettings,
    next: &SystemSettings,
    client_service: Option<Arc<ClientConfigService>>,
) -> ConfigResult<SystemSettingsApplyResult> {
    apply_settings_with_effects_at_path(
        &settings_path(pool),
        previous,
        next,
        client_service,
        Some(Arc::new(pool.clone())),
    )
    .await
}

pub async fn apply_settings_with_effects_for_paths(
    paths: &MCPMatePaths,
    previous: &SystemSettings,
    next: &SystemSettings,
    client_service: Option<Arc<ClientConfigService>>,
) -> ConfigResult<SystemSettingsApplyResult> {
    apply_settings_with_effects_at_path(&paths.config_path(), previous, next, client_service, None).await
}

pub async fn apply_settings_with_effects_for_paths_and_pool(
    paths: &MCPMatePaths,
    pool: &SqlitePool,
    previous: &SystemSettings,
    next: &SystemSettings,
    client_service: Option<Arc<ClientConfigService>>,
) -> ConfigResult<SystemSettingsApplyResult> {
    apply_settings_with_effects_at_path(
        &paths.config_path(),
        previous,
        next,
        client_service,
        Some(Arc::new(pool.clone())),
    )
    .await
}

pub async fn apply_settings_update_with_effects(
    pool: &SqlitePool,
    update: &SystemSettingsUpdate,
    client_service: Option<Arc<ClientConfigService>>,
) -> ConfigResult<SystemSettingsApplyResult> {
    apply_settings_mutation_with_effects_at_path(
        &settings_path(pool),
        client_service,
        Some(Arc::new(pool.clone())),
        |settings| {
            update.apply_to(settings);
            Ok(())
        },
    )
    .await
}

async fn apply_settings_with_effects_at_path(
    path: &Path,
    previous: &SystemSettings,
    next: &SystemSettings,
    client_service: Option<Arc<ClientConfigService>>,
    pool: Option<Arc<SqlitePool>>,
) -> ConfigResult<SystemSettingsApplyResult> {
    apply_settings_mutation_with_effects_at_path(path, client_service, pool, |settings| {
        apply_explicit_settings_changes(settings, previous, next);
        Ok(())
    })
    .await
}

async fn apply_settings_mutation_with_effects_at_path<F>(
    path: &Path,
    client_service: Option<Arc<ClientConfigService>>,
    pool: Option<Arc<SqlitePool>>,
    mutate: F,
) -> ConfigResult<SystemSettingsApplyResult>
where
    F: FnOnce(&mut SystemSettings) -> ConfigResult<()>,
{
    let lock = settings_mutation_lock(path);
    let _guard = lock.lock().await;
    let previous = read_settings_async(path).await?;
    let mut next = previous.clone();
    mutate(&mut next)?;
    next.validate()?;

    let mode_transition = if previous.default_config_mode != next.default_config_mode {
        if client_service.is_none() {
            return Err(ConfigError::DataAccessError(
                "a client configuration service is required to converge a default configuration mode change"
                    .to_string(),
            ));
        }
        let pool = pool.as_deref().ok_or_else(|| {
            ConfigError::DataAccessError(
                "a database pool is required to converge a default configuration mode change".to_string(),
            )
        })?;
        Some(begin_configuration_mode_transition(pool, &previous.default_config_mode, &next.default_config_mode).await?)
    } else {
        None
    };

    write_settings_path_async(path, &next).await?;

    if let Some(transition_id) = mode_transition {
        complete_configuration_mode_transition(
            pool.as_deref().expect("mode transition requires database pool"),
            &transition_id,
            &next.default_config_mode,
            client_service
                .as_deref()
                .expect("mode transition requires client configuration service"),
        )
        .await?;
    }

    let api_port_changed = previous.api_port != next.api_port;
    let mcp_port_changed = previous.mcp_port != next.mcp_port;

    if api_port_changed || mcp_port_changed {
        init_port_config(next.api_port, next.mcp_port);
    }

    let client_reapply_task = if mcp_port_changed {
        Some(spawn_mcp_port_reapply(client_service, pool))
    } else {
        None
    };

    Ok(SystemSettingsApplyResult {
        api_port_changed,
        mcp_port_changed,
        settings: next.clone(),
        client_reapply_task,
    })
}

fn apply_explicit_settings_changes(
    current: &mut SystemSettings,
    previous: &SystemSettings,
    next: &SystemSettings,
) {
    if previous.api_port != next.api_port {
        current.api_port = next.api_port;
    }
    if previous.mcp_port != next.mcp_port {
        current.mcp_port = next.mcp_port;
    }
    if previous.first_contact_behavior != next.first_contact_behavior {
        current.first_contact_behavior = next.first_contact_behavior;
    }
    if previous.inspector_timeout_ms != next.inspector_timeout_ms {
        current.inspector_timeout_ms = next.inspector_timeout_ms;
    }
    if previous.default_config_mode != next.default_config_mode {
        current.default_config_mode = next.default_config_mode.clone();
    }
    if previous.onboarding_completed != next.onboarding_completed {
        current.onboarding_completed = next.onboarding_completed;
    }
    if previous.client_discovery_snapshot_ttl_seconds != next.client_discovery_snapshot_ttl_seconds {
        current.client_discovery_snapshot_ttl_seconds = next.client_discovery_snapshot_ttl_seconds;
    }
    if previous.client_discovery_snapshot_last_success_at != next.client_discovery_snapshot_last_success_at {
        current.client_discovery_snapshot_last_success_at = next.client_discovery_snapshot_last_success_at.clone();
    }
}

async fn begin_configuration_mode_transition(
    pool: &SqlitePool,
    previous_mode: &str,
    target_mode: &str,
) -> ConfigResult<String> {
    let transition_id = format!("configuration-mode-{}", Uuid::new_v4());
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO configuration_mode_transitions (
            transition_id, previous_mode, target_mode, status, created_at
        )
        VALUES (?, ?, ?, 'pending', CURRENT_TIMESTAMP)
        "#,
    )
    .bind(&transition_id)
    .bind(previous_mode)
    .bind(target_mode)
    .execute(&mut *transaction)
    .await
    .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    transaction
        .commit()
        .await
        .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    Ok(transition_id)
}

async fn complete_configuration_mode_transition(
    pool: &SqlitePool,
    transition_id: &str,
    target_mode: &str,
    client_service: &ClientConfigService,
) -> ConfigResult<()> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    let pending_target: Option<String> = sqlx::query_scalar(
        r#"
        SELECT target_mode
        FROM configuration_mode_transitions
        WHERE transition_id = ? AND status = 'pending'
        "#,
    )
    .bind(transition_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    let pending_target = pending_target.ok_or_else(|| {
        ConfigError::DataAccessError(format!(
            "pending configuration mode transition not found: {transition_id}"
        ))
    })?;
    if pending_target != target_mode {
        return Err(ConfigError::DataAccessError(format!(
            "configuration mode transition target mismatch: expected {pending_target}, received {target_mode}"
        )));
    }

    let trigger = crate::core::capability::materializer::MaterializationTrigger::for_consumer(
        "default_config_mode_transition",
        transition_id,
        "system_settings",
    );
    crate::core::capability::materializer::converge_inherited_consumers_for_default_mode_in_transaction(
        pool,
        &mut transaction,
        target_mode,
        &trigger,
    )
    .await
    .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    transaction
        .commit()
        .await
        .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;

    let reapply = client_service
        .reapply_inherited_clients_after_default_mode_change(target_mode)
        .await?;
    if !reapply.failures.is_empty() {
        return Err(ConfigError::DataAccessError(format!(
            "default configuration mode client convergence failed: {}",
            reapply
                .failures
                .iter()
                .map(|(client_id, error)| format!("{client_id}: {error}"))
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }

    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    let updated = sqlx::query(
        r#"
        UPDATE configuration_mode_transitions
        SET status = 'completed', completed_at = CURRENT_TIMESTAMP
        WHERE transition_id = ? AND status = 'pending'
        "#,
    )
    .bind(transition_id)
    .execute(&mut *transaction)
    .await
    .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    if updated.rows_affected() != 1 {
        return Err(ConfigError::DataAccessError(format!(
            "pending configuration mode transition not found: {transition_id}"
        )));
    }
    transaction
        .commit()
        .await
        .map_err(|error| ConfigError::DataAccessError(error.to_string()))
}

pub async fn resume_pending_configuration_mode_transitions(
    paths: &MCPMatePaths,
    pool: &SqlitePool,
    client_service: Option<Arc<ClientConfigService>>,
) -> ConfigResult<usize> {
    let pending = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT transition_id, target_mode
        FROM configuration_mode_transitions
        WHERE status = 'pending'
        ORDER BY created_at, transition_id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    let client_service = if pending.is_empty() {
        None
    } else {
        Some(client_service.ok_or_else(|| {
            ConfigError::DataAccessError(
                "a client configuration service is required to recover pending default mode transitions".to_string(),
            )
        })?)
    };
    for (transition_id, target_mode) in &pending {
        mutate_settings_at_path(&paths.config_path(), |settings| {
            settings.default_config_mode = target_mode.clone();
            Ok(())
        })
        .await?;
        complete_configuration_mode_transition(
            pool,
            transition_id,
            target_mode,
            client_service
                .as_deref()
                .expect("pending transition requires client configuration service"),
        )
        .await?;
    }
    Ok(pending.len())
}

pub fn spawn_mcp_port_reapply_result_logger(
    task: tokio::task::JoinHandle<ConfigResult<crate::clients::HostedClientReapplySummary>>
) {
    tokio::spawn(async move {
        match task.await {
            Ok(Ok(summary)) => {
                tracing::info!(
                    attempted = summary.attempted,
                    applied = summary.applied,
                    scheduled = summary.scheduled,
                    failures = summary.failures.len(),
                    "re-applied hosted/managed clients after MCP port change",
                );
            }
            Ok(Err(err)) => {
                tracing::error!(
                    error = %err,
                    "client re-apply after MCP port change failed",
                );
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "client re-apply task after MCP port change panicked",
                );
            }
        }
    });
}

pub fn get_settings_sync() -> ConfigResult<SystemSettings> {
    get_settings_sync_for_paths(global_paths())
}

pub fn get_settings_sync_for_paths(paths: &MCPMatePaths) -> ConfigResult<SystemSettings> {
    read_settings_sync(&paths.config_path())
}

pub fn set_settings_sync(settings: &SystemSettings) -> ConfigResult<SystemSettings> {
    set_settings_sync_for_paths(global_paths(), settings)
}

pub fn set_settings_sync_for_paths(
    paths: &MCPMatePaths,
    settings: &SystemSettings,
) -> ConfigResult<SystemSettings> {
    settings.validate()?;
    write_settings_path_sync(&paths.config_path(), settings)?;
    Ok(settings.clone())
}

pub async fn get_first_contact_behavior(pool: &SqlitePool) -> ConfigResult<FirstContactBehavior> {
    Ok(get_settings(pool).await?.first_contact_behavior)
}

pub async fn set_first_contact_behavior(
    pool: &SqlitePool,
    behavior: FirstContactBehavior,
) -> ConfigResult<()> {
    mutate_settings_at_path(&settings_path(pool), move |settings| {
        settings.first_contact_behavior = behavior;
        Ok(())
    })
    .await?;
    Ok(())
}

pub async fn get_inspector_timeout_ms(pool: &SqlitePool) -> ConfigResult<u64> {
    Ok(get_settings(pool).await?.inspector_timeout_ms)
}

pub async fn set_inspector_timeout_ms(
    pool: &SqlitePool,
    timeout_ms: u64,
) -> ConfigResult<()> {
    mutate_settings_at_path(&settings_path(pool), move |settings| {
        settings.inspector_timeout_ms = timeout_ms;
        Ok(())
    })
    .await?;
    Ok(())
}

pub async fn set_client_discovery_snapshot_last_success_at(
    pool: &SqlitePool,
    last_success_at: String,
) -> ConfigResult<()> {
    set_client_discovery_snapshot_last_success_at_at_path(&settings_path(pool), last_success_at).await
}

pub async fn set_client_discovery_snapshot_last_success_at_for_paths(
    paths: &MCPMatePaths,
    last_success_at: String,
) -> ConfigResult<()> {
    set_client_discovery_snapshot_last_success_at_at_path(&paths.config_path(), last_success_at).await
}

async fn set_client_discovery_snapshot_last_success_at_at_path(
    path: &Path,
    last_success_at: String,
) -> ConfigResult<()> {
    mutate_settings_at_path(path, move |settings| {
        settings.client_discovery_snapshot_last_success_at = Some(last_success_at);
        Ok(())
    })
    .await?;
    Ok(())
}

pub async fn set_onboarding_completed(
    pool: &SqlitePool,
    completed: bool,
) -> ConfigResult<()> {
    set_onboarding_completed_at_path(&settings_path(pool), completed).await
}

pub async fn set_onboarding_completed_for_paths(
    paths: &MCPMatePaths,
    completed: bool,
) -> ConfigResult<()> {
    set_onboarding_completed_at_path(&paths.config_path(), completed).await
}

async fn set_onboarding_completed_at_path(
    path: &Path,
    completed: bool,
) -> ConfigResult<()> {
    mutate_settings_at_path(path, move |settings| {
        settings.onboarding_completed = completed;
        Ok(())
    })
    .await?;
    Ok(())
}

pub async fn get_default_config_mode(pool: &SqlitePool) -> ConfigResult<String> {
    Ok(get_settings(pool).await?.default_config_mode)
}

#[cfg(test)]
pub async fn set_default_config_mode(
    pool: &SqlitePool,
    mode: &str,
) -> ConfigResult<()> {
    let mode = mode.to_string();
    mutate_settings_at_path(&settings_path(pool), move |settings| {
        settings.default_config_mode = mode;
        Ok(())
    })
    .await?;
    Ok(())
}

async fn write_settings(
    pool: &SqlitePool,
    settings: &SystemSettings,
) -> ConfigResult<()> {
    set_settings(pool, settings).await.map(|_| ())
}

async fn write_settings_path_async(
    path: &Path,
    settings: &SystemSettings,
) -> ConfigResult<()> {
    let mut content = serde_json::to_vec_pretty(settings)?;
    content.push(b'\n');

    settings_path_service(path)?
        .atomic_write_with_backup(
            path,
            &content,
            Some(SETTINGS_BACKUP_LIMIT),
            Some("system_settings_store"),
        )
        .await
        .map_err(|err| ConfigError::FileOperationError(err.to_string()))?;

    Ok(())
}

fn settings_mutation_lock(path: &Path) -> Arc<tokio::sync::Mutex<()>> {
    static SETTINGS_MUTATION_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Weak<tokio::sync::Mutex<()>>>>> = OnceLock::new();

    let locks = SETTINGS_MUTATION_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = locks.lock().expect("settings mutation lock registry poisoned");
    locks.retain(|_, lock| lock.strong_count() > 0);

    if let Some(lock) = locks.get(path).and_then(Weak::upgrade) {
        return lock;
    }

    let lock = Arc::new(tokio::sync::Mutex::new(()));
    locks.insert(path.to_path_buf(), Arc::downgrade(&lock));
    lock
}

async fn mutate_settings_at_path<T, F>(
    path: &Path,
    mutate: F,
) -> ConfigResult<(SystemSettings, T)>
where
    F: FnOnce(&mut SystemSettings) -> ConfigResult<T>,
{
    let lock = settings_mutation_lock(path);
    let _guard = lock.lock().await;
    let mut settings = read_settings_async(path).await?;
    let result = mutate(&mut settings)?;
    settings.validate()?;
    write_settings_path_async(path, &settings).await?;
    Ok((settings, result))
}

fn spawn_mcp_port_reapply(
    client_service: Option<Arc<ClientConfigService>>,
    pool: Option<Arc<SqlitePool>>,
) -> tokio::task::JoinHandle<ConfigResult<crate::clients::HostedClientReapplySummary>> {
    tokio::spawn(async move {
        let service = match (client_service, pool) {
            (Some(service), _) => service,
            (None, Some(pool)) => Arc::new(ClientConfigService::bootstrap(pool).await?),
            (None, None) => {
                let database = crate::config::database::Database::new().await.map_err(|err| {
                    ConfigError::DataAccessError(format!("failed to open database for MCP port re-apply: {err}"))
                })?;
                Arc::new(ClientConfigService::bootstrap(Arc::new(database.pool.clone())).await?)
            }
        };

        service.reapply_hosted_managed_clients_after_mcp_port_change().await
    })
}

fn write_settings_path_sync(
    path: &Path,
    settings: &SystemSettings,
) -> ConfigResult<()> {
    let mut content = serde_json::to_vec_pretty(settings)?;
    content.push(b'\n');

    settings_path_service(path)?
        .atomic_write_with_backup_sync(
            path,
            &content,
            Some(SETTINGS_BACKUP_LIMIT),
            Some("system_settings_store"),
        )
        .map_err(|err| ConfigError::FileOperationError(err.to_string()))?;

    Ok(())
}

fn settings_path_service(path: &Path) -> ConfigResult<PathService> {
    let parent = path.parent().ok_or_else(|| {
        ConfigError::PathResolutionError(format!(
            "system settings path has no parent directory: {}",
            path.display()
        ))
    })?;
    PathService::new()
        .map(|service| service.with_backup_root(parent.join("backups").join("client")))
        .map_err(|error| ConfigError::PathResolutionError(error.to_string()))
}

async fn read_settings_async(path: &Path) -> ConfigResult<SystemSettings> {
    match fs::read(path).await {
        Ok(content) => parse_settings_bytes(&content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(SystemSettings::default()),
        Err(err) => Err(ConfigError::IoError(err)),
    }
}

fn read_settings_sync(path: &Path) -> ConfigResult<SystemSettings> {
    match std::fs::read(path) {
        Ok(content) => parse_settings_bytes(&content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(SystemSettings::default()),
        Err(err) => Err(ConfigError::IoError(err)),
    }
}

fn parse_settings_bytes(content: &[u8]) -> ConfigResult<SystemSettings> {
    let settings: SystemSettings = serde_json::from_slice(content)?;
    settings.validate()?;
    Ok(settings)
}

#[cfg(not(test))]
fn settings_path(_pool: &SqlitePool) -> PathBuf {
    global_paths().config_path()
}

#[cfg(test)]
fn settings_path(pool: &SqlitePool) -> PathBuf {
    struct TestSettingsPath {
        options: std::sync::Weak<sqlx::sqlite::SqliteConnectOptions>,
        path: PathBuf,
    }

    static SETTINGS_PATHS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<usize, TestSettingsPath>>> =
        std::sync::OnceLock::new();
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    static TEST_RUN_ID: std::sync::OnceLock<u128> = std::sync::OnceLock::new();

    let options = pool.connect_options();
    let pool_id = Arc::as_ptr(&options) as usize;
    let paths = SETTINGS_PATHS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut paths = paths.lock().expect("test settings path lock poisoned");

    if let Some(entry) = paths.get(&pool_id) {
        if entry.options.upgrade().is_some() {
            return entry.path.clone();
        }
    }

    let sequence = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let run_id = TEST_RUN_ID.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos()
    });
    let path = std::env::temp_dir().join(format!(
        "mcpmate-system-settings-test-{}-{run_id:x}-{sequence}.json",
        std::process::id()
    ));
    paths.insert(
        pool_id,
        TestSettingsPath {
            options: Arc::downgrade(&options),
            path: path.clone(),
        },
    );

    path
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::oneshot;

    use super::settings_mutation_lock;

    #[tokio::test]
    async fn settings_mutation_lock_blocks_second_acquisition_until_first_guard_releases() {
        let path = std::env::temp_dir().join("mcpmate-settings-mutation-lock-test.json");
        let first_lock = settings_mutation_lock(&path);
        let second_lock = settings_mutation_lock(&path);
        assert!(Arc::ptr_eq(&first_lock, &second_lock));

        let first_guard = first_lock.lock().await;
        assert!(second_lock.try_lock().is_err());

        let (acquired_tx, acquired_rx) = oneshot::channel();
        let waiter = tokio::spawn(async move {
            let _second_guard = second_lock.lock().await;
            acquired_tx.send(()).expect("report second lock acquisition");
        });

        drop(first_guard);
        acquired_rx.await.expect("second acquisition completes after release");
        waiter.await.expect("join second lock waiter");
    }
}
