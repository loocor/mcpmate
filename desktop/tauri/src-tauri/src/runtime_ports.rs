//! Persisted API/MCP ports for the Tauri shell (survives app restart).

use anyhow::{Context, Result};
use mcpmate::common::MCPMatePaths;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedRuntimePorts {
    pub api_port: u16,
    pub mcp_port: u16,
}

impl PersistedRuntimePorts {
    const FILE_NAME: &'static str = "desktop-runtime-ports.json";

    fn is_usable(&self) -> bool {
        self.api_port != 0 && self.mcp_port != 0 && self.api_port != self.mcp_port
    }

    pub fn path(paths: &MCPMatePaths) -> PathBuf {
        paths.base_dir().join("config").join(Self::FILE_NAME)
    }

    /// Load persisted ports if the file exists and values are usable.
    pub fn load(paths: &MCPMatePaths) -> Option<Self> {
        let path = Self::path(paths);
        if !path.exists() {
            return None;
        }
        let data = fs::read(&path).ok()?;
        let parsed: Self = serde_json::from_slice(&data).ok()?;
        if !parsed.is_usable() {
            return None;
        }
        Some(parsed)
    }

    pub fn save(paths: &MCPMatePaths, ports: &Self) -> Result<()> {
        if !ports.is_usable() {
            anyhow::bail!("invalid port values");
        }
        let path = Self::path(paths);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory {}", parent.display())
            })?;
        }
        let payload = serde_json::to_vec_pretty(ports).context("failed to encode runtime ports")?;
        fs::write(&path, payload).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }
}
