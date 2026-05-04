use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::fs;

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{FirstContactBehavior, OnboardingPolicy};
use crate::common::MCPMatePaths;
use crate::common::constants::ports;
use crate::common::paths::global_paths;
use crate::config::client::init::DEFAULT_CONFIG_MODE;
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
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            api_port: ports::API_PORT,
            mcp_port: ports::MCP_PORT,
            first_contact_behavior: FirstContactBehavior::default(),
            inspector_timeout_ms: DEFAULT_INSPECTOR_TIMEOUT_MS,
            default_config_mode: DEFAULT_CONFIG_MODE.to_string(),
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
        Ok(_) => Ok(()),
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

fn write_settings_path_sync(
    path: &Path,
    settings: &SystemSettings,
) -> ConfigResult<()> {
    let mut content = serde_json::to_vec_pretty(settings)?;
    content.push(b'\n');

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, content)?;
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
    use std::sync::Arc;

    let opts = pool.connect_options();
    let id = Arc::as_ptr(&opts) as usize;
    std::env::temp_dir().join(format!("mcpmate-system-settings-test-{id:x}.json"))
}
