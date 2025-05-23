//! Type definitions for runtime environment management

use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Supported runtime types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    /// Node.js runtime (supports npx)
    Node,
    /// uv runtime (Python package manager and environment manager)
    Uv,
    /// Bun.js runtime (experimental)
    Bun,
}

impl RuntimeType {
    /// Get the string representation of the runtime type
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::Node => "node",
            RuntimeType::Uv => "uv",
            RuntimeType::Bun => "bun",
        }
    }

    /// Get the default version
    pub fn default_version(&self) -> &'static str {
        use crate::runtime::constants::*;
        get_default_version(*self)
    }

    /// Get the executable name
    pub fn executable_name(&self) -> &'static str {
        use crate::runtime::constants::*;
        get_executable_name(*self)
    }

    /// Get the download URL for this runtime type and version
    pub fn download_url(
        &self,
        version: &RuntimeVersion,
        platform: &str,
        arch: &str,
    ) -> String {
        match self {
            RuntimeType::Node => {
                let version_str = match version {
                    RuntimeVersion::Latest => "latest",
                    RuntimeVersion::Specific(v) => v,
                };

                let platform_str = match platform {
                    "windows" => "win",
                    "macos" => "darwin",
                    _ => "linux",
                };

                let arch_str = match arch {
                    "aarch64" | "arm64" => "arm64",
                    _ => "x64",
                };

                format!(
                    "https://nodejs.org/dist/{}/node-{}-{}-{}.tar.gz",
                    version_str, version_str, platform_str, arch_str
                )
            }
            RuntimeType::Bun => {
                let platform_str = match platform {
                    "windows" => "win",
                    "macos" => "darwin",
                    _ => "linux",
                };

                let arch_str = match arch {
                    "aarch64" | "arm64" => "aarch64",
                    _ => "x64",
                };

                format!(
                    "https://github.com/oven-sh/bun/releases/latest/download/bun-{}-{}.zip",
                    platform_str, arch_str
                )
            }
            RuntimeType::Uv => {
                let platform_str = match platform {
                    "windows" => "windows",
                    "macos" => "macos",
                    _ => "linux",
                };

                let arch_str = match arch {
                    "aarch64" | "arm64" => "aarch64",
                    _ => "x64",
                };

                format!(
                    "https://github.com/astral-sh/uv/releases/latest/download/uv-{}-{}.tar.gz",
                    platform_str, arch_str
                )
            }
        }
    }
}

impl fmt::Display for RuntimeType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RuntimeType {
    type Err = RuntimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "node" | "nodejs" | "npx" | "npm" => Ok(RuntimeType::Node),
            "uv" | "uvx" => Ok(RuntimeType::Uv),
            "bun" | "bunjs" => Ok(RuntimeType::Bun),
            _ => Err(RuntimeError::UnsupportedRuntimeType(s.to_string())),
        }
    }
}

/// Runtime version specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeVersion {
    /// Latest version
    Latest,
    /// Specific version
    Specific(String),
}

impl fmt::Display for RuntimeVersion {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            RuntimeVersion::Latest => write!(f, "latest"),
            RuntimeVersion::Specific(version) => write!(f, "{}", version),
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

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Path error: {0}")]
    PathError(String),
}

/// Parse runtime type from string
fn parse_runtime_type(s: &str) -> Result<RuntimeType, String> {
    s.parse()
        .map_err(|e| format!("Unsupported runtime type: {}", e))
}

/// Commands for runtime manager
#[derive(Subcommand)]
pub enum Commands {
    /// Install runtime environment
    Install {
        /// Runtime type (node, uv, bun)
        #[arg(value_parser = parse_runtime_type)]
        runtime_type: RuntimeType,
        /// Version number (optional, default to recommended version)
        #[arg(short, long)]
        version: Option<String>,
    },
    /// List installed runtime environments
    List,
    /// Check runtime environment status
    Check {
        /// Runtime type (node, uv, bun)
        #[arg(value_parser = parse_runtime_type)]
        runtime_type: RuntimeType,
        /// Version number (optional)
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Get runtime environment path
    Path {
        /// Runtime type (node, uv, bun)
        #[arg(value_parser = parse_runtime_type)]
        runtime_type: RuntimeType,
        /// Version number (optional)
        #[arg(short, long)]
        version: Option<String>,
    },
}
