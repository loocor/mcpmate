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
    started_at: Instant,
}

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
    capabilities: InspectorCompatibilityCapabilities,
    elapsed_ms: u64,
}

#[derive(Serialize)]
struct InspectorCompatibilityCapabilities {
    counts: InspectorCompatibilityCounts,
    checks: Vec<InspectorCompatibilityCheck>,
}

#[derive(Serialize)]
struct InspectorCompatibilityCounts {
    tools: usize,
    prompts: usize,
    resources: usize,
    resource_templates: usize,
}

#[derive(Serialize)]
struct InspectorCompatibilityCheck {
    id: &'static str,
    status: &'static str,
    observed_count: usize,
}

#[derive(Serialize)]
struct InspectorPackageSafetySnapshot {
    target: InspectorProductSnapshotTarget,
    scanner: InspectorPackageSafetyScanner,
    inventory: InspectorPackageSafetyInventory,
    findings: Vec<Value>,
    elapsed_ms: u64,
}

#[derive(Serialize)]
struct InspectorPackageSafetyScanner {
    provider: Option<String>,
    status: &'static str,
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
            let snapshot = InspectorCompatibilitySnapshot {
                target: native_product_snapshot_target(&native_target, Some(target_id), transport, request.session_id),
                capabilities,
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
            let snapshot = InspectorCompatibilitySnapshot {
                target: proxy_product_snapshot_target(&target, request.session_id),
                capabilities,
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
    let inventory = package_safety_inventory(&target_config.config);
    let snapshot = InspectorPackageSafetySnapshot {
        target: native_product_snapshot_target(&native_target, Some(target_id), transport, None),
        scanner: InspectorPackageSafetyScanner {
            provider: None,
            status: "not_configured",
        },
        inventory,
        findings: Vec::new(),
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
    let chat_request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: Role::System,
                content: "You evaluate MCP server tool surfaces. Use the provided tool definitions to decide whether the server can satisfy the user scenario. Prefer a tool call when one tool clearly matches. If not, explain the gap concisely.".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: Role::User,
                content: format!(
                    "Scenario:\n{scenario}\n\nReturn a concise evaluation with the best matching tool, confidence, missing context, and any schema risks."
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
    let peer = Some(runtime.peer);
    let runtime_owner = Some(runtime.owner);

    Ok(context
        .sessions()
        .open_session(session_id.clone(), target, peer, runtime_owner)
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
) -> Result<InspectorCompatibilityCapabilities, ApiError> {
    let tools = runtime::list_tools(peer).await?;
    let prompts = runtime::list_prompts(peer).await?;
    let resources = runtime::list_resources(peer).await?;
    let resource_templates = runtime::list_resource_templates(peer).await?;

    Ok(InspectorCompatibilityCapabilities {
        counts: InspectorCompatibilityCounts {
            tools: tools.len(),
            prompts: prompts.len(),
            resources: resources.len(),
            resource_templates: resource_templates.len(),
        },
        checks: vec![
            compatibility_check("tools_list", tools.len()),
            compatibility_check("prompts_list", prompts.len()),
            compatibility_check("resources_list", resources.len()),
            compatibility_check("resource_templates_list", resource_templates.len()),
        ],
    })
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
}
