use crate::core::oauth::{OAuthInitiateResult, OAuthStatus};
use crate::macros::resp::api_resp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerOAuthConfigReq {
    pub server_id: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub scopes: Option<String>,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerOAuthInitiateReq {
    pub server_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerOAuthPrepareReq {
    pub server_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub scopes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerOAuthCallbackReq {
    pub state: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerOAuthStatusReq {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerOAuthRevokeReq {
    pub server_id: String,
}

api_resp!(OAuthInitiateResp, OAuthInitiateResult, "Server OAuth initiate response");
api_resp!(OAuthStatusResp, OAuthStatus, "Server OAuth status response");
