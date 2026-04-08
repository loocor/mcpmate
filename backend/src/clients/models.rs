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
            Some("keep_last") => Self {
                policy: BackupPolicy::KeepLast,
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

/// Client onboarding policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingPolicy {
    #[default]
    AutoManage,
    RequireApproval,
    Manual,
}

impl OnboardingPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            OnboardingPolicy::AutoManage => "auto_manage",
            OnboardingPolicy::RequireApproval => "require_approval",
            OnboardingPolicy::Manual => "manual",
        }
    }
}

impl fmt::Display for OnboardingPolicy {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for OnboardingPolicy {
    type Err = ParseOnboardingPolicyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto_manage" => Ok(OnboardingPolicy::AutoManage),
            "require_approval" => Ok(OnboardingPolicy::RequireApproval),
            "manual" => Ok(OnboardingPolicy::Manual),
            _ => Err(ParseOnboardingPolicyError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOnboardingPolicyError;

impl fmt::Display for ParseOnboardingPolicyError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid onboarding policy")
    }
}

impl std::error::Error for ParseOnboardingPolicyError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClientConnectionMode {
    #[default]
    LocalConfigDetected,
    RemoteHttp,
    Manual,
}

impl ClientConnectionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientConnectionMode::LocalConfigDetected => "local_config_detected",
            ClientConnectionMode::RemoteHttp => "remote_http",
            ClientConnectionMode::Manual => "manual",
        }
    }
}

impl fmt::Display for ClientConnectionMode {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ClientConnectionMode {
    type Err = ParseClientConnectionModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local_config_detected" => Ok(ClientConnectionMode::LocalConfigDetected),
            "remote_http" => Ok(ClientConnectionMode::RemoteHttp),
            "manual" => Ok(ClientConnectionMode::Manual),
            _ => Err(ParseClientConnectionModeError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseClientConnectionModeError;

impl fmt::Display for ParseClientConnectionModeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid client connection mode")
    }
}

impl std::error::Error for ParseClientConnectionModeError {}

/// Client approval status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    #[default]
    Approved,
    Suspended,
    Rejected,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Suspended => "suspended",
            ApprovalStatus::Rejected => "rejected",
        }
    }
}

impl fmt::Display for ApprovalStatus {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ApprovalStatus {
    type Err = ParseApprovalStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(ApprovalStatus::Pending),
            "approved" => Ok(ApprovalStatus::Approved),
            "suspended" => Ok(ApprovalStatus::Suspended),
            "rejected" => Ok(ApprovalStatus::Rejected),
            _ => Err(ParseApprovalStatusError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseApprovalStatusError;

impl fmt::Display for ParseApprovalStatusError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid approval status")
    }
}

impl std::error::Error for ParseApprovalStatusError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClientRecordKind {
    #[default]
    TemplateKnown,
    ObservedUnknown,
}

impl ClientRecordKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientRecordKind::TemplateKnown => "template_known",
            ClientRecordKind::ObservedUnknown => "observed_unknown",
        }
    }
}

impl fmt::Display for ClientRecordKind {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ClientRecordKind {
    type Err = ParseClientRecordKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "template_known" => Ok(ClientRecordKind::TemplateKnown),
            "observed_unknown" => Ok(ClientRecordKind::ObservedUnknown),
            _ => Err(ParseClientRecordKindError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseClientRecordKindError;

impl fmt::Display for ParseClientRecordKindError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid client record kind")
    }
}

impl std::error::Error for ParseClientRecordKindError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClientGovernanceKind {
    #[default]
    Passive,
    Active,
}

impl ClientGovernanceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientGovernanceKind::Passive => "passive",
            ClientGovernanceKind::Active => "active",
        }
    }
}

impl fmt::Display for ClientGovernanceKind {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ClientGovernanceKind {
    type Err = ParseClientGovernanceKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "passive" => Ok(ClientGovernanceKind::Passive),
            "active" => Ok(ClientGovernanceKind::Active),
            _ => Err(ParseClientGovernanceKindError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseClientGovernanceKindError;

impl fmt::Display for ParseClientGovernanceKindError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid client governance kind")
    }
}

impl std::error::Error for ParseClientGovernanceKindError {}

/// Default governance when a new client identifier is observed (dashboard + MCP proxy).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FirstContactBehavior {
    /// Reject unknown clients until explicitly registered (no passive row on first MCP connect).
    Deny,
    /// Require approval: unknown clients appear as pending; MCP initialize fails until approved.
    Review,
    /// Auto-approve and enable management for new clients.
    #[default]
    Allow,
}

impl FirstContactBehavior {
    pub fn as_str(&self) -> &'static str {
        match self {
            FirstContactBehavior::Deny => "deny",
            FirstContactBehavior::Review => "review",
            FirstContactBehavior::Allow => "allow",
        }
    }
}

impl fmt::Display for FirstContactBehavior {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for FirstContactBehavior {
    type Err = ParseFirstContactBehaviorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deny" => Ok(FirstContactBehavior::Deny),
            "review" => Ok(FirstContactBehavior::Review),
            "allow" => Ok(FirstContactBehavior::Allow),
            // Legacy four-mode values (pre governance simplification)
            "pending_review" | "allow_then_review" => Ok(FirstContactBehavior::Review),
            _ => Err(ParseFirstContactBehaviorError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseFirstContactBehaviorError;

impl fmt::Display for ParseFirstContactBehaviorError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid first contact behavior")
    }
}

impl std::error::Error for ParseFirstContactBehaviorError {}

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

        let keep_last = BackupPolicySetting::from_pair(Some("keep_last"), Some(99));
        assert_eq!(keep_last.policy, BackupPolicy::KeepLast);
        assert_eq!(keep_last.limit, None);
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

    #[test]
    fn approval_status_supports_suspended() {
        assert_eq!(
            ApprovalStatus::from_str("suspended").expect("parse suspended"),
            ApprovalStatus::Suspended
        );
        assert_eq!(ApprovalStatus::Suspended.as_str(), "suspended");
    }

    #[test]
    fn parses_connection_mode_values() {
        assert_eq!(
            ClientConnectionMode::from_str("local_config_detected").expect("parse local config detected"),
            ClientConnectionMode::LocalConfigDetected
        );
        assert_eq!(
            ClientConnectionMode::from_str("remote_http").expect("parse remote http"),
            ClientConnectionMode::RemoteHttp
        );
        assert_eq!(
            ClientConnectionMode::from_str("manual").expect("parse manual"),
            ClientConnectionMode::Manual
        );
    }

    #[test]
    fn parses_record_kind_values() {
        assert_eq!(
            ClientRecordKind::from_str("template_known").expect("parse template known"),
            ClientRecordKind::TemplateKnown
        );
        assert_eq!(
            ClientRecordKind::from_str("observed_unknown").expect("parse observed unknown"),
            ClientRecordKind::ObservedUnknown
        );
    }

    #[test]
    fn parses_governance_kind_values() {
        assert_eq!(
            ClientGovernanceKind::from_str("passive").expect("parse passive governance kind"),
            ClientGovernanceKind::Passive
        );
        assert_eq!(
            ClientGovernanceKind::from_str("active").expect("parse active governance kind"),
            ClientGovernanceKind::Active
        );
    }

    #[test]
    fn parses_first_contact_behavior_values() {
        assert_eq!(
            FirstContactBehavior::from_str("deny").expect("parse deny"),
            FirstContactBehavior::Deny
        );
        assert_eq!(
            FirstContactBehavior::from_str("review").expect("parse review"),
            FirstContactBehavior::Review
        );
        assert_eq!(
            FirstContactBehavior::from_str("pending_review").expect("legacy pending_review maps to review"),
            FirstContactBehavior::Review
        );
        assert_eq!(
            FirstContactBehavior::from_str("allow_then_review").expect("legacy allow_then_review maps to review"),
            FirstContactBehavior::Review
        );
        assert_eq!(
            FirstContactBehavior::from_str("allow").expect("parse allow"),
            FirstContactBehavior::Allow
        );
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
            serde_json::from_str::<Vec<String>>(raw)
                .map_err(|err| format!("invalid selected_profile_ids payload '{}': {}", raw, err))?
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
