use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rmcp::{
    RoleClient,
    model::{
        CancelledNotification, CancelledNotificationParam, LoggingMessageNotificationParam, ProgressNotificationParam,
        ProgressToken, RequestId, ServerResult,
    },
    service::{RequestHandle, ServiceError},
};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock, broadcast, mpsc, oneshot};

use crate::api::models::inspector::InspectorMode;

const BROADCAST_BUFFER: usize = 64;
const CANCEL_BUFFER: usize = 4;

fn logging_level_to_str(level: &rmcp::model::LoggingLevel) -> &'static str {
    match level {
        rmcp::model::LoggingLevel::Debug => "debug",
        rmcp::model::LoggingLevel::Info => "info",
        rmcp::model::LoggingLevel::Notice => "notice",
        rmcp::model::LoggingLevel::Warning => "warning",
        rmcp::model::LoggingLevel::Error => "error",
        rmcp::model::LoggingLevel::Critical => "critical",
        rmcp::model::LoggingLevel::Alert => "alert",
        rmcp::model::LoggingLevel::Emergency => "emergency",
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum InspectorEvent {
    Started {
        call_id: String,
        server_id: String,
        mode: InspectorMode,
        session_id: Option<String>,
        started_at_epoch_ms: u128,
    },
    Progress {
        call_id: String,
        progress: f64,
        total: Option<f64>,
        message: Option<String>,
    },
    Log {
        call_id: String,
        level: Option<String>,
        logger: Option<String>,
        data: Value,
    },
    Result {
        call_id: String,
        server_id: String,
        elapsed_ms: u64,
        result: Value,
    },
    Error {
        call_id: String,
        server_id: String,
        message: String,
    },
    Cancelled {
        call_id: String,
        server_id: String,
        reason: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum InspectorTerminal {
    Result {
        result: Value,
        elapsed_ms: u64,
        server_id: String,
    },
    Error {
        message: String,
        server_id: String,
    },
    Cancelled {
        reason: Option<String>,
        server_id: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectorCallInfo {
    pub call_id: String,
    pub server_id: String,
    pub mode: InspectorMode,
    pub session_id: Option<String>,
    pub request_id: String,
    pub progress_token: String,
}

pub struct RegisteredCall {
    pub info: InspectorCallInfo,
    pub completion: oneshot::Receiver<InspectorTerminal>,
}

struct CallEntry {
    call_id: String,
    server_id: String,
    progress_token: ProgressToken,
    request_id: RequestId,
    started_at: Instant,
    started_at_system: SystemTime,
    tx: broadcast::Sender<InspectorEvent>,
    cancel_tx: mpsc::Sender<CancelCommand>,
    completion_tx: Mutex<Option<oneshot::Sender<InspectorTerminal>>>,
}

enum CancelCommand {
    External(Option<String>),
}

#[derive(Default)]
struct InnerRegistry {
    calls: RwLock<HashMap<String, Arc<CallEntry>>>,
    progress_index: RwLock<HashMap<String, String>>, // progress_token -> call_id
    request_index: RwLock<HashMap<String, String>>,  // request_id -> call_id
    completed: RwLock<HashMap<String, CompletedCall>>,
}

#[derive(Clone)]
struct CompletedCall {
    event: InspectorEvent,
    expires_at: Instant,
}

#[derive(Clone, Default)]
pub struct InspectorCallRegistry {
    inner: Arc<InnerRegistry>,
}

pub enum CallSubscription {
    Active(broadcast::Receiver<InspectorEvent>),
    Completed(InspectorEvent),
}

impl InspectorCallRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start_call(
        &self,
        call_id: String,
        server_id: String,
        mode: InspectorMode,
        session_id: Option<String>,
        handle: RequestHandle<RoleClient>,
    ) -> RegisteredCall {
        let progress_key = token_key(&handle.progress_token);
        let request_key = request_key(&handle.id);
        let (tx, _) = broadcast::channel(BROADCAST_BUFFER);
        let (cancel_tx, cancel_rx) = mpsc::channel(CANCEL_BUFFER);
        let (completion_tx, completion_rx) = oneshot::channel();

        let entry = Arc::new(CallEntry {
            call_id: call_id.clone(),
            server_id: server_id.clone(),
            progress_token: handle.progress_token.clone(),
            request_id: handle.id.clone(),
            started_at: Instant::now(),
            started_at_system: SystemTime::now(),
            tx: tx.clone(),
            cancel_tx,
            completion_tx: Mutex::new(Some(completion_tx)),
        });

        {
            let mut calls = self.inner.calls.write().await;
            calls.insert(call_id.clone(), entry.clone());
        }
        {
            let mut idx = self.inner.progress_index.write().await;
            idx.insert(progress_key, call_id.clone());
        }
        {
            let mut idx = self.inner.request_index.write().await;
            idx.insert(request_key, call_id.clone());
        }

        // emit started event immediately
        let started_epoch = entry
            .started_at_system
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        let _ = entry.tx.send(InspectorEvent::Started {
            call_id: call_id.clone(),
            server_id: server_id.clone(),
            mode,
            session_id: session_id.clone(),
            started_at_epoch_ms: started_epoch,
        });

        // Spawn worker task to await response / cancel
        tokio::spawn(call_worker(self.clone(), entry.clone(), handle, cancel_rx));

        let info = InspectorCallInfo {
            call_id,
            server_id,
            mode,
            session_id,
            request_id: request_key_from_request_id(entry.request_id.clone()),
            progress_token: progress_token_to_string(&entry.progress_token),
        };

        RegisteredCall {
            info,
            completion: completion_rx,
        }
    }

    pub async fn subscribe(
        &self,
        call_id: &str,
    ) -> Option<CallSubscription> {
        self.purge_completed().await;

        if let Some(entry) = self.inner.calls.read().await.get(call_id).cloned() {
            return Some(CallSubscription::Active(entry.tx.subscribe()));
        }

        self.inner
            .completed
            .read()
            .await
            .get(call_id)
            .cloned()
            .map(|completed| CallSubscription::Completed(completed.event))
    }

    pub async fn cancel_call(
        &self,
        call_id: &str,
        reason: Option<String>,
    ) -> Result<(), String> {
        let entry = {
            let calls = self.inner.calls.read().await;
            calls.get(call_id).cloned()
        }
        .ok_or_else(|| "Call not found".to_string())?;

        entry
            .cancel_tx
            .send(CancelCommand::External(reason))
            .await
            .map_err(|_| "Call already finished".to_string())
    }

    pub async fn emit_progress(
        &self,
        params: &ProgressNotificationParam,
    ) {
        if let Some(call_id) = self
            .inner
            .progress_index
            .read()
            .await
            .get(&token_key(&params.progress_token))
            .cloned()
        {
            if let Some(entry) = self.inner.calls.read().await.get(&call_id).cloned() {
                let _ = entry.tx.send(InspectorEvent::Progress {
                    call_id,
                    progress: params.progress,
                    total: params.total,
                    message: params.message.clone(),
                });
            }
        }
    }

    pub async fn emit_log(
        &self,
        token: Option<&ProgressToken>,
        params: &LoggingMessageNotificationParam,
    ) {
        let Some(token) = token else {
            return;
        };
        if let Some(call_id) = self.inner.progress_index.read().await.get(&token_key(token)).cloned() {
            if let Some(entry) = self.inner.calls.read().await.get(&call_id).cloned() {
                let data = serde_json::to_value(&params.data).unwrap_or(Value::Null);
                let level = Some(logging_level_to_str(&params.level).to_string());
                let _ = entry.tx.send(InspectorEvent::Log {
                    call_id,
                    level,
                    logger: params.logger.clone(),
                    data,
                });
            }
        }
    }

    pub async fn emit_cancelled(
        &self,
        request_id: &RequestId,
        reason: Option<String>,
    ) {
        let maybe_entry = {
            let calls = self.inner.calls.read().await;
            let req_idx = self.inner.request_index.read().await;
            req_idx
                .get(&request_key(request_id))
                .and_then(|id| calls.get(id).cloned())
        };

        if let Some(entry) = maybe_entry {
            self.finish_call(
                &entry.call_id,
                InspectorTerminal::Cancelled {
                    reason,
                    server_id: entry.server_id.clone(),
                },
            )
            .await;
        }
    }

    async fn finish_call(
        &self,
        call_id: &str,
        terminal: InspectorTerminal,
    ) {
        let entry = {
            let mut progress_idx = self.inner.progress_index.write().await;
            let mut request_idx = self.inner.request_index.write().await;
            let mut calls = self.inner.calls.write().await;

            if let Some(entry) = calls.remove(call_id) {
                progress_idx.remove(&token_key(&entry.progress_token));
                request_idx.remove(&request_key(&entry.request_id));
                Some(entry)
            } else {
                None
            }
        };

        if let Some(entry) = entry {
            let terminal_event = match &terminal {
                InspectorTerminal::Result {
                    result,
                    elapsed_ms,
                    server_id,
                } => InspectorEvent::Result {
                    call_id: entry.call_id.clone(),
                    server_id: server_id.clone(),
                    elapsed_ms: *elapsed_ms,
                    result: result.clone(),
                },
                InspectorTerminal::Error { message, server_id } => InspectorEvent::Error {
                    call_id: entry.call_id.clone(),
                    server_id: server_id.clone(),
                    message: message.clone(),
                },
                InspectorTerminal::Cancelled { reason, server_id } => InspectorEvent::Cancelled {
                    call_id: entry.call_id.clone(),
                    server_id: server_id.clone(),
                    reason: reason.clone(),
                },
            };

            let _ = entry.tx.send(terminal_event.clone());

            if let Some(tx) = entry.completion_tx.lock().await.take() {
                let _ = tx.send(terminal);
            }

            self.inner.completed.write().await.insert(
                entry.call_id.clone(),
                CompletedCall {
                    event: terminal_event,
                    expires_at: Instant::now() + Duration::from_secs(30),
                },
            );
        }
    }

    async fn purge_completed(&self) {
        let mut completed = self.inner.completed.write().await;
        let now = Instant::now();
        completed.retain(|_, entry| entry.expires_at > now);
    }
}

fn token_key(token: &ProgressToken) -> String {
    match &token.0 {
        rmcp::model::NumberOrString::Number(n) => n.to_string(),
        rmcp::model::NumberOrString::String(s) => s.to_string(),
    }
}

fn progress_token_to_string(token: &ProgressToken) -> String {
    token_key(token)
}

fn request_key(request_id: &RequestId) -> String {
    match request_id {
        RequestId::Number(n) => n.to_string(),
        RequestId::String(s) => s.to_string(),
    }
}

fn request_key_from_request_id(request_id: RequestId) -> String {
    request_key(&request_id)
}

async fn call_worker(
    registry: InspectorCallRegistry,
    entry: Arc<CallEntry>,
    handle: RequestHandle<RoleClient>,
    mut cancel_rx: mpsc::Receiver<CancelCommand>,
) {
    let started_at = entry.started_at;
    let server_id = entry.server_id.clone();
    let call_id = entry.call_id.clone();
    let request_id = handle.id.clone();
    let peer = handle.peer.clone();
    let response_fut = handle.await_response();
    tokio::pin!(response_fut);

    let terminal = tokio::select! {
        cmd = cancel_rx.recv() => {
            let reason = match cmd {
                Some(CancelCommand::External(reason)) => reason,
                None => None,
            };

            let cancel_notification = CancelledNotification {
                params: CancelledNotificationParam {
                    request_id: request_id.clone(),
                    reason: reason.clone(),
                },
                method: rmcp::model::CancelledNotificationMethod,
                extensions: Default::default(),
            };
            let _ = peer.send_notification(cancel_notification.into()).await;

            InspectorTerminal::Cancelled { reason, server_id }
        }
        resp = &mut response_fut => {
            match resp {
                Ok(ServerResult::CallToolResult(res)) => {
                    let value = serde_json::to_value(res).unwrap_or(Value::Null);
                    let elapsed_ms = started_at.elapsed().as_millis() as u64;
                    InspectorTerminal::Result { result: value, elapsed_ms, server_id }
                }
                Ok(other) => {
                    let msg = format!("Unexpected server result: {:?}", other);
                    InspectorTerminal::Error { message: msg, server_id }
                }
                Err(ServiceError::Timeout { .. }) => {
                    InspectorTerminal::Error {
                        message: "Request timed out".to_string(),
                        server_id,
                    }
                }
                Err(e) => {
                    InspectorTerminal::Error {
                        message: e.to_string(),
                        server_id,
                    }
                }
            }
        }
    };

    registry.finish_call(&call_id, terminal).await;
}
