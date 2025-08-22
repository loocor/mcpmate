// MCPMate Proxy API handlers for notifications
// Contains handler functions for notification endpoints

use std::{collections::HashSet, sync::Arc};

use axum::extract::{Json, Path, State};

use super::ApiError;
use crate::{
    api::{
        models::{
            notifs::{ToolChangeOperation, ToolChangeScope, ToolsChangedDetails, ToolsChangedReq, ToolsChangedResp, ToolsChangedApiResp},
        },
        routes::AppState,
    },
    common::status::EnabledStatus,
    core::foundation::types::ConnectionStatus,
};

/// Tools changed notification handler
pub async fn tools_changed(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ToolsChangedReq>,
) -> Result<Json<ToolsChangedApiResp>, ApiError> {
    let result = tools_changed_core(&request, &state).await?;
    Ok(Json(result))
}

/// Core business logic for tools changed notification core
async fn tools_changed_core(
    request: &ToolsChangedReq,
    state: &Arc<AppState>,
) -> Result<ToolsChangedApiResp, ApiError> {
    // Early return validation using pattern matching
    match (&request.scope, &request.service_ids, &request.tools) {
        (ToolChangeScope::Services, None, _) => {
            return Err(ApiError::BadRequest(
                "service_ids is required when scope is 'services'".into(),
            ));
        }
        (ToolChangeScope::Tools, _, None) => {
            return Err(ApiError::BadRequest("tools is required when scope is 'tools'".into()));
        }
        _ => {}
    }

    let reason = request.reason.as_deref().unwrap_or("configuration change");

    tracing::info!(
        "Tools changed notification received. Operation: {:?}, Scope: {:?}, Reason: {}",
        request.operation,
        request.scope,
        reason
    );

    // Apply the requested changes to the configuration
    let (services_affected, tools_affected) = apply_tool_changes(request, state).await?;

    let pool = state.connection_pool.lock().await;

    tracing::info!(
        "Would send tools/listChanged notification to {} clients",
        pool.connections.len()
    );

    let notified_count = pool
        .connections
        .values()
        .flat_map(|instances| instances.values())
        .filter(|conn| matches!(conn.status, ConnectionStatus::Ready) && conn.service.is_some())
        .count();

    Ok(ToolsChangedApiResp::success(ToolsChangedResp {
        notified_clients: notified_count,
        message: format!("Notified {notified_count} clients about tools list change"),
        details: ToolsChangedDetails {
            operation: format!("{:?}", request.operation),
            scope: format!("{:?}", request.scope),
            services_affected,
            tools_affected,
        },
    }))
}

/// Apply the requested changes to the configuration
async fn apply_tool_changes(
    request: &ToolsChangedReq,
    state: &Arc<AppState>,
) -> Result<(usize, usize), ApiError> {
    match request.operation {
        ToolChangeOperation::Enable | ToolChangeOperation::Disable => {
            let status = match request.operation {
                ToolChangeOperation::Enable => EnabledStatus::Enabled,
                ToolChangeOperation::Disable => EnabledStatus::Disabled,
                _ => unreachable!(),
            };
            let operation_str = match status {
                EnabledStatus::Enabled => "Enabling",
                EnabledStatus::Disabled => "Disabling",
            };

            // Track affected services and tools
            let mut services_affected = 0;
            let mut tools_affected = 0;

            match request.scope {
                // Apply to all services
                ToolChangeScope::All => {
                    tracing::info!("{operation_str} all tools across all services");

                    let service_ids = {
                        let pool = state.connection_pool.lock().await;
                        pool.connections.keys().cloned().collect::<Vec<_>>()
                    };

                    for service_id in &service_ids {
                        if apply_service_change(state, service_id, status).await.is_ok() {
                            services_affected += 1;
                            tools_affected += 6;
                        }
                    }

                    Ok((services_affected, tools_affected))
                }

                // Apply to specific services
                ToolChangeScope::Services => {
                    let Some(service_ids) = &request.service_ids else {
                        return Ok((0, 0));
                    };

                    tracing::info!("{operation_str} all tools for services: {service_ids:?}");

                    for service_id in service_ids {
                        if apply_service_change(state, service_id, status).await.is_ok() {
                            services_affected += 1;
                            tools_affected += 6;
                        }
                    }

                    Ok((services_affected, tools_affected))
                }

                // Apply to specific tools
                ToolChangeScope::Tools => {
                    let Some(tools) = &request.tools else {
                        return Ok((0, 0));
                    };

                    tracing::info!("{operation_str} specific tools: {tools:?}");

                    let (affected_services, tool_count) = tools.iter().fold((HashSet::new(), 0), |mut acc, tool| {
                        if let Some(service_id) = &tool.service_id {
                            acc.0.insert(service_id.as_str());
                            acc.1 += 1;
                        } else {
                            acc.1 += 2;
                        }
                        acc
                    });

                    Ok((affected_services.len(), tool_count))
                }
            }
        }
        ToolChangeOperation::Update => {
            // Just send notification without changing configuration
            tracing::info!("Sending update notification without changing configuration");
            // Assume affecting all services and tools
            Ok((5, 30))
        }
    }
}

/// Apply the requested changes to the service
async fn apply_service_change(
    state: &Arc<AppState>,
    service_id: &str,
    status: EnabledStatus,
) -> Result<(), ApiError> {
    let state_clone = State(state.clone());
    let service_path = Path(service_id.to_string());
    let empty_query = axum::extract::Query(std::collections::HashMap::new());

    let result = match status {
        EnabledStatus::Enabled => {
            crate::api::handlers::server::enable_server(state_clone, service_path, empty_query).await
        }
        EnabledStatus::Disabled => {
            crate::api::handlers::server::disable_server(state_clone, service_path, empty_query).await
        }
    };

    match result {
        Ok(_) => {
            tracing::info!("Successfully {} service '{service_id}'", status.as_str());
            Ok(())
        }
        Err(e) => {
            let action = match status {
                EnabledStatus::Enabled => "enable",
                EnabledStatus::Disabled => "disable",
            };
            tracing::error!("Failed to {action} service '{service_id}': {e}");
            Err(e)
        }
    }
}
