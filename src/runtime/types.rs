//! Type definitions for runtime environment management

use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

use crate::conf::constants::commands;

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
        match self {
            RuntimeType::Node => "latest",
            RuntimeType::Uv => "latest",
            RuntimeType::Bun => "latest",
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
    pub fn executable_name_for_command(
        &self,
        command: &str,
    ) -> String {
        let exe_name = match command {
            commands::NPX => "npx",
            commands::UVX => "uv",
            commands::BUNX => "bunx",
            _ => self.as_str(),
        };

        if cfg!(windows) {
            format!("{}.exe", exe_name)
        } else {
            exe_name.to_string()
        }
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
                format!(
                    "https://nodejs.org/dist/{}/node-{}-{}-{}.tar.gz",
                    version_str, version_str, platform, arch
                )
            }
            RuntimeType::Bun => {
                let version_str = match version {
                    RuntimeVersion::Latest => "latest",
                    RuntimeVersion::Specific(v) => v,
                };
                format!(
                    "https://github.com/oven-sh/bun/releases/{}/download/bun-{}-{}.zip",
                    if version_str == "latest" {
                        "latest".to_string()
                    } else {
                        format!("tag/bun-v{}", version_str)
                    },
                    platform,
                    arch
                )
            }
            RuntimeType::Uv => {
                let version_str = match version {
                    RuntimeVersion::Latest => "latest",
                    RuntimeVersion::Specific(v) => v,
                };
                format!(
                    "https://github.com/astral-sh/uv/releases/{}/download/uv-{}-{}.tar.gz",
                    if version_str == "latest" {
                        "latest".to_string()
                    } else {
                        format!("tag/{}", version_str)
                    },
                    platform,
                    arch
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
        match self {
            RuntimeType::Node => write!(f, "node"),
            RuntimeType::Uv => write!(f, "uv"),
            RuntimeType::Bun => write!(f, "bun"),
        }
    }
}

/// Execution context for runtime operations
/// Determines whether operations are running in CLI or API mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionContext {
    /// CLI mode - allows process exit and console output
    Cli,
    /// API mode - returns errors, no console output
    Api,
}

impl FromStr for RuntimeType {
    type Err = RuntimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "node" | "nodejs" | commands::NPX | "npm" => Ok(RuntimeType::Node),
            "uv" | commands::UVX => Ok(RuntimeType::Uv),
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
    /// Resolving DNS
    ResolvingDns,
    /// Establishing connection
    Connecting,
    /// Sending HTTP request
    SendingRequest,
    /// Waiting for response
    WaitingResponse,
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
            DownloadStage::ResolvingDns => write!(f, "Resolving DNS"),
            DownloadStage::Connecting => write!(f, "Connecting"),
            DownloadStage::SendingRequest => write!(f, "Sending request"),
            DownloadStage::WaitingResponse => write!(f, "Waiting for response"),
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
    /// Enable interactive mode for timeout handling
    pub interactive: bool,
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
            .field("interactive", &self.interactive)
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
            interactive: false,
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
        /// Enable interactive mode for timeout handling
        #[arg(long)]
        interactive: bool,
        /// Enable quiet mode (minimal output, send events only)
        #[arg(short, long)]
        quiet: bool,
        /// Database file path (for saving runtime config in quiet mode)
        #[arg(long)]
        database: Option<String>,
    },
    /// List installed runtime environments
    List,
    // Check and Path commands are removed to align with simplified API structure
    // Functionality is covered by `list` in API, and CLI `list` shows all details.
}
