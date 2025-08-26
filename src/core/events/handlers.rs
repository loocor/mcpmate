//! Event handlers for the core event system

use std::sync::Arc;

use tokio::task;
use tracing::{debug, error, info, warn};

use super::{Event, EventBus};
use crate::core::foundation::error::CoreResult;

/// Simplified event handlers without complex callback system
pub struct EventHandlers {
    /// Optional suit service for cache invalidation
    pub suit_service: Option<Arc<crate::core::suit::SuitService>>,
    /// Optional connection pool for server management
    pub connection_pool: Option<Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>>,
    /// Optional event-driven capability manager for server capability sync
    pub event_capability_manager: Option<Arc<crate::core::events::capability::EventDrivenCapabilityManager>>,
}

impl Default for EventHandlers {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandlers {
    /// Create new event handlers
    pub fn new() -> Self {
        Self {
            suit_service: None,
            connection_pool: None,
            event_capability_manager: None,
        }
    }

    /// Set suit service for cache invalidation
    pub fn set_suit_service(
        &mut self,
        suit_service: Arc<crate::core::suit::SuitService>,
    ) {
        self.suit_service = Some(suit_service);
        info!("Set suit service for event handlers");
    }

    /// Set connection pool for server management
    pub fn set_connection_pool(
        &mut self,
        connection_pool: Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
    ) {
        self.connection_pool = Some(connection_pool);
        info!("Set connection pool for event handlers");
    }

    /// Set event-driven capability manager for server capability sync
    pub fn set_event_capability_manager(
        &mut self,
        event_capability_manager: Arc<crate::core::events::capability::EventDrivenCapabilityManager>,
    ) {
        self.event_capability_manager = Some(event_capability_manager);
        info!("Set event-driven capability manager for event handlers");
    }

    /// Initialize event handlers and register with the global event bus
    pub fn init(self) -> CoreResult<()> {
        info!("Initializing core event handlers");

        // Register the main event handler
        EventBus::global().subscribe({
            let handlers = Arc::new(self);
            move |event| {
                let handlers = Arc::clone(&handlers);
                task::spawn(async move {
                    handlers.handle_event(event).await;
                });
            }
        });

        info!("Core event handlers initialized successfully");
        Ok(())
    }

    /// Handle events with appropriate actions
    async fn handle_event(
        &self,
        event: Event,
    ) {
        match event {
            // Config suit status changed - trigger server management
            Event::ConfigSuitStatusChanged { suit_id, enabled } => {
                debug!("Handling ConfigSuitStatusChanged: {} -> {}", suit_id, enabled);

                // Invalidate cache first
                if let Some(suit_service) = &self.suit_service {
                    suit_service.invalidate_cache().await;
                }

                // Sync servers from active suits using connection pool
                if let Some(connection_pool) = &self.connection_pool {
                    let mut pool = connection_pool.lock().await;
                    if let Err(e) = pool.sync_servers_from_active_suits().await {
                        error!("Failed to sync servers after suit change: {}", e);
                    }
                }
            }

            // Server enabled in suit changed - trigger immediate server management for active suits
            Event::ServerEnabledInSuitChanged {
                server_id: _,
                server_name,
                suit_id,
                enabled,
            } => {
                debug!(
                    "Handling ServerEnabledInSuitChanged: server '{}' in suit '{}' -> {}",
                    server_name, suit_id, enabled
                );

                // Invalidate cache
                if let Some(suit_service) = &self.suit_service {
                    suit_service.invalidate_cache().await;
                }

                // Use connection pool to manage server status
                if let Some(connection_pool) = &self.connection_pool {
                    let mut pool = connection_pool.lock().await;
                    if let Err(e) = pool.update_server_status(&server_name, enabled).await {
                        error!("Failed to update server '{}' status: {}", server_name, e);
                    }
                }
            }

            // Events that require server synchronization (but not specific server management)
            Event::ServerGlobalStatusChanged { .. } | Event::DatabaseChanged | Event::ConfigReloaded => {
                debug!("Handling server sync event: {:?}", event);

                // Invalidate cache
                if let Some(suit_service) = &self.suit_service {
                    suit_service.invalidate_cache().await;
                }
            }

            // Events that don't require server synchronization
            Event::ToolEnabledInSuitChanged { .. }
            | Event::ResourceEnabledInSuitChanged { .. }
            | Event::PromptEnabledInSuitChanged { .. } => {
                debug!("Configuration changed, no server sync needed: {:?}", event);
            }

            Event::CacheUpdated {
                server_id,
                server_name,
                update_type,
            } => {
                debug!(
                    "Cache updated for server '{}' ({}): {:?}",
                    server_name, server_id, update_type
                );
            }

            Event::CacheInvalidated { server_id, server_name } => {
                debug!("Cache invalidated for server '{}' ({})", server_name, server_id);
            }

            Event::CacheCleared => {
                debug!("Cache cleared");
            }

            // Transport and runtime events (handled by wait mechanism)
            Event::ServerTransportReady { transport_type, ready } => {
                debug!("Transport event: {:?} ready={}", transport_type, ready);
            }

            Event::RuntimeCheckStarted { runtime_type, version } => {
                debug!("Runtime check started: {} {:?}", runtime_type, version);
            }

            Event::RuntimeCheckSuccess {
                runtime_type,
                version,
                bin_path,
            } => {
                debug!("Runtime check success: {} {} at {}", runtime_type, version, bin_path);
            }

            Event::RuntimeCheckFailed { runtime_type, error } => {
                debug!("Runtime check failed: {} - {}", runtime_type, error);
            }

            Event::RuntimeDownloadStarted { runtime_type, version } => {
                debug!("Runtime download started: {} {}", runtime_type, version);
            }

            Event::RuntimeDownloadCompleted {
                runtime_type,
                version,
                install_path,
            } => {
                debug!(
                    "Runtime download completed: {} {} at {}",
                    runtime_type, version, install_path
                );
            }

            Event::RuntimeReady {
                runtime_type,
                version,
                bin_path,
            } => {
                debug!("Runtime ready: {} {} at {}", runtime_type, version, bin_path);
            }

            Event::RuntimeSetupFailed { runtime_type, error } => {
                debug!("Runtime setup failed: {} - {}", runtime_type, error);
            }

            // New server management events - log for debugging
            Event::ConfigApplicationStarted {
                suit_id,
                servers_to_start,
                servers_to_stop,
            } => {
                info!(
                    "Config application started for suit {}: {} servers to start, {} to stop",
                    suit_id,
                    servers_to_start.len(),
                    servers_to_stop.len()
                );
            }

            Event::ServerConnectionStartup {
                server_name,
                stage,
                progress,
            } => {
                debug!("Server {} startup: {} ({}%)", server_name, stage, progress);
            }

            Event::ServerConnectionStartupCompleted {
                server_id,
                server_name,
                success,
                error,
            } => {
                if success {
                    info!("Server {} (ID: {}) startup completed successfully", server_name, server_id);

                    // Trigger capability sync for newly connected server using lightweight manager
                    if let Some(event_capability_manager) = &self.event_capability_manager {
                        debug!(
                            "Server '{}' (ID: {}) connected successfully, triggering event-driven capability sync",
                            server_name, server_id
                        );

                        match event_capability_manager.sync_single_server(&server_id).await {
                            Ok(_) => {
                                debug!(
                                    "Successfully synced capabilities for newly connected server '{}' (ID: {}) (event-driven)",
                                    server_name, server_id
                                );
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to sync capabilities for newly connected server '{}' (ID: {}): {}",
                                    server_name, server_id, e
                                );
                            }
                        }
                    } else {
                        debug!(
                            "No event-driven capability manager available for server '{}' (ID: {}) capability sync",
                            server_name, server_id
                        );
                    }
                } else {
                    let error_message = error.unwrap_or_else(|| "Unknown error".to_string());
                    warn!("Server {} (ID: {}) startup failed: {}", server_name, server_id, error_message);
                    debug!(
                        "Server '{}' (ID: {}) connection failed: {}, skipping capability sync",
                        server_name, server_id, error_message
                    );
                }
            }

            Event::ServerConnectionShutdown { server_name } => {
                debug!("Server {} shutdown initiated", server_name);
            }

            Event::ServerConnectionShutdownCompleted { server_name, success } => {
                if success {
                    info!("Server {} shutdown completed successfully", server_name);
                } else {
                    warn!("Server {} shutdown failed", server_name);
                }
            }

            Event::ConfigApplicationCompleted {
                suit_id,
                total_servers,
                started_servers,
                stopped_servers,
                failed_operations,
                duration_ms,
            } => {
                info!(
                    "Config application completed for suit {}: {} total, {} started, {} stopped, {} failed in {}ms",
                    suit_id,
                    total_servers,
                    started_servers.len(),
                    stopped_servers.len(),
                    failed_operations.len(),
                    duration_ms
                );

                if !failed_operations.is_empty() {
                    warn!("Failed operations: {:?}", failed_operations);
                }
            }

            Event::ConfigApplicationProgress {
                suit_id,
                stage,
                progress,
                estimated_remaining_seconds,
            } => {
                debug!(
                    "Config application progress for suit {}: {} ({}%{})",
                    suit_id,
                    stage,
                    progress,
                    estimated_remaining_seconds
                        .map(|s| format!(", ~{}s remaining", s))
                        .unwrap_or_default()
                );
            }
        }
    }
}

/// Initialize the event system with default handlers
pub fn init() -> CoreResult<()> {
    let handlers = EventHandlers::new();
    handlers.init()
}

/// Initialize the event system with custom handlers
pub fn init_with_handlers(handlers: EventHandlers) -> CoreResult<()> {
    handlers.init()
}
