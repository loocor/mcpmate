use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SseEventKind {
    Progress,
    Partial,
    Log,
    Result,
    Error,
    Cancelled,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub event: SseEventKind,
    pub call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for SseEventKind {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let s = match self {
            SseEventKind::Progress => "progress",
            SseEventKind::Partial => "partial",
            SseEventKind::Log => "log",
            SseEventKind::Result => "result",
            SseEventKind::Error => "error",
            SseEventKind::Cancelled => "cancelled",
            SseEventKind::Heartbeat => "heartbeat",
        };
        f.write_str(s)
    }
}
