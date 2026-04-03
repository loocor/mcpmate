use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfigInput {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub scopes: Option<String>,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OAuthConnectionState {
    NotConfigured,
    Disconnected,
    Connected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OAuthInitiateResult {
    pub server_id: String,
    pub authorization_url: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OAuthStatus {
    pub server_id: String,
    pub configured: bool,
    pub state: OAuthConnectionState,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub client_id: Option<String>,
    pub scopes: Option<String>,
    pub redirect_uri: Option<String>,
    pub has_client_secret: bool,
    pub manual_authorization_override: bool,
    pub expires_at: Option<String>,
}
