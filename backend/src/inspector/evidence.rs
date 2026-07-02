use serde::Serialize;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::inspector::contract::{InspectorMode, InspectorProxyMode, InspectorProxyScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorOperationKind {
    ToolCall,
    CapabilityList,
    PromptGet,
    ResourceRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorOperationStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorEvidenceLayer {
    Platform,
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectorOperationRecord {
    pub operation_id: String,
    pub kind: InspectorOperationKind,
    pub status: InspectorOperationStatus,
    pub server_id: String,
    pub mode: InspectorMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub request_id: String,
    pub progress_token: String,
    pub started_at_epoch_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at_epoch_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectorEvidenceRow {
    pub layer: InspectorEvidenceLayer,
    pub kind: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectorEvidenceEvent {
    pub sequence: u64,
    pub layer: InspectorEvidenceLayer,
    pub event: String,
    pub occurred_at_epoch_ms: u128,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectorEvidenceSnapshot {
    pub operation: InspectorOperationRecord,
    pub platform_rows: Vec<InspectorEvidenceRow>,
    pub mcp_rows: Vec<InspectorEvidenceRow>,
    pub events: Vec<InspectorEvidenceEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
}

pub struct InspectorSyncEvidenceInput {
    pub operation_id: String,
    pub kind: InspectorOperationKind,
    pub server_id: String,
    pub mode: InspectorMode,
    pub session_id: Option<String>,
    pub request_id: String,
    pub progress_token: String,
    pub platform_kind: &'static str,
    pub platform_payload: Value,
    pub mcp_kind: &'static str,
    pub mcp_payload: Value,
    pub started_at_epoch_ms: u128,
    pub completed_at_epoch_ms: u128,
    pub elapsed_ms: u64,
    pub response: Option<Value>,
}

pub struct InspectorResponseEvidenceInput {
    pub response_kind: InspectorResponseKind,
    pub mode: InspectorMode,
    pub session_id: Option<String>,
    pub server_id: Option<String>,
    pub platform_payload: Value,
    pub mcp_payload: Value,
    pub started_at_epoch_ms: u128,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorResponseKind {
    PromptGet,
    ResourceRead,
}

pub struct InspectorCapabilityListEvidenceInput {
    pub capability_kind: &'static str,
    pub mode: InspectorMode,
    pub session_id: Option<String>,
    pub refresh: bool,
    pub proxy_mode: Option<InspectorProxyMode>,
    pub proxy_scope: Option<InspectorProxyScope>,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub scratch_id: Option<String>,
    pub meta: Vec<Value>,
    pub items: Vec<Value>,
    pub started_at_epoch_ms: u128,
    pub elapsed_ms: u64,
}

pub struct InspectorToolCallEvidenceInput {
    pub call_id: String,
    pub server_id: String,
    pub mode: InspectorMode,
    pub session_id: Option<String>,
    pub request_id: String,
    pub progress_token: String,
    pub status: InspectorOperationStatus,
    pub started_at_epoch_ms: u128,
    pub completed_at_epoch_ms: Option<u128>,
    pub elapsed_ms: Option<u64>,
    pub events: Vec<InspectorEvidenceEvent>,
    pub response: Option<Value>,
}

#[derive(Serialize)]
struct InspectorCapabilityListMcpPayload {
    capability_kind: &'static str,
    total: usize,
    items: Vec<Value>,
}

#[derive(Serialize)]
struct InspectorCapabilityListResponse {
    capability_kind: &'static str,
    total: usize,
}

impl InspectorEvidenceRow {
    pub fn platform(
        kind: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            layer: InspectorEvidenceLayer::Platform,
            kind: kind.into(),
            payload,
        }
    }

    pub fn mcp(
        kind: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            layer: InspectorEvidenceLayer::Mcp,
            kind: kind.into(),
            payload,
        }
    }
}

impl InspectorEvidenceEvent {
    pub fn new(
        sequence: u64,
        layer: InspectorEvidenceLayer,
        event: impl Into<String>,
        occurred_at_epoch_ms: u128,
        payload: Value,
    ) -> Self {
        Self {
            sequence,
            layer,
            event: event.into(),
            occurred_at_epoch_ms,
            payload,
        }
    }

    pub fn to_row(&self) -> InspectorEvidenceRow {
        InspectorEvidenceRow::from_event(self)
    }
}

impl InspectorEvidenceRow {
    pub fn from_event(event: &InspectorEvidenceEvent) -> Self {
        Self {
            layer: event.layer,
            kind: event.event.clone(),
            payload: event.payload.clone(),
        }
    }
}

fn partition_event_rows(events: &[InspectorEvidenceEvent]) -> (Vec<InspectorEvidenceRow>, Vec<InspectorEvidenceRow>) {
    let mut platform_rows = Vec::new();
    let mut mcp_rows = Vec::new();

    for event in events {
        let row = InspectorEvidenceRow::from_event(event);
        match event.layer {
            InspectorEvidenceLayer::Platform => platform_rows.push(row),
            InspectorEvidenceLayer::Mcp => mcp_rows.push(row),
        }
    }

    (platform_rows, mcp_rows)
}

impl InspectorEvidenceSnapshot {
    pub fn sync_success(input: InspectorSyncEvidenceInput) -> Self {
        let events = vec![
            InspectorEvidenceEvent::new(
                0,
                InspectorEvidenceLayer::Platform,
                input.platform_kind,
                input.started_at_epoch_ms,
                input.platform_payload.clone(),
            ),
            InspectorEvidenceEvent::new(
                1,
                InspectorEvidenceLayer::Mcp,
                input.mcp_kind,
                input.completed_at_epoch_ms,
                input.mcp_payload.clone(),
            ),
        ];

        let (platform_rows, mcp_rows) = partition_event_rows(&events);

        Self {
            operation: InspectorOperationRecord {
                operation_id: input.operation_id,
                kind: input.kind,
                status: InspectorOperationStatus::Succeeded,
                server_id: input.server_id,
                mode: input.mode,
                session_id: input.session_id,
                request_id: input.request_id,
                progress_token: input.progress_token,
                started_at_epoch_ms: input.started_at_epoch_ms,
                completed_at_epoch_ms: Some(input.completed_at_epoch_ms),
                elapsed_ms: Some(input.elapsed_ms),
            },
            platform_rows,
            mcp_rows,
            events,
            response: input.response,
        }
    }

    pub fn from_events(
        operation: InspectorOperationRecord,
        initial_platform_rows: Vec<InspectorEvidenceRow>,
        events: Vec<InspectorEvidenceEvent>,
        response: Option<Value>,
    ) -> Self {
        let (mut platform_rows, mcp_rows) = partition_event_rows(&events);
        platform_rows.splice(0..0, initial_platform_rows);

        Self {
            operation,
            platform_rows,
            mcp_rows,
            events,
            response,
        }
    }

    pub fn into_json_value(self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

impl InspectorResponseKind {
    fn operation_kind(self) -> InspectorOperationKind {
        match self {
            Self::PromptGet => InspectorOperationKind::PromptGet,
            Self::ResourceRead => InspectorOperationKind::ResourceRead,
        }
    }

    fn platform_kind(self) -> &'static str {
        match self {
            Self::PromptGet => "prompt_get",
            Self::ResourceRead => "resource_read",
        }
    }

    fn mcp_kind(self) -> &'static str {
        match self {
            Self::PromptGet => "prompt_result",
            Self::ResourceRead => "resource_result",
        }
    }
}

pub fn sync_response_json(input: InspectorResponseEvidenceInput) -> Result<Value, serde_json::Error> {
    let completed_at_epoch_ms = current_epoch_ms();
    let operation_id = crate::generate_id!("inspev");
    let platform_kind = input.response_kind.platform_kind();
    let request_id = format!("{}-request", platform_kind);
    let progress_token = format!("{}-snapshot", operation_id);
    let snapshot = InspectorEvidenceSnapshot::sync_success(InspectorSyncEvidenceInput {
        operation_id,
        kind: input.response_kind.operation_kind(),
        server_id: input.server_id.unwrap_or_else(|| "catalog".to_string()),
        mode: input.mode,
        session_id: input.session_id,
        request_id,
        progress_token,
        platform_kind,
        platform_payload: input.platform_payload,
        mcp_kind: input.response_kind.mcp_kind(),
        mcp_payload: input.mcp_payload.clone(),
        started_at_epoch_ms: input.started_at_epoch_ms,
        completed_at_epoch_ms,
        elapsed_ms: input.elapsed_ms,
        response: Some(input.mcp_payload),
    });

    snapshot.into_json_value()
}

pub fn capability_list_json(input: InspectorCapabilityListEvidenceInput) -> Result<Value, serde_json::Error> {
    let completed_at_epoch_ms = current_epoch_ms();
    let operation_id = crate::generate_id!("inspev");
    let request_id = format!("{}-list", input.capability_kind);
    let progress_token = format!("{}-snapshot", operation_id);
    let server_id = evidence_server_id(&input.meta).unwrap_or_else(|| "catalog".to_string());
    let total = input.items.len();
    let platform_payload = json!({
        "operation_id": operation_id,
        "capability_kind": input.capability_kind,
        "mode": input.mode,
        "session_id": input.session_id,
        "refresh": input.refresh,
        "proxy_mode": input.proxy_mode,
        "proxy_scope": input.proxy_scope,
        "server_id": input.server_id,
        "server_name": input.server_name,
        "scratch_id": input.scratch_id,
        "meta": input.meta,
    });
    let mcp_payload = serde_json::to_value(InspectorCapabilityListMcpPayload {
        capability_kind: input.capability_kind,
        total,
        items: input.items,
    })?;
    let response = serde_json::to_value(InspectorCapabilityListResponse {
        capability_kind: input.capability_kind,
        total,
    })?;

    let snapshot = InspectorEvidenceSnapshot::sync_success(InspectorSyncEvidenceInput {
        operation_id,
        kind: InspectorOperationKind::CapabilityList,
        server_id,
        mode: input.mode,
        session_id: input.session_id,
        request_id,
        progress_token,
        platform_kind: "request",
        platform_payload,
        mcp_kind: "capability_list",
        mcp_payload,
        started_at_epoch_ms: input.started_at_epoch_ms,
        completed_at_epoch_ms,
        elapsed_ms: input.elapsed_ms,
        response: Some(response),
    });

    snapshot.into_json_value()
}

pub fn tool_call_snapshot(input: InspectorToolCallEvidenceInput) -> InspectorEvidenceSnapshot {
    let operation = InspectorOperationRecord {
        operation_id: input.call_id.clone(),
        kind: InspectorOperationKind::ToolCall,
        status: input.status,
        server_id: input.server_id.clone(),
        mode: input.mode,
        session_id: input.session_id.clone(),
        request_id: input.request_id.clone(),
        progress_token: input.progress_token.clone(),
        started_at_epoch_ms: input.started_at_epoch_ms,
        completed_at_epoch_ms: input.completed_at_epoch_ms,
        elapsed_ms: input.elapsed_ms,
    };

    let platform_rows = vec![InspectorEvidenceRow::platform(
        "operation",
        json!({
            "call_id": input.call_id,
            "server_id": input.server_id,
            "mode": input.mode,
            "session_id": input.session_id,
            "request_id": input.request_id,
            "progress_token": input.progress_token,
        }),
    )];

    InspectorEvidenceSnapshot::from_events(operation, platform_rows, input.events, input.response)
}

fn evidence_server_id(meta: &[Value]) -> Option<String> {
    meta.iter()
        .filter_map(|entry| entry.get("server_id").and_then(Value::as_str))
        .next()
        .map(str::to_string)
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn evidence_snapshot_keeps_platform_and_mcp_rows_separate() {
        let snapshot = InspectorEvidenceSnapshot {
            operation: InspectorOperationRecord {
                operation_id: "call-1".to_string(),
                kind: InspectorOperationKind::ToolCall,
                status: InspectorOperationStatus::Succeeded,
                server_id: "server-1".to_string(),
                mode: InspectorMode::Native,
                session_id: None,
                request_id: "request-1".to_string(),
                progress_token: "progress-1".to_string(),
                started_at_epoch_ms: 100,
                completed_at_epoch_ms: Some(150),
                elapsed_ms: Some(50),
            },
            platform_rows: vec![InspectorEvidenceRow::platform("request", json!({"timeout_ms": 5000}))],
            mcp_rows: vec![InspectorEvidenceRow::mcp("result", json!({"content": []}))],
            events: vec![InspectorEvidenceEvent::new(
                0,
                InspectorEvidenceLayer::Platform,
                "started",
                100,
                json!({"call_id": "call-1"}),
            )],
            response: Some(json!({"content": []})),
        };

        assert_eq!(snapshot.platform_rows.len(), 1);
        assert_eq!(snapshot.platform_rows[0].layer, InspectorEvidenceLayer::Platform);
        assert_eq!(snapshot.mcp_rows.len(), 1);
        assert_eq!(snapshot.mcp_rows[0].layer, InspectorEvidenceLayer::Mcp);
        assert_eq!(snapshot.response, Some(json!({"content": []})));
    }

    #[test]
    fn sync_success_builds_matching_rows_and_events() {
        let snapshot = InspectorEvidenceSnapshot::sync_success(InspectorSyncEvidenceInput {
            operation_id: "op-1".to_string(),
            kind: InspectorOperationKind::PromptGet,
            server_id: "server-1".to_string(),
            mode: InspectorMode::Native,
            session_id: Some("session-1".to_string()),
            request_id: "prompt-request".to_string(),
            progress_token: "op-1-snapshot".to_string(),
            platform_kind: "prompt_get",
            platform_payload: json!({"name": "hello"}),
            mcp_kind: "prompt_result",
            mcp_payload: json!({"messages": []}),
            started_at_epoch_ms: 100,
            completed_at_epoch_ms: 140,
            elapsed_ms: 40,
            response: Some(json!({"messages": []})),
        });

        assert_eq!(snapshot.operation.kind, InspectorOperationKind::PromptGet);
        assert_eq!(snapshot.platform_rows[0].kind, "prompt_get");
        assert_eq!(snapshot.mcp_rows[0].kind, "prompt_result");
        assert_eq!(snapshot.events.len(), 2);
        assert_eq!(snapshot.events[0].layer, InspectorEvidenceLayer::Platform);
        assert_eq!(snapshot.events[1].layer, InspectorEvidenceLayer::Mcp);
        assert_eq!(snapshot.response, Some(json!({"messages": []})));
    }

    #[test]
    fn sync_success_rows_are_derived_from_events() {
        let snapshot = InspectorEvidenceSnapshot::sync_success(InspectorSyncEvidenceInput {
            operation_id: "op-1".to_string(),
            kind: InspectorOperationKind::PromptGet,
            server_id: "server-1".to_string(),
            mode: InspectorMode::Native,
            session_id: None,
            request_id: "prompt-request".to_string(),
            progress_token: "op-1-snapshot".to_string(),
            platform_kind: "prompt_get",
            platform_payload: json!({"name": "hello"}),
            mcp_kind: "prompt_result",
            mcp_payload: json!({"messages": []}),
            started_at_epoch_ms: 100,
            completed_at_epoch_ms: 140,
            elapsed_ms: 40,
            response: Some(json!({"messages": []})),
        });

        let expected_platform_row = InspectorEvidenceRow::from_event(&snapshot.events[0]);
        let expected_mcp_row = InspectorEvidenceRow::from_event(&snapshot.events[1]);

        assert_eq!(snapshot.platform_rows, vec![expected_platform_row]);
        assert_eq!(snapshot.mcp_rows, vec![expected_mcp_row]);
    }

    #[test]
    fn sync_response_json_uses_mcp_payload_as_response() {
        let snapshot = sync_response_json(InspectorResponseEvidenceInput {
            response_kind: InspectorResponseKind::ResourceRead,
            mode: InspectorMode::Native,
            session_id: None,
            server_id: Some("server-1".to_string()),
            platform_payload: json!({"uri": "file:///hello.txt"}),
            mcp_payload: json!({"contents": []}),
            started_at_epoch_ms: 100,
            elapsed_ms: 25,
        })
        .expect("response evidence");

        assert_eq!(snapshot["operation"]["kind"], "resource_read");
        assert_eq!(snapshot["operation"]["server_id"], "server-1");
        assert_eq!(snapshot["platform_rows"][0]["layer"], "platform");
        assert_eq!(snapshot["mcp_rows"][0]["layer"], "mcp");
        assert_eq!(snapshot["response"], json!({"contents": []}));
    }

    #[test]
    fn sync_response_json_maps_prompt_response_kind() {
        let snapshot = sync_response_json(InspectorResponseEvidenceInput {
            response_kind: InspectorResponseKind::PromptGet,
            mode: InspectorMode::Proxy,
            session_id: Some("session-1".to_string()),
            server_id: Some("server-1".to_string()),
            platform_payload: json!({"name": "hello_prompt"}),
            mcp_payload: json!({"messages": []}),
            started_at_epoch_ms: 100,
            elapsed_ms: 25,
        })
        .expect("response evidence");

        assert_eq!(snapshot["operation"]["kind"], "prompt_get");
        assert_eq!(snapshot["platform_rows"][0]["kind"], "prompt_get");
        assert_eq!(snapshot["mcp_rows"][0]["kind"], "prompt_result");
        assert_eq!(snapshot["response"], json!({"messages": []}));
    }

    #[test]
    fn capability_list_json_keeps_platform_mcp_and_summary() {
        let snapshot = capability_list_json(InspectorCapabilityListEvidenceInput {
            capability_kind: "tools",
            mode: InspectorMode::Proxy,
            session_id: Some("session-1".to_string()),
            refresh: true,
            proxy_mode: None,
            proxy_scope: None,
            server_id: Some("server-1".to_string()),
            server_name: None,
            scratch_id: None,
            meta: vec![json!({"server_id": "server-1", "source": "direct_proxy"})],
            items: vec![json!({"name": "echo"})],
            started_at_epoch_ms: 100,
            elapsed_ms: 35,
        })
        .expect("capability evidence");

        assert_eq!(snapshot["operation"]["kind"], "capability_list");
        assert_eq!(snapshot["operation"]["server_id"], "server-1");
        assert_eq!(snapshot["platform_rows"][0]["kind"], "request");
        assert_eq!(snapshot["mcp_rows"][0]["kind"], "capability_list");
        assert_eq!(snapshot["mcp_rows"][0]["payload"]["total"], 1);
        assert_eq!(snapshot["events"][0]["layer"], "platform");
        assert_eq!(snapshot["events"][1]["layer"], "mcp");
        assert_eq!(snapshot["response"], json!({"capability_kind": "tools", "total": 1}));
    }

    #[test]
    fn tool_call_snapshot_builds_operation_row_and_event_layers() {
        let snapshot = tool_call_snapshot(InspectorToolCallEvidenceInput {
            call_id: "call-1".to_string(),
            server_id: "server-1".to_string(),
            mode: InspectorMode::Native,
            session_id: Some("session-1".to_string()),
            request_id: "request-1".to_string(),
            progress_token: "progress-1".to_string(),
            status: InspectorOperationStatus::Succeeded,
            started_at_epoch_ms: 100,
            completed_at_epoch_ms: Some(150),
            elapsed_ms: Some(50),
            events: vec![InspectorEvidenceEvent::new(
                0,
                InspectorEvidenceLayer::Mcp,
                "result",
                150,
                json!({"event": "result"}),
            )],
            response: Some(json!({"content": []})),
        });

        assert_eq!(snapshot.operation.operation_id, "call-1");
        assert_eq!(snapshot.operation.kind, InspectorOperationKind::ToolCall);
        assert_eq!(snapshot.operation.status, InspectorOperationStatus::Succeeded);
        assert_eq!(snapshot.platform_rows[0].kind, "operation");
        assert_eq!(snapshot.mcp_rows[0].kind, "result");
        assert_eq!(snapshot.response, Some(json!({"content": []})));
    }
}
