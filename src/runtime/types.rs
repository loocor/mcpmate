//! Simplified runtime types for file-system based runtime management

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

use crate::config::constants::commands;

/// Supported runtime types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    /// uv runtime (Python package manager and environment manager)
    Uv,
    /// Bun.js runtime (supports bunx and bun x for npx compatibility)
    Bun,
}

impl RuntimeType {
    // Default version constants
    pub const DEFAULT_BUN_VERSION: &'static str = "latest";
    pub const DEFAULT_UV_VERSION: &'static str = "latest";

    /// Get the string representation of the runtime type
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::Uv => "uv",
            RuntimeType::Bun => "bun",
        }
    }

    /// Get the default version
    pub fn default_version(&self) -> &'static str {
        match self {
            RuntimeType::Uv => Self::DEFAULT_UV_VERSION,
            RuntimeType::Bun => Self::DEFAULT_BUN_VERSION,
        }
    }

    /// Get the executable name
    pub fn executable_name(&self) -> String {
        let base_name = self.as_str();
        if cfg!(windows) {
            format!("{}.exe", base_name)
        } else {
            base_name.to_string()
        }
    }

    /// Get the executable name for a specific command
    /// Note: npx commands are automatically converted to "bun x" in transport layer
    pub fn executable_name_for_command(
        &self,
        command: &str,
    ) -> String {
        let exe_name = match command {
            commands::UVX => "uvx",
            commands::BUNX => "bunx",
            _ => self.as_str(),
        };

        if cfg!(windows) {
            format!("{}.exe", exe_name)
        } else {
            exe_name.to_string()
        }
    }
}

impl fmt::Display for RuntimeType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            RuntimeType::Uv => write!(f, "uv"),
            RuntimeType::Bun => write!(f, "bun"),
        }
    }
}

impl FromStr for RuntimeType {
    type Err = RuntimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "uv" | commands::UVX => Ok(RuntimeType::Uv),
            "bun" | "bunjs" | commands::BUNX => Ok(RuntimeType::Bun),
            "node" | "nodejs" | "npm" | commands::NPX => Ok(RuntimeType::Bun),
            _ => Err(RuntimeError::UnsupportedRuntimeType(s.to_string())),
        }
    }
}

/// Runtime related errors
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Unsupported runtime type: {0}")]
    UnsupportedRuntimeType(String),

    #[error("Unsupported platform: {os} {arch}")]
    UnsupportedPlatform { os: String, arch: String },

    #[error("Version {version} does not exist in runtime {runtime_type}")]
    VersionNotFound {
        runtime_type: String,
        version: String,
    },

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Download timeout after {seconds} seconds")]
    DownloadTimeout { seconds: u64 },

    #[error("Download cancelled by user")]
    DownloadCancelled,

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
