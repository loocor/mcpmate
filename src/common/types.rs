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
        format!("{}{}", base_name, Self::executable_extension())
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

        let exe_name = match command {
            commands::UVX => "uvx",
            commands::BUNX => "bunx",
            _ => self.as_str(),
        };

        format!("{}{}", exe_name, Self::executable_extension())
    }

    /// Get all supported runtime types
    pub fn all() -> &'static [RuntimeType] {
        &[RuntimeType::Uv, RuntimeType::Bun]
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
        use super::constants::commands;

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
}
