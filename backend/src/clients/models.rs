use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Supported template output formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TemplateFormat {
    #[default]
    Json,
    Json5,
    Toml,
    Yaml,
}

impl TemplateFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateFormat::Json => "json",
            TemplateFormat::Json5 => "json5",
            TemplateFormat::Toml => "toml",
            TemplateFormat::Yaml => "yaml",
        }
    }
}

/// Storage type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageKind {
    #[serde(alias = "file_system")]
    #[default]
    File,
    Kv,
    Custom,
}

/// Backup retention policy for client configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BackupPolicy {
    KeepLast,
    KeepN,
    Off,
}

impl BackupPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupPolicy::KeepLast => "keep_last",
            BackupPolicy::KeepN => "keep_n",
            BackupPolicy::Off => "off",
        }
    }
}

/// Backup policy with optional limit parameter (for keep_n)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupPolicySetting {
    pub policy: BackupPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl Default for BackupPolicySetting {
    fn default() -> Self {
        // Default strategy: keep_n with a retention limit of 30
        Self {
            policy: BackupPolicy::KeepN,
            limit: Some(30),
        }
    }
}

impl BackupPolicySetting {
    pub fn from_pair(
        policy: Option<&str>,
        limit: Option<u32>,
    ) -> Self {
        match policy {
            Some("off") => Self {
                policy: BackupPolicy::Off,
                limit: None,
            },
            Some("keep_n") => {
                let normalized = limit.unwrap_or(30).max(1);
                Self {
                    policy: BackupPolicy::KeepN,
                    limit: Some(normalized),
                }
            }
            _ => Self::default(),
        }
    }

    pub fn as_db_pair(&self) -> (&'static str, Option<u32>) {
        match self.policy {
            BackupPolicy::Off => (self.policy.as_str(), None),
            BackupPolicy::KeepLast => (self.policy.as_str(), None),
            BackupPolicy::KeepN => (self.policy.as_str(), Some(self.limit.unwrap_or(30).max(1))),
        }
    }

    pub fn should_backup(&self) -> bool {
        !matches!(self.policy, BackupPolicy::Off)
    }

    pub fn retention_limit(&self) -> Option<usize> {
        match self.policy {
            BackupPolicy::Off => None,
            BackupPolicy::KeepLast => Some(1),
            BackupPolicy::KeepN => Some(self.limit.unwrap_or(30).max(1) as usize),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_policy_from_pair() {
        // Default now maps to keep_n with limit 30
        let default_setting = BackupPolicySetting::from_pair(None, None);
        assert_eq!(default_setting.policy, BackupPolicy::KeepN);
        assert_eq!(default_setting.limit, Some(30));

        let off = BackupPolicySetting::from_pair(Some("off"), Some(3));
        assert_eq!(off.policy, BackupPolicy::Off);
        assert_eq!(off.limit, None);

        let keep_n = BackupPolicySetting::from_pair(Some("keep_n"), Some(2));
        assert_eq!(keep_n.policy, BackupPolicy::KeepN);
        assert_eq!(keep_n.limit, Some(2));

        let keep_n_default = BackupPolicySetting::from_pair(Some("keep_n"), None);
        assert_eq!(keep_n_default.limit, Some(30));
    }

    #[test]
    fn parses_client_capability_config_from_parts() {
        let config = ClientCapabilityConfig::from_parts(
            Some("profiles"),
            Some("[\"prof-a\",\"prof-b\"]"),
            Some("custom-prof".to_string()),
        )
        .expect("capability config");

        assert_eq!(config.capability_source, CapabilitySource::Profiles);
        assert_eq!(config.selected_profile_ids, vec!["prof-a", "prof-b"]);
        assert_eq!(config.custom_profile_id.as_deref(), Some("custom-prof"));
    }

    #[test]
    fn defaults_client_capability_config_to_activated() {
        let config = ClientCapabilityConfig::from_parts(None, None, None).expect("default capability config");

        assert_eq!(config.capability_source, CapabilitySource::Activated);
        assert!(config.selected_profile_ids.is_empty());
        assert!(config.custom_profile_id.is_none());
    }
}

// Default now derives on enum with #[default]

/// Client configuration container type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContainerType {
    #[default]
    ObjectMap,
    Array,
}

/// Merge strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    #[default]
    Replace,
    DeepMerge,
}

/// Detection method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DetectionMethod {
    FilePath,
    BundleId,
    ConfigPath,
}

/// Template storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StorageConfig {
    pub kind: StorageKind,
    pub path_strategy: Option<String>,
    pub adapter: Option<String>,
}

/// Managed mode endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ManagedEndpointConfig {
    pub source: Option<String>,
}

/// Single transport format rule
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FormatRule {
    pub template: serde_json::Value,
    pub requires_type_field: bool,
}

/// Template configuration mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigMapping {
    pub container_keys: Vec<String>,
    pub container_type: ContainerType,
    pub merge_strategy: MergeStrategy,
    pub keep_original_config: bool,
    pub managed_endpoint: Option<ManagedEndpointConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_source: Option<String>,
    pub format_rules: HashMap<String, FormatRule>,
}

impl Default for ConfigMapping {
    fn default() -> Self {
        Self {
            container_keys: Vec::new(),
            container_type: ContainerType::ObjectMap,
            merge_strategy: MergeStrategy::Replace,
            keep_original_config: false,
            managed_endpoint: None,
            managed_source: None,
            format_rules: HashMap::new(),
        }
    }
}

impl ClientCapabilityConfig {
    pub fn from_parts(
        capability_source: Option<&str>,
        selected_profile_ids_json: Option<&str>,
        custom_profile_id: Option<String>,
    ) -> Result<Self, String> {
        let capability_source = capability_source
            .map(CapabilitySource::from_str)
            .transpose()
            .map_err(|_| {
                format!(
                    "invalid capability_source '{}': expected activated|profiles|custom",
                    capability_source.unwrap_or_default()
                )
            })?
            .unwrap_or_default();

        let selected_profile_ids = if let Some(raw) = selected_profile_ids_json {
            serde_json::from_str::<Vec<String>>(raw).map_err(|err| {
                format!("invalid selected_profile_ids payload '{}': {}", raw, err)
            })?
        } else {
            Vec::new()
        };

        Ok(Self {
            capability_source,
            selected_profile_ids,
            custom_profile_id,
        })
    }
}

/// Detection rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DetectionRule {
    pub method: DetectionMethod,
    pub value: String,
    pub config_path: Option<String>,
    pub priority: Option<u32>,
}

impl Default for DetectionRule {
    fn default() -> Self {
        Self {
            method: DetectionMethod::FilePath,
            value: String::new(),
            config_path: None,
            priority: None,
        }
    }
}

/// Client template definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ClientTemplate {
    pub identifier: String,
    pub display_name: Option<String>,
    pub version: Option<String>,
    pub format: TemplateFormat,
    pub protocol_revision: Option<String>,
    pub storage: StorageConfig,
    pub detection: HashMap<String, Vec<DetectionRule>>,
    pub config_mapping: ConfigMapping,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ClientTemplate {
    pub fn platform_rules(
        &self,
        platform: &str,
    ) -> Option<&[DetectionRule]> {
        self.detection.get(platform).map(Vec::as_slice)
    }
}

/// MCP configuration mode
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigMode {
    Native,
    #[default]
    Managed,
}

/// Capability source for client-scoped configuration and runtime policy.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySource {
    #[default]
    Activated,
    Profiles,
    Custom,
}

impl CapabilitySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilitySource::Activated => "activated",
            CapabilitySource::Profiles => "profiles",
            CapabilitySource::Custom => "custom",
        }
    }
}

impl fmt::Display for CapabilitySource {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseCapabilitySourceError;

impl fmt::Display for ParseCapabilitySourceError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid capability source")
    }
}

impl std::error::Error for ParseCapabilitySourceError {}

impl FromStr for CapabilitySource {
    type Err = ParseCapabilitySourceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "activated" => Ok(CapabilitySource::Activated),
            "profiles" => Ok(CapabilitySource::Profiles),
            "custom" => Ok(CapabilitySource::Custom),
            _ => Err(ParseCapabilitySourceError),
        }
    }
}

/// Persisted per-client capability configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct ClientCapabilityConfig {
    pub capability_source: CapabilitySource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_profile_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_profile_id: Option<String>,
}

/// Server context input to template rendering
#[derive(Debug, Clone, Serialize, Default)]
#[serde(default)]
pub struct ServerTemplateInput {
    pub name: String,
    pub display_name: Option<String>,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
    pub headers: HashMap<String, String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
