use std::sync::Arc;

use serde_json::{Map, Value};

use crate::{
    audit::{AuditAction, AuditEvent, AuditService, AuditStatus},
    core::proxy::server::ClientContext,
};

pub async fn emit_event(
    audit_service: Option<&Arc<AuditService>>,
    event: crate::audit::AuditEventDto,
) {
    if let Some(audit_service) = audit_service {
        audit_service.emit(event).await;
    }
}

pub fn build_mcp_event(
    action: AuditAction,
    status: AuditStatus,
    client: Option<&ClientContext>,
    protocol_version: Option<String>,
    target: Option<String>,
    duration_ms: Option<u64>,
    data: Option<Map<String, Value>>,
    error_message: Option<String>,
) -> crate::audit::AuditEventDto {
    let mut event = AuditEvent::new(action, status)
        .with_mcp_method(mcp_method_name(action))
        .with_direction("client_to_server");

    if let Some(client) = client {
        event = apply_client_context(event, client);
    }
    if let Some(protocol_version) = protocol_version {
        event = event.with_protocol_version(protocol_version);
    }
    if let Some(target) = target {
        event = event.with_target(target);
    }
    apply_common_fields(event, duration_ms, data, error_message).build()
}

pub fn build_rest_event(
    action: AuditAction,
    status: AuditStatus,
    method: &str,
    route: &str,
    duration_ms: Option<u64>,
    server_id: Option<String>,
    profile_id: Option<String>,
    data: Option<Map<String, Value>>,
    error_message: Option<String>,
) -> crate::audit::AuditEventDto {
    let mut event = AuditEvent::new(action, status).with_http_route(method.to_string(), route.to_string());
    event = apply_common_fields(event, duration_ms, data, error_message);
    if let Some(server_id) = server_id {
        event = event.with_server_id(server_id);
    }
    if let Some(profile_id) = profile_id {
        event = event.with_profile_id(profile_id);
    }
    event.build()
}

fn apply_client_context(
    mut event: AuditEvent,
    client: &ClientContext,
) -> AuditEvent {
    event = event.with_client_id(client.client_id.clone());
    if let Some(profile_id) = &client.profile_id {
        event = event.with_profile_id(profile_id.clone());
    }
    if let Some(session_id) = &client.session_id {
        event = event.with_session_id(session_id.clone());
    }
    event
}

fn apply_common_fields(
    mut event: AuditEvent,
    duration_ms: Option<u64>,
    data: Option<Map<String, Value>>,
    error_message: Option<String>,
) -> AuditEvent {
    if let Some(duration_ms) = duration_ms {
        event = event.with_duration_ms(duration_ms);
    }
    if let Some(data) = data {
        event = event.with_mcp_data(data);
    }
    if let Some(error_message) = error_message {
        event = event.with_error(None::<String>, error_message);
    }
    event
}

pub(crate) fn mcp_method_name(action: AuditAction) -> &'static str {
    match action {
        AuditAction::Initialize => "initialize",
        AuditAction::ToolsList => "tools/list",
        AuditAction::ToolsCall => "tools/call",
        AuditAction::ResourcesList => "resources/list",
        AuditAction::ResourcesRead => "resources/read",
        AuditAction::PromptsList => "prompts/list",
        AuditAction::PromptsGet => "prompts/get",
        AuditAction::NotificationProgress => "notifications/progress",
        AuditAction::NotificationCancelled => "notifications/cancelled",
        AuditAction::LoggingSetLevel => "logging/setLevel",
        AuditAction::NotificationMessage => "notifications/message",
        AuditAction::ServerCreate
        | AuditAction::ServerImport
        | AuditAction::ServerUpdate
        | AuditAction::ServerDelete
        | AuditAction::ServerEnable
        | AuditAction::ServerDisable
        | AuditAction::ProfileCreate
        | AuditAction::ProfileUpdate
        | AuditAction::ProfileDelete
        | AuditAction::ProfileActivate
        | AuditAction::ProfileDeactivate
        | AuditAction::ProfileServerEnable
        | AuditAction::ProfileServerDisable
        | AuditAction::ProfileServerRemove
        | AuditAction::ServerInstanceDisconnect
        | AuditAction::ServerInstanceForceDisconnect
        | AuditAction::ServerInstanceReconnect
        | AuditAction::ServerInstanceResetReconnect
        | AuditAction::ServerInstanceRecover
        | AuditAction::ServerInstanceCancel
        | AuditAction::ServerCacheReset
        | AuditAction::ClientManageEnable
        | AuditAction::ClientManageDisable
        | AuditAction::ClientSettingsUpdate
        | AuditAction::ClientConfigApply
        | AuditAction::ClientConfigRestore
        | AuditAction::ClientConfigImport
        | AuditAction::ClientCapabilityUpdate
        | AuditAction::ClientBackupDelete
        | AuditAction::ClientBackupPolicyUpdate
        | AuditAction::CoreSourceApply
        | AuditAction::LocalCoreServiceStart
        | AuditAction::LocalCoreServiceRestart
        | AuditAction::LocalCoreServiceStop
        | AuditAction::LocalCoreServiceInstall
        | AuditAction::LocalCoreServiceUninstall
        | AuditAction::DesktopManagedCoreStart
        | AuditAction::DesktopManagedCoreRestart
        | AuditAction::DesktopManagedCoreStop
        | AuditAction::RuntimeInstall
        | AuditAction::RuntimeCacheReset
        | AuditAction::CapabilityGrant
        | AuditAction::CapabilityRevoke
        | AuditAction::AuditPolicyUpdate => "",
    }
}
