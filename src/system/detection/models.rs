// Data models for application detection

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a client application definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientApp {
    pub id: String,
    pub identifier: String,
    pub display_name: String,
    pub description: Option<String>,
    pub enabled: bool,
}

/// Represents a detected application with its installation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedApp {
    pub client_app: ClientApp,
    pub version: Option<String>,
    pub install_path: PathBuf,
    pub config_path: PathBuf,
    pub confidence: f32,
    pub verified_methods: Vec<String>,
}

/// Represents a detection rule for a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRule {
    pub id: String,
    pub client_app_id: String,
    pub platform: String,
    pub detection_method: DetectionMethod,
    pub detection_value: String,
    pub config_path_template: String,
    pub priority: i32,
    pub enabled: bool,
}

/// Detection methods supported by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    BundleId,    // macOS Bundle ID
    FilePath,    // File/directory path check
    Registry,    // Windows registry check
    Command,     // Command execution check
}

impl DetectionMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "bundle_id" => Some(Self::BundleId),
            "file_path" => Some(Self::FilePath),
            "registry" => Some(Self::Registry),
            "command" => Some(Self::Command),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BundleId => "bundle_id",
            Self::FilePath => "file_path",
            Self::Registry => "registry",
            Self::Command => "command",
        }
    }
}

/// Result of a detection attempt
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub success: bool,
    pub install_path: Option<PathBuf>,
    pub version: Option<String>,
    pub method: DetectionMethod,
    pub confidence: f32,
}

impl DetectionResult {
    pub fn success(
        install_path: PathBuf,
        version: Option<String>,
        method: DetectionMethod,
        confidence: f32,
    ) -> Self {
        Self {
            success: true,
            install_path: Some(install_path),
            version,
            method,
            confidence,
        }
    }

    pub fn failure(method: DetectionMethod) -> Self {
        Self {
            success: false,
            install_path: None,
            version: None,
            method,
            confidence: 0.0,
        }
    }
}
