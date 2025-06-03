use serde::{Deserialize, Serialize};

/// Client detection and query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub identifier: String,
    pub display_name: String,
    pub detected: bool,
    pub install_path: Option<String>,
    pub config_path: String,
    pub config_exists: bool,
    pub has_mcp_config: bool,
    pub supported_transports: Vec<String>,
    pub supported_runtimes: Vec<String>,
    pub last_detected_at: Option<String>,
}

/// Request for configuration management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub mode: ConfigMode,
    pub preview_only: bool,
    pub force_overwrite: bool,
    pub selected_config: SelectedConfig,
}

/// Configuration mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigMode {
    Hosted,
    Transparent,
}

/// Selected configuration for server selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectedConfig {
    /// Use a configuration suit by ID
    Suit { config_suit_id: String },
    /// Use specific servers by their IDs
    Servers { server_ids: Vec<String> },
    /// Use default configuration (all enabled servers)
    Default,
}

/// Configuration management response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub success: bool,
    pub preview: serde_json::Value,
    pub applied: bool,
    pub backup_path: Option<String>,
    pub warnings: Vec<String>,
}

/// Configuration view response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigViewResponse {
    pub config_path: String,
    pub config_exists: bool,
    pub content: serde_json::Value,
    pub has_mcp_config: bool,
    pub mcp_servers_count: u32,
    pub last_modified: Option<String>,
}

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

/// API error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }

    pub fn error_with_details(
        code: &str,
        message: &str,
        details: serde_json::Value,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: Some(details),
            }),
        }
    }
}

/// Convert from anyhow::Error to API error
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: err.to_string(),
            details: None,
        }
    }
}
