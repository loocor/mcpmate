//! Event handlers for the recore event system

use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::task;
use tracing::{debug, error, info};

use super::{Event, EventBus};
use crate::recore::foundation::error::RecoreResult;

/// Callback type for server synchronization
pub type ServerSyncCallback = Arc<dyn Fn() -> BoxFuture<'static, RecoreResult<()>> + Send + Sync>;

/// Event handlers with dependency injection
pub struct EventHandlers {
    /// Callback for server synchronization
    server_sync_callback: Option<ServerSyncCallback>,
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
            server_sync_callback: None,
        }
    }

    /// Register server sync callback
    pub fn register_server_sync_callback<F>(
        &mut self,
        callback: F,
    ) where
        F: Fn() -> BoxFuture<'static, RecoreResult<()>> + Send + Sync + 'static,
    {
        self.server_sync_callback = Some(Arc::new(callback));
        info!("Registered server sync callback for recore events");
    }

    /// Initialize event handlers and register with the global event bus
    pub fn init(self) -> RecoreResult<()> {
        info!("Initializing recore event handlers");

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

        info!("Recore event handlers initialized successfully");
        Ok(())
    }

    /// Handle events with appropriate actions
    async fn handle_event(
        &self,
        event: Event,
    ) {
        match event {
            // Events that require server synchronization
            Event::ServerGlobalStatusChanged { .. }
            | Event::ConfigSuitStatusChanged { .. }
            | Event::ServerEnabledInSuitChanged { .. }
            | Event::DatabaseChanged
            | Event::ConfigReloaded => {
                debug!("Handling server sync event: {:?}", event);
                self.trigger_server_sync().await;
            }

            // Events that don't require server synchronization
            Event::ToolEnabledInSuitChanged { .. }
            | Event::ResourceEnabledInSuitChanged { .. }
            | Event::PromptEnabledInSuitChanged { .. } => {
                debug!("Configuration changed, no server sync needed: {:?}", event);
                // These events only affect protocol-level configuration
                // No need to reconnect to servers
            }

            // Transport and runtime events (handled by wait mechanism)
            Event::ServerTransportReady {
                transport_type,
                ready,
            } => {
                debug!("Transport event: {:?} ready={}", transport_type, ready);
                // These events are handled by the wait mechanism
                // No additional action needed here
            }

            Event::RuntimeCheckStarted {
                runtime_type,
                version,
            } => {
                debug!("Runtime check started: {} {:?}", runtime_type, version);
            }

            Event::RuntimeCheckSuccess {
                runtime_type,
                version,
                bin_path,
            } => {
                debug!(
                    "Runtime check success: {} {} at {}",
                    runtime_type, version, bin_path
                );
            }

            Event::RuntimeCheckFailed {
                runtime_type,
                error,
            } => {
                debug!("Runtime check failed: {} - {}", runtime_type, error);
            }

            Event::RuntimeDownloadStarted {
                runtime_type,
                version,
            } => {
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
                debug!(
                    "Runtime ready: {} {} at {}",
                    runtime_type, version, bin_path
                );
            }

            Event::RuntimeSetupFailed {
                runtime_type,
                error,
            } => {
                debug!("Runtime setup failed: {} - {}", runtime_type, error);
            }
        }
    }

    /// Trigger server synchronization if callback is registered
    async fn trigger_server_sync(&self) {
        if let Some(callback) = &self.server_sync_callback {
            debug!("Triggering server synchronization");
            match callback().await {
                Ok(()) => {
                    debug!("Server synchronization completed successfully");
                }
                Err(e) => {
                    error!("Server synchronization failed: {}", e);
                }
            }
        } else {
            debug!("No server sync callback registered, skipping synchronization");
        }
    }
}

/// Initialize the event system with default handlers
pub fn init() -> RecoreResult<()> {
    let handlers = EventHandlers::new();
    handlers.init()
}

/// Initialize the event system with custom handlers
pub fn init_with_handlers(handlers: EventHandlers) -> RecoreResult<()> {
    handlers.init()
}
