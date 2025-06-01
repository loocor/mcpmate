//! Event handlers for the MCPMate event system

use tracing::{debug, error, info};

use super::{Event, EventBus};
use crate::core::http::proxy::core::HttpProxyServer;

/// Register all event handlers
pub fn register_handlers() {
    info!("Registering event handlers");

    // Register handler for server global status changes
    EventBus::global().subscribe(handle_server_global_status_changed);

    // Register handler for config suit status changes
    EventBus::global().subscribe(handle_config_suit_status_changed);

    // Register handler for server enabled in suit changes
    EventBus::global().subscribe(handle_server_enabled_in_suit_changed);

    // Register handler for database changes
    EventBus::global().subscribe(handle_database_changed);

    // Register handler for config reloaded
    EventBus::global().subscribe(handle_config_reloaded);
}

/// Handle server global status changed events
fn handle_server_global_status_changed(event: Event) {
    if let Event::ServerGlobalStatusChanged {
        server_id,
        server_name,
        enabled,
    } = event
    {
        debug!(
            "Handling server global status changed event: server_id={}, server_name={}, enabled={}",
            server_id, server_name, enabled
        );

        // Trigger server connections sync
        // This is done asynchronously in a separate task to avoid blocking the event handler
        tokio::spawn(async move {
            if let Err(e) = HttpProxyServer::sync_server_connections().await {
                error!("Failed to sync server connections: {}", e);
            }
        });
    }
}

/// Handle config suit status changed events
fn handle_config_suit_status_changed(event: Event) {
    if let Event::ConfigSuitStatusChanged { suit_id, enabled } = event {
        debug!(
            "Handling config suit status changed event: suit_id={}, enabled={}",
            suit_id, enabled
        );

        // Trigger server connections sync
        tokio::spawn(async move {
            if let Err(e) = HttpProxyServer::sync_server_connections().await {
                error!("Failed to sync server connections: {}", e);
            }
        });
    }
}

/// Handle server enabled in suit changed events
fn handle_server_enabled_in_suit_changed(event: Event) {
    if let Event::ServerEnabledInSuitChanged {
        server_id,
        server_name,
        suit_id,
        enabled,
    } = event
    {
        debug!(
            "Handling server enabled in suit changed event: server_id={}, server_name={}, suit_id={}, enabled={}",
            server_id, server_name, suit_id, enabled
        );

        // Trigger server connections sync
        tokio::spawn(async move {
            if let Err(e) = HttpProxyServer::sync_server_connections().await {
                error!("Failed to sync server connections: {}", e);
            }
        });
    }
}

/// Handle database changed events
fn handle_database_changed(event: Event) {
    if let Event::DatabaseChanged = event {
        debug!("Handling database changed event");

        // Trigger server connections sync
        tokio::spawn(async move {
            if let Err(e) = HttpProxyServer::sync_server_connections().await {
                error!("Failed to sync server connections: {}", e);
            }
        });
    }
}

/// Handle config reloaded events
fn handle_config_reloaded(event: Event) {
    if let Event::ConfigReloaded = event {
        debug!("Handling config reloaded event");

        // Trigger server connections sync
        tokio::spawn(async move {
            if let Err(e) = HttpProxyServer::sync_server_connections().await {
                error!("Failed to sync server connections: {}", e);
            }
        });
    }
}
