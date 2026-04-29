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
        // Default strategy: keep_n with a retention limit of 5
        Self {
            policy: BackupPolicy::KeepN,
            limit: Some(5),
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
                let normalized = limit.unwrap_or(5).max(1);
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
            BackupPolicy::KeepN => (self.policy.as_str(), Some(self.limit.unwrap_or(5).max(1))),
        }
    }

    pub fn should_backup(&self) -> bool {
        !matches!(self.policy, BackupPolicy::Off)
    }

    pub fn retention_limit(&self) -> Option<usize> {
        match self.policy {
            BackupPolicy::Off => None,
            BackupPolicy::KeepLast => Some(1),
            BackupPolicy::KeepN => Some(self.limit.unwrap_or(5).max(1) as usize),
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
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Suspended => "suspended",
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
pub enum AttachmentState {
    #[default]
    Attached,
    Detached,
    NotApplicable,
}

impl AttachmentState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AttachmentState::Attached => "attached",
            AttachmentState::Detached => "detached",
            AttachmentState::NotApplicable => "not_applicable",
        }
    }
}

impl fmt::Display for AttachmentState {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AttachmentState {
    type Err = ParseAttachmentStateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "attached" => Ok(AttachmentState::Attached),
            "detached" => Ok(AttachmentState::Detached),
            "not_applicable" => Ok(AttachmentState::NotApplicable),
            _ => Err(ParseAttachmentStateError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseAttachmentStateError;

impl fmt::Display for ParseAttachmentStateError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid attachment state")
    }
}

impl std::error::Error for ParseAttachmentStateError {}

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
    #[default]
    Review,
    /// Auto-approve and enable management for new clients.
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
        // Default now maps to keep_n with limit 5
        let default_setting = BackupPolicySetting::from_pair(None, None);
        assert_eq!(default_setting.policy, BackupPolicy::KeepN);
        assert_eq!(default_setting.limit, Some(5));

        let off = BackupPolicySetting::from_pair(Some("off"), Some(3));
        assert_eq!(off.policy, BackupPolicy::Off);
        assert_eq!(off.limit, None);

        let keep_n = BackupPolicySetting::from_pair(Some("keep_n"), Some(2));
        assert_eq!(keep_n.policy, BackupPolicy::KeepN);
        assert_eq!(keep_n.limit, Some(2));

        let keep_n_default = BackupPolicySetting::from_pair(Some("keep_n"), None);
        assert_eq!(keep_n_default.limit, Some(5));

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
    fn parses_unify_route_mode_values() {
        assert_eq!(
            UnifyRouteMode::from_str("broker_only").expect("parse broker_only"),
            UnifyRouteMode::BrokerOnly
        );
        assert_eq!(
            UnifyRouteMode::from_str("server_level").expect("parse server_level"),
            UnifyRouteMode::ServerLevel
        );
        assert_eq!(
            UnifyRouteMode::from_str("capability_level").expect("parse capability_level"),
            UnifyRouteMode::CapabilityLevel
        );
    }

    #[test]
    fn defaults_unify_direct_exposure_config_to_broker_only() {
        let config = UnifyDirectExposureConfig::default();

        assert_eq!(config.route_mode, UnifyRouteMode::BrokerOnly);
        assert!(config.selected_server_ids.is_empty());
        assert!(config.selected_tool_surfaces.is_empty());
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
    #[serde(default)]
    pub template: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers_field: Option<String>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
    #[serde(default, alias = "requires_type_field")]
    pub include_type: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
}

impl FormatRule {
    fn template_string_matches(
        value: &serde_json::Value,
        candidates: &[&str],
    ) -> bool {
        let Some(raw) = value.as_str() else {
            return false;
        };

        candidates.iter().any(|candidate| raw.trim() == *candidate)
    }

    pub fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        let Some(template_map) = self.template.as_object() else {
            return normalized;
        };

        let mut extra_fields = self.extra_fields.clone();

        for (key, value) in template_map {
            if Self::template_string_matches(value, &["{{command}}", "{{{command}}}"]) {
                normalized.command_field.get_or_insert_with(|| key.clone());
                continue;
            }

            if Self::template_string_matches(value, &["{{{json args}}}", "{{json args}}", "{{args}}", "{{{args}}}"]) {
                normalized.args_field.get_or_insert_with(|| key.clone());
                continue;
            }

            if Self::template_string_matches(value, &["{{{json env}}}", "{{json env}}", "{{env}}", "{{{env}}}"]) {
                normalized.env_field.get_or_insert_with(|| key.clone());
                continue;
            }

            if Self::template_string_matches(value, &["{{url}}", "{{{url}}}"]) {
                normalized.url_field.get_or_insert_with(|| key.clone());
                continue;
            }

            if Self::template_string_matches(
                value,
                &["{{{json headers}}}", "{{json headers}}", "{{headers}}", "{{{headers}}}"],
            ) {
                normalized.headers_field.get_or_insert_with(|| key.clone());
                continue;
            }

            if key == "type" {
                if let Some(type_value) = value.as_str() {
                    if !type_value.contains("{{") {
                        normalized.include_type = true;
                        normalized.type_value.get_or_insert_with(|| type_value.to_string());
                        continue;
                    }
                }
            }

            extra_fields.entry(key.clone()).or_insert_with(|| value.clone());
        }

        normalized.extra_fields = extra_fields;
        normalized
    }

    pub fn has_dimensions(&self) -> bool {
        let normalized = self.normalized();
        normalized.command_field.is_some()
            || normalized.args_field.is_some()
            || normalized.env_field.is_some()
            || normalized.include_type
            || normalized.type_value.is_some()
            || normalized.url_field.is_some()
            || normalized.headers_field.is_some()
            || !normalized.extra_fields.is_empty()
    }

    pub fn validate_for_transport(
        &self,
        transport: &str,
    ) -> Result<(), String> {
        let normalized = self.normalized();

        if normalized.include_type
            && normalized
                .type_value
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
        {
            return Err("Missing required format rule field: type_value".to_string());
        }

        match transport {
            "stdio" if normalized.command_field.is_none() => {
                return Err("Missing required stdio rule field: command_field".to_string());
            }
            "sse" | "streamable_http" if normalized.url_field.is_none() => {
                return Err(format!("Missing required {transport} rule field: url_field"));
            }
            _ => {}
        }

        Ok(())
    }

    pub fn to_template(&self) -> serde_json::Value {
        let normalized = self.normalized();
        if !normalized.has_dimensions() {
            return normalized.template.clone();
        }

        let mut map = serde_json::Map::new();

        if normalized.include_type
            && let Some(type_value) = normalized
                .type_value
                .clone()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        {
            map.insert("type".to_string(), serde_json::Value::String(type_value));
        }

        if let Some(command_key) = &normalized.command_field {
            map.insert(
                command_key.clone(),
                serde_json::Value::String("{{command}}".to_string()),
            );
        }

        if let Some(args_key) = &normalized.args_field {
            map.insert(
                args_key.clone(),
                serde_json::Value::String("{{{json args}}}".to_string()),
            );
        }

        if let Some(env_key) = &normalized.env_field {
            map.insert(env_key.clone(), serde_json::Value::String("{{{json env}}}".to_string()));
        }

        if let Some(url_key) = &normalized.url_field {
            map.insert(url_key.clone(), serde_json::Value::String("{{{url}}}".to_string()));
        }

        if let Some(headers_key) = &normalized.headers_field {
            map.insert(
                headers_key.clone(),
                serde_json::Value::String("{{{json headers}}}".to_string()),
            );
        }

        for (key, value) in &normalized.extra_fields {
            map.insert(key.clone(), value.clone());
        }

        serde_json::Value::Object(map)
    }
}

/// Structured parsing rules for reading and locating MCP server config inside a client config file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ClientConfigFileParse {
    pub format: TemplateFormat,
    pub container_type: ContainerType,
    pub container_keys: Vec<String>,
}

impl Default for ClientConfigFileParse {
    fn default() -> Self {
        Self {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: Vec::new(),
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse: Option<ClientConfigFileParse>,
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
            parse: None,
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

/// Database-backed runtime render definition for a client.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ClientRenderDefinition {
    pub identifier: String,
    pub format: TemplateFormat,
    pub storage: StorageConfig,
    pub config_mapping: ConfigMapping,
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

/// Unify direct-exposure route mode.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnifyRouteMode {
    #[default]
    BrokerOnly,
    ServerLevel,
    CapabilityLevel,
}

impl UnifyRouteMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnifyRouteMode::BrokerOnly => "broker_only",
            UnifyRouteMode::ServerLevel => "server_level",
            UnifyRouteMode::CapabilityLevel => "capability_level",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct UnifyDirectCapabilityIds {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompt_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub template_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct UnifyDirectExposureIntent {
    #[serde(default)]
    pub route_mode: UnifyRouteMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub server_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "UnifyDirectCapabilityIds::is_empty")]
    pub capability_ids: UnifyDirectCapabilityIds,
}

impl UnifyDirectCapabilityIds {
    pub fn is_empty(&self) -> bool {
        self.tool_ids.is_empty()
            && self.prompt_ids.is_empty()
            && self.resource_ids.is_empty()
            && self.template_ids.is_empty()
    }
}

impl UnifyDirectExposureIntent {
    pub fn from_parts(intent_json: Option<&str>) -> Result<Self, String> {
        match intent_json.filter(|raw| !raw.trim().is_empty()) {
            Some(raw) => serde_json::from_str::<Self>(raw)
                .map_err(|err| format!("invalid unify_direct_exposure_intent payload '{}': {}", raw, err)),
            None => Ok(Self::default()),
        }
    }
}

impl fmt::Display for UnifyRouteMode {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseUnifyRouteModeError;

impl fmt::Display for ParseUnifyRouteModeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid unify route mode")
    }
}

impl std::error::Error for ParseUnifyRouteModeError {}

impl FromStr for UnifyRouteMode {
    type Err = ParseUnifyRouteModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "broker_only" => Ok(UnifyRouteMode::BrokerOnly),
            "server_level" => Ok(UnifyRouteMode::ServerLevel),
            "capability_level" => Ok(UnifyRouteMode::CapabilityLevel),
            _ => Err(ParseUnifyRouteModeError),
        }
    }
}

/// Explicit tool surface selected for direct exposure in Unify capability-level mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub struct UnifyDirectToolSurface {
    pub server_id: String,
    pub tool_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub struct UnifyDirectPromptSurface {
    pub server_id: String,
    pub prompt_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub struct UnifyDirectResourceSurface {
    pub server_id: String,
    pub resource_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub struct UnifyDirectTemplateSurface {
    pub server_id: String,
    pub uri_template: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct UnifyDirectExposureDiagnostics {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalid_server_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalid_tool_surfaces: Vec<UnifyDirectToolSurfaceDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalid_prompt_surfaces: Vec<UnifyDirectPromptSurfaceDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalid_resource_surfaces: Vec<UnifyDirectResourceSurfaceDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalid_template_surfaces: Vec<UnifyDirectTemplateSurfaceDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalid_capability_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub struct UnifyDirectToolSurfaceDiagnostic {
    pub server_id: String,
    pub tool_name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub struct UnifyDirectPromptSurfaceDiagnostic {
    pub server_id: String,
    pub prompt_name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub struct UnifyDirectResourceSurfaceDiagnostic {
    pub server_id: String,
    pub resource_uri: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub struct UnifyDirectTemplateSurfaceDiagnostic {
    pub server_id: String,
    pub uri_template: String,
    pub reason: String,
}

/// Per-client Unify direct-exposure state.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct UnifyDirectExposureConfig {
    #[serde(default)]
    pub route_mode: UnifyRouteMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_server_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_tool_surfaces: Vec<UnifyDirectToolSurface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_prompt_surfaces: Vec<UnifyDirectPromptSurface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_resource_surfaces: Vec<UnifyDirectResourceSurface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_template_surfaces: Vec<UnifyDirectTemplateSurface>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientCapabilityConfigState {
    pub capability_config: ClientCapabilityConfig,
    pub custom_profile_missing: bool,
    pub unify_direct_exposure_intent: UnifyDirectExposureIntent,
    pub unify_direct_exposure: UnifyDirectExposureConfig,
    pub unify_direct_exposure_diagnostics: UnifyDirectExposureDiagnostics,
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
