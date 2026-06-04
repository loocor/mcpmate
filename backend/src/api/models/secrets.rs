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
    OAuthAccessToken,
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
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SecretUsageData {
    pub alias: String,
    pub server_id: String,
    pub location: SecretUsageLocationData,
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
