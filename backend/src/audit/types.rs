use std::collections::BTreeMap;

use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const MAX_STRING_LEN: usize = 1024;
pub const MAX_TEXT_LEN: usize = 8192;
pub const MAX_MAP_SIZE: usize = 100;
const REDACTED_VALUE: &str = "[REDACTED]";
const REDACTION_KEYWORDS: [&str; 6] = ["password", "token", "secret", "api_key", "auth", "credential"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    McpRequest,
    ServerConfig,
    ProfileConfig,
    ClientConfig,
    RuntimeControl,
    CapabilityControl,
    Management,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    Success,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Initialize,
    ToolsList,
    ToolsCall,
    ResourcesList,
    ResourcesRead,
    PromptsList,
    PromptsGet,
    NotificationProgress,
    NotificationCancelled,
    LoggingSetLevel,
    NotificationMessage,
    ServerCreate,
    ServerImport,
    ServerUpdate,
    ServerDelete,
    ServerEnable,
    ServerDisable,
    ProfileCreate,
    ProfileUpdate,
    ProfileDelete,
    ProfileActivate,
    ProfileDeactivate,
    ClientManageEnable,
    ClientManageDisable,
    ClientSettingsUpdate,
    ClientConfigApply,
    ClientConfigRestore,
    ClientConfigImport,
    ClientCapabilityUpdate,
    ClientDelete,
    ClientBackupDelete,
    ClientBackupPolicyUpdate,
    ClientApprove,
    ClientReject,
    ClientSuspend,
    OnboardingPolicyUpdate,
    FirstContactBehaviorUpdate,
    CoreSourceApply,
    LocalCoreServiceStart,
    LocalCoreServiceRestart,
    LocalCoreServiceStop,
    LocalCoreServiceInstall,
    LocalCoreServiceUninstall,
    DesktopManagedCoreStart,
    DesktopManagedCoreRestart,
    DesktopManagedCoreStop,
    CapabilityGrant,
    CapabilityRevoke,
    // Profile server management
    ProfileServerEnable,
    ProfileServerDisable,
    ProfileServerRemove,
    // Server instance management
    ServerInstanceDisconnect,
    ServerInstanceForceDisconnect,
    ServerInstanceReconnect,
    ServerInstanceResetReconnect,
    ServerInstanceRecover,
    ServerInstanceCancel,
    // Server capability cache
    ServerCacheReset,
    // Runtime management
    RuntimeInstall,
    RuntimeCacheReset,
    // Audit configuration
    AuditPolicyUpdate,
}

impl AuditAction {
    pub fn category(self) -> AuditCategory {
        match self {
            Self::Initialize
            | Self::ToolsList
            | Self::ToolsCall
            | Self::ResourcesList
            | Self::ResourcesRead
            | Self::PromptsList
            | Self::PromptsGet
            | Self::NotificationProgress
            | Self::NotificationCancelled
            | Self::LoggingSetLevel
            | Self::NotificationMessage => AuditCategory::McpRequest,
            Self::ServerCreate
            | Self::ServerImport
            | Self::ServerUpdate
            | Self::ServerDelete
            | Self::ServerEnable
            | Self::ServerDisable => AuditCategory::ServerConfig,
            Self::ProfileCreate
            | Self::ProfileUpdate
            | Self::ProfileDelete
            | Self::ProfileActivate
            | Self::ProfileDeactivate => AuditCategory::ProfileConfig,
            Self::ClientManageEnable
            | Self::ClientManageDisable
            | Self::ClientSettingsUpdate
            | Self::ClientConfigApply
            | Self::ClientConfigRestore
            | Self::ClientConfigImport
            | Self::ClientCapabilityUpdate
            | Self::ClientDelete
            | Self::ClientBackupDelete
            | Self::ClientBackupPolicyUpdate
            | Self::ClientApprove
            | Self::ClientReject
            | Self::ClientSuspend => AuditCategory::ClientConfig,
            Self::CoreSourceApply
            | Self::LocalCoreServiceStart
            | Self::LocalCoreServiceRestart
            | Self::LocalCoreServiceStop
            | Self::LocalCoreServiceInstall
            | Self::LocalCoreServiceUninstall
            | Self::DesktopManagedCoreStart
            | Self::DesktopManagedCoreRestart
            | Self::DesktopManagedCoreStop
            | Self::RuntimeInstall
            | Self::RuntimeCacheReset
            | Self::AuditPolicyUpdate
            | Self::OnboardingPolicyUpdate
            | Self::FirstContactBehaviorUpdate => AuditCategory::Management,
            Self::CapabilityGrant | Self::CapabilityRevoke => AuditCategory::ProfileConfig,
            Self::ProfileServerEnable | Self::ProfileServerDisable | Self::ProfileServerRemove => {
                AuditCategory::ProfileConfig
            }
            Self::ServerInstanceDisconnect
            | Self::ServerInstanceForceDisconnect
            | Self::ServerInstanceReconnect
            | Self::ServerInstanceResetReconnect
            | Self::ServerInstanceRecover
            | Self::ServerInstanceCancel
            | Self::ServerCacheReset => AuditCategory::ServerConfig,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AuditEventDto {
    pub id: Option<i64>,
    pub category: AuditCategory,
    pub action: AuditAction,
    pub status: AuditStatus,
    pub occurred_at_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<String>,
}

impl Default for AuditEventDto {
    fn default() -> Self {
        Self {
            id: None,
            category: AuditCategory::Management,
            action: AuditAction::AuditPolicyUpdate,
            status: AuditStatus::Success,
            occurred_at_ms: 0,
            actor: None,
            request_id: None,
            client_id: None,
            client_name: None,
            profile_id: None,
            profile_name: None,
            server_id: None,
            server_name: None,
            session_id: None,
            protocol_version: None,
            http_method: None,
            route: None,
            mcp_method: None,
            target: None,
            direction: None,
            error_code: None,
            error_message: None,
            detail: None,
            duration_ms: None,
            data: None,
            task_id: None,
            related_task_id: None,
            progress_token: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    dto: AuditEventDto,
}

impl AuditEvent {
    pub fn new(
        action: AuditAction,
        status: AuditStatus,
    ) -> Self {
        Self {
            dto: AuditEventDto {
                id: None,
                category: action.category(),
                action,
                status,
                occurred_at_ms: Utc::now().timestamp_millis(),
                ..AuditEventDto::default()
            },
        }
    }

    pub fn with_actor(
        mut self,
        actor: impl Into<String>,
    ) -> Self {
        self.dto.actor = sanitize_optional_string(Some(actor.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_request_id(
        mut self,
        request_id: impl Into<String>,
    ) -> Self {
        self.dto.request_id = sanitize_optional_string(Some(request_id.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_client_id(
        mut self,
        client_id: impl Into<String>,
    ) -> Self {
        self.dto.client_id = sanitize_optional_string(Some(client_id.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_profile_id(
        mut self,
        profile_id: impl Into<String>,
    ) -> Self {
        self.dto.profile_id = sanitize_optional_string(Some(profile_id.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_server_id(
        mut self,
        server_id: impl Into<String>,
    ) -> Self {
        self.dto.server_id = sanitize_optional_string(Some(server_id.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_session_id(
        mut self,
        session_id: impl Into<String>,
    ) -> Self {
        self.dto.session_id = sanitize_optional_string(Some(session_id.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_protocol_version(
        mut self,
        protocol_version: impl Into<String>,
    ) -> Self {
        self.dto.protocol_version = sanitize_optional_string(Some(protocol_version.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_http_route(
        mut self,
        method: impl Into<String>,
        route: impl Into<String>,
    ) -> Self {
        self.dto.http_method = sanitize_optional_string(Some(method.into()), MAX_STRING_LEN);
        self.dto.route = sanitize_optional_string(Some(route.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_mcp_method(
        mut self,
        method: impl Into<String>,
    ) -> Self {
        self.dto.mcp_method = sanitize_optional_string(Some(method.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_target(
        mut self,
        target: impl Into<String>,
    ) -> Self {
        self.dto.target = sanitize_optional_string(Some(target.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_direction(
        mut self,
        direction: impl Into<String>,
    ) -> Self {
        self.dto.direction = sanitize_optional_string(Some(direction.into()), MAX_STRING_LEN);
        self
    }

    pub fn with_duration_ms(
        mut self,
        duration_ms: u64,
    ) -> Self {
        self.dto.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_error(
        mut self,
        error_code: Option<impl Into<String>>,
        error_message: impl Into<String>,
    ) -> Self {
        self.dto.error_code = error_code.map(|value| truncate_string(&value.into(), MAX_STRING_LEN));
        self.dto.error_message = sanitize_optional_string(Some(error_message.into()), MAX_TEXT_LEN);
        self
    }

    pub fn with_detail(
        mut self,
        detail: impl Into<String>,
    ) -> Self {
        self.dto.detail = sanitize_optional_string(Some(detail.into()), MAX_TEXT_LEN);
        self
    }

    pub fn with_data(
        mut self,
        value: Value,
    ) -> Self {
        self.dto.data = Some(sanitize_json_value(value));
        self
    }

    pub fn with_mcp_data(
        mut self,
        data: Map<String, Value>,
    ) -> Self {
        self.dto.data = Some(sanitize_json_value(Value::Object(data)));
        self
    }

    pub fn with_task_metadata(
        mut self,
        task_id: Option<String>,
        related_task_id: Option<String>,
        progress_token: Option<String>,
    ) -> Self {
        self.dto.task_id = sanitize_optional_string(task_id, MAX_STRING_LEN);
        self.dto.related_task_id = sanitize_optional_string(related_task_id, MAX_STRING_LEN);
        self.dto.progress_token = sanitize_optional_string(progress_token, MAX_STRING_LEN);
        self
    }

    pub fn occurred_at_ms(
        mut self,
        occurred_at_ms: i64,
    ) -> Self {
        self.dto.occurred_at_ms = occurred_at_ms;
        self
    }

    pub fn build(self) -> AuditEventDto {
        self.dto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct AuditFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<AuditCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<AuditAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AuditStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_occurred_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_occurred_at_ms: Option<i64>,
}

impl AuditFilter {
    pub fn normalized(&self) -> Self {
        Self {
            category: self.category,
            action: self.action,
            status: self.status,
            actor: sanitize_optional_string(self.actor.clone(), MAX_STRING_LEN),
            client_id: sanitize_optional_string(self.client_id.clone(), MAX_STRING_LEN),
            profile_id: sanitize_optional_string(self.profile_id.clone(), MAX_STRING_LEN),
            server_id: sanitize_optional_string(self.server_id.clone(), MAX_STRING_LEN),
            session_id: sanitize_optional_string(self.session_id.clone(), MAX_STRING_LEN),
            request_id: sanitize_optional_string(self.request_id.clone(), MAX_STRING_LEN),
            task_id: sanitize_optional_string(self.task_id.clone(), MAX_STRING_LEN),
            related_task_id: sanitize_optional_string(self.related_task_id.clone(), MAX_STRING_LEN),
            progress_token: sanitize_optional_string(self.progress_token.clone(), MAX_STRING_LEN),
            from_occurred_at_ms: self.from_occurred_at_ms,
            to_occurred_at_ms: self.to_occurred_at_ms,
        }
    }

    pub fn scope_map(&self) -> BTreeMap<String, String> {
        let filter = self.normalized();
        let mut scope = BTreeMap::new();
        insert_scope_value(&mut scope, "category", filter.category.map(enum_key));
        insert_scope_value(&mut scope, "action", filter.action.map(enum_key));
        insert_scope_value(&mut scope, "status", filter.status.map(enum_key));
        insert_scope_value(&mut scope, "actor", filter.actor);
        insert_scope_value(&mut scope, "client_id", filter.client_id);
        insert_scope_value(&mut scope, "profile_id", filter.profile_id);
        insert_scope_value(&mut scope, "server_id", filter.server_id);
        insert_scope_value(&mut scope, "session_id", filter.session_id);
        insert_scope_value(&mut scope, "request_id", filter.request_id);
        insert_scope_value(&mut scope, "task_id", filter.task_id);
        insert_scope_value(&mut scope, "related_task_id", filter.related_task_id);
        insert_scope_value(&mut scope, "progress_token", filter.progress_token);
        insert_scope_value(
            &mut scope,
            "from_occurred_at_ms",
            filter.from_occurred_at_ms.map(|v| v.to_string()),
        );
        insert_scope_value(
            &mut scope,
            "to_occurred_at_ms",
            filter.to_occurred_at_ms.map(|v| v.to_string()),
        );
        scope
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AuditSortCursor {
    pub occurred_at_ms: i64,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AuditCursorScope {
    pub filters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AuditCursor {
    pub sort: AuditSortCursor,
    pub scope: AuditCursorScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AuditListPage {
    pub events: Vec<AuditEventDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

pub fn sanitize_optional_string(
    value: Option<String>,
    max_len: usize,
) -> Option<String> {
    value
        .map(|value| truncate_string(&value, max_len))
        .filter(|value| !value.is_empty())
}

pub fn truncate_string(
    value: &str,
    max_len: usize,
) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }

    value.chars().take(max_len).collect()
}

pub fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    REDACTION_KEYWORDS.iter().any(|keyword| normalized.contains(keyword))
}

pub fn sanitize_json_value(value: Value) -> Value {
    match value {
        Value::String(value) => Value::String(truncate_string(&value, MAX_TEXT_LEN)),
        Value::Array(values) => Value::Array(values.into_iter().take(MAX_MAP_SIZE).map(sanitize_json_value).collect()),
        Value::Object(map) => {
            let sanitized = map
                .into_iter()
                .take(MAX_MAP_SIZE)
                .map(|(key, value)| {
                    let sanitized_key = truncate_string(&key, MAX_STRING_LEN);
                    let sanitized_value = if is_sensitive_key(&sanitized_key) {
                        Value::String(REDACTED_VALUE.to_string())
                    } else {
                        sanitize_json_value(value)
                    };
                    (sanitized_key, sanitized_value)
                })
                .collect();
            Value::Object(sanitized)
        }
        other => other,
    }
}

fn insert_scope_value(
    scope: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        scope.insert(key.to_string(), value);
    }
}

fn enum_key<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn action_category_mapping_is_stable() {
        assert_eq!(AuditAction::ToolsCall.category(), AuditCategory::McpRequest);
        assert_eq!(AuditAction::ServerEnable.category(), AuditCategory::ServerConfig);
        assert_eq!(AuditAction::ProfileActivate.category(), AuditCategory::ProfileConfig);
        assert_eq!(AuditAction::RuntimeInstall.category(), AuditCategory::Management);
        assert_eq!(AuditAction::AuditPolicyUpdate.category(), AuditCategory::Management);
        assert_eq!(AuditAction::CapabilityGrant.category(), AuditCategory::ProfileConfig);
    }

    #[test]
    fn serializes_expected_fields() {
        let event = AuditEvent::new(AuditAction::ToolsCall, AuditStatus::Success)
            .with_request_id("req-1")
            .with_client_id("client-a")
            .with_profile_id("profile-a")
            .with_server_id("server-a")
            .with_session_id("session-a")
            .with_mcp_method("tools/call")
            .with_target("search")
            .with_duration_ms(42)
            .build();

        let value = serde_json::to_value(&event).expect("serialize audit event");
        assert_eq!(value.get("category").and_then(Value::as_str), Some("mcp_request"));
        assert_eq!(value.get("action").and_then(Value::as_str), Some("tools_call"));
        assert_eq!(value.get("status").and_then(Value::as_str), Some("success"));
        assert_eq!(value.get("request_id").and_then(Value::as_str), Some("req-1"));
        assert_eq!(value.get("client_id").and_then(Value::as_str), Some("client-a"));
    }

    #[test]
    fn truncates_oversized_fields() {
        let oversized = "你".repeat(MAX_STRING_LEN + 20);
        let event = AuditEvent::new(AuditAction::AuditPolicyUpdate, AuditStatus::Failed)
            .with_actor(oversized.clone())
            .with_error(None::<String>, oversized)
            .build();

        assert_eq!(event.actor.expect("actor").chars().count(), MAX_STRING_LEN);
        assert_eq!(
            event.error_message.expect("error message").chars().count(),
            MAX_TEXT_LEN.min(MAX_STRING_LEN + 20)
        );
    }

    #[test]
    fn redacts_sensitive_nested_values() {
        let payload = json!({
            "token": "secret-token",
            "nested": {
                "password": "super-secret",
                "safe": "visible"
            },
            "items": [
                { "api_key": "abc" },
                { "message": "ok" }
            ]
        });

        let sanitized = sanitize_json_value(payload);
        assert_eq!(sanitized.get("token").and_then(Value::as_str), Some(REDACTED_VALUE));
        assert_eq!(sanitized["nested"]["password"].as_str(), Some(REDACTED_VALUE));
        assert_eq!(sanitized["nested"]["safe"].as_str(), Some("visible"));
        assert_eq!(sanitized["items"][0]["api_key"].as_str(), Some(REDACTED_VALUE));
    }

    #[test]
    fn filter_scope_is_deterministic() {
        let filter = AuditFilter {
            category: Some(AuditCategory::McpRequest),
            action: Some(AuditAction::ToolsCall),
            status: Some(AuditStatus::Failed),
            client_id: Some("client-a".to_string()),
            server_id: Some("server-a".to_string()),
            ..AuditFilter::default()
        };

        let scope = filter.scope_map();
        assert_eq!(scope.get("category").map(String::as_str), Some("mcp_request"));
        assert_eq!(scope.get("action").map(String::as_str), Some("tools_call"));
        assert_eq!(scope.get("status").map(String::as_str), Some("failed"));
        assert_eq!(scope.get("client_id").map(String::as_str), Some("client-a"));
    }
}
