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
    common::server::{ServerType, TransportType},
    recore::transport::{
        connect_http_server,                     // connect to an HTTP server
        connect_sse_server,                      // connect to an SSE server
        connect_stdio_server_with_ct_and_db, // connect to a stdio server with a cancellation token and a database
        connect_stdio_server_with_runtime_cache, // connect to a stdio server with a runtime cache
    },
};

/// Connection state update events for parallel processing
#[derive(Debug)]
pub enum ConnectionEvent {
    Connecting {
        server_name: String,
        instance_id: String,
    },
    Connected {
        server_name: String,
        instance_id: String,
        service: RunningService<RoleClient, ()>,
        tools: Vec<Tool>,
        capabilities: Option<rmcp::model::ServerCapabilities>,
        process_id: Option<u32>,
    },
    Failed {
        server_name: String,
        instance_id: String,
        error: String,
    },
    TokenStore {
        server_name: String,
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
                .filter_map(|name| {
                    pool.get_default_instance_id(name)
                        .ok()
                        .map(|instance_id| (name.clone(), instance_id))
                })
                .collect::<Vec<_>>()
        };

        if server_instances.is_empty() {
            tracing::debug!("No servers to connect to");
            return Ok(());
        }

        tracing::info!(
            "Starting parallel connection to {} servers",
            server_instances.len()
        );

        // Create parallel connection tasks
        let connection_tasks: Vec<_> = server_instances
            .into_iter()
            .map(|(server_name, instance_id)| {
                let event_tx = self.event_tx.clone();
                let pool = self.pool.clone();

                tokio::spawn(Self::connect_single_server(
                    server_name,
                    instance_id,
                    pool,
                    event_tx,
                ))
            })
            .collect();

        // Wait for all connection tasks to complete
        let results = futures::future::join_all(connection_tasks).await;

        // Count successful tasks
        let successful_tasks = results.iter().filter(|r| r.is_ok()).count();
        tracing::info!(
            "Parallel connection completed: {}/{} tasks successful",
            successful_tasks,
            results.len()
        );

        Ok(())
    }

    /// Single server connection task (stateless, parallelizable)
    async fn connect_single_server(
        server_name: String,
        instance_id: String,
        pool: Arc<Mutex<UpstreamConnectionPool>>,
        event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    ) -> Result<()> {
        // Send connecting event
        let _ = event_tx.send(ConnectionEvent::Connecting {
            server_name: server_name.clone(),
            instance_id: instance_id.clone(),
        });

        // Get configuration and dependencies (read-only)
        let (config, database, runtime_cache, server_type) = {
            let pool = pool.lock().await;
            let server_config = match pool.config.mcp_servers.get(&server_name) {
                Some(config) => config,
                None => {
                    let error_msg = format!("Server configuration for '{}' not found", server_name);
                    let _ = event_tx.send(ConnectionEvent::Failed {
                        server_name,
                        instance_id,
                        error: error_msg,
                    });
                    return Ok(());
                }
            };

            (
                pool.config.clone(),
                pool.database.clone(),
                pool.runtime_cache.clone(),
                server_config.kind,
            )
        };

        // Perform actual connection (stateless operation)
        match Self::perform_connection(
            &server_name,
            &instance_id,
            &config,
            database,
            runtime_cache,
            server_type,
            event_tx.clone(),
        )
        .await
        {
            Ok((service, tools, capabilities, process_id)) => {
                let _ = event_tx.send(ConnectionEvent::Connected {
                    server_name,
                    instance_id,
                    service,
                    tools,
                    capabilities,
                    process_id,
                });
            }
            Err(e) => {
                let _ = event_tx.send(ConnectionEvent::Failed {
                    server_name,
                    instance_id,
                    error: e.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Pure connection logic extracted from existing methods
    /// This function is stateless and can be called in parallel
    async fn perform_connection(
        server_name: &str,
        instance_id: &str,
        config: &Arc<crate::recore::models::Config>,
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
        let server_config = config.mcp_servers.get(server_name).unwrap();

        match server_type {
            ServerType::Stdio => {
                Self::perform_stdio_connection(
                    server_name,
                    instance_id,
                    server_config,
                    database,
                    runtime_cache,
                    event_tx,
                )
                .await
            }
            ServerType::Sse => {
                let (service, tools, capabilities) =
                    connect_sse_server(server_name, server_config).await?;
                Ok((service, tools, capabilities, None))
            }
            ServerType::StreamableHttp => {
                Self::perform_http_connection(
                    server_name,
                    server_config,
                    TransportType::StreamableHttp,
                )
                .await
            }
        }
    }

    /// Perform stdio connection with proper token management
    async fn perform_stdio_connection(
        server_name: &str,
        instance_id: &str,
        server_config: &crate::recore::models::MCPServerConfig,
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
            server_name: server_name.to_string(),
            instance_id: instance_id.to_string(),
            token: ct.clone(),
        });

        // Get database pool if available
        let database_pool = database.as_ref().map(|db| &db.pool);

        // Connect using recore transport
        if let Some(runtime_cache) = runtime_cache {
            connect_stdio_server_with_runtime_cache(
                server_name,
                server_config,
                ct,
                database_pool,
                &runtime_cache,
            )
            .await
        } else {
            connect_stdio_server_with_ct_and_db(server_name, server_config, ct, database_pool).await
        }
    }

    /// Perform HTTP connection
    async fn perform_http_connection(
        server_name: &str,
        server_config: &crate::recore::models::MCPServerConfig,
        transport_type: TransportType,
    ) -> Result<(
        RunningService<RoleClient, ()>,
        Vec<Tool>,
        Option<rmcp::model::ServerCapabilities>,
        Option<u32>,
    )> {
        let (service, tools, capabilities) =
            connect_http_server(server_name, server_config, transport_type).await?;

        Ok((service, tools, capabilities, None))
    }

    /// Centralized event handler for connection state updates
    async fn handle_connection_events(
        pool: Arc<Mutex<UpstreamConnectionPool>>,
        mut event_rx: mpsc::UnboundedReceiver<ConnectionEvent>,
    ) {
        while let Some(event) = event_rx.recv().await {
            match event {
                ConnectionEvent::Connecting {
                    server_name,
                    instance_id,
                } => {
                    let mut pool = pool.lock().await;
                    if let Ok(conn) = pool.get_instance_mut(&server_name, &instance_id) {
                        conn.update_initializing();
                        tracing::debug!(
                            "Updated '{}' instance '{}' to connecting state",
                            server_name,
                            instance_id
                        );
                    }
                }
                ConnectionEvent::Connected {
                    server_name,
                    instance_id,
                    service,
                    tools,
                    capabilities,
                    process_id,
                } => {
                    let mut pool = pool.lock().await;

                    // Update connection with service
                    pool.update_connection(
                        &server_name,
                        &instance_id,
                        service,
                        tools,
                        capabilities,
                    );

                    // Update process ID if available
                    if let Some(pid) = process_id {
                        if let Ok(conn) = pool.get_instance_mut(&server_name, &instance_id) {
                            conn.process_id = Some(pid);
                        }
                    }

                    tracing::info!(
                        "Successfully connected to '{}' instance '{}'",
                        server_name,
                        instance_id
                    );
                }
                ConnectionEvent::Failed {
                    server_name,
                    instance_id,
                    error,
                } => {
                    let mut pool = pool.lock().await;
                    if let Ok(conn) = pool.get_instance_mut(&server_name, &instance_id) {
                        conn.update_error(error.clone());
                        tracing::error!(
                            "Failed to connect to '{}' instance '{}': {}",
                            server_name,
                            instance_id,
                            error
                        );
                    }
                }
                ConnectionEvent::TokenStore {
                    server_name,
                    instance_id,
                    token,
                } => {
                    let mut pool = pool.lock().await;
                    pool.cancellation_tokens
                        .entry(server_name.clone())
                        .or_default()
                        .insert(instance_id.clone(), token);
                    tracing::debug!(
                        "Stored cancellation token for '{}' instance '{}'",
                        server_name,
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
