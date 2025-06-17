// Data models and structures for client handlers

use crate::common::ClientCategory;
use serde::Deserialize;

/// Database row structure for client_apps table
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClientAppRow {
    pub id: String,
    pub identifier: String,
    pub display_name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub category: Option<String>,
    pub enabled: bool,
    pub detected: bool,
    pub last_detected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub install_path: Option<String>,
    pub config_path: Option<String>,
    pub version: Option<String>,
    pub detection_method: Option<String>,
    pub config_mode: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ClientAppRow {
    /// Get the category as a ClientCategory enum
    pub fn get_category(&self) -> ClientCategory {
        self.category
            .as_ref()
            .and_then(|c| ClientCategory::parse(c))
            .unwrap_or_default()
    }
}

/// Query parameters for client detection
#[derive(Debug, Deserialize)]
pub struct ClientsQuery {
    #[serde(default)]
    pub force_refresh: bool,
}

/// Simple structure to hold detection results
#[derive(Debug, Clone)]
pub struct SimpleDetectedApp {
    pub install_path: std::path::PathBuf,
    pub config_path: std::path::PathBuf,
}
