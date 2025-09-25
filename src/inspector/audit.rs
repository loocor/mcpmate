use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditCallRecord {
    pub call_id: String,
    pub mode: String,
    pub capability: String,
    pub action: String,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub target: Option<String>,
    pub args_hash: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub elapsed_ms: Option<u64>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub result_size_bytes: Option<u64>,
}

impl AuditCallRecord { pub fn new(call_id: &str) -> Self { Self { call_id: call_id.to_string(), mode: "proxy".into(), capability: String::new(), action: String::new(), server_id: None, server_name: None, target: None, args_hash: None, started_at: Utc::now(), finished_at: None, elapsed_ms: None, status: "pending".into(), error_code: None, error_message: None, result_size_bytes: None } } }

