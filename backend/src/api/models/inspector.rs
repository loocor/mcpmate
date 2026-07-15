use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Operating mode for Inspector endpoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InspectorMode {
    /// Aggregate/managed view: unique naming, profile-aware (recommended)
    #[default]
    Proxy,
    /// Direct upstream view: single server/instance, no unique naming
    Native,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorListQuery {
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
    #[serde(default)]
    pub refresh: bool,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorToolCallReq {
    pub tool: String,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub arguments: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub mode: InspectorMode,
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorPromptGetReq {
    pub name: String,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    pub arguments: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub mode: InspectorMode,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorResourceReadQuery {
    pub uri: String,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
    pub timeout_ms: Option<u64>,
}

// ==============================
// Strongly-typed response models (for OpenAPI and clients)
// ==============================

use crate::macros::resp::api_resp;

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorToolsListData {
    pub mode: String,
    // NOTE: rmcp::model types don't implement JsonSchema in this build; use Value for OpenAPI
    pub tools: Vec<serde_json::Value>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Vec<serde_json::Value>>,
    pub elapsed_ms: u64,
}
api_resp!(
    InspectorToolsListResp,
    InspectorToolsListData,
    "Inspector tools list response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorPromptsListData {
    pub mode: String,
    pub prompts: Vec<serde_json::Value>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Vec<serde_json::Value>>,
    pub elapsed_ms: u64,
}
api_resp!(
    InspectorPromptsListResp,
    InspectorPromptsListData,
    "Inspector prompts list response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorResourcesListData {
    pub mode: String,
    pub resources: Vec<serde_json::Value>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Vec<serde_json::Value>>,
    pub elapsed_ms: u64,
}
api_resp!(
    InspectorResourcesListResp,
    InspectorResourcesListData,
    "Inspector resources list response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorTemplatesListData {
    pub mode: String,
    pub templates: Vec<serde_json::Value>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Vec<serde_json::Value>>,
    pub elapsed_ms: u64,
}
api_resp!(
    InspectorTemplatesListResp,
    InspectorTemplatesListData,
    "Inspector templates list response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorPromptGetData {
    pub result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    pub elapsed_ms: u64,
}
api_resp!(
    InspectorPromptGetResp,
    InspectorPromptGetData,
    "Inspector prompt get response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorResourceReadData {
    pub result: serde_json::Value,
    pub server_id: Option<String>,
    pub elapsed_ms: u64,
}
api_resp!(
    InspectorResourceReadResp,
    InspectorResourceReadData,
    "Inspector resource read response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorToolCallData {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}
api_resp!(
    InspectorToolCallResp,
    InspectorToolCallData,
    "Inspector tool call response (accepted or completed)"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorToolCallStartData {
    pub call_id: String,
    pub server_id: String,
    pub mode: InspectorMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub request_id: String,
    pub progress_token: String,
}
api_resp!(
    InspectorToolCallStartResp,
    InspectorToolCallStartData,
    "Inspector tool call start response"
);

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorToolCallCancelReq {
    pub call_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorToolCallCancelData {
    pub cancelled: bool,
}
api_resp!(
    InspectorToolCallCancelResp,
    InspectorToolCallCancelData,
    "Inspector tool call cancel response"
);

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorSessionOpenReq {
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorSessionOpenData {
    pub session_id: String,
    pub server_id: String,
    pub mode: InspectorMode,
    pub expires_at_epoch_ms: u128,
}
api_resp!(
    InspectorSessionOpenResp,
    InspectorSessionOpenData,
    "Inspector session open response"
);

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorSessionCloseReq {
    pub session_id: String,
}

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorSessionCloseData {
    pub closed: bool,
}
api_resp!(
    InspectorSessionCloseResp,
    InspectorSessionCloseData,
    "Inspector session close response"
);

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorCallEventsQuery {
    pub call_id: String,
}

#[cfg(test)]
mod tests {
    use super::{InspectorListQuery, InspectorPromptGetReq, InspectorResourceReadQuery, InspectorSessionOpenReq};

    #[test]
    fn every_inspector_operation_accepts_an_independent_timeout() {
        let list: InspectorListQuery = serde_json::from_value(serde_json::json!({
            "server_id": "server-1",
            "timeout_ms": 125
        }))
        .expect("deserialize list query");
        let prompt: InspectorPromptGetReq = serde_json::from_value(serde_json::json!({
            "name": "prompt",
            "timeout_ms": 250
        }))
        .expect("deserialize prompt request");
        let resource: InspectorResourceReadQuery = serde_json::from_value(serde_json::json!({
            "uri": "file:///resource",
            "timeout_ms": 500
        }))
        .expect("deserialize resource request");
        let session: InspectorSessionOpenReq = serde_json::from_value(serde_json::json!({
            "server_id": "server-1",
            "timeout_ms": 1_000
        }))
        .expect("deserialize session request");

        assert_eq!(list.timeout_ms, Some(125));
        assert_eq!(prompt.timeout_ms, Some(250));
        assert_eq!(resource.timeout_ms, Some(500));
        assert_eq!(session.timeout_ms, Some(1_000));
    }
}
