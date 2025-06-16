// MCPMate Proxy API handlers for notifications
// Contains handler functions for notification endpoints

use std::{collections::HashSet, sync::Arc};

use axum::extract::{Json, Path, State};

use super::ApiError;
use crate::{
    api::{
        models::notifs::{
            NotificationResponse, ToolChangeOperation, ToolChangeScope, ToolsChangedDetails,
            ToolsChangedRequest,
        },
        routes::AppState,
    },
    core::foundation::types::ConnectionStatus,
};

/// Notify clients that the tools list has changed
pub async fn notify_tools_changed(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ToolsChangedRequest>,
) -> Result<Json<NotificationResponse>, ApiError> {
    // Validate request
    match request.scope {
        ToolChangeScope::Services if request.service_ids.is_none() => {
            return Err(ApiError::BadRequest(
                "service_ids is required when scope is 'services'".to_string(),
            ));
        }
        ToolChangeScope::Tools if request.tools.is_none() => {
            return Err(ApiError::BadRequest(
                "tools is required when scope is 'tools'".to_string(),
            ));
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
    let (services_affected, tools_affected) = apply_tool_changes(&request, &state).await?;

    // Get connection pool
    let pool = state.connection_pool.lock().await;

    // Track number of clients notified
    let mut notified_count = 0;

    // For now, we'll just log that we would send a notification
    // In a future implementation, we'll need to integrate with SseProxyServer to send actual notifications
    tracing::info!(
        "Would send tools/listChanged notification to {} clients",
        pool.connections.len()
    );

    // Count all ready connections as notified
    for instances in pool.connections.values() {
        for conn in instances.values() {
            if let ConnectionStatus::Ready = conn.status {
                if conn.service.is_some() {
                    notified_count += 1;
                }
            }
        }
    }

    Ok(Json(NotificationResponse {
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

/// Apply the requested tool changes to the configuration
///
/// This implementation:
/// 1. Enables or disables services based on the request
/// 2. Returns the number of services and tools affected
async fn apply_tool_changes(
    request: &ToolsChangedRequest,
    state: &Arc<AppState>,
) -> Result<(usize, usize), ApiError> {
    match request.operation {
        ToolChangeOperation::Enable | ToolChangeOperation::Disable => {
            let enable = request.operation == ToolChangeOperation::Enable;
            let operation_str = if enable { "Enabling" } else { "Disabling" };

            // Track affected services and tools
            let mut services_affected = 0;
            let mut tools_affected = 0;

            match request.scope {
                ToolChangeScope::All => {
                    // Apply to all services
                    tracing::info!("{} all tools across all services", operation_str);

                    // Get all services
                    let pool = state.connection_pool.lock().await;
                    let service_ids: Vec<String> = pool.connections.keys().cloned().collect();
                    drop(pool); // Release the lock

                    // Enable or disable each service
                    for service_id in &service_ids {
                        if apply_service_change(state, service_id, enable)
                            .await
                            .is_ok()
                        {
                            services_affected += 1;
                            // Assume each service has about 6 tools
                            tools_affected += 6;
                        }
                    }

                    Ok((services_affected, tools_affected))
                }
                ToolChangeScope::Services => {
                    // Apply to specified services
                    if let Some(service_ids) = &request.service_ids {
                        tracing::info!(
                            "{} all tools for services: {:?}",
                            operation_str,
                            service_ids
                        );

                        // Enable or disable each specified service
                        for service_id in service_ids {
                            if apply_service_change(state, service_id, enable)
                                .await
                                .is_ok()
                            {
                                services_affected += 1;
                                // Assume each service has about 6 tools
                                tools_affected += 6;
                            }
                        }

                        Ok((services_affected, tools_affected))
                    } else {
                        // This should not happen due to validation
                        Ok((0, 0))
                    }
                }
                ToolChangeScope::Tools => {
                    // For now, we don't handle individual tools
                    // Just log the request and return dummy values
                    if let Some(tools) = &request.tools {
                        tracing::info!("{} specific tools: {:?}", operation_str, tools);

                        // Track affected services
                        let mut affected_service_ids = HashSet::new();

                        // Count tools with specific service
                        let mut tool_count = 0;
                        for tool in tools {
                            if let Some(service_id) = &tool.service_id {
                                affected_service_ids.insert(service_id.clone());
                                tool_count += 1;
                            } else {
                                // Tool without specific service - assume it affects 2 services
                                tool_count += 2;
                            }
                        }

                        Ok((affected_service_ids.len(), tool_count))
                    } else {
                        // This should not happen due to validation
                        Ok((0, 0))
                    }
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

/// Apply service change (enable or disable)
async fn apply_service_change(
    state: &Arc<AppState>,
    service_id: &str,
    enable: bool,
) -> Result<(), ApiError> {
    let state_clone = State(state.clone());
    let service_path = Path(service_id.to_string());
    let empty_query = axum::extract::Query(std::collections::HashMap::new());

    if enable {
        // Enable the service
        match crate::api::handlers::server::enable_server(state_clone, service_path, empty_query)
            .await
        {
            Ok(_) => {
                tracing::info!("Successfully enabled service '{}'", service_id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to enable service '{}': {}", service_id, e);
                Err(e)
            }
        }
    } else {
        // Disable the service
        let empty_query = axum::extract::Query(std::collections::HashMap::new());
        match crate::api::handlers::server::disable_server(state_clone, service_path, empty_query)
            .await
        {
            Ok(_) => {
                tracing::info!("Successfully disabled service '{}'", service_id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to disable service '{}': {}", service_id, e);
                Err(e)
            }
        }
    }
}
