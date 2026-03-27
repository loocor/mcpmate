use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    audit::{
        AuditAction, AuditCategory, AuditEventDto, AuditRetentionPolicy, AuditRetentionPolicySetting, AuditStatus,
    },
    macros::resp::api_resp,
};

#[derive(Debug, Clone, Deserialize, JsonSchema, Default)]
pub struct AuditListReq {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub category: Option<AuditCategory>,
    pub action: Option<AuditAction>,
    pub status: Option<AuditStatus>,
    pub actor: Option<String>,
    pub client_id: Option<String>,
    pub profile_id: Option<String>,
    pub server_id: Option<String>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub task_id: Option<String>,
    pub related_task_id: Option<String>,
    pub progress_token: Option<String>,
    pub from_occurred_at_ms: Option<i64>,
    pub to_occurred_at_ms: Option<i64>,
}

impl AuditListReq {
    pub fn into_filter(self) -> crate::audit::AuditFilter {
        crate::audit::AuditFilter {
            category: self.category,
            action: self.action,
            status: self.status,
            actor: self.actor,
            client_id: self.client_id,
            profile_id: self.profile_id,
            server_id: self.server_id,
            session_id: self.session_id,
            request_id: self.request_id,
            task_id: self.task_id,
            related_task_id: self.related_task_id,
            progress_token: self.progress_token,
            from_occurred_at_ms: self.from_occurred_at_ms,
            to_occurred_at_ms: self.to_occurred_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AuditListData {
    pub events: Vec<AuditEventDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

api_resp!(AuditListResp, AuditListData, "Audit list response");

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AuditPolicyData {
    pub policy: AuditRetentionPolicy,
    pub sweep_interval_secs: u64,
}

impl From<AuditRetentionPolicySetting> for AuditPolicyData {
    fn from(setting: AuditRetentionPolicySetting) -> Self {
        Self {
            policy: setting.policy,
            sweep_interval_secs: setting.sweep_interval_secs,
        }
    }
}

api_resp!(AuditPolicyResp, AuditPolicyData, "Audit retention policy response");

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AuditPolicySetReq {
    pub policy: AuditRetentionPolicy,
    #[serde(default = "default_sweep_interval_secs")]
    pub sweep_interval_secs: u64,
}

fn default_sweep_interval_secs() -> u64 {
    3600
}

impl From<AuditPolicySetReq> for AuditRetentionPolicySetting {
    fn from(req: AuditPolicySetReq) -> Self {
        Self {
            policy: req.policy,
            sweep_interval_secs: req.sweep_interval_secs,
        }
    }
}
