use crate::macros::resp::api_resp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecretKindPayload {
    Generic,
    Token,
    ApiKey,
    Password,
    #[serde(rename = "oauth_client_secret")]
    OAuthClientSecret,
    #[serde(rename = "oauth_access_token")]
    OAuthAccessToken,
    #[serde(rename = "oauth_refresh_token")]
    OAuthRefreshToken,
    UrlCredential,
    HeaderValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretCreateReq {
    pub alias: String,
    pub kind: SecretKindPayload,
    pub value: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub origin: Option<SecretOriginData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretUpdateReq {
    pub alias: String,
    #[serde(default)]
    pub kind: Option<SecretKindPayload>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub origin: Option<SecretOriginData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretDeleteReq {
    pub alias: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretDetailsReq {
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretUsageReq {
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretOriginData {
    #[serde(default)]
    pub server_id: Option<String>,
    #[serde(default)]
    pub server_name: Option<String>,
    #[serde(default)]
    pub server_kind: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub field_group: Option<String>,
    #[serde(default)]
    pub field_key: Option<String>,
    #[serde(default)]
    pub field_index: Option<i64>,
    #[serde(default)]
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretMetadataData {
    pub alias: String,
    pub placeholder: String,
    pub kind: String,
    pub label: Option<String>,
    pub origin: Option<SecretOriginData>,
    pub provider_id: String,
    pub provider_kind: String,
    pub version: u64,
    pub used_by_count: u64,
    pub historical_usage_count: u64,
    pub unknown_usage_count: u64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecretUsageLocationData {
    StdioCommand,
    StdioArgument { index: usize },
    StdioEnv { name: String },
    StreamableHttpUrl,
    StreamableHttpHeader { name: String },
    OAuthToken,
    LlmProviderApiKey,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretUsageData {
    pub alias: String,
    pub server_id: String,
    pub location: SecretUsageLocationData,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretStoreProviderData {
    pub provider_id: String,
    pub provider_kind: String,
    pub provider_mode: String,
    pub security_level: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretStoreIssueData {
    pub reason_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretStoreStatusData {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SecretStoreProviderData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<SecretStoreIssueData>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretListData {
    pub secrets: Vec<SecretMetadataData>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretUsageListData {
    pub usages: Vec<SecretUsageData>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretDeleteData {
    pub alias: String,
    pub deleted: bool,
}

api_resp!(SecretMetadataResp, SecretMetadataData, "Secret metadata response");
api_resp!(SecretListResp, SecretListData, "Secret list response");
api_resp!(SecretUsageListResp, SecretUsageListData, "Secret usage list response");
api_resp!(SecretDeleteResp, SecretDeleteData, "Secret delete response");
api_resp!(
    SecretStoreStatusResp,
    SecretStoreStatusData,
    "Secret store readiness response"
);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderModePayload {
    OperatingSystem,
    Passphrase,
    LocalFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderSwitchReq {
    pub mode: ProviderModePayload,
    /// New passphrase when switching to passphrase mode.
    #[serde(default)]
    pub passphrase: Option<String>,
    /// Current passphrase when switching away from passphrase mode.
    #[serde(default)]
    pub current_passphrase: Option<String>,
    /// Required confirmation phrase when switching provider modes.
    #[serde(default)]
    pub confirmation_phrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProviderSwitchData {
    pub new_status: SecretStoreStatusData,
}

api_resp!(ProviderSwitchResp, ProviderSwitchData, "Provider switch response");

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretStoreUnlockReq {
    pub passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PassphraseRotateReq {
    pub current_passphrase: String,
    pub new_passphrase: String,
    pub confirm: String,
}

// ── Password Protection ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PasswordStatusData {
    pub enabled: bool,
    pub scope: Vec<String>,
    pub has_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PasswordSetReq {
    pub password: String,
    pub confirm: String,
    #[serde(default)]
    pub scope: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PasswordSetData {
    pub enabled: bool,
    pub scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PasswordVerifyReq {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PasswordVerifyData {
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PasswordChangeReq {
    pub old_password: String,
    pub new_password: String,
    pub confirm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PasswordClearReq {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PasswordScopeUpdateReq {
    pub scope: Vec<String>,
    pub current_password: String,
}

api_resp!(PasswordStatusResp, PasswordStatusData, "Password status response");
api_resp!(PasswordSetResp, PasswordSetData, "Password set response");
api_resp!(PasswordVerifyResp, PasswordVerifyData, "Password verify response");
