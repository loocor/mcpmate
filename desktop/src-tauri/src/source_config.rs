use crate::runtime_ports::PersistedRuntimePorts;
use anyhow::{Context, Result};
use mcpmate::common::{constants::ports, MCPMatePaths};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopCoreSourceKind {
    Localhost,
    Remote,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalCoreRuntimeMode {
    Service,
    #[default]
    DesktopManaged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalhostCoreConfig {
    pub api_port: u16,
    pub mcp_port: u16,
}

impl Default for LocalhostCoreConfig {
    fn default() -> Self {
        Self {
            api_port: ports::API_PORT,
            mcp_port: ports::MCP_PORT,
        }
    }
}

impl LocalhostCoreConfig {
    fn apply_constraints(&mut self) {
        if self.api_port == 0 {
            self.api_port = ports::API_PORT;
        }
        if self.mcp_port == 0 || self.mcp_port == self.api_port {
            self.mcp_port = ports::MCP_PORT;
            if self.mcp_port == self.api_port {
                self.mcp_port = ports::MCP_PORT + 1;
            }
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemoteCoreConfig {
    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopCoreSourceConfig {
    pub selected_source: DesktopCoreSourceKind,
    #[serde(default)]
    pub localhost_runtime_mode: LocalCoreRuntimeMode,
    #[serde(default)]
    pub localhost: LocalhostCoreConfig,
    #[serde(default)]
    pub remote: RemoteCoreConfig,
}

impl Default for DesktopCoreSourceConfig {
    fn default() -> Self {
        Self {
            selected_source: DesktopCoreSourceKind::Localhost,
            localhost_runtime_mode: LocalCoreRuntimeMode::DesktopManaged,
            localhost: LocalhostCoreConfig::default(),
            remote: RemoteCoreConfig::default(),
        }
    }
}

impl DesktopCoreSourceConfig {
    const FILE_NAME: &'static str = "desktop-core-source.json";

    pub fn path(paths: &MCPMatePaths) -> PathBuf {
        paths.base_dir().join("config").join(Self::FILE_NAME)
    }

    pub fn load(paths: &MCPMatePaths) -> Result<Self> {
        let path = Self::path(paths);
        if !path.exists() {
            let mut fallback = Self::default();
            if let Some(ports) = PersistedRuntimePorts::load(paths) {
                fallback.localhost = LocalhostCoreConfig {
                    api_port: ports.api_port,
                    mcp_port: ports.mcp_port,
                };
            }
            fallback.apply_constraints();
            return Ok(fallback);
        }

        let data = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        let mut parsed: Self = serde_json::from_slice(&data)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        parsed.apply_constraints();
        Ok(parsed)
    }

    pub fn save(paths: &MCPMatePaths, config: &Self) -> Result<()> {
        let path = Self::path(paths);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory {}", parent.display())
            })?;
        }

        let mut copy = config.clone();
        copy.apply_constraints();
        let payload = serde_json::to_vec_pretty(&copy)
            .context("failed to encode desktop core source config")?;
        fs::write(&path, payload).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn apply_constraints(&mut self) {
        self.localhost.apply_constraints();
        self.remote.base_url = self.remote.base_url.trim().to_string();
    }
}
