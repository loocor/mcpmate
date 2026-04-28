//! Event handlers for the core event system

use super::{Event, EventBus};
use crate::core::foundation::error::CoreResult;
use crate::core::proxy::server::ProxyServer;
use std::sync::Arc;
use tokio::task;
use tracing::{debug, error, info, warn};

fn global_proxy_server() -> Option<ProxyServer> {
    ProxyServer::global().and_then(|server| server.try_lock().ok().map(|guard| guard.clone()))
}

/// Simplified event handlers without complex callback system
pub struct EventHandlers {
    /// Optional profile service for cache invalidation
    pub profile_service: Option<Arc<crate::core::profile::ProfileService>>,
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
            profile_service: None,
            connection_pool: None,
            event_capability_manager: None,
        }
    }

    /// Set profile service for cache invalidation
    pub fn set_profile_service(
        &mut self,
        profile_service: Arc<crate::core::profile::ProfileService>,
    ) {
        self.profile_service = Some(profile_service);
        info!("Set profile service for event handlers");
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

    async fn invalidate_profile_cache(&self) {
        if let Some(profile_service) = &self.profile_service {
            profile_service.invalidate_cache().await;
        }
    }

    async fn invalidate_cache_and_sync_servers(&self) {
        self.invalidate_profile_cache().await;
        if let Some(connection_pool) = &self.connection_pool {
            let mut pool = connection_pool.lock().await;
            if let Err(e) = pool.sync_servers_from_active_profile().await {
                error!("Failed to sync servers after database/config event: {}", e);
            }
        }
    }

    async fn notify_all_list_changed(
        &self,
        context: &str,
    ) {
        if let Some(proxy_server) = global_proxy_server() {
            let refreshed = proxy_server.refresh_all_bound_sessions().await;
            let (t, p, r) = proxy_server.notify_all_list_changed().await;
            debug!(
                "{}: refreshed={} bound sessions, list_changed (tools={}, prompts={}, resources={})",
                context, refreshed, t, p, r
            );
        }
    }

    /// Handle events with appropriate actions
    async fn handle_event(
        &self,
        event: Event,
    ) {
        match event {
            // Config profile status changed - trigger server management and listChanged
            Event::ProfileStatusChanged { profile_id, enabled } => {
                debug!("Handling ProfileStatusChanged: {} -> {}", profile_id, enabled);
                self.invalidate_cache_and_sync_servers().await;
                self.notify_all_list_changed("Profile change").await;
            }

            // Server enabled in profile changed - trigger immediate server management for active profile
            Event::ServerEnabledInProfileChanged {
                server_id,
                server_name,
                profile_id,
                enabled,
            } => {
                debug!(
                    "Handling ServerEnabledInProfileChanged: server '{}' in profile '{}' -> {}",
                    server_name, profile_id, enabled
                );

                self.invalidate_profile_cache().await;

                if let Some(connection_pool) = &self.connection_pool {
                    let mut pool = connection_pool.lock().await;
                    if let Err(e) = pool.update_server_status(&server_id, enabled).await {
                        error!(
                            "Failed to update server '{}' (ID: {}) status: {}",
                            server_name, server_id, e
                        );
                    }
                }

                self.notify_all_list_changed("Server enabled change").await;
            }

            // Events that require server synchronization (but not specific server management)
            Event::ServerGlobalStatusChanged { .. } | Event::DatabaseChanged | Event::ConfigReloaded => {
                debug!("Handling server sync event: {:?}", event);
                self.invalidate_cache_and_sync_servers().await;
                self.notify_all_list_changed("Server sync event").await;
            }

            // Emit listChanged notifications for downstream clients
            Event::ToolEnabledInProfileChanged { .. } => {
                debug!("Tool configuration changed: emitting tools/list_changed");
                self.invalidate_profile_cache().await;
                if let Some(proxy_server) = global_proxy_server() {
                    let count = proxy_server.notify_tool_list_changed().await;
                    debug!("tools/list_changed notified {} client(s)", count);
                }
            }
            Event::PromptEnabledInProfileChanged { .. } => {
                debug!("Prompt configuration changed: emitting prompts/list_changed");
                self.invalidate_profile_cache().await;
                if let Some(proxy_server) = global_proxy_server() {
                    let count = proxy_server.notify_prompt_list_changed().await;
                    debug!("prompts/list_changed notified {} client(s)", count);
                }
            }
            Event::ClientVisibleDirectSurfaceChanged { client_id } => {
                debug!(client = %client_id, "Client visible direct surface changed: emitting tools/prompts/resources list_changed");
                if let Some(proxy_server) = global_proxy_server() {
                    let (tools_count, prompts_count, resources_count) = proxy_server.notify_all_list_changed().await;
                    debug!(
                        client = %client_id,
                        "tools/list_changed={}, prompts/list_changed={}, resources/list_changed={}",
                        tools_count,
                        prompts_count,
                        resources_count
                    );
                }
            }
            Event::ResourceEnabledInProfileChanged { .. } => {
                debug!("Resource configuration changed: emitting resources/list_changed");
                self.invalidate_profile_cache().await;
                if let Some(proxy_server) = global_proxy_server() {
                    let count = proxy_server.notify_resource_list_changed().await;
                    debug!("resources/list_changed notified {} client(s)", count);
                }
            }

            Event::ResourceTemplateEnabledInProfileChanged { .. } => {
                debug!("Resource template configuration changed: emitting resources/list_changed");
                self.invalidate_profile_cache().await;
                if let Some(proxy_server) = global_proxy_server() {
                    let count = proxy_server.notify_resource_list_changed().await;
                    debug!(
                        "resources/list_changed notified {} client(s) (via template change)",
                        count
                    );
                }
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
                // If resources on a server may have changed content, notify subscribed clients
                if let Some(proxy_server) = global_proxy_server() {
                    let notified = proxy_server.notify_resource_updates_for_server(&server_id).await;
                    if notified > 0 {
                        info!(server = %server_name, notified, "resources/updated broadcast for subscribed URIs");
                    }
                }
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
                profile_id,
                servers_to_start,
                servers_to_stop,
            } => {
                info!(
                    "Config application started for profile {}: {} servers to start, {} to stop",
                    profile_id,
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
                    info!(
                        "Server {} (ID: {}) startup completed successfully",
                        server_name, server_id
                    );

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
                    warn!(
                        "Server {} (ID: {}) startup failed: {}",
                        server_name, server_id, error_message
                    );
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
                profile_id,
                total_servers,
                started_servers,
                stopped_servers,
                failed_operations,
                duration_ms,
            } => {
                info!(
                    "Config application completed for profile {}: {} total, {} started, {} stopped, {} failed in {}ms",
                    profile_id,
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
                profile_id,
                stage,
                progress,
                estimated_remaining_seconds,
            } => {
                debug!(
                    "Config application progress for profile {}: {} ({}%{})",
                    profile_id,
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
