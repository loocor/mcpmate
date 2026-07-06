use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use crate::inspector::contract::{InspectorMode, InspectorProxyMode, InspectorProxyScope};
use crate::inspector::target::{
    InspectorCapabilityListRequest, InspectorCapabilityPatchRequest, InspectorLlmEvaluationRequest,
    InspectorPromptGetRequest, InspectorResourceReadRequest, InspectorSnapshotRequest, InspectorTarget,
    InspectorTargetRequest, InspectorToolCallRequest,
};
use crate::inspector::workspace::{InspectorServerProvenance, InspectorServerRecordInput};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorListQuery {
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
    #[serde(default)]
    pub refresh: bool,
    #[serde(default)]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(default)]
    pub proxy_scope: Option<InspectorProxyScope>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorToolCallReq {
    pub tool: String,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    pub arguments: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub mode: InspectorMode,
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(default)]
    pub proxy_scope: Option<InspectorProxyScope>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorCapabilityPatchUpsertReq {
    pub capability_kind: String,
    pub capability_key: String,
    pub patch: Map<String, Value>,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorLlmEvaluationReq {
    pub scenario: String,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub max_tools: Option<usize>,
    #[serde(default)]
    pub dimensions: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<Value>,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
    #[serde(default)]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(default)]
    pub proxy_scope: Option<InspectorProxyScope>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorPromptGetReq {
    pub name: String,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    pub arguments: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub mode: InspectorMode,
    #[serde(default)]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(default)]
    pub proxy_scope: Option<InspectorProxyScope>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorSnapshotQuery {
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
    #[serde(default)]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(default)]
    pub proxy_scope: Option<InspectorProxyScope>,
    #[serde(default)]
    pub spec_version: Option<String>,
    #[serde(default)]
    pub package_source: Option<String>,
    #[serde(default)]
    pub scan_depth: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorResourceReadQuery {
    pub uri: String,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
    #[serde(default)]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(default)]
    pub proxy_scope: Option<InspectorProxyScope>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorScratchServerCreateReq {
    pub name: String,
    pub config: Value,
    #[serde(default)]
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorScratchServerDeleteReq {
    pub record_id: String,
}

fn inspector_target_request(
    mode: InspectorMode,
    server_id: &Option<String>,
    server_name: &Option<String>,
    scratch_id: &Option<String>,
    proxy_mode: Option<InspectorProxyMode>,
    proxy_scope: Option<InspectorProxyScope>,
) -> InspectorTargetRequest {
    InspectorTargetRequest {
        mode,
        server_id: server_id.clone(),
        server_name: server_name.clone(),
        scratch_id: scratch_id.clone(),
        proxy_mode,
        proxy_scope,
    }
}

impl From<&InspectorListQuery> for InspectorTargetRequest {
    fn from(query: &InspectorListQuery) -> Self {
        inspector_target_request(
            query.mode,
            &query.server_id,
            &query.server_name,
            &query.scratch_id,
            query.proxy_mode,
            query.proxy_scope,
        )
    }
}

impl From<&InspectorListQuery> for InspectorCapabilityListRequest {
    fn from(query: &InspectorListQuery) -> Self {
        Self {
            target: query.into(),
            session_id: query.session_id.clone(),
            refresh: query.refresh,
        }
    }
}

impl From<&InspectorListQuery> for InspectorSnapshotRequest {
    fn from(query: &InspectorListQuery) -> Self {
        Self {
            target: query.into(),
            session_id: query.session_id.clone(),
            spec_version: None,
            package_source: None,
            scan_depth: None,
        }
    }
}

impl From<&InspectorSnapshotQuery> for InspectorTargetRequest {
    fn from(query: &InspectorSnapshotQuery) -> Self {
        inspector_target_request(
            query.mode,
            &query.server_id,
            &query.server_name,
            &query.scratch_id,
            query.proxy_mode,
            query.proxy_scope,
        )
    }
}

impl From<&InspectorSnapshotQuery> for InspectorSnapshotRequest {
    fn from(query: &InspectorSnapshotQuery) -> Self {
        Self {
            target: query.into(),
            session_id: query.session_id.clone(),
            spec_version: query.spec_version.clone(),
            package_source: query.package_source.clone(),
            scan_depth: query.scan_depth.clone(),
        }
    }
}

impl From<&InspectorToolCallReq> for InspectorTargetRequest {
    fn from(req: &InspectorToolCallReq) -> Self {
        inspector_target_request(
            req.mode,
            &req.server_id,
            &req.server_name,
            &req.scratch_id,
            req.proxy_mode,
            req.proxy_scope,
        )
    }
}

impl From<&InspectorToolCallReq> for InspectorToolCallRequest {
    fn from(req: &InspectorToolCallReq) -> Self {
        Self {
            target: req.into(),
            session_id: req.session_id.clone(),
            tool: req.tool.clone(),
            arguments: req.arguments.clone(),
            timeout_ms: req.timeout_ms,
        }
    }
}

impl From<&InspectorCapabilityPatchUpsertReq> for InspectorTargetRequest {
    fn from(req: &InspectorCapabilityPatchUpsertReq) -> Self {
        inspector_target_request(req.mode, &req.server_id, &req.server_name, &req.scratch_id, None, None)
    }
}

impl From<&InspectorCapabilityPatchUpsertReq> for InspectorCapabilityPatchRequest {
    fn from(req: &InspectorCapabilityPatchUpsertReq) -> Self {
        Self {
            target: req.into(),
            capability_kind: req.capability_kind.clone(),
            capability_key: req.capability_key.clone(),
            patch: req.patch.clone(),
        }
    }
}

impl From<&InspectorLlmEvaluationReq> for InspectorTargetRequest {
    fn from(req: &InspectorLlmEvaluationReq) -> Self {
        inspector_target_request(
            req.mode,
            &req.server_id,
            &req.server_name,
            &req.scratch_id,
            req.proxy_mode,
            req.proxy_scope,
        )
    }
}

impl From<&InspectorLlmEvaluationReq> for InspectorLlmEvaluationRequest {
    fn from(req: &InspectorLlmEvaluationReq) -> Self {
        Self {
            target: req.into(),
            session_id: req.session_id.clone(),
            provider_id: req.provider_id.clone(),
            scenario: req.scenario.clone(),
            max_tools: req.max_tools,
            dimensions: req.dimensions.clone(),
            evidence: req.evidence.clone(),
        }
    }
}

impl From<&InspectorSessionOpenReq> for InspectorTargetRequest {
    fn from(req: &InspectorSessionOpenReq) -> Self {
        inspector_target_request(
            req.mode,
            &req.server_id,
            &req.server_name,
            &req.scratch_id,
            req.proxy_mode,
            req.proxy_scope,
        )
    }
}

impl From<&InspectorPromptGetReq> for InspectorTargetRequest {
    fn from(req: &InspectorPromptGetReq) -> Self {
        inspector_target_request(
            req.mode,
            &req.server_id,
            &req.server_name,
            &req.scratch_id,
            req.proxy_mode,
            req.proxy_scope,
        )
    }
}

impl From<&InspectorPromptGetReq> for InspectorPromptGetRequest {
    fn from(req: &InspectorPromptGetReq) -> Self {
        Self {
            target: req.into(),
            session_id: req.session_id.clone(),
            name: req.name.clone(),
            arguments: req.arguments.clone(),
        }
    }
}

impl From<&InspectorResourceReadQuery> for InspectorTargetRequest {
    fn from(req: &InspectorResourceReadQuery) -> Self {
        inspector_target_request(
            req.mode,
            &req.server_id,
            &req.server_name,
            &req.scratch_id,
            req.proxy_mode,
            req.proxy_scope,
        )
    }
}

impl From<&InspectorResourceReadQuery> for InspectorResourceReadRequest {
    fn from(req: &InspectorResourceReadQuery) -> Self {
        Self {
            target: req.into(),
            session_id: req.session_id.clone(),
            uri: req.uri.clone(),
        }
    }
}

impl TryFrom<&InspectorScratchServerCreateReq> for InspectorServerRecordInput {
    type Error = serde_json::Error;

    fn try_from(req: &InspectorScratchServerCreateReq) -> Result<Self, Self::Error> {
        Ok(Self {
            name: req.name.clone(),
            config: serde_json::from_value(req.config.clone())?,
            provenance: InspectorServerProvenance::Scratch {
                origin: req.origin.clone(),
            },
        })
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
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
    pub scratch_id: Option<String>,
    #[serde(default)]
    pub mode: InspectorMode,
    #[serde(default)]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(default)]
    pub proxy_scope: Option<InspectorProxyScope>,
}

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorTargetData {
    pub mode: InspectorMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scratch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_scope: Option<InspectorProxyScope>,
}

impl From<&InspectorTarget> for InspectorTargetData {
    fn from(target: &InspectorTarget) -> Self {
        Self {
            mode: target.mode(),
            server_id: target.server_id().map(str::to_string),
            scratch_id: target.scratch_id().map(str::to_string),
            proxy_mode: target.proxy_mode(),
            proxy_scope: target.proxy_scope(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorSessionOpenData {
    pub session_id: String,
    pub target: InspectorTargetData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scratch_id: Option<String>,
    pub mode: InspectorMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_mode: Option<InspectorProxyMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_scope: Option<InspectorProxyScope>,
    pub expires_at_epoch_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handshake: Option<crate::inspector::handshake::InspectorSessionHandshakeData>,
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

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorSessionRefreshReq {
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

api_resp!(
    InspectorSessionRefreshResp,
    InspectorSessionOpenData,
    "Inspector session refresh response"
);

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorCallEventsQuery {
    pub call_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectorToolCallEvidenceQuery {
    pub call_id: String,
}

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorToolCallEvidenceData {
    pub snapshot: serde_json::Value,
}
api_resp!(
    InspectorToolCallEvidenceResp,
    InspectorToolCallEvidenceData,
    "Inspector tool call evidence response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorCompatibilitySnapshotData {
    pub snapshot: serde_json::Value,
}
api_resp!(
    InspectorCompatibilitySnapshotResp,
    InspectorCompatibilitySnapshotData,
    "Inspector compatibility snapshot response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorPackageSafetySnapshotData {
    pub snapshot: serde_json::Value,
}
api_resp!(
    InspectorPackageSafetySnapshotResp,
    InspectorPackageSafetySnapshotData,
    "Inspector package safety snapshot response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorCapabilityPatchData {
    pub record: serde_json::Value,
}
api_resp!(
    InspectorCapabilityPatchResp,
    InspectorCapabilityPatchData,
    "Inspector capability patch response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorLlmEvaluationData {
    pub evaluation: serde_json::Value,
}
api_resp!(
    InspectorLlmEvaluationResp,
    InspectorLlmEvaluationData,
    "Inspector LLM evaluation response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorScratchServerListData {
    pub records: Vec<serde_json::Value>,
    pub total: usize,
}
api_resp!(
    InspectorScratchServerListResp,
    InspectorScratchServerListData,
    "Inspector scratch server list response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorScratchServerCreateData {
    pub record: serde_json::Value,
}
api_resp!(
    InspectorScratchServerCreateResp,
    InspectorScratchServerCreateData,
    "Inspector scratch server create response"
);

#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
pub struct InspectorScratchServerDeleteData {
    pub record_id: String,
    pub deleted: bool,
}
api_resp!(
    InspectorScratchServerDeleteResp,
    InspectorScratchServerDeleteData,
    "Inspector scratch server delete response"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_data_keeps_scratch_records_out_of_server_id() {
        let target = InspectorTarget::native_scratch("scratch-a".to_string());
        let data = InspectorTargetData::from(&target);

        assert_eq!(data.mode, InspectorMode::Native);
        assert_eq!(data.server_id, None);
        assert_eq!(data.scratch_id.as_deref(), Some("scratch-a"));
        assert_eq!(data.proxy_mode, None);
        assert_eq!(data.proxy_scope, None);
    }

    #[test]
    fn tool_call_request_converts_target_contract_fields() {
        let req = InspectorToolCallReq {
            tool: "time_convert_time".to_string(),
            server_id: Some("server-a".to_string()),
            server_name: Some("Fetch".to_string()),
            scratch_id: None,
            arguments: None,
            mode: InspectorMode::Proxy,
            timeout_ms: Some(8_000),
            session_id: Some("session-a".to_string()),
            proxy_mode: Some(InspectorProxyMode::Unify),
            proxy_scope: Some(InspectorProxyScope::ActiveCatalog),
        };

        let domain = InspectorToolCallRequest::from(&req);

        assert_eq!(domain.target.mode, InspectorMode::Proxy);
        assert_eq!(domain.target.server_id.as_deref(), Some("server-a"));
        assert_eq!(domain.target.server_name.as_deref(), Some("Fetch"));
        assert_eq!(domain.target.scratch_id, None);
        assert_eq!(domain.target.proxy_mode, Some(InspectorProxyMode::Unify));
        assert_eq!(domain.target.proxy_scope, Some(InspectorProxyScope::ActiveCatalog));
        assert_eq!(domain.session_id.as_deref(), Some("session-a"));
        assert_eq!(domain.tool, "time_convert_time");
        assert_eq!(domain.timeout_ms, Some(8_000));
    }
}
