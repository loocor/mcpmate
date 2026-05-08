use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::fs;

use crate::clients::ClientConfigService;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{FirstContactBehavior, OnboardingPolicy};
use crate::common::MCPMatePaths;
use crate::common::constants::ports;
use crate::common::paths::global_paths;
use crate::config::client::init::DEFAULT_CONFIG_MODE;
use crate::system::config::init_port_config;
use crate::system::paths::get_path_service;

pub const DEFAULT_INSPECTOR_TIMEOUT_MS: u64 = 8_000;
const SETTINGS_BACKUP_LIMIT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemSettings {
    pub api_port: u16,
    pub mcp_port: u16,
    pub first_contact_behavior: FirstContactBehavior,
    pub inspector_timeout_ms: u64,
    pub default_config_mode: String,
    #[serde(default)]
    pub onboarding_completed: bool,
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
    settings.validate()?;
    write_settings_path_async(&settings_path(pool), settings).await?;
    Ok(settings.clone())
}

/// Outcome of [`apply_settings_with_effects`].
pub struct SystemSettingsApplyResult {
    /// `api_port` changed between old and new settings.
    pub api_port_changed: bool,
    /// `mcp_port` changed between old and new settings.
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
/// - `previous` must be the settings snapshot **before** the change (used to diff ports).
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

async fn apply_settings_with_effects_at_path(
    path: &Path,
    previous: &SystemSettings,
    next: &SystemSettings,
    client_service: Option<Arc<ClientConfigService>>,
    pool: Option<Arc<SqlitePool>>,
) -> ConfigResult<SystemSettingsApplyResult> {
    next.validate()?;

    write_settings_path_async(path, next).await?;

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
    let mut settings = get_settings(pool).await?;
    settings.first_contact_behavior = behavior;
    set_settings(pool, &settings).await?;
    Ok(())
}

pub async fn get_inspector_timeout_ms(pool: &SqlitePool) -> ConfigResult<u64> {
    Ok(get_settings(pool).await?.inspector_timeout_ms)
}

pub async fn set_inspector_timeout_ms(
    pool: &SqlitePool,
    timeout_ms: u64,
) -> ConfigResult<()> {
    let mut settings = get_settings(pool).await?;
    settings.inspector_timeout_ms = timeout_ms;
    set_settings(pool, &settings).await?;
    Ok(())
}

pub async fn get_default_config_mode(pool: &SqlitePool) -> ConfigResult<String> {
    Ok(get_settings(pool).await?.default_config_mode)
}

pub async fn set_default_config_mode(
    pool: &SqlitePool,
    mode: &str,
) -> ConfigResult<()> {
    let mut settings = get_settings(pool).await?;
    settings.default_config_mode = mode.to_string();
    set_settings(pool, &settings).await?;
    Ok(())
}

async fn write_settings(
    pool: &SqlitePool,
    settings: &SystemSettings,
) -> ConfigResult<()> {
    write_settings_path_async(&settings_path(pool), settings).await
}

async fn write_settings_path_async(
    path: &Path,
    settings: &SystemSettings,
) -> ConfigResult<()> {
    let mut content = serde_json::to_vec_pretty(settings)?;
    content.push(b'\n');

    get_path_service()
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

    get_path_service()
        .atomic_write_with_backup_sync(
            path,
            &content,
            Some(SETTINGS_BACKUP_LIMIT),
            Some("system_settings_store"),
        )
        .map_err(|err| ConfigError::FileOperationError(err.to_string()))?;

    Ok(())
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
