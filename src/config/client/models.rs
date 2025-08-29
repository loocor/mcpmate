// Client management models for configuration generation
// Reuses detection models from system::detection, adds config-specific models

use crate::common::ClientCategory;
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// Re-export detection models to avoid duplication
pub use crate::system::detection::models::{Client, DetectedApp, DetectionMethod, DetectionResult, DetectionRule};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(description = "Configuration file format type - standard, mixed, or array")]
pub enum ClientConfigType {
    #[default]
    #[schemars(description = "Standard JSON configuration format")]
    Standard,
    #[schemars(description = "Mixed configuration with existing content")]
    Mixed,
    #[schemars(description = "Array-based configuration format")]
    Array,
}

/// Client application definition (unified for both JSON and DB)
/// Based on client table structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDefinition {
    // Database primary fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub identifier: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(default = "default_application_category")]
    pub category: ClientCategory,

    // Database state fields (auto-managed, JSON files should omit these)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_detected_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,

    // Nested configuration data
    pub detection_rules: HashMap<String, Vec<DetectionRuleDefinition>>, // platform -> rules
    pub config_rules: ConfigRulesDefinition,
}

/// Detection rule definition (unified for both JSON and DB)
/// Based on client_detection_rules table structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRuleDefinition {
    // Database primary fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,

    // Business logic fields
    #[serde(rename = "method")]
    pub detection_method: String,
    #[serde(rename = "value")]
    pub detection_value: String,
    pub config_path: Option<String>,
    #[serde(default)]
    pub priority: i32,

    // Database state fields
    #[serde(
        default = "crate::api::models::default_true_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Config rules definition (unified for both JSON and DB)
/// Based on client_config_rules table structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRulesDefinition {
    // Database primary fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,

    // Business logic fields
    pub top_level_key: String,
    #[serde(default)]
    pub config_type: ClientConfigType,
    pub supported_transports: Vec<String>,
    pub supported_runtimes: HashMap<String, Vec<String>>,
    pub format_rules: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub security_features: Option<serde_json::Value>,

    // Database state fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Root configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfigFile {
    pub version: String,
    pub client: Vec<ClientDefinition>,
}

/// Helper function for default application category
fn default_application_category() -> ClientCategory {
    ClientCategory::Application
}

/// Load client configuration from JSON file
pub async fn load_client_config<P: AsRef<Path>>(path: P) -> Result<ClientConfigFile> {
    let content = tokio::fs::read_to_string(path.as_ref()).await?;
    let config: ClientConfigFile = serde_json::from_str(&content)?;
    Ok(config)
}

/// Client rule manager for hot reloading and version management
pub struct ClientRuleManager {
    config_file_path: String,
    current_version: Option<String>,
    last_modified: Option<std::time::SystemTime>,
}

impl ClientRuleManager {
    /// Create new rule manager
    pub fn new(config_file_path: String) -> Self {
        Self {
            config_file_path,
            current_version: None,
            last_modified: None,
        }
    }

    /// Check if rules need to be reloaded
    pub async fn needs_reload(&self) -> Result<bool> {
        let metadata = tokio::fs::metadata(&self.config_file_path).await?;
        let modified = metadata.modified()?;

        Ok(self.last_modified.is_none_or(|last| modified > last))
    }

    /// Load or reload rules from file
    pub async fn load_rules(&mut self) -> Result<ClientConfigFile> {
        let config = load_client_config(&self.config_file_path).await?;

        // Update tracking information
        self.current_version = Some(config.version.clone());
        let metadata = tokio::fs::metadata(&self.config_file_path).await?;
        self.last_modified = Some(metadata.modified()?);

        tracing::info!(
            "Loaded client rules version {} from {}",
            config.version,
            self.config_file_path
        );

        Ok(config)
    }

    /// Get current version
    pub fn current_version(&self) -> Option<&String> {
        self.current_version.as_ref()
    }

    /// Validate rules configuration
    pub fn validate_rules(
        &self,
        config: &ClientConfigFile,
    ) -> Result<()> {
        // Basic validation
        if config.client.is_empty() {
            return Err(anyhow::anyhow!("No client defined in configuration"));
        }

        for client in &config.client {
            // Validate client identifier
            if client.identifier.is_empty() {
                return Err(anyhow::anyhow!("Client identifier cannot be empty"));
            }

            // Validate detection rules
            if client.detection_rules.is_empty() {
                return Err(anyhow::anyhow!("Client '{}' has no detection rules", client.identifier));
            }

            // Validate config rules
            if client.config_rules.supported_transports.is_empty() {
                return Err(anyhow::anyhow!(
                    "Client '{}' has no supported transports",
                    client.identifier
                ));
            }

            if client.config_rules.format_rules.is_empty() {
                return Err(anyhow::anyhow!("Client '{}' has no format rules", client.identifier));
            }
        }

        tracing::info!("Client rules validation passed");
        Ok(())
    }
}

/// Implementation helpers for ClientDefinition
impl ClientDefinition {
    /// Prepare for database insertion (set required fields, generate IDs)
    pub fn prepare_for_db_insert(
        &mut self,
        client_id: String,
    ) {
        self.id = Some(client_id.clone());
        self.enabled = Some(false); // Default to disabled
        self.detected = Some(false);

        // Set description if not provided
        if self.description.is_none() {
            self.description = Some(format!("{} - Auto-configured client", self.display_name));
        }

        // Prepare nested detection rules
        for (platform, rules) in &mut self.detection_rules {
            for rule in rules {
                rule.prepare_for_db_insert(client_id.clone(), self.identifier.clone(), platform.clone());
            }
        }

        // Prepare config rules
        self.config_rules
            .prepare_for_db_insert(client_id.clone(), self.identifier.clone());
    }
}

impl DetectionRuleDefinition {
    /// Prepare for database insertion
    pub fn prepare_for_db_insert(
        &mut self,
        client_id: String,
        identifier: String,
        platform: String,
    ) {
        self.id = Some(crate::generate_id!("rule"));
        self.client_id = Some(client_id);
        self.identifier = Some(identifier);
        self.platform = Some(platform);
        self.enabled = Some(true);

        // Set config_path fallback
        if self.config_path.is_none() {
            self.config_path = Some(self.detection_value.clone());
        }
    }
}

impl ConfigRulesDefinition {
    /// Prepare for database insertion
    pub fn prepare_for_db_insert(
        &mut self,
        client_id: String,
        identifier: String,
    ) {
        self.id = Some(crate::generate_id!("conf"));
        self.client_id = Some(client_id);
        self.identifier = Some(identifier);
    }
}

/// Configuration rule for generating client configs
/// Maps to client_config_rules table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRule {
    pub id: String,
    pub client_id: String,
    pub identifier: String,
    pub top_level_key: String,
    pub config_type: ClientConfigType,
    pub supported_transports: Vec<String>,
    pub supported_runtimes: HashMap<String, Vec<String>>,
    pub format_rules: HashMap<String, FormatRule>,
    pub security_features: Option<SecurityFeatures>,
}

/// Format rule for a specific transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatRule {
    pub template: HashMap<String, serde_json::Value>,
    pub requires_type_field: bool,
}

/// Security features for client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFeatures {
    pub supports_inputs: Option<bool>,
    pub supports_env_file: Option<bool>,
}

/// Configuration generation mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationMode {
    /// Direct connection to original servers (transparent proxy)
    Transparent,
    /// Connection through MCPMate proxy (hosted mode)
    Hosted,
}

/// Request for configuration generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub identifier: String,
    pub mode: GenerationMode,
    pub profile_id: Option<String>,
    pub servers: Option<Vec<String>>, // Specific servers to include
}

/// Generated configuration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedConfig {
    pub identifier: String,
    pub mode: GenerationMode,
    pub config_content: String, // JSON string
    pub config_path: String,
    pub backup_needed: bool,
    pub preview: bool,
}

/// Configuration application request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRequest {
    pub identifier: String,
    pub config: GeneratedConfig,
    pub create_backup: bool,
    pub dry_run: bool,
}

/// Configuration application result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationResult {
    pub success: bool,
    pub identifier: String,
    pub config_path: String,
    pub backup_path: Option<String>,
    pub error_message: Option<String>,
}

/// Batch configuration application result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchApplicationResult {
    pub success_count: usize,
    pub successful_client: Vec<String>,
    pub failed_client: std::collections::HashMap<String, String>,
}

/// Client runtime status (extends Client with runtime info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatus {
    pub client: Client,
    pub detected: bool,
    pub last_detected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub install_path: Option<String>,
    pub config_path: Option<String>,
    pub version: Option<String>,
    pub detection_method: Option<String>,
}

impl From<Client> for ClientStatus {
    fn from(client: Client) -> Self {
        Self {
            client,
            detected: false,
            last_detected_at: None,
            install_path: None,
            config_path: None,
            version: None,
            detection_method: None,
        }
    }
}

impl From<DetectedApp> for ClientStatus {
    fn from(detected_app: DetectedApp) -> Self {
        Self {
            client: detected_app.client,
            detected: true,
            last_detected_at: Some(chrono::Utc::now()),
            install_path: Some(detected_app.install_path.to_string_lossy().to_string()),
            config_path: Some(detected_app.config_path.to_string_lossy().to_string()),
            version: detected_app.version,
            detection_method: Some(detected_app.verified_methods.join(",")),
        }
    }
}

/// Helper functions for working with JSON fields in database
impl ConfigRule {
    /// Parse supported_transports from JSON string
    pub fn parse_supported_transports(json_str: &str) -> Result<Vec<String>, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Parse supported_runtimes from JSON string
    pub fn parse_supported_runtimes(json_str: &str) -> Result<HashMap<String, Vec<String>>, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Parse format_rules from JSON string
    pub fn parse_format_rules(json_str: &str) -> Result<HashMap<String, FormatRule>, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Parse security_features from JSON string
    pub fn parse_security_features(json_str: &str) -> Result<SecurityFeatures, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}
