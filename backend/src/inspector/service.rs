use mcpmate_llm::{ChatMessage, ChatRequest, ChatResponse, LlmTool, Role, StoredLlmProvider};
use rmcp::RoleClient;
use rmcp::model::{
    CallToolRequest, CallToolRequestParams, ClientRequest, LoggingMessageNotificationParam, ProgressNotificationParam,
    ProgressToken, RequestId,
};
use rmcp::service::{Peer, PeerRequestOptions};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;
use tokio::time::Duration;

use crate::api::handlers::ApiError;
use crate::common::RuntimeType;
use crate::core::capability;
use crate::core::capability::naming::NamingKind;
use crate::core::capability::resolver;
use crate::core::models::MCPServerConfig;
use crate::inspector::calls::{InspectorCallInfo, InspectorCallRegistry, InspectorTerminal, RegisteredCall};
use crate::inspector::context::InspectorServiceContext;
use crate::inspector::contract::{InspectorMode, InspectorProxyMode, InspectorProxyScope};
use crate::inspector::evidence::{
    self, InspectorCapabilityListEvidenceInput, InspectorResponseEvidenceInput, InspectorResponseKind,
};
use crate::inspector::runtime::{self, InspectorAcquiredPeer, InspectorRuntimeOwner};
use crate::inspector::sessions::{ActiveSession, InspectorSessionInfo, SessionLookup};
use crate::inspector::target::{
    InspectorCapabilityListRequest, InspectorCapabilityPatchRequest, InspectorLlmEvaluationRequest,
    InspectorNativeTarget, InspectorPromptGetRequest, InspectorProxyTarget, InspectorResourceReadRequest,
    InspectorServerReference, InspectorSnapshotRequest, InspectorTarget, InspectorTargetError, InspectorTargetRequest,
    InspectorToolCallRequest,
};
use crate::inspector::workspace::{
    InspectorCapabilityPatchInput, InspectorCapabilityPatchKind, InspectorCapabilityPatchRecord, InspectorPatchTarget,
    InspectorServerProvenance, InspectorServerRecord, InspectorServerRecordInput,
};

#[derive(Debug, Clone)]
pub struct ToolCallOutcome {
    pub result: Option<Value>,
    pub server_id: Option<String>,
    pub elapsed_ms: u64,
    pub message: Option<String>,
}

pub struct PreparedCall {
    pub info: InspectorCallInfo,
    pub completion: oneshot::Receiver<InspectorTerminal>,
    pub server_id: String,
    runtime_owner: Option<InspectorRuntimeOwner>,
}

pub struct PreparedLlmEvaluation {
    pub provider_id: Option<String>,
    pub chat_request: ChatRequest,
    target: InspectorTarget,
    session_id: Option<String>,
    scenario: String,
    tool_names: Vec<String>,
    dimensions: Vec<String>,
    evidence_summary_count: usize,
    started_at: Instant,
}

const CURRENT_MCP_SPEC_VERSION: &str = "2025-11-25";
const DEFAULT_PACKAGE_SAFETY_SOURCE: &str = "server_config";
const DEFAULT_PACKAGE_SAFETY_SCAN_DEPTH: &str = "standard";
const DEFAULT_LLM_EVALUATION_DIMENSION: &str = "capability_surface";
const MAX_LLM_EVALUATION_EVIDENCE_CHARS: usize = 12_000;

#[derive(Serialize)]
struct InspectorProductSnapshotTarget {
    mode: InspectorMode,
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scratch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy_mode: Option<InspectorProxyMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy_scope: Option<InspectorProxyScope>,
    transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

#[derive(Serialize)]
struct InspectorCompatibilitySnapshot {
    target: InspectorProductSnapshotTarget,
    #[serde(flatten)]
    analysis: InspectorCompatibilityAnalysis,
    elapsed_ms: u64,
}

#[derive(Serialize)]
struct InspectorCompatibilityAnalysis {
    spec: InspectorCompatibilitySpecSelection,
    observed: InspectorCompatibilityObserved,
    summary: InspectorCompatibilitySummary,
    inferred_best_fit_version: &'static str,
    capabilities: InspectorCompatibilityCapabilities,
    requirements: Vec<InspectorCompatibilityRequirement>,
}

#[derive(Serialize)]
struct InspectorCompatibilitySpecSelection {
    selected_version: &'static str,
    current_version: &'static str,
    available_versions: Vec<&'static str>,
}

#[derive(Serialize)]
struct InspectorCompatibilityObserved {
    protocol_version: Option<String>,
    counts: InspectorCompatibilityCounts,
    metrics: Vec<InspectorCompatibilityMetric>,
}

#[derive(Serialize)]
struct InspectorCompatibilitySummary {
    total: usize,
    implemented: usize,
    partial: usize,
    not_advertised: usize,
    unknown: usize,
}

#[derive(Serialize)]
struct InspectorCompatibilityRequirement {
    id: &'static str,
    title: &'static str,
    category: &'static str,
    status: &'static str,
    expected: InspectorCompatibilityRequirementExpected,
    observed: InspectorCompatibilityRequirementObserved,
    diff: InspectorCompatibilityRequirementDiff,
}

#[derive(Serialize)]
struct InspectorCompatibilityRequirementExpected {
    version: &'static str,
    description: &'static str,
    required: bool,
}

#[derive(Serialize)]
struct InspectorCompatibilityRequirementObserved {
    count: Option<usize>,
    total: Option<usize>,
    detail: String,
}

#[derive(Serialize)]
struct InspectorCompatibilityRequirementDiff {
    left_label: &'static str,
    left: &'static str,
    right_label: &'static str,
    right: String,
}

#[derive(Clone, Copy)]
struct InspectorSpecVersion {
    version: &'static str,
}

#[derive(Clone, Copy)]
struct InspectorSpecRequirement {
    id: &'static str,
    title: &'static str,
    category: &'static str,
    description: &'static str,
    required: bool,
    probe: InspectorSpecRequirementProbe,
}

#[derive(Clone, Copy)]
enum InspectorSpecRequirementProbe {
    Surface(InspectorCompatibilitySurface),
    ToolInputSchema,
    ToolNameFormat,
    ToolOutputSchema,
    PromptArguments,
    ResourceUri,
    ResourceMimeType,
    ProtocolVersionNegotiation,
}

#[derive(Clone, Copy)]
enum InspectorCompatibilitySurface {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

#[derive(Clone, Copy)]
enum InspectorCompatibilityStatus {
    Implemented,
    Partial,
    NotAdvertised,
    Unknown,
}

impl InspectorCompatibilityStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Partial => "partial",
            Self::NotAdvertised => "not_advertised",
            Self::Unknown => "unknown",
        }
    }
}

const MCP_SPEC_VERSIONS: &[InspectorSpecVersion] = &[
    InspectorSpecVersion { version: "2024-11-05" },
    InspectorSpecVersion { version: "2025-03-26" },
    InspectorSpecVersion { version: "2025-06-18" },
    InspectorSpecVersion { version: "2025-11-25" },
];

const MCP_SPEC_REQUIREMENTS: &[InspectorSpecRequirement] = &[
    InspectorSpecRequirement {
        id: "server_tools",
        title: "Tools surface",
        category: "server_features",
        description: "Server exposes callable tools through tools/list and tools/call when tools are advertised.",
        required: false,
        probe: InspectorSpecRequirementProbe::Surface(InspectorCompatibilitySurface::Tools),
    },
    InspectorSpecRequirement {
        id: "server_prompts",
        title: "Prompts surface",
        category: "server_features",
        description: "Server exposes prompt templates through prompts/list and prompts/get when prompts are advertised.",
        required: false,
        probe: InspectorSpecRequirementProbe::Surface(InspectorCompatibilitySurface::Prompts),
    },
    InspectorSpecRequirement {
        id: "server_resources",
        title: "Resources surface",
        category: "server_features",
        description: "Server exposes resources through resources/list and resources/read when resources are advertised.",
        required: false,
        probe: InspectorSpecRequirementProbe::Surface(InspectorCompatibilitySurface::Resources),
    },
    InspectorSpecRequirement {
        id: "server_resource_templates",
        title: "Resource templates surface",
        category: "server_features",
        description: "Server exposes resource templates when templated resources are advertised.",
        required: false,
        probe: InspectorSpecRequirementProbe::Surface(InspectorCompatibilitySurface::ResourceTemplates),
    },
    InspectorSpecRequirement {
        id: "tools_input_schema",
        title: "Tool input schema",
        category: "tools",
        description: "Every listed tool should provide an input schema object for client-side validation.",
        required: true,
        probe: InspectorSpecRequirementProbe::ToolInputSchema,
    },
    InspectorSpecRequirement {
        id: "tools_name_format",
        title: "Tool name format",
        category: "tools",
        description: "Tool names should use MCP-compatible name characters so clients can reference them reliably.",
        required: true,
        probe: InspectorSpecRequirementProbe::ToolNameFormat,
    },
    InspectorSpecRequirement {
        id: "tools_output_schema",
        title: "Tool output schema",
        category: "tools",
        description: "Tools may expose an output schema when structured output is available.",
        required: false,
        probe: InspectorSpecRequirementProbe::ToolOutputSchema,
    },
    InspectorSpecRequirement {
        id: "prompts_arguments",
        title: "Prompt arguments",
        category: "prompts",
        description: "Listed prompts should expose argument metadata when they require user-provided values.",
        required: false,
        probe: InspectorSpecRequirementProbe::PromptArguments,
    },
    InspectorSpecRequirement {
        id: "resources_uri",
        title: "Resource URI",
        category: "resources",
        description: "Every listed resource should expose a URI that can be passed to resources/read.",
        required: true,
        probe: InspectorSpecRequirementProbe::ResourceUri,
    },
    InspectorSpecRequirement {
        id: "resources_mime_type",
        title: "Resource MIME type",
        category: "resources",
        description: "Resources may expose MIME type metadata to help clients render content safely.",
        required: false,
        probe: InspectorSpecRequirementProbe::ResourceMimeType,
    },
    InspectorSpecRequirement {
        id: "protocol_version_negotiation",
        title: "Protocol version negotiation",
        category: "base_protocol",
        description: "Client and server agree on one protocol version during initialization.",
        required: true,
        probe: InspectorSpecRequirementProbe::ProtocolVersionNegotiation,
    },
];

#[derive(Serialize)]
struct InspectorPackageSafetySnapshot {
    target: InspectorProductSnapshotTarget,
    #[serde(flatten)]
    analysis: InspectorPackageSafetyAnalysis,
    elapsed_ms: u64,
}

#[derive(Serialize)]
struct InspectorPackageSafetyAnalysis {
    input: InspectorPackageSafetyInput,
    scanner: InspectorPackageSafetyScanner,
    inventory: InspectorPackageSafetyInventory,
    summary: InspectorPackageSafetySummary,
    findings: Vec<InspectorPackageSafetyFinding>,
    recommendations: Vec<InspectorPackageSafetyRecommendation>,
}

#[derive(Serialize)]
struct InspectorPackageSafetyInput {
    source: String,
    scan_depth: String,
}

#[derive(Serialize)]
struct InspectorPackageSafetyScanner {
    provider: String,
    status: &'static str,
}

#[derive(Serialize)]
struct InspectorPackageSafetySummary {
    total: usize,
    high: usize,
    medium: usize,
    low: usize,
    info: usize,
}

#[derive(Serialize)]
struct InspectorPackageSafetyFinding {
    id: &'static str,
    severity: &'static str,
    title: &'static str,
    detail: String,
    recommendation: &'static str,
}

#[derive(Serialize)]
struct InspectorPackageSafetyRecommendation {
    id: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
struct InspectorCompatibilityCapabilities {
    counts: InspectorCompatibilityCounts,
    checks: Vec<InspectorCompatibilityCheck>,
}

#[derive(Clone, Copy, Serialize)]
struct InspectorCompatibilityCounts {
    tools: usize,
    prompts: usize,
    resources: usize,
    resource_templates: usize,
}

#[derive(Serialize)]
struct InspectorCompatibilityMetric {
    id: &'static str,
    value: usize,
}

struct InspectorCompatibilityEvidence {
    counts: InspectorCompatibilityCounts,
    tools: Vec<Value>,
    prompts: Vec<Value>,
    resources: Vec<Value>,
    resource_templates: Vec<Value>,
}

#[derive(Serialize)]
struct InspectorCompatibilityCheck {
    id: &'static str,
    status: &'static str,
    observed_count: usize,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum InspectorPackageSafetyInventory {
    Stdio {
        command: String,
        runtime: Option<String>,
        fingerprint: String,
        args_count: usize,
    },
    Http {
        url_fingerprint: String,
        base: String,
    },
}

#[derive(Serialize)]
struct InspectorLlmEvaluationSnapshot {
    target: InspectorLlmEvaluationTarget,
    provider: InspectorLlmEvaluationProvider,
    scenario: String,
    dimensions: Vec<String>,
    evidence_summary_count: usize,
    tool_count: usize,
    tool_names: Vec<String>,
    message: InspectorLlmEvaluationMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<mcpmate_llm::TokenUsage>,
    elapsed_ms: u64,
}

#[derive(Serialize)]
struct InspectorLlmEvaluationTarget {
    mode: InspectorMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scratch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy_mode: Option<InspectorProxyMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy_scope: Option<InspectorProxyScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

#[derive(Serialize)]
struct InspectorLlmEvaluationProvider {
    id: String,
    name: String,
    provider_type: String,
    model_id: String,
}

#[derive(Serialize)]
struct InspectorLlmEvaluationMessage {
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<mcpmate_llm::ToolCall>>,
}

struct AcquiredProxyTargetPeer {
    acquired: InspectorAcquiredPeer,
    target: InspectorProxyTarget,
}

struct ResolvedToolCallTarget {
    target: InspectorTarget,
    server_id: String,
    upstream_tool_name: String,
}

struct ProxyCapabilityReference {
    server_filter: Option<String>,
    upstream_name: String,
}

enum AmbiguousProxyTargetPolicy {
    Optional,
    Required {
        missing_active_catalog_message: &'static str,
    },
}

static GLOBAL_CALL_REGISTRY: OnceLock<Arc<InspectorCallRegistry>> = OnceLock::new();

pub fn set_call_registry(registry: Arc<InspectorCallRegistry>) {
    let _ = GLOBAL_CALL_REGISTRY.set(registry);
}

#[derive(Clone, Copy)]
enum CapabilityKind {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

impl CapabilityKind {
    fn response_key(self) -> &'static str {
        match self {
            CapabilityKind::Tools => "tools",
            CapabilityKind::Prompts => "prompts",
            CapabilityKind::Resources => "resources",
            CapabilityKind::ResourceTemplates => "templates",
        }
    }

    fn patch_kind(self) -> InspectorCapabilityPatchKind {
        match self {
            CapabilityKind::Tools => InspectorCapabilityPatchKind::Tools,
            CapabilityKind::Prompts => InspectorCapabilityPatchKind::Prompts,
            CapabilityKind::Resources => InspectorCapabilityPatchKind::Resources,
            CapabilityKind::ResourceTemplates => InspectorCapabilityPatchKind::ResourceTemplates,
        }
    }
}

struct CapabilityPayload {
    mode: String,
    items: Vec<Value>,
    meta: Vec<Value>,
}

pub async fn list_tools(
    context: &InspectorServiceContext<'_>,
    request: InspectorCapabilityListRequest,
) -> Result<Value, ApiError> {
    list_capability_response(context, &request, CapabilityKind::Tools).await
}

pub async fn list_prompts(
    context: &InspectorServiceContext<'_>,
    request: InspectorCapabilityListRequest,
) -> Result<Value, ApiError> {
    list_capability_response(context, &request, CapabilityKind::Prompts).await
}

pub async fn list_resources(
    context: &InspectorServiceContext<'_>,
    request: InspectorCapabilityListRequest,
) -> Result<Value, ApiError> {
    list_capability_response(context, &request, CapabilityKind::Resources).await
}

pub async fn list_templates(
    context: &InspectorServiceContext<'_>,
    request: InspectorCapabilityListRequest,
) -> Result<Value, ApiError> {
    list_capability_response(context, &request, CapabilityKind::ResourceTemplates).await
}

pub async fn compatibility_snapshot(
    context: &InspectorServiceContext<'_>,
    request: InspectorSnapshotRequest,
) -> Result<Value, ApiError> {
    let started = Instant::now();
    match request.target.mode {
        InspectorMode::Native => {
            let native_target =
                resolve_expected_native_target(request.target.clone(), "Expected native Inspector mode", true).await?;
            let target_id = native_target.reference_id().to_string();
            let transport = native_target_transport(context, &native_target).await?;
            let acquired = acquire_native_direct_peer(context, &native_target, request.session_id.as_deref()).await?;
            let capabilities = collect_compatibility_capabilities(acquired.peer()).await;
            acquired.cancel_runtime().await;
            let capabilities = capabilities?;
            let analysis = build_compatibility_analysis(capabilities, request.spec_version.as_deref())?;
            let snapshot = InspectorCompatibilitySnapshot {
                target: native_product_snapshot_target(&native_target, Some(target_id), transport, request.session_id),
                analysis,
                elapsed_ms: started.elapsed().as_millis() as u64,
            };

            snapshot_to_value(snapshot, "Inspector compatibility snapshot")
        }
        InspectorMode::Proxy => {
            request.target.ensure_proxy_mode().map_err(inspector_target_error)?;
            let active_session =
                get_active_proxy_session_for_request(context, request.session_id.as_deref(), &request.target).await?;

            let fallback_target = if active_session.is_some() {
                None
            } else {
                Some(resolve_expected_proxy_target(request.target.clone(), "Expected proxy Inspector mode").await?)
            };
            let AcquiredProxyTargetPeer { acquired, target } =
                acquire_proxy_target_peer(context, active_session, fallback_target).await?;

            let capabilities = collect_compatibility_capabilities(acquired.peer()).await;
            acquired.cancel_runtime().await;
            let capabilities = capabilities?;
            let analysis = build_compatibility_analysis(capabilities, request.spec_version.as_deref())?;
            let snapshot = InspectorCompatibilitySnapshot {
                target: proxy_product_snapshot_target(&target, request.session_id),
                analysis,
                elapsed_ms: started.elapsed().as_millis() as u64,
            };

            snapshot_to_value(snapshot, "Inspector compatibility snapshot")
        }
    }
}

pub async fn package_safety_snapshot(
    context: &InspectorServiceContext<'_>,
    request: InspectorSnapshotRequest,
) -> Result<Value, ApiError> {
    let started = Instant::now();
    let native_target = resolve_expected_native_target(
        request.target,
        "Package safety snapshots require native Inspector mode",
        true,
    )
    .await?;
    let target_id = native_target.reference_id().to_string();
    let target_config = native_target_config(context, &native_target).await?;
    let transport = target_config.config.kind.client_format().to_string();
    let analysis = build_package_safety_analysis(
        &target_config.config,
        request.package_source.as_deref(),
        request.scan_depth.as_deref(),
    )?;
    let snapshot = InspectorPackageSafetySnapshot {
        target: native_product_snapshot_target(&native_target, Some(target_id), transport, None),
        analysis,
        elapsed_ms: started.elapsed().as_millis() as u64,
    };

    snapshot_to_value(snapshot, "Inspector package safety snapshot")
}

pub async fn upsert_capability_patch(
    context: &InspectorServiceContext<'_>,
    request: InspectorCapabilityPatchRequest,
) -> Result<InspectorCapabilityPatchRecord, ApiError> {
    let native_target = resolve_expected_native_target(
        request.target,
        "Capability patches currently require native Inspector mode",
        false,
    )
    .await?;
    let capability_kind = parse_patch_kind(&request.capability_kind)?;
    let target = patch_target_from_native(&native_target);
    let record = context
        .workspace()
        .upsert_capability_patch(InspectorCapabilityPatchInput {
            target,
            capability_kind,
            capability_key: request.capability_key,
            patch: request.patch,
        })
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
    Ok(record)
}

pub async fn prepare_llm_evaluation(
    context: &InspectorServiceContext<'_>,
    request: InspectorLlmEvaluationRequest,
) -> Result<PreparedLlmEvaluation, ApiError> {
    let scenario = request.scenario.trim().to_string();
    if scenario.is_empty() {
        return Err(ApiError::BadRequest("LLM evaluation scenario is required".to_string()));
    }

    let target = resolve_request_target(request.target.clone()).await?;
    let payload = list_capability_payload(
        context,
        &InspectorCapabilityListRequest {
            target: request.target,
            session_id: request.session_id.clone(),
            refresh: true,
        },
        CapabilityKind::Tools,
    )
    .await?;
    let max_tools = request.max_tools.unwrap_or(usize::MAX);
    let tools = payload
        .items
        .iter()
        .filter_map(llm_tool_from_capability)
        .take(max_tools)
        .collect::<Vec<_>>();

    if tools.is_empty() {
        return Err(ApiError::BadRequest(
            "LLM evaluation requires at least one inspectable tool".to_string(),
        ));
    }

    let tool_names = tools.iter().map(|tool| tool.name.clone()).collect::<Vec<_>>();
    let dimensions = normalize_llm_evaluation_dimensions(request.dimensions)?;
    let evidence_summary = summarize_llm_evaluation_evidence(&request.evidence);
    let evidence_prompt = format_llm_evaluation_evidence_prompt(&dimensions, &evidence_summary);

    let chat_request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: Role::System,
                content: "You evaluate MCP Inspector evidence. Use only the provided facts, selected dimensions, and tool definitions. Return concise recommendations with strengths, risks, actions, confidence, and evidence references when possible.".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: Role::User,
                content: format!(
                    "Scenario:\n{scenario}\n\n{evidence_prompt}\n\nReturn a concise evaluation with recommended actions and call out any missing evidence."
                ),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        tools: Some(tools),
        temperature: Some(0.0),
        max_tokens: Some(1024),
    };

    Ok(PreparedLlmEvaluation {
        provider_id: request.provider_id,
        chat_request,
        target,
        session_id: request.session_id,
        scenario,
        tool_names,
        dimensions,
        evidence_summary_count: evidence_summary.len(),
        started_at: Instant::now(),
    })
}

pub fn finish_llm_evaluation(
    prepared: PreparedLlmEvaluation,
    provider: StoredLlmProvider,
    response: ChatResponse,
) -> Result<Value, ApiError> {
    let snapshot = InspectorLlmEvaluationSnapshot {
        target: llm_evaluation_target(&prepared.target, prepared.session_id),
        provider: InspectorLlmEvaluationProvider {
            id: provider.id,
            name: provider.name,
            provider_type: provider.provider_type,
            model_id: provider.model_id,
        },
        scenario: prepared.scenario,
        dimensions: prepared.dimensions,
        evidence_summary_count: prepared.evidence_summary_count,
        tool_count: prepared.tool_names.len(),
        tool_names: prepared.tool_names,
        message: InspectorLlmEvaluationMessage {
            content: response.message.content,
            tool_calls: response.message.tool_calls,
        },
        usage: response.usage,
        elapsed_ms: prepared.started_at.elapsed().as_millis() as u64,
    };

    snapshot_to_value(snapshot, "Inspector LLM evaluation")
}

pub fn list_scratch_server_records(
    context: &InspectorServiceContext<'_>
) -> Result<Vec<InspectorServerRecord>, ApiError> {
    let records = context
        .workspace()
        .list_server_records()
        .map_err(|error| ApiError::InternalError(error.to_string()))?
        .into_iter()
        .filter(|record| matches!(record.provenance, InspectorServerProvenance::Scratch { .. }))
        .collect::<Vec<_>>();
    Ok(records)
}

pub fn create_scratch_server_record(
    context: &InspectorServiceContext<'_>,
    input: InspectorServerRecordInput,
) -> Result<InspectorServerRecord, ApiError> {
    context
        .workspace()
        .create_server_record(input)
        .map_err(|error| ApiError::InternalError(error.to_string()))
}

pub fn delete_scratch_server_record(
    context: &InspectorServiceContext<'_>,
    record_id: &str,
) -> Result<bool, ApiError> {
    let Some(record) = context
        .workspace()
        .get_server_record(record_id)
        .map_err(|error| ApiError::InternalError(error.to_string()))?
    else {
        return Ok(false);
    };
    if !matches!(record.provenance, InspectorServerProvenance::Scratch { .. }) {
        return Err(ApiError::BadRequest(format!(
            "Inspector server record '{}' is not a scratch server",
            record_id
        )));
    }
    context
        .workspace()
        .delete_server_record(record_id)
        .map_err(|error| ApiError::InternalError(error.to_string()))
}

pub async fn prompt_get(
    context: &InspectorServiceContext<'_>,
    request: InspectorPromptGetRequest,
) -> Result<Value, ApiError> {
    let start = Instant::now();
    let started_at_epoch_ms = current_epoch_ms();
    match request.target.mode {
        InspectorMode::Proxy => {
            request.target.ensure_proxy_mode().map_err(inspector_target_error)?;
            let active_session =
                get_active_proxy_session_for_request(context, request.session_id.as_deref(), &request.target).await?;
            let reference = resolve_proxy_capability_reference(
                NamingKind::Prompt,
                &request.name,
                &request.target.server_id,
                &request.target.server_name,
                active_session.as_ref(),
                AmbiguousProxyTargetPolicy::Optional,
            )
            .await?;
            let acquired = acquire_proxy_capability_peer(
                context,
                active_session,
                request.target.proxy_mode,
                request.target.proxy_scope,
                reference.server_filter.clone(),
            )
            .await?;
            let res = runtime::get_prompt(acquired.peer(), &reference.upstream_name, request.arguments.clone()).await;
            acquired.cancel_runtime().await;
            let res = res?;
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let evidence = response_evidence_json(InspectorResponseEvidenceInput {
                response_kind: InspectorResponseKind::PromptGet,
                mode: request.target.mode,
                session_id: request.session_id.clone(),
                server_id: reference.server_filter.clone(),
                platform_payload: json!({
                    "name": request.name,
                    "server_id": request.target.server_id,
                    "server_name": request.target.server_name,
                    "proxy_mode": request.target.proxy_mode,
                    "proxy_scope": request.target.proxy_scope,
                }),
                mcp_payload: serde_json::to_value(&res).unwrap_or(Value::Null),
                started_at_epoch_ms,
                elapsed_ms,
            })?;

            Ok(json!({
                "result": res,
                "server_id": reference.server_filter,
                "elapsed_ms": elapsed_ms,
                "evidence": evidence,
            }))
        }
        InspectorMode::Native => {
            let native_target =
                resolve_expected_native_target(request.target.clone(), "Expected native Inspector mode", true).await?;
            let target_id = native_target.reference_id().to_string();
            let acquired = acquire_native_direct_peer(context, &native_target, request.session_id.as_deref()).await?;
            let res = runtime::get_prompt(acquired.peer(), &request.name, request.arguments.clone()).await;
            acquired.cancel_runtime().await;
            let res = res?;
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let evidence = response_evidence_json(InspectorResponseEvidenceInput {
                response_kind: InspectorResponseKind::PromptGet,
                mode: request.target.mode,
                session_id: request.session_id.clone(),
                server_id: Some(target_id.clone()),
                platform_payload: native_response_platform_payload(
                    &native_target,
                    &request.target.server_id,
                    &request.target.server_name,
                    [("name", json!(request.name))],
                ),
                mcp_payload: serde_json::to_value(&res).unwrap_or(Value::Null),
                started_at_epoch_ms,
                elapsed_ms,
            })?;
            Ok(json!({
                "result": res,
                "server_id": target_id,
                "elapsed_ms": elapsed_ms,
                "evidence": evidence,
            }))
        }
    }
}

pub async fn resource_read(
    context: &InspectorServiceContext<'_>,
    request: InspectorResourceReadRequest,
) -> Result<Value, ApiError> {
    let start = Instant::now();
    let started_at_epoch_ms = current_epoch_ms();
    match request.target.mode {
        InspectorMode::Proxy => {
            request.target.ensure_proxy_mode().map_err(inspector_target_error)?;
            let active_session =
                get_active_proxy_session_for_request(context, request.session_id.as_deref(), &request.target).await?;
            let reference = resolve_proxy_capability_reference(
                NamingKind::Resource,
                &request.uri,
                &request.target.server_id,
                &request.target.server_name,
                active_session.as_ref(),
                AmbiguousProxyTargetPolicy::Optional,
            )
            .await?;
            let acquired = acquire_proxy_capability_peer(
                context,
                active_session,
                request.target.proxy_mode,
                request.target.proxy_scope,
                reference.server_filter.clone(),
            )
            .await?;
            let res = runtime::read_resource(acquired.peer(), &reference.upstream_name).await;
            acquired.cancel_runtime().await;
            let res = res?;
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let evidence = response_evidence_json(InspectorResponseEvidenceInput {
                response_kind: InspectorResponseKind::ResourceRead,
                mode: request.target.mode,
                session_id: request.session_id.clone(),
                server_id: reference.server_filter.clone(),
                platform_payload: json!({
                    "uri": request.uri,
                    "server_id": request.target.server_id,
                    "server_name": request.target.server_name,
                    "proxy_mode": request.target.proxy_mode,
                    "proxy_scope": request.target.proxy_scope,
                }),
                mcp_payload: serde_json::to_value(&res).unwrap_or(Value::Null),
                started_at_epoch_ms,
                elapsed_ms,
            })?;
            Ok(json!({
                "result": res,
                "server_id": reference.server_filter,
                "elapsed_ms": elapsed_ms,
                "evidence": evidence,
            }))
        }
        InspectorMode::Native => {
            let native_target =
                resolve_expected_native_target(request.target.clone(), "Expected native Inspector mode", true).await?;
            let target_id = native_target.reference_id().to_string();
            let acquired = acquire_native_direct_peer(context, &native_target, request.session_id.as_deref()).await?;
            let res = runtime::read_resource(acquired.peer(), &request.uri).await;
            acquired.cancel_runtime().await;
            let res = res?;
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let evidence = response_evidence_json(InspectorResponseEvidenceInput {
                response_kind: InspectorResponseKind::ResourceRead,
                mode: request.target.mode,
                session_id: request.session_id.clone(),
                server_id: Some(target_id.clone()),
                platform_payload: native_response_platform_payload(
                    &native_target,
                    &request.target.server_id,
                    &request.target.server_name,
                    [("uri", json!(request.uri))],
                ),
                mcp_payload: serde_json::to_value(&res).unwrap_or(Value::Null),
                started_at_epoch_ms,
                elapsed_ms,
            })?;
            Ok(json!({
                "result": res,
                "server_id": target_id,
                "elapsed_ms": elapsed_ms,
                "evidence": evidence,
            }))
        }
    }
}

pub async fn call_tool(
    context: &InspectorServiceContext<'_>,
    request: InspectorToolCallRequest,
) -> Result<ToolCallOutcome, ApiError> {
    let prepared = start_tool_call_internal(context, &request).await?;
    let PreparedCall {
        completion,
        mut runtime_owner,
        ..
    } = prepared;

    let result = match completion.await {
        Ok(result) => result,
        Err(_) => {
            InspectorRuntimeOwner::cancel_taken(&mut runtime_owner).await;
            return Err(ApiError::InternalError("Inspector call channel dropped".into()));
        }
    };

    InspectorRuntimeOwner::cancel_optional(runtime_owner).await;

    match result {
        InspectorTerminal::Result {
            result,
            elapsed_ms,
            server_id,
        } => Ok(ToolCallOutcome {
            result: Some(result),
            server_id: Some(server_id),
            elapsed_ms,
            message: Some("completed".to_string()),
        }),
        InspectorTerminal::Error { message, .. } => Err(ApiError::InternalError(message)),
        InspectorTerminal::Cancelled { reason, .. } => Err(ApiError::Conflict(
            reason.unwrap_or_else(|| "Inspector call cancelled".to_string()),
        )),
    }
}

pub async fn start_tool_call(
    context: &InspectorServiceContext<'_>,
    request: InspectorToolCallRequest,
) -> Result<InspectorCallInfo, ApiError> {
    let prepared = start_tool_call_internal(context, &request).await?;
    let info = prepared.info.clone();

    tokio::spawn(async move {
        let PreparedCall {
            completion,
            runtime_owner,
            ..
        } = prepared;

        let _ = completion.await;

        InspectorRuntimeOwner::cancel_optional(runtime_owner).await;
    });

    Ok(info)
}

pub async fn open_session(
    context: &InspectorServiceContext<'_>,
    request: InspectorTargetRequest,
) -> Result<InspectorSessionInfo, ApiError> {
    let target = resolve_request_target(request).await?;
    let session_id = crate::generate_id!("inspses");
    let runtime_env = context.runtime_environment();
    let runtime = runtime_env.connect_target(&target).await?;
    let handshake = Some(crate::inspector::handshake::build_session_handshake(&runtime.peer));
    let peer = Some(runtime.peer);
    let runtime_owner = Some(runtime.owner);

    Ok(context
        .sessions()
        .open_session(session_id.clone(), target, peer, runtime_owner, handshake)
        .await)
}

pub async fn close_session(
    context: &InspectorServiceContext<'_>,
    session_id: &str,
) -> Result<bool, ApiError> {
    if let Some(closed) = context.sessions().close_session(session_id).await {
        closed.cleanup_runtime().await;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn refresh_session(
    context: &InspectorServiceContext<'_>,
    session_id: &str,
) -> Result<InspectorSessionInfo, ApiError> {
    match context.sessions().get_session_or_expired(session_id).await {
        SessionLookup::Active(session) => Ok(InspectorSessionInfo {
            session_id: session.session_id,
            target: session.target,
            expires_at_epoch_ms: session.expires_at_epoch_ms,
            handshake: session.handshake,
        }),
        SessionLookup::Expired(closed) => {
            closed.cleanup_runtime().await;
            Err(ApiError::NotFound(format!(
                "Inspector session '{}' not found or expired",
                session_id
            )))
        }
        SessionLookup::Missing => Err(ApiError::NotFound(format!(
            "Inspector session '{}' not found or expired",
            session_id
        ))),
    }
}

async fn get_inspector_timeout_default(context: &InspectorServiceContext<'_>) -> Option<u64> {
    let db = context.database()?;
    crate::system::settings::get_inspector_timeout_ms(&db.pool).await.ok()
}

async fn start_tool_call_internal(
    context: &InspectorServiceContext<'_>,
    request: &InspectorToolCallRequest,
) -> Result<PreparedCall, ApiError> {
    if matches!(request.target.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }

    let timeout = if let Some(timeout_ms) = request.timeout_ms {
        Duration::from_millis(timeout_ms)
    } else {
        let default_ms = get_inspector_timeout_default(context).await.unwrap_or(60_000);
        Duration::from_millis(default_ms)
    };

    let active_session = if let Some(session_id) = request.session_id.as_ref() {
        Some(get_active_session(context, session_id).await?)
    } else {
        None
    };
    request
        .target
        .ensure_session_options_unchanged(active_session.is_some())
        .map_err(inspector_target_error)?;

    let resolved_target = resolve_tool_call_target(context, request, active_session.as_ref()).await?;

    let acquired = acquire_target_peer(context, &resolved_target.target, active_session).await?;
    let (peer, runtime_owner) = acquired.into_parts();

    let params = CallToolRequestParams::new(resolved_target.upstream_tool_name)
        .with_arguments(request.arguments.clone().unwrap_or_default());
    let mcp_request = ClientRequest::CallToolRequest(CallToolRequest::new(params));

    let mut options = PeerRequestOptions::no_options();
    options.timeout = Some(timeout);

    let call_id = crate::generate_id!("inspcall");
    let mut runtime_owner = runtime_owner;
    let handle = match peer.send_cancellable_request(mcp_request, options).await {
        Ok(handle) => handle,
        Err(err) => {
            InspectorRuntimeOwner::cancel_taken(&mut runtime_owner).await;
            return Err(ApiError::InternalError(err.to_string()));
        }
    };

    let RegisteredCall { info, completion } = context
        .calls()
        .start_call(
            call_id,
            resolved_target.server_id.clone(),
            request.target.mode,
            request.session_id.clone(),
            handle,
        )
        .await;

    Ok(PreparedCall {
        info,
        completion,
        server_id: resolved_target.server_id,
        runtime_owner,
    })
}

async fn resolve_tool_call_target(
    context: &InspectorServiceContext<'_>,
    request: &InspectorToolCallRequest,
    active_session: Option<&ActiveSession>,
) -> Result<ResolvedToolCallTarget, ApiError> {
    let (native_target, server_id, mut upstream_tool_name) = match request.target.mode {
        InspectorMode::Proxy => {
            request.target.ensure_proxy_mode().map_err(inspector_target_error)?;
            let reference = resolve_proxy_capability_reference(
                NamingKind::Tool,
                &request.tool,
                &request.target.server_id,
                &request.target.server_name,
                active_session,
                AmbiguousProxyTargetPolicy::Required {
                    missing_active_catalog_message: "Active-catalog Inspector sessions require a unique proxy tool name",
                },
            )
            .await?;
            let server_id = reference
                .server_filter
                .ok_or_else(|| ApiError::InternalError("Expected resolved proxy tool target server".into()))?;
            (None, server_id, reference.upstream_name)
        }
        InspectorMode::Native => {
            let native_target = if let Some(session) = active_session {
                session
                    .target
                    .as_native()
                    .cloned()
                    .ok_or_else(|| ApiError::BadRequest("Native Inspector session is missing native target".into()))?
            } else {
                resolve_expected_native_target(request.target.clone(), "Expected native Inspector mode", false).await?
            };
            let target_id = native_target.reference_id().to_string();
            let upstream_tool_name = resolve_patched_tool_name(context, &native_target, &request.tool)
                .unwrap_or_else(|| request.tool.clone());
            (Some(native_target), target_id, upstream_tool_name)
        }
    };

    if matches!(request.target.mode, InspectorMode::Native)
        && native_target
            .as_ref()
            .and_then(InspectorNativeTarget::server_id)
            .is_some()
    {
        if let Ok((resolved_server_name, resolved_tool_name)) =
            capability::naming::resolve_unique_name(NamingKind::Tool, &upstream_tool_name).await
        {
            let resolved_server_id = resolver::to_id(&resolved_server_name).await.ok().flatten();
            let matches_server = resolved_server_id
                .as_deref()
                .map(|id| id == server_id)
                .unwrap_or_else(|| resolved_server_name == server_id);
            if matches_server {
                upstream_tool_name = resolved_tool_name;
            }
        }
    }

    if let Some(session) = active_session {
        session
            .target
            .ensure_session_binding(request.target.mode, &server_id)
            .map_err(inspector_target_error)?;

        return Ok(ResolvedToolCallTarget {
            target: session.target.clone(),
            server_id,
            upstream_tool_name,
        });
    }

    let target = match request.target.mode {
        InspectorMode::Native => InspectorTarget::Native(
            native_target.ok_or_else(|| ApiError::InternalError("Expected native Inspector target".into()))?,
        ),
        InspectorMode::Proxy => InspectorTarget::proxy(
            InspectorProxyTarget::from_tool_call_reference(
                request.target.proxy_mode,
                request.target.proxy_scope,
                server_id.clone(),
            )
            .map_err(inspector_target_error)?,
        ),
    };

    Ok(ResolvedToolCallTarget {
        target,
        server_id,
        upstream_tool_name,
    })
}

async fn resolve_proxy_capability_reference(
    kind: NamingKind,
    requested_name: &str,
    server_id: &Option<String>,
    server_name: &Option<String>,
    active_session: Option<&ActiveSession>,
    ambiguous_target_policy: AmbiguousProxyTargetPolicy,
) -> Result<ProxyCapabilityReference, ApiError> {
    if let Ok((resolved_server_name, _upstream)) = capability::naming::resolve_unique_name(kind, requested_name).await {
        let resolved_server_id = resolver::to_id(&resolved_server_name)
            .await
            .ok()
            .flatten()
            .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", resolved_server_name)))?;
        return Ok(ProxyCapabilityReference {
            server_filter: Some(resolved_server_id),
            upstream_name: requested_name.to_string(),
        });
    }

    let active_session_server_id = active_session.and_then(|session| session.target.server_id().map(str::to_string));
    let explicit_server_id = if let Some(active_session_server_id) = active_session_server_id {
        Some(active_session_server_id)
    } else if server_id.is_some() || server_name.is_some() {
        Some(resolve_server(server_id, server_name).await?)
    } else {
        None
    };

    let Some(resolved_server_id) = explicit_server_id else {
        return match ambiguous_target_policy {
            AmbiguousProxyTargetPolicy::Optional => Ok(ProxyCapabilityReference {
                server_filter: None,
                upstream_name: requested_name.to_string(),
            }),
            AmbiguousProxyTargetPolicy::Required {
                missing_active_catalog_message,
            } if active_session.is_some() => Err(ApiError::BadRequest(missing_active_catalog_message.to_string())),
            AmbiguousProxyTargetPolicy::Required { .. } => {
                Err(ApiError::BadRequest("server_id or server_name is required".into()))
            }
        };
    };

    let resolved_server_name = resolver::to_name(&resolved_server_id)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", resolved_server_id)))?;

    Ok(ProxyCapabilityReference {
        server_filter: Some(resolved_server_id),
        upstream_name: capability::naming::generate_unique_name(kind, &resolved_server_name, requested_name),
    })
}

async fn acquire_proxy_capability_peer(
    context: &InspectorServiceContext<'_>,
    active_session: Option<ActiveSession>,
    proxy_mode: Option<InspectorProxyMode>,
    proxy_scope: Option<InspectorProxyScope>,
    server_filter: Option<String>,
) -> Result<InspectorAcquiredPeer, ApiError> {
    let proxy_target = proxy_target(proxy_mode, proxy_scope, server_filter.map(|server_id| vec![server_id]))?;
    let target = InspectorTarget::proxy(proxy_target);
    acquire_target_peer(context, &target, active_session).await
}

async fn resolve_request_target(request: InspectorTargetRequest) -> Result<InspectorTarget, ApiError> {
    if matches!(request.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }
    let resolved_server_id = resolve_target_request_server_id(&request).await?;
    request.into_target(resolved_server_id).map_err(inspector_target_error)
}

async fn resolve_expected_native_target(
    request: InspectorTargetRequest,
    proxy_mode_error: &'static str,
    require_native_enabled: bool,
) -> Result<InspectorNativeTarget, ApiError> {
    if !matches!(request.mode, InspectorMode::Native) {
        return Err(ApiError::BadRequest(proxy_mode_error.into()));
    }
    if require_native_enabled {
        ensure_native_allowed()?;
    }
    let resolved_server_id = resolve_target_request_server_id(&request).await?;
    request
        .into_native_target(resolved_server_id)
        .map_err(|error| native_endpoint_target_error(error, proxy_mode_error))
}

async fn resolve_expected_proxy_target(
    request: InspectorTargetRequest,
    mode_error: &'static str,
) -> Result<InspectorProxyTarget, ApiError> {
    if !matches!(request.mode, InspectorMode::Proxy) {
        return Err(ApiError::BadRequest(mode_error.into()));
    }

    let target = resolve_request_target(request).await?;
    let InspectorTarget::Proxy(proxy_target) = target else {
        return Err(ApiError::InternalError(mode_error.into()));
    };
    Ok(proxy_target)
}

async fn list_capability_response(
    context: &InspectorServiceContext<'_>,
    request: &InspectorCapabilityListRequest,
    kind: CapabilityKind,
) -> Result<Value, ApiError> {
    let start = Instant::now();
    let started_at_epoch_ms = current_epoch_ms();
    let mut payload = list_capability_payload(context, request, kind).await?;
    apply_capability_patches_for_list_request(context, request, kind, &mut payload.items).await?;
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let evidence = list_capability_evidence_snapshot(kind, request, &payload, started_at_epoch_ms, elapsed_ms)?;
    Ok(json!({
        "mode": payload.mode,
        kind.response_key(): payload.items,
        "total": payload.items.len(),
        "meta": payload.meta,
        "elapsed_ms": elapsed_ms,
        "evidence": evidence,
    }))
}

fn list_capability_evidence_snapshot(
    kind: CapabilityKind,
    request: &InspectorCapabilityListRequest,
    payload: &CapabilityPayload,
    started_at_epoch_ms: u128,
    elapsed_ms: u64,
) -> Result<Value, ApiError> {
    evidence::capability_list_json(InspectorCapabilityListEvidenceInput {
        capability_kind: kind.response_key(),
        mode: request.target.mode,
        session_id: request.session_id.clone(),
        refresh: request.refresh,
        proxy_mode: request.target.proxy_mode,
        proxy_scope: request.target.proxy_scope,
        server_id: request.target.server_id.clone(),
        server_name: request.target.server_name.clone(),
        scratch_id: request.target.scratch_id.clone(),
        meta: payload.meta.clone(),
        items: payload.items.clone(),
        started_at_epoch_ms,
        elapsed_ms,
    })
    .map_err(|error| ApiError::InternalError(format!("Failed to serialize Inspector evidence: {}", error)))
}

fn response_evidence_json(input: InspectorResponseEvidenceInput) -> Result<Value, ApiError> {
    evidence::sync_response_json(input)
        .map_err(|error| ApiError::InternalError(format!("Failed to serialize Inspector evidence: {}", error)))
}

async fn list_capability_payload(
    context: &InspectorServiceContext<'_>,
    request: &InspectorCapabilityListRequest,
    kind: CapabilityKind,
) -> Result<CapabilityPayload, ApiError> {
    let (items, meta_entries) = match request.target.mode {
        InspectorMode::Proxy => {
            let active_session =
                get_active_proxy_session_for_request(context, request.session_id.as_deref(), &request.target).await?;
            let (extracted, meta) = if let Some(session) = active_session {
                let peer = session
                    .peer
                    .ok_or_else(|| ApiError::InternalError("Inspector session peer is not available".into()))?;
                list_proxy_capability_payload_from_peer(
                    &peer,
                    session.target.server_id().map(str::to_string),
                    session.target.proxy_mode(),
                    session.target.proxy_scope(),
                    "direct_proxy_session",
                    kind,
                )
                .await?
            } else {
                let proxy_target =
                    resolve_expected_proxy_target(request.target.clone(), "Expected proxy Inspector mode").await?;
                list_proxy_capability_payload(context, proxy_target, kind).await?
            };
            (extracted, vec![meta])
        }
        InspectorMode::Native => {
            let native_target =
                resolve_expected_native_target(request.target.clone(), "Expected native Inspector mode", true).await?;
            let (extracted, meta) =
                list_native_capability_payload(context, &native_target, request.session_id.as_deref(), kind).await?;
            (extracted, vec![meta])
        }
    };

    Ok(CapabilityPayload {
        mode: format!("{:?}", request.target.mode).to_lowercase(),
        items,
        meta: meta_entries,
    })
}

async fn list_proxy_capability_payload(
    context: &InspectorServiceContext<'_>,
    proxy_target: InspectorProxyTarget,
    kind: CapabilityKind,
) -> Result<(Vec<Value>, Value), ApiError> {
    let proxy_mode = proxy_target.proxy_mode();
    let proxy_scope = proxy_target.proxy_scope();
    let meta_server_id = proxy_target.server_id().map(str::to_string);
    let target = InspectorTarget::proxy(proxy_target);
    let acquired = acquire_target_peer(context, &target, None).await?;
    let result = list_proxy_capability_payload_from_peer(
        acquired.peer(),
        meta_server_id,
        Some(proxy_mode),
        Some(proxy_scope),
        "direct_proxy",
        kind,
    )
    .await;
    acquired.cancel_runtime().await;
    result
}

async fn list_proxy_capability_payload_from_peer(
    peer: &Peer<RoleClient>,
    meta_server_id: Option<String>,
    proxy_mode: Option<InspectorProxyMode>,
    proxy_scope: Option<InspectorProxyScope>,
    source: &str,
    kind: CapabilityKind,
) -> Result<(Vec<Value>, Value), ApiError> {
    let started = Instant::now();
    let result = match kind {
        CapabilityKind::Tools => serialize_items(runtime::list_tools(peer).await?),
        CapabilityKind::Prompts => serialize_items(runtime::list_prompts(peer).await?),
        CapabilityKind::Resources => serialize_items(runtime::list_resources(peer).await?),
        CapabilityKind::ResourceTemplates => serialize_items(runtime::list_resource_templates(peer).await?),
    };
    let items = result?;
    Ok((
        items,
        json!({
            "server_id": meta_server_id,
            "cache_hit": false,
            "source": source,
            "proxy_mode": proxy_mode,
            "proxy_scope": proxy_scope,
            "duration_ms": started.elapsed().as_millis() as u64,
            "had_peer": true,
        }),
    ))
}

fn proxy_target(
    proxy_mode: Option<InspectorProxyMode>,
    proxy_scope: Option<InspectorProxyScope>,
    target_server_ids: Option<Vec<String>>,
) -> Result<InspectorProxyTarget, ApiError> {
    InspectorProxyTarget::from_parts(proxy_mode, proxy_scope, target_server_ids).map_err(inspector_target_error)
}

fn inspector_target_error(error: InspectorTargetError) -> ApiError {
    ApiError::BadRequest(error.to_string())
}

fn native_endpoint_target_error(
    error: InspectorTargetError,
    proxy_mode_error: &'static str,
) -> ApiError {
    match error {
        InspectorTargetError::ExpectedNativeMode => ApiError::BadRequest(proxy_mode_error.into()),
        error => inspector_target_error(error),
    }
}

async fn resolve_target_request_server_id(request: &InspectorTargetRequest) -> Result<Option<String>, ApiError> {
    match request.server_reference().map_err(inspector_target_error)? {
        Some(reference) => resolve_server_reference(reference).await.map(Some),
        None => Ok(None),
    }
}

async fn resolve_server_reference(reference: InspectorServerReference) -> Result<String, ApiError> {
    match reference {
        InspectorServerReference::Id(server_id) => Ok(server_id),
        InspectorServerReference::Name(server_name) => resolver::to_id(&server_name)
            .await
            .ok()
            .flatten()
            .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", server_name))),
    }
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

async fn list_native_capability_payload(
    context: &InspectorServiceContext<'_>,
    native_target: &InspectorNativeTarget,
    session_id: Option<&str>,
    kind: CapabilityKind,
) -> Result<(Vec<Value>, Value), ApiError> {
    let started = Instant::now();
    let target_id = native_target.reference_id().to_string();
    let acquired = acquire_native_direct_peer(context, native_target, session_id).await?;
    let result = match kind {
        CapabilityKind::Tools => serialize_items(runtime::list_tools(acquired.peer()).await?),
        CapabilityKind::Prompts => serialize_items(runtime::list_prompts(acquired.peer()).await?),
        CapabilityKind::Resources => serialize_items(runtime::list_resources(acquired.peer()).await?),
        CapabilityKind::ResourceTemplates => serialize_items(runtime::list_resource_templates(acquired.peer()).await?),
    };
    acquired.cancel_runtime().await;

    let items = result?;
    let mut meta = json!({
        "server_id": target_id,
        "cache_hit": false,
        "source": "direct_native",
        "duration_ms": started.elapsed().as_millis() as u64,
        "had_peer": true,
    });
    if let Some(scratch_id) = native_target.scratch_id()
        && let Some(meta_object) = meta.as_object_mut()
    {
        meta_object.insert("scratch_id".to_string(), json!(scratch_id));
    }

    Ok((items, meta))
}

fn serialize_items<T: Serialize>(items: Vec<T>) -> Result<Vec<Value>, ApiError> {
    items
        .into_iter()
        .map(|item| {
            serde_json::to_value(item)
                .map_err(|error| ApiError::InternalError(format!("Failed to serialize Inspector item: {}", error)))
        })
        .collect()
}

async fn apply_capability_patches_for_list_request(
    context: &InspectorServiceContext<'_>,
    request: &InspectorCapabilityListRequest,
    kind: CapabilityKind,
    items: &mut [Value],
) -> Result<(), ApiError> {
    if !matches!(request.target.mode, InspectorMode::Native) {
        return Ok(());
    }

    let native_target =
        resolve_expected_native_target(request.target.clone(), "Expected native Inspector mode", false).await?;
    let target = patch_target_from_native(&native_target);
    let patches = capability_patches_for_target(context, &target, kind.patch_kind())?;
    apply_capability_patches(items, kind, &patches);
    Ok(())
}

fn apply_capability_patches(
    items: &mut [Value],
    kind: CapabilityKind,
    patches: &[InspectorCapabilityPatchRecord],
) {
    if patches.is_empty() {
        return;
    }

    for item in items {
        let Some(item_key) = capability_item_key(kind, item).map(str::to_string) else {
            continue;
        };
        let Some(patch) = patches.iter().find(|patch| patch.capability_key == item_key) else {
            continue;
        };
        let Some(item_object) = item.as_object_mut() else {
            continue;
        };
        for (key, value) in &patch.patch {
            item_object.insert(key.clone(), value.clone());
        }
        item_object.insert(
            "inspector_patch".to_string(),
            json!({
                "id": patch.id,
                "capability_key": patch.capability_key,
            }),
        );
    }
}

fn resolve_patched_tool_name(
    context: &InspectorServiceContext<'_>,
    native_target: &InspectorNativeTarget,
    requested_tool_name: &str,
) -> Option<String> {
    let target = patch_target_from_native(native_target);
    let patches = capability_patches_for_target(context, &target, InspectorCapabilityPatchKind::Tools).ok()?;
    patches
        .into_iter()
        .find(|patch| {
            patch
                .patch
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|patched_name| patched_name == requested_tool_name)
        })
        .map(|patch| patch.capability_key)
}

fn capability_patches_for_target(
    context: &InspectorServiceContext<'_>,
    target: &InspectorPatchTarget,
    kind: InspectorCapabilityPatchKind,
) -> Result<Vec<InspectorCapabilityPatchRecord>, ApiError> {
    context
        .workspace()
        .list_capability_patches()
        .map_err(|error| ApiError::InternalError(error.to_string()))
        .map(|records| {
            records
                .into_iter()
                .filter(|record| record.target == *target && record.capability_kind == kind)
                .collect()
        })
}

fn capability_item_key(
    kind: CapabilityKind,
    item: &Value,
) -> Option<&str> {
    match kind {
        CapabilityKind::Tools | CapabilityKind::Prompts => item.get("name").and_then(Value::as_str),
        CapabilityKind::Resources => item.get("uri").and_then(Value::as_str),
        CapabilityKind::ResourceTemplates => item
            .get("uriTemplate")
            .or_else(|| item.get("uri_template"))
            .and_then(Value::as_str),
    }
}

fn parse_patch_kind(raw: &str) -> Result<InspectorCapabilityPatchKind, ApiError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "tool" | "tools" => Ok(InspectorCapabilityPatchKind::Tools),
        "prompt" | "prompts" => Ok(InspectorCapabilityPatchKind::Prompts),
        "resource" | "resources" => Ok(InspectorCapabilityPatchKind::Resources),
        "template" | "templates" | "resource_template" | "resource_templates" => {
            Ok(InspectorCapabilityPatchKind::ResourceTemplates)
        }
        _ => Err(ApiError::BadRequest(format!(
            "Invalid capability_kind '{}'; expected tools, prompts, resources, or resource_templates",
            raw
        ))),
    }
}

fn patch_target_from_native(native_target: &InspectorNativeTarget) -> InspectorPatchTarget {
    match native_target {
        InspectorNativeTarget::Managed { server_id } => InspectorPatchTarget::ManagedRegistry {
            server_id: server_id.clone(),
        },
        InspectorNativeTarget::Scratch { record_id } => InspectorPatchTarget::ScratchWorkspace {
            record_id: record_id.clone(),
        },
    }
}

async fn collect_compatibility_capabilities(
    peer: &Peer<RoleClient>
) -> Result<InspectorCompatibilityEvidence, ApiError> {
    let tools = runtime::list_tools(peer).await?;
    let prompts = runtime::list_prompts(peer).await?;
    let resources = runtime::list_resources(peer).await?;
    let resource_templates = runtime::list_resource_templates(peer).await?;
    let tools = tools
        .into_iter()
        .map(|tool| snapshot_to_value(tool, "compatibility tool"))
        .collect::<Result<Vec<_>, _>>()?;
    let prompts = prompts
        .into_iter()
        .map(|prompt| snapshot_to_value(prompt, "compatibility prompt"))
        .collect::<Result<Vec<_>, _>>()?;
    let resources = resources
        .into_iter()
        .map(|resource| snapshot_to_value(resource, "compatibility resource"))
        .collect::<Result<Vec<_>, _>>()?;
    let resource_templates = resource_templates
        .into_iter()
        .map(|template| snapshot_to_value(template, "compatibility resource template"))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(compatibility_evidence_from_items(
        tools,
        prompts,
        resources,
        resource_templates,
    ))
}

#[cfg(test)]
fn compatibility_capabilities_from_counts(
    tools: usize,
    prompts: usize,
    resources: usize,
    resource_templates: usize,
) -> InspectorCompatibilityEvidence {
    InspectorCompatibilityEvidence {
        counts: InspectorCompatibilityCounts {
            tools,
            prompts,
            resources,
            resource_templates,
        },
        tools: Vec::new(),
        prompts: Vec::new(),
        resources: Vec::new(),
        resource_templates: Vec::new(),
    }
}

fn compatibility_evidence_from_items(
    tools: Vec<Value>,
    prompts: Vec<Value>,
    resources: Vec<Value>,
    resource_templates: Vec<Value>,
) -> InspectorCompatibilityEvidence {
    InspectorCompatibilityEvidence {
        counts: InspectorCompatibilityCounts {
            tools: tools.len(),
            prompts: prompts.len(),
            resources: resources.len(),
            resource_templates: resource_templates.len(),
        },
        tools,
        prompts,
        resources,
        resource_templates,
    }
}

fn compatibility_capabilities_from_evidence(
    evidence: &InspectorCompatibilityEvidence
) -> InspectorCompatibilityCapabilities {
    InspectorCompatibilityCapabilities {
        counts: evidence.counts,
        checks: vec![
            compatibility_check("tools_list", evidence.counts.tools),
            compatibility_check("prompts_list", evidence.counts.prompts),
            compatibility_check("resources_list", evidence.counts.resources),
            compatibility_check("resource_templates_list", evidence.counts.resource_templates),
        ],
    }
}

fn build_compatibility_analysis(
    evidence: InspectorCompatibilityEvidence,
    requested_spec_version: Option<&str>,
) -> Result<InspectorCompatibilityAnalysis, ApiError> {
    let selected_version = resolve_mcp_spec_version(requested_spec_version)?;
    let capabilities = compatibility_capabilities_from_evidence(&evidence);
    let requirements = compatibility_requirements_for_evidence(selected_version, &evidence);
    let summary = compatibility_summary(&requirements);

    Ok(InspectorCompatibilityAnalysis {
        spec: InspectorCompatibilitySpecSelection {
            selected_version,
            current_version: CURRENT_MCP_SPEC_VERSION,
            available_versions: MCP_SPEC_VERSIONS.iter().map(|version| version.version).collect(),
        },
        observed: InspectorCompatibilityObserved {
            protocol_version: None,
            counts: capabilities.counts,
            metrics: compatibility_metrics_from_evidence(&evidence),
        },
        summary,
        inferred_best_fit_version: infer_best_fit_mcp_spec_version(&capabilities.counts),
        capabilities,
        requirements,
    })
}

fn resolve_mcp_spec_version(requested_spec_version: Option<&str>) -> Result<&'static str, ApiError> {
    let Some(raw_version) = requested_spec_version
        .map(str::trim)
        .filter(|version| !version.is_empty())
    else {
        return Ok(CURRENT_MCP_SPEC_VERSION);
    };

    MCP_SPEC_VERSIONS
        .iter()
        .find(|version| version.version == raw_version)
        .map(|version| version.version)
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Unsupported MCP specification version '{}'; supported versions are {}",
                raw_version,
                MCP_SPEC_VERSIONS
                    .iter()
                    .map(|version| version.version)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

fn compatibility_requirements_for_evidence(
    spec_version: &'static str,
    evidence: &InspectorCompatibilityEvidence,
) -> Vec<InspectorCompatibilityRequirement> {
    MCP_SPEC_REQUIREMENTS
        .iter()
        .map(|requirement| {
            let observed = compatibility_requirement_observation(requirement.probe, evidence);

            InspectorCompatibilityRequirement {
                id: requirement.id,
                title: requirement.title,
                category: requirement.category,
                status: observed.status.as_str(),
                expected: InspectorCompatibilityRequirementExpected {
                    version: spec_version,
                    description: requirement.description,
                    required: requirement.required,
                },
                observed: InspectorCompatibilityRequirementObserved {
                    count: observed.count,
                    total: observed.total,
                    detail: observed.detail.clone(),
                },
                diff: InspectorCompatibilityRequirementDiff {
                    left_label: "Spec requirement",
                    left: requirement.description,
                    right_label: "Observed server",
                    right: observed.detail,
                },
            }
        })
        .collect()
}

struct InspectorCompatibilityObservation {
    status: InspectorCompatibilityStatus,
    count: Option<usize>,
    total: Option<usize>,
    detail: String,
}

fn compatibility_requirement_observation(
    probe: InspectorSpecRequirementProbe,
    evidence: &InspectorCompatibilityEvidence,
) -> InspectorCompatibilityObservation {
    match probe {
        InspectorSpecRequirementProbe::Surface(surface) => {
            let count = compatibility_surface_count(surface, &evidence.counts);
            InspectorCompatibilityObservation {
                status: if count > 0 {
                    InspectorCompatibilityStatus::Implemented
                } else {
                    InspectorCompatibilityStatus::NotAdvertised
                },
                count: Some(count),
                total: None,
                detail: if count > 0 {
                    format!("{count} item(s) observed")
                } else {
                    "No items advertised by this server".to_string()
                },
            }
        }
        InspectorSpecRequirementProbe::ToolInputSchema => compatibility_list_observation(
            &evidence.tools,
            evidence.counts.tools,
            "tool(s) expose input schema objects",
            "No tools advertised by this server",
            |item| has_object_field(item, &["inputSchema", "input_schema"]),
            true,
        ),
        InspectorSpecRequirementProbe::ToolNameFormat => compatibility_list_observation(
            &evidence.tools,
            evidence.counts.tools,
            "tool name(s) use MCP-compatible characters",
            "No tools advertised by this server",
            |item| {
                first_string_field(item, &["name"])
                    .as_deref()
                    .is_some_and(is_mcp_name_compatible)
            },
            true,
        ),
        InspectorSpecRequirementProbe::ToolOutputSchema => compatibility_list_observation(
            &evidence.tools,
            evidence.counts.tools,
            "tool(s) expose output schema objects",
            "No tools advertised by this server",
            |item| has_object_field(item, &["outputSchema", "output_schema"]),
            false,
        ),
        InspectorSpecRequirementProbe::PromptArguments => compatibility_list_observation(
            &evidence.prompts,
            evidence.counts.prompts,
            "prompt(s) expose argument metadata",
            "No prompts advertised by this server",
            |item| item.get("arguments").is_some_and(Value::is_array),
            false,
        ),
        InspectorSpecRequirementProbe::ResourceUri => compatibility_list_observation(
            &evidence.resources,
            evidence.counts.resources,
            "resource(s) expose readable URIs",
            "No resources advertised by this server",
            |item| first_string_field(item, &["uri"]).is_some(),
            true,
        ),
        InspectorSpecRequirementProbe::ResourceMimeType => compatibility_list_observation(
            &evidence.resources,
            evidence.counts.resources,
            "resource(s) expose MIME type metadata",
            "No resources advertised by this server",
            |item| first_string_field(item, &["mimeType", "mime_type"]).is_some(),
            false,
        ),
        InspectorSpecRequirementProbe::ProtocolVersionNegotiation => InspectorCompatibilityObservation {
            status: InspectorCompatibilityStatus::Unknown,
            count: None,
            total: None,
            detail: "Not observable from the current Inspector snapshot".to_string(),
        },
    }
}

fn compatibility_surface_count(
    surface: InspectorCompatibilitySurface,
    counts: &InspectorCompatibilityCounts,
) -> usize {
    match surface {
        InspectorCompatibilitySurface::Tools => counts.tools,
        InspectorCompatibilitySurface::Prompts => counts.prompts,
        InspectorCompatibilitySurface::Resources => counts.resources,
        InspectorCompatibilitySurface::ResourceTemplates => counts.resource_templates,
    }
}

fn compatibility_list_observation(
    items: &[Value],
    advertised_count: usize,
    positive_detail: &'static str,
    empty_detail: &'static str,
    predicate: impl Fn(&Value) -> bool,
    required_when_advertised: bool,
) -> InspectorCompatibilityObservation {
    if advertised_count == 0 {
        return InspectorCompatibilityObservation {
            status: InspectorCompatibilityStatus::NotAdvertised,
            count: Some(0),
            total: Some(0),
            detail: empty_detail.to_string(),
        };
    }

    if items.is_empty() {
        return InspectorCompatibilityObservation {
            status: InspectorCompatibilityStatus::Unknown,
            count: None,
            total: Some(advertised_count),
            detail: "Only aggregate counts are available; item-level metadata was not captured".to_string(),
        };
    }

    let matched = items.iter().filter(|item| predicate(item)).count();
    let total = items.len();
    let status = if matched == total {
        InspectorCompatibilityStatus::Implemented
    } else if matched == 0 && !required_when_advertised {
        InspectorCompatibilityStatus::NotAdvertised
    } else {
        InspectorCompatibilityStatus::Partial
    };

    InspectorCompatibilityObservation {
        status,
        count: Some(matched),
        total: Some(total),
        detail: format!("{matched}/{total} {positive_detail}"),
    }
}

fn compatibility_metrics_from_evidence(evidence: &InspectorCompatibilityEvidence) -> Vec<InspectorCompatibilityMetric> {
    vec![
        compatibility_metric(
            "tools_input_schema",
            evidence
                .tools
                .iter()
                .filter(|item| has_object_field(item, &["inputSchema", "input_schema"]))
                .count(),
        ),
        compatibility_metric(
            "tools_output_schema",
            evidence
                .tools
                .iter()
                .filter(|item| has_object_field(item, &["outputSchema", "output_schema"]))
                .count(),
        ),
        compatibility_metric(
            "tools_valid_name",
            evidence
                .tools
                .iter()
                .filter(|item| {
                    first_string_field(item, &["name"])
                        .as_deref()
                        .is_some_and(is_mcp_name_compatible)
                })
                .count(),
        ),
        compatibility_metric(
            "prompts_arguments",
            evidence
                .prompts
                .iter()
                .filter(|item| item.get("arguments").is_some_and(Value::is_array))
                .count(),
        ),
        compatibility_metric(
            "resources_uri",
            evidence
                .resources
                .iter()
                .filter(|item| first_string_field(item, &["uri"]).is_some())
                .count(),
        ),
        compatibility_metric(
            "resources_mime_type",
            evidence
                .resources
                .iter()
                .filter(|item| first_string_field(item, &["mimeType", "mime_type"]).is_some())
                .count(),
        ),
        compatibility_metric("resource_templates", evidence.resource_templates.len()),
    ]
}

fn compatibility_metric(
    id: &'static str,
    value: usize,
) -> InspectorCompatibilityMetric {
    InspectorCompatibilityMetric { id, value }
}

fn has_object_field(
    item: &Value,
    names: &[&str],
) -> bool {
    names.iter().any(|name| item.get(*name).is_some_and(Value::is_object))
}

fn is_mcp_name_compatible(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

fn compatibility_summary(requirements: &[InspectorCompatibilityRequirement]) -> InspectorCompatibilitySummary {
    let mut summary = InspectorCompatibilitySummary {
        total: requirements.len(),
        implemented: 0,
        partial: 0,
        not_advertised: 0,
        unknown: 0,
    };

    for requirement in requirements {
        match requirement.status {
            "implemented" => summary.implemented += 1,
            "partial" => summary.partial += 1,
            "not_advertised" => summary.not_advertised += 1,
            _ => summary.unknown += 1,
        }
    }

    summary
}

fn infer_best_fit_mcp_spec_version(counts: &InspectorCompatibilityCounts) -> &'static str {
    if counts.resource_templates > 0 || counts.prompts > 0 || counts.resources > 0 || counts.tools > 0 {
        CURRENT_MCP_SPEC_VERSION
    } else {
        "2024-11-05"
    }
}

fn compatibility_check(
    id: &'static str,
    observed_count: usize,
) -> InspectorCompatibilityCheck {
    InspectorCompatibilityCheck {
        id,
        status: "pass",
        observed_count,
    }
}

fn native_product_snapshot_target(
    native_target: &InspectorNativeTarget,
    reference_id: Option<String>,
    transport: String,
    session_id: Option<String>,
) -> InspectorProductSnapshotTarget {
    InspectorProductSnapshotTarget {
        mode: InspectorMode::Native,
        source: native_target_source(native_target),
        reference_id,
        server_id: native_target.server_id().map(str::to_string),
        scratch_id: native_target.scratch_id().map(str::to_string),
        proxy_mode: None,
        proxy_scope: None,
        transport,
        session_id,
    }
}

fn proxy_product_snapshot_target(
    proxy_target: &InspectorProxyTarget,
    session_id: Option<String>,
) -> InspectorProductSnapshotTarget {
    InspectorProductSnapshotTarget {
        mode: InspectorMode::Proxy,
        source: "mcp_proxy",
        reference_id: None,
        server_id: proxy_target.server_id().map(str::to_string),
        scratch_id: None,
        proxy_mode: Some(proxy_target.proxy_mode()),
        proxy_scope: Some(proxy_target.proxy_scope()),
        transport: "streamable_http".to_string(),
        session_id,
    }
}

async fn native_target_transport(
    context: &InspectorServiceContext<'_>,
    native_target: &InspectorNativeTarget,
) -> Result<String, ApiError> {
    let config = context
        .runtime_environment()
        .native_target_config(native_target)
        .await?;
    Ok(config.config.kind.client_format().to_string())
}

async fn native_target_config(
    context: &InspectorServiceContext<'_>,
    native_target: &InspectorNativeTarget,
) -> Result<runtime::InspectorNativeTargetConfig, ApiError> {
    context.runtime_environment().native_target_config(native_target).await
}

fn native_target_source(native_target: &InspectorNativeTarget) -> &'static str {
    match native_target {
        InspectorNativeTarget::Managed { .. } => "managed_registry",
        InspectorNativeTarget::Scratch { .. } => "scratch_workspace",
    }
}

fn native_response_platform_payload<const N: usize>(
    native_target: &InspectorNativeTarget,
    server_id: &Option<String>,
    server_name: &Option<String>,
    fields: [(&'static str, Value); N],
) -> Value {
    let mut payload = Map::new();
    for (key, value) in fields {
        payload.insert(key.to_string(), value);
    }
    payload.insert("server_id".to_string(), json!(server_id));
    payload.insert("server_name".to_string(), json!(server_name));
    payload.insert("scratch_id".to_string(), json!(native_target.scratch_id()));
    payload.insert("source".to_string(), json!(native_target_source(native_target)));
    Value::Object(payload)
}

fn package_safety_inventory(config: &MCPServerConfig) -> InspectorPackageSafetyInventory {
    match config.kind {
        crate::common::server::ServerType::Stdio => {
            let command = config.command.as_deref().unwrap_or_default();
            let args = config.args.clone().unwrap_or_default();
            let runtime_type = RuntimeType::from_command(command).map(|runtime| runtime.as_str().to_string());
            let fingerprint = crate::config::server::fingerprint::fingerprint_for_stdio(command, &args);

            InspectorPackageSafetyInventory::Stdio {
                command: command.to_string(),
                runtime: runtime_type,
                fingerprint,
                args_count: args.len(),
            }
        }
        crate::common::server::ServerType::Sse | crate::common::server::ServerType::StreamableHttp => {
            let url = config.url.as_deref().unwrap_or_default();
            let signature = crate::config::server::fingerprint::url_signature(url);
            InspectorPackageSafetyInventory::Http {
                url_fingerprint: signature.fingerprint,
                base: signature.base,
            }
        }
    }
}

fn build_package_safety_analysis(
    config: &MCPServerConfig,
    requested_source: Option<&str>,
    requested_scan_depth: Option<&str>,
) -> Result<InspectorPackageSafetyAnalysis, ApiError> {
    let source = resolve_package_safety_source(requested_source)?;
    let scan_depth = resolve_package_safety_scan_depth(requested_scan_depth)?;
    let inventory = package_safety_inventory(config);
    let findings = package_safety_findings(config, source, scan_depth);
    let summary = package_safety_summary(&findings);
    let recommendations = package_safety_recommendations(&findings);

    Ok(InspectorPackageSafetyAnalysis {
        input: InspectorPackageSafetyInput {
            source: source.to_string(),
            scan_depth: scan_depth.to_string(),
        },
        scanner: InspectorPackageSafetyScanner {
            provider: "local_rules".to_string(),
            status: "completed",
        },
        inventory,
        summary,
        findings,
        recommendations,
    })
}

fn resolve_package_safety_source(requested_source: Option<&str>) -> Result<&'static str, ApiError> {
    let source = requested_source
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PACKAGE_SAFETY_SOURCE);

    match source {
        "server_config" => Ok("server_config"),
        "runtime_cache" => Err(ApiError::BadRequest(
            "Package safety source 'runtime_cache' is not implemented yet; use 'server_config' for local rules"
                .to_string(),
        )),
        _ => Err(ApiError::BadRequest(format!(
            "Unsupported package safety source '{}'; supported source is server_config",
            source
        ))),
    }
}

fn resolve_package_safety_scan_depth(requested_scan_depth: Option<&str>) -> Result<&'static str, ApiError> {
    let scan_depth = requested_scan_depth
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PACKAGE_SAFETY_SCAN_DEPTH);

    match scan_depth {
        "standard" => Ok("standard"),
        "deep" => Ok("deep"),
        _ => Err(ApiError::BadRequest(format!(
            "Unsupported package safety scan depth '{}'; supported depths are standard and deep",
            scan_depth
        ))),
    }
}

fn package_safety_findings(
    config: &MCPServerConfig,
    source: &str,
    scan_depth: &str,
) -> Vec<InspectorPackageSafetyFinding> {
    let mut findings = Vec::new();

    findings.push(InspectorPackageSafetyFinding {
        id: "scan_source_selected",
        severity: "info",
        title: "Scan source selected",
        detail: format!("Using '{source}' facts with '{scan_depth}' depth."),
        recommendation: "Use runtime cache facts when package lockfiles are available; use server config facts for quick inspection.",
    });

    match config.kind {
        crate::common::server::ServerType::Stdio => {
            let command = config.command.as_deref().unwrap_or_default().trim();
            let args = config.args.as_deref().unwrap_or_default();
            if command.is_empty() {
                findings.push(InspectorPackageSafetyFinding {
                    id: "stdio_missing_command",
                    severity: "high",
                    title: "Missing stdio command",
                    detail: "The server config does not define a command to execute.".to_string(),
                    recommendation: "Define an explicit command before trusting or inspecting this server.",
                });
            } else {
                findings.push(InspectorPackageSafetyFinding {
                    id: "stdio_runtime_detected",
                    severity: "info",
                    title: "Stdio runtime detected",
                    detail: format!("Command '{command}' is used to start the server."),
                    recommendation: "Review command provenance and package manager behavior before enabling the server broadly.",
                });
            }

            if is_package_runner(command) && !args.is_empty() {
                findings.push(InspectorPackageSafetyFinding {
                    id: "stdio_package_reference",
                    severity: "medium",
                    title: "Package runner invocation",
                    detail: format!(
                        "The server starts through '{}' with package or script arguments.",
                        command
                    ),
                    recommendation: "Resolve the package identity, lock version, and advisory status before managed adoption.",
                });
            }

            if command.starts_with('/') || args.iter().any(|arg| looks_like_local_path(arg)) {
                findings.push(InspectorPackageSafetyFinding {
                    id: "local_path_reference",
                    severity: "low",
                    title: "Local path reference",
                    detail: "The server appears to reference local code or a local executable.".to_string(),
                    recommendation: "Treat this as a development server and review source changes directly.",
                });
            }
        }
        crate::common::server::ServerType::Sse | crate::common::server::ServerType::StreamableHttp => {
            let url = config.url.as_deref().unwrap_or_default();
            if url.starts_with("https://") {
                findings.push(InspectorPackageSafetyFinding {
                    id: "remote_https_endpoint",
                    severity: "info",
                    title: "Remote HTTPS endpoint",
                    detail: "The server uses a remote HTTPS endpoint.".to_string(),
                    recommendation: "Review endpoint ownership, authentication, and vendor trust before managed adoption.",
                });
            } else if url.starts_with("http://") {
                findings.push(InspectorPackageSafetyFinding {
                    id: "remote_plain_http_endpoint",
                    severity: "high",
                    title: "Plain HTTP endpoint",
                    detail: "The server uses an unencrypted HTTP endpoint.".to_string(),
                    recommendation: "Use HTTPS or restrict this server to a trusted local network.",
                });
            } else {
                findings.push(InspectorPackageSafetyFinding {
                    id: "remote_endpoint_unknown",
                    severity: "medium",
                    title: "Endpoint is missing or unparseable",
                    detail: "The server URL is missing or does not use an expected HTTP scheme.".to_string(),
                    recommendation: "Confirm the endpoint before running deeper compatibility or LLM evaluation.",
                });
            }
        }
    }

    findings
}

fn is_package_runner(command: &str) -> bool {
    matches!(
        command,
        "npx" | "npm" | "bunx" | "bun" | "uvx" | "uv" | "pipx" | "python" | "python3" | "node"
    )
}

fn looks_like_local_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("./")
        || value.starts_with("../")
        || value.contains("/src/")
        || value.ends_with(".py")
        || value.ends_with(".js")
        || value.ends_with(".ts")
}

fn package_safety_summary(findings: &[InspectorPackageSafetyFinding]) -> InspectorPackageSafetySummary {
    let mut summary = InspectorPackageSafetySummary {
        total: findings.len(),
        high: 0,
        medium: 0,
        low: 0,
        info: 0,
    };

    for finding in findings {
        match finding.severity {
            "high" => summary.high += 1,
            "medium" => summary.medium += 1,
            "low" => summary.low += 1,
            _ => summary.info += 1,
        }
    }

    summary
}

fn package_safety_recommendations(
    findings: &[InspectorPackageSafetyFinding]
) -> Vec<InspectorPackageSafetyRecommendation> {
    let has_high = findings.iter().any(|finding| finding.severity == "high");
    let has_package_reference = findings.iter().any(|finding| finding.id == "stdio_package_reference");

    let mut recommendations = Vec::new();
    if has_high {
        recommendations.push(InspectorPackageSafetyRecommendation {
            id: "review_before_use",
            message: "Review high-severity package safety findings before enabling or recommending this server.",
        });
    }
    if has_package_reference {
        recommendations.push(InspectorPackageSafetyRecommendation {
            id: "resolve_package_identity",
            message: "Resolve package identity, version, and advisory status before treating this server as managed.",
        });
    }
    if recommendations.is_empty() {
        recommendations.push(InspectorPackageSafetyRecommendation {
            id: "continue_with_context",
            message: "No blocking local rule finding was produced; continue with compatibility and runtime evidence review.",
        });
    }
    recommendations
}

fn normalize_llm_evaluation_dimensions(dimensions: Vec<String>) -> Result<Vec<String>, ApiError> {
    let mut normalized = dimensions
        .into_iter()
        .map(|dimension| dimension.trim().to_ascii_lowercase())
        .filter(|dimension| !dimension.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();

    for dimension in &normalized {
        if !is_supported_llm_evaluation_dimension(dimension) {
            return Err(ApiError::BadRequest(format!(
                "Unsupported Inspector LLM evaluation dimension '{}'; supported dimensions are capability_surface, compatibility, package_safety, and debuggability",
                dimension
            )));
        }
    }

    if normalized.is_empty() {
        Ok(vec![DEFAULT_LLM_EVALUATION_DIMENSION.to_string()])
    } else {
        Ok(normalized)
    }
}

fn is_supported_llm_evaluation_dimension(dimension: &str) -> bool {
    matches!(
        dimension,
        "capability_surface" | "compatibility" | "package_safety" | "debuggability"
    )
}

fn summarize_llm_evaluation_evidence(evidence: &[Value]) -> Vec<Value> {
    evidence
        .iter()
        .take(12)
        .enumerate()
        .map(|(index, value)| {
            json!({
                "index": index,
                "kind": value.get("kind").or_else(|| value.get("type")).cloned().unwrap_or(Value::Null),
                "summary": value.get("summary").or_else(|| value.get("title")).cloned().unwrap_or_else(|| {
                    match value {
                        Value::Object(map) => json!(map.keys().take(8).cloned().collect::<Vec<_>>()),
                        _ => value.clone(),
                    }
                }),
                "payload": value,
            })
        })
        .collect()
}

fn format_llm_evaluation_evidence_prompt(
    dimensions: &[String],
    evidence_summary: &[Value],
) -> String {
    let evidence_json = serde_json::to_string_pretty(evidence_summary).unwrap_or_else(|_| "[]".to_string());
    let bounded_evidence = if evidence_json.len() > MAX_LLM_EVALUATION_EVIDENCE_CHARS {
        format!(
            "{}\n... truncated after {} characters",
            &evidence_json[..MAX_LLM_EVALUATION_EVIDENCE_CHARS],
            MAX_LLM_EVALUATION_EVIDENCE_CHARS
        )
    } else {
        evidence_json
    };

    format!(
        "Selected dimensions:\n{}\n\nInspector evidence:\n{}",
        dimensions.join(", "),
        bounded_evidence
    )
}

fn llm_tool_from_capability(item: &Value) -> Option<LlmTool> {
    let name = first_string_field(item, &["name", "tool_name", "unique_name"])?;
    let description = first_string_field(item, &["description"]).unwrap_or_default();
    let parameters = item
        .get("inputSchema")
        .or_else(|| item.get("input_schema"))
        .or_else(|| item.get("schema"))
        .cloned()
        .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));

    Some(LlmTool {
        name,
        description,
        parameters,
    })
}

fn first_string_field(
    item: &Value,
    fields: &[&str],
) -> Option<String> {
    fields.iter().find_map(|field| {
        item.get(*field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn llm_evaluation_target(
    target: &InspectorTarget,
    session_id: Option<String>,
) -> InspectorLlmEvaluationTarget {
    InspectorLlmEvaluationTarget {
        mode: target.mode(),
        server_id: target.server_id().map(str::to_string),
        scratch_id: target.scratch_id().map(str::to_string),
        proxy_mode: target.proxy_mode(),
        proxy_scope: target.proxy_scope(),
        session_id,
    }
}

fn snapshot_to_value(
    snapshot: impl Serialize,
    label: &'static str,
) -> Result<Value, ApiError> {
    serde_json::to_value(snapshot)
        .map_err(|error| ApiError::InternalError(format!("Failed to serialize {label}: {error}")))
}

fn native_mode_enabled() -> bool {
    match std::env::var("MCPMATE_INSPECTOR_NATIVE") {
        Ok(val) => {
            let val_lower = val.trim().to_ascii_lowercase();
            // Explicit "off" values disable; everything else (including unknown) enables
            !matches!(val_lower.as_str(), "0" | "false" | "off" | "no")
        }
        // Default to enabled if not set
        Err(_) => true,
    }
}

async fn acquire_target_peer(
    context: &InspectorServiceContext<'_>,
    target: &InspectorTarget,
    active_session: Option<ActiveSession>,
) -> Result<InspectorAcquiredPeer, ApiError> {
    let session_peer = active_session.and_then(|session| session.peer);
    let runtime_env = context.runtime_environment();
    runtime_env.acquire_target_peer(target, session_peer).await
}

async fn acquire_proxy_target_peer(
    context: &InspectorServiceContext<'_>,
    active_session: Option<ActiveSession>,
    fallback_target: Option<InspectorProxyTarget>,
) -> Result<AcquiredProxyTargetPeer, ApiError> {
    let target = if let Some(session) = active_session.as_ref() {
        session
            .target
            .as_proxy()
            .cloned()
            .ok_or_else(|| ApiError::InternalError("Expected proxy Inspector session target".into()))?
    } else {
        fallback_target.ok_or_else(|| ApiError::InternalError("Expected proxy Inspector target".into()))?
    };
    let acquired = acquire_target_peer(context, &InspectorTarget::proxy(target.clone()), active_session).await?;
    Ok(AcquiredProxyTargetPeer { acquired, target })
}

fn ensure_native_allowed() -> Result<(), ApiError> {
    if native_mode_enabled() {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Native inspector mode is disabled; set MCPMATE_INSPECTOR_NATIVE=on (or unset) to enable".into(),
        ))
    }
}

async fn acquire_native_direct_peer(
    context: &InspectorServiceContext<'_>,
    native_target: &InspectorNativeTarget,
    inspector_session_id: Option<&str>,
) -> Result<InspectorAcquiredPeer, ApiError> {
    let active_session = if let Some(inspector_session_id) = inspector_session_id {
        let session = get_active_session_ref(context, inspector_session_id).await?;

        session
            .target
            .ensure_session_binding(InspectorMode::Native, native_target.reference_id())
            .map_err(inspector_target_error)?;
        Some(session)
    } else {
        None
    };

    let target = InspectorTarget::Native(native_target.clone());
    acquire_target_peer(context, &target, active_session).await
}

async fn get_active_session(
    context: &InspectorServiceContext<'_>,
    session_id: &str,
) -> Result<ActiveSession, ApiError> {
    get_active_session_ref(context, session_id).await
}

async fn get_active_proxy_session_for_request(
    context: &InspectorServiceContext<'_>,
    session_id: Option<&str>,
    request: &InspectorTargetRequest,
) -> Result<Option<ActiveSession>, ApiError> {
    let Some(session_id) = session_id else {
        return Ok(None);
    };

    request
        .ensure_session_options_unchanged(true)
        .map_err(inspector_target_error)?;

    let session = get_active_session_ref(context, session_id).await?;
    session
        .target
        .ensure_mode(InspectorMode::Proxy)
        .map_err(inspector_target_error)?;

    Ok(Some(session))
}

async fn get_active_session_ref(
    context: &InspectorServiceContext<'_>,
    session_id: &str,
) -> Result<ActiveSession, ApiError> {
    match context.sessions().get_session_or_expired(session_id).await {
        SessionLookup::Active(session) => Ok(session),
        SessionLookup::Expired(closed) => {
            closed.cleanup_runtime().await;
            Err(ApiError::NotFound(format!(
                "Inspector session '{}' not found or expired",
                session_id
            )))
        }
        SessionLookup::Missing => Err(ApiError::NotFound(format!(
            "Inspector session '{}' not found or expired",
            session_id
        ))),
    }
}

async fn resolve_server(
    server_id: &Option<String>,
    server_name: &Option<String>,
) -> Result<String, ApiError> {
    let reference = InspectorTargetRequest::managed_server_reference(server_id, server_name)
        .ok_or_else(|| inspector_target_error(InspectorTargetError::MissingServerReference))?;
    resolve_server_reference(reference).await
}

pub(crate) async fn inspector_forward_progress(params: &ProgressNotificationParam) -> bool {
    if let Some(registry) = GLOBAL_CALL_REGISTRY.get() {
        registry.emit_progress(params).await;
    }
    false
}

pub(crate) async fn inspector_forward_log(
    token: Option<&ProgressToken>,
    params: &LoggingMessageNotificationParam,
) -> bool {
    if let Some(registry) = GLOBAL_CALL_REGISTRY.get() {
        registry.emit_log(token, params).await;
    }
    false
}

pub(crate) async fn inspector_forward_cancel(
    request_id: &RequestId,
    reason: Option<String>,
) -> bool {
    if let Some(registry) = GLOBAL_CALL_REGISTRY.get() {
        registry.emit_cancelled(request_id, reason).await;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_response_platform_payload_marks_managed_source() {
        let server_id = Some("server-1".to_string());
        let server_name = Some("Fetch".to_string());
        let target = InspectorNativeTarget::Managed {
            server_id: "server-1".to_string(),
        };

        let payload =
            native_response_platform_payload(&target, &server_id, &server_name, [("name", json!("hello_prompt"))]);

        assert_eq!(payload["name"], json!("hello_prompt"));
        assert_eq!(payload["server_id"], json!("server-1"));
        assert_eq!(payload["server_name"], json!("Fetch"));
        assert_eq!(payload["scratch_id"], Value::Null);
        assert_eq!(payload["source"], json!("managed_registry"));
    }

    #[test]
    fn native_response_platform_payload_marks_scratch_source() {
        let server_id = None;
        let server_name = None;
        let target = InspectorNativeTarget::Scratch {
            record_id: "scratch-1".to_string(),
        };

        let payload =
            native_response_platform_payload(&target, &server_id, &server_name, [("uri", json!("test://hello"))]);

        assert_eq!(payload["uri"], json!("test://hello"));
        assert_eq!(payload["server_id"], Value::Null);
        assert_eq!(payload["server_name"], Value::Null);
        assert_eq!(payload["scratch_id"], json!("scratch-1"));
        assert_eq!(payload["source"], json!("scratch_workspace"));
    }

    #[test]
    fn native_product_snapshot_target_serializes_managed_source() {
        let target = InspectorNativeTarget::Managed {
            server_id: "server-1".to_string(),
        };
        let value = snapshot_to_value(
            native_product_snapshot_target(
                &target,
                Some("server-1".to_string()),
                "stdio".to_string(),
                Some("session-1".to_string()),
            ),
            "test target",
        )
        .expect("serialize target");

        assert_eq!(value["mode"], json!("native"));
        assert_eq!(value["source"], json!("managed_registry"));
        assert_eq!(value["reference_id"], json!("server-1"));
        assert_eq!(value["server_id"], json!("server-1"));
        assert_eq!(value["scratch_id"], Value::Null);
        assert_eq!(value["transport"], json!("stdio"));
        assert_eq!(value["session_id"], json!("session-1"));
    }

    #[test]
    fn proxy_product_snapshot_target_serializes_proxy_contract() {
        let target = InspectorProxyTarget::from_parts(
            Some(InspectorProxyMode::Unify),
            Some(InspectorProxyScope::ActiveCatalog),
            Some(vec!["server-1".to_string()]),
        )
        .expect("proxy target");
        let value =
            snapshot_to_value(proxy_product_snapshot_target(&target, None), "test target").expect("serialize target");

        assert_eq!(value["mode"], json!("proxy"));
        assert_eq!(value["source"], json!("mcp_proxy"));
        assert_eq!(value["server_id"], json!("server-1"));
        assert_eq!(value["proxy_mode"], json!("unify"));
        assert_eq!(value["proxy_scope"], json!("active_catalog"));
        assert_eq!(value["transport"], json!("streamable_http"));
        assert_eq!(value["session_id"], Value::Null);
    }

    #[test]
    fn package_safety_inventory_serializes_stdio_contract() {
        let config = MCPServerConfig {
            kind: crate::common::server::ServerType::Stdio,
            command: Some("uvx".to_string()),
            args: Some(vec!["mcp-server-fetch".to_string()]),
            url: None,
            env: None,
            headers: None,
        };
        let value =
            snapshot_to_value(package_safety_inventory(&config), "test inventory").expect("serialize inventory");

        assert_eq!(value["kind"], json!("stdio"));
        assert_eq!(value["command"], json!("uvx"));
        assert_eq!(value["runtime"], json!("uv"));
        assert_eq!(value["args_count"], json!(1));
        assert!(
            value["fingerprint"]
                .as_str()
                .is_some_and(|fingerprint| !fingerprint.is_empty())
        );
    }

    #[test]
    fn compatibility_analysis_rejects_unreleased_spec_versions() {
        let capabilities = compatibility_capabilities_from_counts(1, 0, 1, 0);

        let error = match build_compatibility_analysis(capabilities, Some("2026-07-01")) {
            Ok(_) => panic!("unreleased spec version should be rejected"),
            Err(error) => error,
        };

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn compatibility_analysis_serializes_diff_ready_requirements() {
        let capabilities = compatibility_capabilities_from_counts(2, 1, 1, 1);
        let analysis = build_compatibility_analysis(capabilities, Some("2025-11-25")).expect("compatibility analysis");
        let value = snapshot_to_value(analysis, "compatibility analysis").expect("serialize analysis");

        assert_eq!(value["spec"]["selected_version"], json!("2025-11-25"));
        assert_eq!(value["spec"]["current_version"], json!("2025-11-25"));
        assert_eq!(value["inferred_best_fit_version"], json!("2025-11-25"));
        assert_eq!(value["summary"]["implemented"], json!(4));
        assert_eq!(value["summary"]["not_advertised"], json!(0));
        assert_eq!(value["requirements"][0]["expected"]["version"], json!("2025-11-25"));
        assert_eq!(
            value["requirements"][0]["diff"]["left_label"],
            json!("Spec requirement")
        );
        assert_eq!(
            value["requirements"][0]["diff"]["right_label"],
            json!("Observed server")
        );
    }

    #[test]
    fn compatibility_analysis_reports_tool_schema_requirements() {
        let evidence = compatibility_evidence_from_items(
            vec![json!({
                "name": "valid_tool",
                "description": "Run a valid tool",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                },
                "outputSchema": {
                    "type": "object",
                    "properties": {}
                }
            })],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let analysis = build_compatibility_analysis(evidence, Some("2025-11-25")).expect("compatibility analysis");
        let value = snapshot_to_value(analysis, "compatibility analysis").expect("serialize analysis");

        assert_eq!(value["summary"]["implemented"], json!(4));
        assert_eq!(
            compatibility_requirement_status_value(&value, "tools_input_schema"),
            Some("implemented")
        );
        assert_eq!(
            compatibility_requirement_status_value(&value, "tools_name_format"),
            Some("implemented")
        );
        assert_eq!(
            compatibility_requirement_status_value(&value, "tools_output_schema"),
            Some("implemented")
        );
    }

    #[test]
    fn compatibility_analysis_reports_partial_tool_name_format() {
        let evidence = compatibility_evidence_from_items(
            vec![
                json!({
                    "name": "valid_tool",
                    "inputSchema": { "type": "object", "properties": {} }
                }),
                json!({
                    "name": "invalid tool name",
                    "inputSchema": { "type": "object", "properties": {} }
                }),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let analysis = build_compatibility_analysis(evidence, Some("2025-11-25")).expect("compatibility analysis");
        let value = snapshot_to_value(analysis, "compatibility analysis").expect("serialize analysis");

        assert_eq!(value["summary"]["partial"], json!(1));
        assert_eq!(
            compatibility_requirement_status_value(&value, "tools_name_format"),
            Some("partial")
        );
    }

    #[test]
    fn package_safety_analysis_marks_local_rule_findings() {
        let config = MCPServerConfig {
            kind: crate::common::server::ServerType::Stdio,
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
            ]),
            url: None,
            env: None,
            headers: None,
        };
        let analysis = build_package_safety_analysis(&config, Some("server_config"), Some("standard"))
            .expect("package safety analysis");
        let value = snapshot_to_value(analysis, "package safety analysis").expect("serialize package safety");

        assert_eq!(value["scanner"]["provider"], json!("local_rules"));
        assert_eq!(value["scanner"]["status"], json!("completed"));
        assert_eq!(value["input"]["source"], json!("server_config"));
        assert_eq!(value["input"]["scan_depth"], json!("standard"));
        assert!(value["findings"].as_array().is_some_and(|findings| {
            findings
                .iter()
                .any(|finding| finding["id"] == json!("stdio_package_reference"))
        }));
    }

    #[test]
    fn package_safety_analysis_rejects_unknown_source() {
        let config = stdio_config("uvx", &["mcp-server-fetch"]);
        let error = expect_bad_request(
            build_package_safety_analysis(&config, Some("advisory_network"), Some("standard")),
            "unknown source should be rejected",
        );

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn package_safety_analysis_rejects_runtime_cache_until_implemented() {
        let config = stdio_config("uvx", &["mcp-server-fetch"]);
        let error = expect_bad_request(
            build_package_safety_analysis(&config, Some("runtime_cache"), Some("standard")),
            "runtime cache should not silently fall back",
        );

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn package_safety_analysis_rejects_unknown_scan_depth() {
        let config = stdio_config("uvx", &["mcp-server-fetch"]);
        let error = expect_bad_request(
            build_package_safety_analysis(&config, Some("server_config"), Some("advisory")),
            "unknown scan depth should be rejected",
        );

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn llm_evaluation_dimensions_reject_unknown_dimension() {
        let error = normalize_llm_evaluation_dimensions(vec!["security".to_string()])
            .expect_err("unknown dimension should be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn llm_evaluation_dimensions_normalize_allowed_values() {
        let dimensions = normalize_llm_evaluation_dimensions(vec![
            " Package_Safety ".to_string(),
            "package_safety".to_string(),
            "Compatibility".to_string(),
        ])
        .expect("dimensions");

        assert_eq!(dimensions, vec!["compatibility", "package_safety"]);
    }

    #[test]
    fn llm_evaluation_snapshot_serializes_dimensions_and_evidence_summary() {
        let prepared = PreparedLlmEvaluation {
            provider_id: Some("provider-1".to_string()),
            chat_request: ChatRequest {
                messages: Vec::new(),
                tools: None,
                temperature: None,
                max_tokens: None,
            },
            target: InspectorTarget::native_scratch("scratch-1".to_string()),
            session_id: Some("session-1".to_string()),
            scenario: "Review this server".to_string(),
            tool_names: vec!["time_convert_time".to_string()],
            dimensions: vec!["compatibility".to_string(), "package_safety".to_string()],
            evidence_summary_count: 2,
            started_at: Instant::now(),
        };
        let provider = StoredLlmProvider {
            id: "provider-1".to_string(),
            name: "Local".to_string(),
            provider_type: "openai_chat".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model_id: "test-model".to_string(),
            secret_alias: None,
            default_params_json: None,
            is_default: true,
            created_at: None,
            updated_at: None,
        };
        let response = ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: "Compatibility is strong; package safety needs review.".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            usage: None,
        };
        let value = finish_llm_evaluation(prepared, provider, response).expect("serialize evaluation");

        assert_eq!(value["dimensions"], json!(["compatibility", "package_safety"]));
        assert_eq!(value["evidence_summary_count"], json!(2));
    }

    #[test]
    fn llm_tool_from_capability_maps_tool_metadata() {
        let item = json!({
            "name": "time_convert_time",
            "description": "Convert time between zones",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "timezone": { "type": "string" }
                }
            }
        });
        let tool = llm_tool_from_capability(&item).expect("tool metadata");

        assert_eq!(tool.name, "time_convert_time");
        assert_eq!(tool.description, "Convert time between zones");
        assert_eq!(tool.parameters["properties"]["timezone"]["type"], json!("string"));
    }

    #[test]
    fn llm_evaluation_snapshot_serializes_tool_calls_and_usage() {
        let prepared = PreparedLlmEvaluation {
            provider_id: Some("provider-1".to_string()),
            chat_request: ChatRequest {
                messages: Vec::new(),
                tools: None,
                temperature: None,
                max_tokens: None,
            },
            target: InspectorTarget::native_scratch("scratch-1".to_string()),
            session_id: Some("session-1".to_string()),
            scenario: "Convert time".to_string(),
            tool_names: vec!["time_convert_time".to_string()],
            dimensions: vec!["capability_surface".to_string()],
            evidence_summary_count: 0,
            started_at: Instant::now(),
        };
        let provider = StoredLlmProvider {
            id: "provider-1".to_string(),
            name: "Local".to_string(),
            provider_type: "openai_chat".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model_id: "test-model".to_string(),
            secret_alias: None,
            default_params_json: None,
            is_default: true,
            created_at: None,
            updated_at: None,
        };
        let response = ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: "Use time_convert_time.".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            usage: Some(mcpmate_llm::TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };
        let value = finish_llm_evaluation(prepared, provider, response).expect("serialize evaluation");

        assert_eq!(value["target"]["mode"], json!("native"));
        assert_eq!(value["target"]["scratch_id"], json!("scratch-1"));
        assert_eq!(value["provider"]["id"], json!("provider-1"));
        assert_eq!(value["provider"]["model_id"], json!("test-model"));
        assert_eq!(value["tool_count"], json!(1));
        assert_eq!(value["usage"]["total_tokens"], json!(15));
    }

    fn stdio_config(
        command: &str,
        args: &[&str],
    ) -> MCPServerConfig {
        MCPServerConfig {
            kind: crate::common::server::ServerType::Stdio,
            command: Some(command.to_string()),
            args: Some(args.iter().map(|arg| (*arg).to_string()).collect()),
            url: None,
            env: None,
            headers: None,
        }
    }

    fn compatibility_requirement_status_value<'a>(
        value: &'a Value,
        id: &str,
    ) -> Option<&'a str> {
        value["requirements"]
            .as_array()?
            .iter()
            .find(|requirement| requirement["id"] == json!(id))?
            .get("status")?
            .as_str()
    }

    fn expect_bad_request<T>(
        result: Result<T, ApiError>,
        message: &str,
    ) -> ApiError {
        match result {
            Ok(_) => panic!("{message}"),
            Err(error) => error,
        }
    }
}
