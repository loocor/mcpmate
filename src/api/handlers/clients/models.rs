// Data models and structures for client handlers

use serde::Deserialize;

/// Database row structure for client_apps table
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct ClientAppRow {
    pub id: String,
    pub identifier: String,
    pub display_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub detected: bool,
    pub last_detected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub install_path: Option<String>,
    pub config_path: Option<String>,
    pub version: Option<String>,
    pub detection_method: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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
