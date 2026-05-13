use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::macros::resp::api_resp;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Onboarding status payload")]
pub struct OnboardingStatusData {
    #[schemars(description = "Whether onboarding has been completed")]
    pub completed: bool,
    #[schemars(description = "Number of MCP servers currently configured")]
    pub servers_count: usize,
    #[schemars(description = "Number of MCP clients currently tracked")]
    pub clients_count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request to update onboarding completion state")]
pub struct OnboardingCompleteReq {
    #[schemars(description = "Whether onboarding should be marked as completed")]
    pub completed: bool,
}

api_resp!(OnboardingStatusResp, OnboardingStatusData, "Onboarding status response");

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Single runtime availability entry")]
pub struct RuntimeEntry {
    #[schemars(description = "Runtime name (e.g. node, python)")]
    pub name: String,
    #[schemars(description = "Whether this runtime is available on the system")]
    pub available: bool,
    #[schemars(description = "Version string, if available")]
    pub version: Option<String>,
    #[schemars(description = "Resolved filesystem path, if available")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Runtime detection payload")]
pub struct RuntimeCheckData {
    #[schemars(description = "List of detected runtimes")]
    pub runtimes: Vec<RuntimeEntry>,
    #[schemars(description = "Whether at least one JS runtime (node/bun) is available")]
    pub has_js_runtime: bool,
    #[schemars(description = "Whether at least one Python runtime (python3/uv) is available")]
    pub has_python_runtime: bool,
}

api_resp!(RuntimeCheckResp, RuntimeCheckData, "Runtime detection response");

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client config source to scan during onboarding")]
pub struct OnboardingServerScanClient {
    pub identifier: String,
    pub display_name: Option<String>,
    pub config_path: String,
    #[serde(default)]
    pub config_file_parse: Option<crate::api::models::client::ClientConfigFileParseData>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request to scan selected clients for existing MCP servers")]
pub struct OnboardingServerScanReq {
    pub clients: Vec<OnboardingServerScanClient>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Existing MCP server found in selected client configs")]
pub struct OnboardingServerCandidate {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
    pub source_clients: Vec<String>,
    pub source_client_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Per-client scan error")]
pub struct OnboardingServerScanError {
    pub client_name: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Onboarding server scan payload")]
pub struct OnboardingServerScanData {
    pub candidates: Vec<OnboardingServerCandidate>,
    pub errors: Vec<OnboardingServerScanError>,
}

api_resp!(
    OnboardingServerScanResp,
    OnboardingServerScanData,
    "Onboarding server scan response"
);
