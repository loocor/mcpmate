// Common type definitions for MCPMate
// This module contains shared enums and types used across the application

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Client application category - application or extension")]
pub enum ClientCategory {
    #[default]
    #[schemars(description = "Standalone application that runs independently")]
    Application,
    #[schemars(description = "Extension or plugin requiring host application")]
    Extension,
}

impl fmt::Display for ClientCategory {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ClientCategory::Application => write!(f, "application"),
            ClientCategory::Extension => write!(f, "extension"),
        }
    }
}

impl ClientCategory {
    pub fn is_application(&self) -> bool {
        matches!(self, ClientCategory::Application)
    }

    pub fn is_extension(&self) -> bool {
        matches!(self, ClientCategory::Extension)
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "application" | "app" => Some(ClientCategory::Application),
            "extension" | "ext" => Some(ClientCategory::Extension),
            _ => None,
        }
    }

    pub fn all() -> &'static [ClientCategory] {
        &[ClientCategory::Application, ClientCategory::Extension]
    }
}

impl FromStr for ClientCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "application" | "app" => Ok(ClientCategory::Application),
            "extension" | "ext" => Ok(ClientCategory::Extension),
            _ => Err(format!("Invalid client category: {}", s)),
        }
    }
}

/// Supported runtime types for MCPMate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    /// uv runtime (Python package manager and environment manager)
    Uv,
    /// Bun.js runtime
    Bun,
    /// Node.js runtime
    Node,
}

impl RuntimeType {
    // Default version constants
    pub const DEFAULT_BUN_VERSION: &'static str = "latest";
    pub const DEFAULT_UV_VERSION: &'static str = "latest";
    pub const DEFAULT_NODE_VERSION: &'static str = "lts";

    /// Get the string representation of the runtime type
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::Uv => "uv",
            RuntimeType::Bun => "bun",
            RuntimeType::Node => "node",
        }
    }

    /// Get the default version
    pub fn default_version(&self) -> &'static str {
        match self {
            RuntimeType::Uv => Self::DEFAULT_UV_VERSION,
            RuntimeType::Bun => Self::DEFAULT_BUN_VERSION,
            RuntimeType::Node => Self::DEFAULT_NODE_VERSION,
        }
    }

    /// Resolve a runtime type from a command alias or runtime name.
    pub fn from_command(command: &str) -> Option<Self> {
        use super::constants::commands;

        match command.trim().to_ascii_lowercase().as_str() {
            commands::UV | commands::UVX => Some(RuntimeType::Uv),
            "bunjs" | commands::BUN | commands::BUNX => Some(RuntimeType::Bun),
            "nodejs" | commands::NODE | commands::NPM | commands::NPX => Some(RuntimeType::Node),
            _ => None,
        }
    }

    /// Get the canonical executable command for the runtime.
    pub fn canonical_command(&self) -> &'static str {
        use super::constants::commands;

        match self {
            RuntimeType::Uv => commands::UV,
            RuntimeType::Bun => commands::BUN,
            RuntimeType::Node => commands::NODE,
        }
    }

    /// Get the executable name
    pub fn executable_name(&self) -> String {
        self.executable_name_for_command(self.canonical_command())
    }

    /// Get the platform-specific executable extension
    pub fn executable_extension() -> &'static str {
        if cfg!(windows) { ".exe" } else { "" }
    }

    /// Get the executable name for a specific command
    pub fn executable_name_for_command(
        &self,
        command: &str,
    ) -> String {
        use super::constants::commands;

        match (self, command.trim().to_ascii_lowercase().as_str()) {
            (RuntimeType::Uv, commands::UVX) => format!("{}{}", commands::UVX, Self::executable_extension()),
            (RuntimeType::Bun, commands::BUNX) => format!("{}{}", commands::BUNX, Self::executable_extension()),
            (RuntimeType::Node, commands::NPM) if cfg!(windows) => format!("{}.cmd", commands::NPM),
            (RuntimeType::Node, commands::NPX) if cfg!(windows) => format!("{}.cmd", commands::NPX),
            (RuntimeType::Node, commands::NPM) => format!("{}{}", commands::NPM, Self::executable_extension()),
            (RuntimeType::Node, commands::NPX) => format!("{}{}", commands::NPX, Self::executable_extension()),
            _ => format!("{}{}", self.canonical_command(), Self::executable_extension()),
        }
    }

    /// Get all supported runtime types
    pub fn all() -> &'static [RuntimeType] {
        &[RuntimeType::Uv, RuntimeType::Bun, RuntimeType::Node]
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
            RuntimeType::Node => write!(f, "node"),
        }
    }
}

impl FromStr for RuntimeType {
    type Err = RuntimeError;

    /// Parse a runtime type from a canonical name only (`uv`, `bun`, `node`).
    ///
    /// Unlike `from_command`, this does NOT accept CLI aliases (`uvx`, `bunx`,
    /// `npm`, `npx`) since it is used for API input parsing where only the
    /// canonical runtime name is valid.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "uv" => Ok(RuntimeType::Uv),
            "bun" => Ok(RuntimeType::Bun),
            "node" => Ok(RuntimeType::Node),
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
    VersionNotFound { runtime_type: String, version: String },

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_category_display() {
        assert_eq!(ClientCategory::Application.to_string(), "application");
        assert_eq!(ClientCategory::Extension.to_string(), "extension");
    }

    #[test]
    fn test_client_category_parse() {
        assert_eq!(ClientCategory::parse("application"), Some(ClientCategory::Application));
        assert_eq!(ClientCategory::parse("extension"), Some(ClientCategory::Extension));
        assert_eq!(ClientCategory::parse("app"), Some(ClientCategory::Application));
        assert_eq!(ClientCategory::parse("ext"), Some(ClientCategory::Extension));
        assert_eq!(ClientCategory::parse("invalid"), None);
    }

    #[test]
    fn test_client_category_from_str() {
        use std::str::FromStr;
        assert_eq!(ClientCategory::from_str("application"), Ok(ClientCategory::Application));
        assert_eq!(ClientCategory::from_str("extension"), Ok(ClientCategory::Extension));
        assert_eq!(ClientCategory::from_str("app"), Ok(ClientCategory::Application));
        assert_eq!(ClientCategory::from_str("ext"), Ok(ClientCategory::Extension));
        assert!(ClientCategory::from_str("invalid").is_err());
    }

    #[test]
    fn test_client_category_predicates() {
        assert!(ClientCategory::Application.is_application());
        assert!(!ClientCategory::Application.is_extension());
        assert!(ClientCategory::Extension.is_extension());
        assert!(!ClientCategory::Extension.is_application());
    }

    #[test]
    fn test_client_category_serialization() {
        let app = ClientCategory::Application;
        let ext = ClientCategory::Extension;

        let app_json = serde_json::to_string(&app).unwrap();
        let ext_json = serde_json::to_string(&ext).unwrap();

        assert_eq!(app_json, "\"application\"");
        assert_eq!(ext_json, "\"extension\"");

        let app_deserialized: ClientCategory = serde_json::from_str(&app_json).unwrap();
        let ext_deserialized: ClientCategory = serde_json::from_str(&ext_json).unwrap();

        assert_eq!(app_deserialized, app);
        assert_eq!(ext_deserialized, ext);
    }

    #[test]
    fn test_runtime_type_from_command_aliases() {
        assert_eq!(RuntimeType::from_command("uv"), Some(RuntimeType::Uv));
        assert_eq!(RuntimeType::from_command("uvx"), Some(RuntimeType::Uv));
        assert_eq!(RuntimeType::from_command("bun"), Some(RuntimeType::Bun));
        assert_eq!(RuntimeType::from_command("bunx"), Some(RuntimeType::Bun));
        assert_eq!(RuntimeType::from_command("node"), Some(RuntimeType::Node));
        assert_eq!(RuntimeType::from_command("nodejs"), Some(RuntimeType::Node));
        assert_eq!(RuntimeType::from_command("npm"), Some(RuntimeType::Node));
        assert_eq!(RuntimeType::from_command("npx"), Some(RuntimeType::Node));
        assert_eq!(RuntimeType::from_command("python"), None);
    }

    #[test]
    fn test_runtime_type_defaults_and_executable_names() {
        assert_eq!(RuntimeType::Node.default_version(), "lts");
        assert_eq!(
            RuntimeType::Node.executable_name(),
            format!("node{}", RuntimeType::executable_extension())
        );
        let expected_npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
        let expected_npx = if cfg!(windows) { "npx.cmd" } else { "npx" };
        assert_eq!(RuntimeType::Node.executable_name_for_command("npm"), expected_npm);
        assert_eq!(RuntimeType::Node.executable_name_for_command("npx"), expected_npx);
    }
}
