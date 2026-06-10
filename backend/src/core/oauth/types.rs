use anyhow::Result;
use mcpmate_secrets::parse_placeholder;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthPrepareInput {
    pub redirect_uri: String,
    pub scopes: Option<String>,
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
#[serde(rename_all = "snake_case")]
pub enum OAuthCustodyState {
    Missing,
    Secure,
    LegacyPlaintext,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OAuthStatusIssue {
    pub code: String,
    pub message: String,
}

/// Classify the custody state for a set of OAuth credential values.
///
/// Returns `(custody_state, requires_reconnect, issue)`.
pub fn classify_custody(
    secure_store_available: bool,
    values: &[&str],
) -> Result<(OAuthCustodyState, bool, Option<OAuthStatusIssue>)> {
    if !secure_store_available {
        return Ok((
            OAuthCustodyState::Unavailable,
            true,
            Some(OAuthStatusIssue {
                code: "secure_store_unavailable".to_string(),
                message: "Secure Store is unavailable; unlock or initialize it before connecting OAuth.".to_string(),
            }),
        ));
    }

    for value in values
        .iter()
        .filter(|value| !value.trim().is_empty())
    {
        if parse_placeholder(value.trim())?.is_none() {
            return Ok((
                OAuthCustodyState::LegacyPlaintext,
                true,
                Some(OAuthStatusIssue {
                    code: "legacy_plaintext_oauth_credentials".to_string(),
                    message: "OAuth credentials were saved before Secure Store custody; reconnect OAuth to store them securely.".to_string(),
                }),
            ));
        }
    }

    Ok((OAuthCustodyState::Secure, false, None))
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
    pub custody_state: OAuthCustodyState,
    pub requires_reconnect: bool,
    pub issue: Option<OAuthStatusIssue>,
}
