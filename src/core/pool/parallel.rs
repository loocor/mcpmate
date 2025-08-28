//! Pool parallel connection management functionality
//!
//! Provides high-performance parallel connection capabilities for UpstreamConnectionPool including:
//! - parallel connection to multiple servers
//! - event-driven state management
//! - stateless connection logic for better parallelization
//! - centralized connection event handling

use anyhow::Result;
use rmcp::{RoleClient, model::Tool, service::RunningService};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use tracing;

use super::UpstreamConnectionPool;
use crate::{
    common::{
        server::{ServerType, TransportType},
        sync::SyncHelper,
    },
    core::transport::{
        connect_http_server, // connect to an HTTP server
        connect_sse_server,  // connect to an SSE server
    },
};

/// Configuration bundle for connection operations
struct ConnectionConfig {
    config: Arc<crate::core::models::Config>,
    database: Option<Arc<crate::config::database::Database>>,
    runtime_cache: Option<Arc<crate::runtime::RuntimeCache>>,
    server_type: ServerType,
}

/// Connection state update events for parallel processing
#[derive(Debug)]
pub enum ConnectionEvent {
    Connecting {
        server_id: String,
        instance_id: String,
    },
    Connected {
        server_id: String,
        instance_id: String,
        service: RunningService<RoleClient, ()>,
        tools: Vec<Tool>,
        capabilities: Option<rmcp::model::ServerCapabilities>,
        process_id: Option<u32>,
    },
    Failed {
        server_id: String,
        instance_id: String,
        error: String,
    },
    TokenStore {
        server_id: String,
        instance_id: String,
        token: CancellationToken,
    },
}

/// Parallel connection manager that separates connection logic from state management
pub struct ParallelConnectionManager {
    pool: Arc<Mutex<UpstreamConnectionPool>>,
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,
}

impl ParallelConnectionManager {
    /// Create a new parallel connection manager
    pub fn new(pool: Arc<Mutex<UpstreamConnectionPool>>) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Start the centralized state management task
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            Self::handle_connection_events(pool_clone, event_rx).await;
        });

        Self { pool, event_tx }
    }

    /// High-performance parallel connection to all servers
    pub async fn trigger_connect_all_parallel(&self) -> Result<()> {
        // Get server instances (read-only operation)
        let server_instances = {
            let pool = self.pool.lock().await;
            pool.connections
                .keys()
                .filter_map(|server_id| {
                    pool.get_default_instance_id(server_id)
                        .ok()
                        .map(|instance_id| (server_id.clone(), instance_id))
                })
                .collect::<Vec<_>>()
        };

        if server_instances.is_empty() {
            tracing::debug!("No servers to connect to");
            return Ok(());
        }

        tracing::info!("Starting parallel connection to {} servers", server_instances.len());

        // Clone needed values outside the closure to avoid lifetime issues
        let event_tx = self.event_tx.clone();
        let pool = self.pool.clone();

        // Use unified sync framework for concurrent connections
        let sync_result = SyncHelper::execute_concurrent_sync(
            server_instances,
            "server_connections",
            4, // max concurrent connections
            move |(server_id, instance_id)| {
                let event_tx = event_tx.clone();
                let pool = pool.clone();
                async move { Self::connect_single_server(server_id, instance_id, pool, event_tx).await }
            },
        )
        .await;

        tracing::info!(
            "Parallel connection completed: {}/{} tasks successful ({:.1}% success rate)",
            sync_result.synced,
            sync_result.processed,
            sync_result.success_rate()
        );

        Ok(())
    }

    /// Single server connection task (stateless, parallelizable)
    async fn connect_single_server(
        server_id: String,
        instance_id: String,
        pool: Arc<Mutex<UpstreamConnectionPool>>,
        event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    ) -> Result<()> {
        // Send connecting event
        Self::send_event(
            &event_tx,
            ConnectionEvent::Connecting {
                server_id: server_id.clone(),
                instance_id: instance_id.clone(),
            },
        );

        // Get configuration and dependencies (read-only)
        let connection_config = match Self::get_connection_config(&pool, &server_id).await {
            Ok(config) => config,
            Err(error_msg) => {
                Self::send_event(
                    &event_tx,
                    ConnectionEvent::Failed {
                        server_id,
                        instance_id,
                        error: error_msg,
                    },
                );
                return Ok(());
            }
        };

        // Perform actual connection (stateless operation)
        match Self::perform_connection(
            &server_id,
            &instance_id,
            &connection_config.config,
            connection_config.database,
            connection_config.runtime_cache,
            connection_config.server_type,
            event_tx.clone(),
        )
        .await
        {
            Ok((service, tools, capabilities, process_id)) => {
                Self::send_event(
                    &event_tx,
                    ConnectionEvent::Connected {
                        server_id,
                        instance_id,
                        service,
                        tools,
                        capabilities,
                        process_id,
                    },
                );
            }
            Err(e) => {
                Self::send_event(
                    &event_tx,
                    ConnectionEvent::Failed {
                        server_id,
                        instance_id,
                        error: e.to_string(),
                    },
                );
            }
        }

        Ok(())
    }

    /// Helper to extract connection configuration from pool
    async fn get_connection_config(
        pool: &Arc<Mutex<UpstreamConnectionPool>>,
        server_id: &str,
    ) -> Result<ConnectionConfig, String> {
        let pool = pool.lock().await;
        let server_config = match pool.config.mcp_servers.get(server_id) {
            Some(config) => config,
            None => {
                return Err(format!("Server configuration for '{}' not found", server_id));
            }
        };

        Ok(ConnectionConfig {
            config: pool.config.clone(),
            database: pool.database.clone(),
            runtime_cache: pool.runtime_cache.clone(),
            server_type: server_config.kind,
        })
    }

    /// Helper to send events with error handling
    fn send_event(
        event_tx: &mpsc::UnboundedSender<ConnectionEvent>,
        event: ConnectionEvent,
    ) {
        let _ = event_tx.send(event);
    }

    /// Pure connection logic extracted from existing methods
    /// This function is stateless and can be called in parallel
    async fn perform_connection(
        server_id: &str,
        instance_id: &str,
        config: &Arc<crate::core::models::Config>,
        database: Option<Arc<crate::config::database::Database>>,
        runtime_cache: Option<Arc<crate::runtime::RuntimeCache>>,
        server_type: ServerType,
        event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    ) -> Result<(
        RunningService<RoleClient, ()>,
        Vec<Tool>,
        Option<rmcp::model::ServerCapabilities>,
        Option<u32>,
    )> {
        let server_config = config.mcp_servers.get(server_id).unwrap();

        match server_type {
            ServerType::Stdio => {
                Self::perform_stdio_connection(server_id, instance_id, server_config, database, runtime_cache, event_tx)
                    .await
            }
            ServerType::Sse => {
                let (service, tools, capabilities) = connect_sse_server(server_id, server_config).await?;
                Ok((service, tools, capabilities, None))
            }
            ServerType::StreamableHttp => {
                Self::perform_http_connection(server_id, server_config, TransportType::StreamableHttp).await
            }
        }
    }

    /// Perform stdio connection with proper token management
    async fn perform_stdio_connection(
        server_id: &str,
        instance_id: &str,
        server_config: &crate::core::models::MCPServerConfig,
        database: Option<Arc<crate::config::database::Database>>,
        runtime_cache: Option<Arc<crate::runtime::RuntimeCache>>,
        event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    ) -> Result<(
        RunningService<RoleClient, ()>,
        Vec<Tool>,
        Option<rmcp::model::ServerCapabilities>,
        Option<u32>,
    )> {
        // Create cancellation token
        let ct = CancellationToken::new();

        // Store the token for later use
        let _ = event_tx.send(ConnectionEvent::TokenStore {
            server_id: server_id.to_string(),
            instance_id: instance_id.to_string(),
            token: ct.clone(),
        });

        // Get database pool if available
        let database_pool = database.as_ref().map(|db| &db.pool);

        // Use the unified transport interface to reduce code duplication
        crate::core::transport::unified::connect_server(
            server_id,
            server_config,
            ServerType::Stdio,
            TransportType::Stdio,
            Some(ct),
            database_pool,
            runtime_cache.as_ref().map(|rc| rc.as_ref()),
        )
        .await
    }

    /// Perform HTTP connection
    async fn perform_http_connection(
        server_id: &str,
        server_config: &crate::core::models::MCPServerConfig,
        transport_type: TransportType,
    ) -> Result<(
        RunningService<RoleClient, ()>,
        Vec<Tool>,
        Option<rmcp::model::ServerCapabilities>,
        Option<u32>,
    )> {
        let (service, tools, capabilities) = connect_http_server(server_id, server_config, transport_type).await?;

        Ok((service, tools, capabilities, None))
    }

    /// Centralized event handler for connection state updates
    async fn handle_connection_events(
        pool: Arc<Mutex<UpstreamConnectionPool>>,
        mut event_rx: mpsc::UnboundedReceiver<ConnectionEvent>,
    ) {
        while let Some(event) = event_rx.recv().await {
            match event {
                // Handle connection initializing
                ConnectionEvent::Connecting { server_id, instance_id } => {
                    let mut pool = pool.lock().await;
                    if let Ok(conn) = pool.get_instance_mut(&server_id, &instance_id) {
                        conn.update_initializing();
                        tracing::debug!("Updated '{}' instance '{}' to connecting state", server_id, instance_id);
                    }
                }

                // Handle connection success
                ConnectionEvent::Connected {
                    server_id,
                    instance_id,
                    service,
                    tools,
                    capabilities,
                    process_id,
                } => {
                    let mut pool = pool.lock().await;

                    // Update connection with service
                    pool.update_connection(&server_id, &instance_id, service, tools, capabilities);

                    // Update process ID if available
                    if let Some(pid) = process_id {
                        if let Ok(conn) = pool.get_instance_mut(&server_id, &instance_id) {
                            conn.process_id = Some(pid);
                        }
                    }

                    tracing::debug!("Successfully connected to '{}' instance '{}'", server_id, instance_id);

                    // Get the actual server name for the event
                    let actual_server_name = if let Some(db) = &pool.database {
                        crate::config::operations::utils::get_server_name(&db.pool, &server_id)
                            .await
                            .unwrap_or_else(|_| server_id.clone())
                    } else {
                        server_id.clone()
                    };

                    // Release the pool lock before publishing the event
                    drop(pool);

                    // Publish connection success event for capability sync
                    crate::core::events::EventBus::global().publish(
                        crate::core::events::Event::ServerConnectionStartupCompleted {
                            server_id: server_id.clone(),
                            server_name: actual_server_name,
                            success: true,
                            error: None,
                        },
                    );
                }

                // Handle connection failure
                ConnectionEvent::Failed {
                    server_id,
                    instance_id,
                    error,
                } => {
                    let mut pool = pool.lock().await;
                    if let Ok(conn) = pool.get_instance_mut(&server_id, &instance_id) {
                        conn.update_error_with_escalation(error.clone());
                        tracing::error!(
                            "Failed to connect to '{}' instance '{}': {}",
                            server_id,
                            instance_id,
                            error
                        );
                    }

                    // Get the actual server name for the event
                    let actual_server_name = if let Some(db) = &pool.database {
                        crate::config::operations::utils::get_server_name(&db.pool, &server_id)
                            .await
                            .unwrap_or_else(|_| server_id.clone())
                    } else {
                        server_id.clone()
                    };

                    // Release the pool lock before publishing the event
                    drop(pool);

                    // Publish connection failure event
                    crate::core::events::EventBus::global().publish(
                        crate::core::events::Event::ServerConnectionStartupCompleted {
                            server_id: server_id.clone(),
                            server_name: actual_server_name,
                            success: false,
                            error: Some(error.to_string()),
                        },
                    );
                }

                // Store the token for later use
                ConnectionEvent::TokenStore {
                    server_id,
                    instance_id,
                    token,
                } => {
                    let mut pool = pool.lock().await;
                    pool.cancellation_tokens
                        .entry(server_id.clone())
                        .or_default()
                        .insert(instance_id.clone(), token);
                    tracing::debug!(
                        "Stored cancellation token for '{}' instance '{}'",
                        server_id,
                        instance_id
                    );
                }
            }
        }
    }
}

impl UpstreamConnectionPool {
    /// Create a parallel connection manager for this pool
    pub fn create_parallel_manager(pool: Arc<Mutex<Self>>) -> ParallelConnectionManager {
        ParallelConnectionManager::new(pool)
    }

    /// Trigger parallel connection to all servers using the new manager
    pub async fn trigger_connect_all_parallel_new(pool: Arc<Mutex<Self>>) -> Result<()> {
        let manager = Self::create_parallel_manager(pool);
        manager.trigger_connect_all_parallel().await
    }
}
