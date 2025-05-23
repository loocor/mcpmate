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

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Current downloaded bytes
    pub downloaded: u64,
    /// Total bytes to download (if known)
    pub total: Option<u64>,
    /// Download speed in bytes per second
    pub speed: Option<u64>,
    /// Current stage of the download process
    pub stage: DownloadStage,
    /// Stage-specific message
    pub message: Option<String>,
}

impl DownloadProgress {
    /// Calculate download percentage (0-100)
    pub fn percentage(&self) -> Option<f64> {
        self.total.map(|total| {
            if total == 0 {
                100.0
            } else {
                (self.downloaded as f64 / total as f64) * 100.0
            }
        })
    }

    /// Check if download is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.stage, DownloadStage::Complete)
    }
}

/// Download stages
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStage {
    /// Initializing download
    Initializing,
    /// Downloading file
    Downloading,
    /// Extracting archive
    Extracting,
    /// Post-processing
    PostProcessing,
    /// Download complete
    Complete,
    /// Download failed
    Failed(String),
}

impl fmt::Display for DownloadStage {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            DownloadStage::Initializing => write!(f, "Initializing"),
            DownloadStage::Downloading => write!(f, "Downloading"),
            DownloadStage::Extracting => write!(f, "Extracting"),
            DownloadStage::PostProcessing => write!(f, "Post-processing"),
            DownloadStage::Complete => write!(f, "Complete"),
            DownloadStage::Failed(err) => write!(f, "Failed: {}", err),
        }
    }
}

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

/// Download configuration
pub struct DownloadConfig {
    /// Request timeout in seconds
    pub timeout: Option<u64>,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Progress callback
    pub progress_callback: Option<ProgressCallback>,
    /// Enable verbose logging
    pub verbose: bool,
}

impl std::fmt::Debug for DownloadConfig {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DownloadConfig")
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field(
                "progress_callback",
                &self.progress_callback.as_ref().map(|_| "<callback>"),
            )
            .field("verbose", &self.verbose)
            .finish()
    }
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            timeout: Some(300), // 5 minutes default
            max_retries: 3,
            progress_callback: None,
            verbose: false,
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
        /// Request timeout in seconds
        #[arg(short, long, default_value = "300")]
        timeout: u64,
        /// Maximum retry attempts
        #[arg(short, long, default_value = "3")]
        max_retries: u32,
        /// Enable verbose logging
        #[arg(long)]
        verbose: bool,
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
