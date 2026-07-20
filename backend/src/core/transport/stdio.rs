// Stdio transport implementation for core
// Contains functions for connecting to stdio-based MCP servers

use anyhow::{Context, Result};
use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    transport::{IntoTransport, TokioChildProcess},
};
use sysinfo;
use tokio::{io::AsyncReadExt, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::core::foundation::utils::{
    get_connection_timeout, // connection timeout
    get_tools_timeout,      // tools timeout
    prepare_command,        // prepare command
};
use crate::core::models::MCPServerConfig;

/// Prepare and configure command with environment variables.
///
/// Uses the unified resolver (managed → enriched PATH) to find the
/// executable.  For python commands, adds a transport-specific UV
/// Python fallback as a last resort.
async fn prepare_server_command(server_config: &MCPServerConfig) -> Result<(tokio::process::Command, String)> {
    let command = server_config
        .command
        .as_ref()
        .context("Command not specified for stdio server")?;

    let transformed_command = crate::core::foundation::utils::transform_command(command);

    let paths = crate::common::paths::global_paths();
    let resolver = crate::runtime::CommandResolver::new(paths);

    tracing::debug!(
        "[stdio:resolve] command='{}' (original='{}') enriched_PATH={}",
        transformed_command,
        command,
        resolver.enriched_path()
    );

    // Use the unified resolver (managed → enriched PATH → UV Python fallback)
    let resolved = resolver.resolve(&transformed_command);

    let cmd = match resolved {
        Some(ref r) => {
            tracing::debug!(
                "[stdio:resolve] Resolved '{}' via {:?} → {}",
                transformed_command,
                r.source,
                r.path.display()
            );
            prepare_command(&r.path.to_string_lossy(), server_config.args.as_ref())
        }
        None => {
            tracing::warn!(
                "[stdio:resolve] Command '{}' not found, using as-is",
                transformed_command
            );
            prepare_command(&transformed_command, server_config.args.as_ref())
        }
    };

    // Ensure runtime directories exist
    paths
        .ensure_directories()
        .context("Failed to create necessary directories")?;

    Ok((cmd, transformed_command))
}

/// Apply environment variables to command
async fn setup_command_environment(
    cmd: &mut tokio::process::Command,
    server_config: &MCPServerConfig,
    transformed_command: &str,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
) -> Result<()> {
    // Add environment variables if any
    if let Some(env) = &server_config.env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    crate::common::env::sanitize_ambient_network_environment(cmd);

    // Prepare environment variables based on runtime configuration
    if let Err(e) = crate::config::runtime::prepare_command_env_with_db(cmd, transformed_command, database_pool).await {
        tracing::warn!(
            "Failed to prepare environment for command '{}': {}",
            transformed_command,
            e
        );
        tracing::info!("Attempting to continue with default environment");
    } else {
        tracing::debug!("Environment prepared for command '{}'", transformed_command);
    }

    Ok(())
}

/// Connect to server with timeout handling
async fn connect_with_timeout(
    cmd: tokio::process::Command,
    ct: CancellationToken,
    server_name: &str,
    connection_timeout: std::time::Duration,
) -> Result<crate::core::transport::ClientService> {
    if let Ok(path) = std::env::var("PATH") {
        tracing::debug!("[PATH_DEBUG] Server '{}' spawning with PATH: {}", server_name, path);
    }

    // Use builder to capture stderr for logging
    let (child_process, stderr_handle) = TokioChildProcess::builder(cmd)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            tracing::error!("Failed to create child process for '{}': {}", server_name, e);
            ct.cancel();
            anyhow::anyhow!("Failed to create child process: {e}")
        })?;

    // Spawn stderr monitoring task if stderr is available
    if let Some(mut stderr) = stderr_handle {
        let server_label = server_name.to_string();
        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            loop {
                match stderr.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]);
                        for line in output.lines() {
                            if !line.trim().is_empty() {
                                // Transport layer maintains neutral phrasing to avoid mismatch between name and argument
                                tracing::info!("Log from server {}: {}", server_label, line.trim());
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Stderr read error for server '{}': {}", server_label, e);
                        break;
                    }
                }
            }
        });
    }

    initialize_service_with_timeout(child_process, ct, server_name, connection_timeout).await
}

pub(crate) async fn connect_stdio_initialized_for_validation(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
) -> Result<crate::core::transport::ClientService> {
    let (mut cmd, transformed_command) = prepare_server_command(server_config).await?;
    setup_command_environment(&mut cmd, server_config, &transformed_command, database_pool).await?;
    let command = server_config
        .command
        .as_ref()
        .expect("command already validated in prepare_server_command");

    connect_with_timeout(cmd, ct, server_name, get_connection_timeout(command)).await
}

async fn initialize_service_with_timeout<T, E, A>(
    transport: T,
    ct: CancellationToken,
    server_name: &str,
    connection_timeout: std::time::Duration,
) -> Result<crate::core::transport::ClientService>
where
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    crate::core::transport::unified::initialize_client_service(server_name, transport, ct, connection_timeout).await
}

/// Get tools from service with timeout handling
async fn get_tools_with_timeout(
    service: &crate::core::transport::ClientService,
    server_name: &str,
    tools_timeout: std::time::Duration,
    ct: CancellationToken,
) -> Result<Vec<Tool>> {
    timeout(tools_timeout, service.list_all_tools())
        .await
        .map_err(|_| {
            ct.cancel();
            anyhow::anyhow!(
                "Timeout listing tools for server '{server_name}' after {}s",
                tools_timeout.as_secs()
            )
        })?
        .map_err(|e| anyhow::anyhow!("Failed to list tools: {e}"))
}

/// Cancel service with timeout to prevent hanging
async fn cancel_service_safely(service: crate::core::transport::ClientService) {
    match tokio::time::timeout(std::time::Duration::from_secs(3), service.cancel()).await {
        Ok(Ok(_)) => {
            tracing::debug!("Service cancelled successfully");
        }
        Ok(Err(cancel_err)) => {
            tracing::warn!("Error cancelling service: {}", cancel_err);
        }
        Err(_) => {
            tracing::warn!("Service cancellation timeout, resources may be leaked");
        }
    }
}

/// Universal stdio server connection function with optional database support
pub async fn connect_stdio_server(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    connect_stdio_server_inner(server_name, server_config, ct, database_pool, None).await
}

pub async fn connect_stdio_server_with_timeouts(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    connection_timeout: std::time::Duration,
    tools_timeout: std::time::Duration,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    connect_stdio_server_inner(
        server_name,
        server_config,
        ct,
        database_pool,
        Some((connection_timeout, tools_timeout)),
    )
    .await
}

async fn connect_stdio_server_inner(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    custom_timeouts: Option<(std::time::Duration, std::time::Duration)>,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    // Prepare command — unified resolver handles managed + enriched PATH
    let (mut cmd, transformed_command) = prepare_server_command(server_config).await?;

    // Setup environment variables
    setup_command_environment(&mut cmd, server_config, &transformed_command, database_pool).await?;

    let command = server_config
        .command
        .as_ref()
        .expect("command already validated in prepare_server_command");
    let (connection_timeout, tools_timeout) =
        custom_timeouts.unwrap_or_else(|| (get_connection_timeout(command), get_tools_timeout(command)));

    tracing::debug!(
        "Using timeouts for '{}': connection={}s, tools={}s",
        server_name,
        connection_timeout.as_secs(),
        tools_timeout.as_secs()
    );

    // Connect to server with timeout handling
    // server_name here is a display label (e.g., "Gitmcp (SERVxxxx)") provided by the caller
    let service = connect_with_timeout(cmd, ct.clone(), server_name, connection_timeout).await?;

    // Get tools with timeout handling
    let tools = match get_tools_with_timeout(&service, server_name, tools_timeout, ct.clone()).await {
        Ok(tools) => tools,
        Err(e) => {
            cancel_service_safely(service).await;
            return Err(e);
        }
    };

    // Get process ID and capabilities
    let pid = get_process_id_for_server(server_name, server_config).await;
    let capabilities = service.peer_info().map(|info| info.capabilities.clone());

    tracing::debug!(
        "Connected to server '{}', found {} tools, capabilities: {:?}, process ID: {:?}",
        server_name,
        tools.len(),
        capabilities
            .as_ref()
            .map(|c| format!("resources={}", c.resources.is_some())),
        pid
    );

    Ok((service, tools, capabilities, pid))
}

/// Helper function to get process ID for a server
async fn get_process_id_for_server(
    server_name: &str,
    server_config: &MCPServerConfig,
) -> Option<u32> {
    // Wait a short time for the process to fully start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    if let Some(command) = &server_config.command {
        // Get the command name (last part of the path)
        let cmd_name = command.split('/').next_back().unwrap_or(command);

        // Create a new System instance and only refresh process info
        let mut system = sysinfo::System::new();
        system.refresh_processes();

        // Find the process by name
        for (pid, process) in system.processes() {
            if process.name() == cmd_name {
                tracing::debug!(
                    "Found process for server '{}': PID={}, name={}",
                    server_name,
                    pid,
                    process.name()
                );
                return Some(pid.as_u32());
            }
        }

        tracing::debug!(
            "Process not found for server '{}' with command '{}'",
            server_name,
            cmd_name
        );
    }

    None
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use rmcp::{
        ErrorData, RoleServer, ServerHandler, ServiceExt,
        model::{ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo},
        service::RequestContext,
    };
    use tokio_util::sync::CancellationToken;

    use super::{get_tools_with_timeout, initialize_service_with_timeout};

    #[derive(Clone)]
    struct FailingToolsServer {
        tools_list_calls: Arc<AtomicUsize>,
    }

    impl ServerHandler for FailingToolsServer {
        fn get_info(&self) -> ServerInfo {
            ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
        }

        async fn list_tools(
            &self,
            _request: Option<PaginatedRequestParams>,
            _context: RequestContext<RoleServer>,
        ) -> Result<ListToolsResult, ErrorData> {
            self.tools_list_calls.fetch_add(1, Ordering::SeqCst);
            Err(ErrorData::internal_error("tools/list is unavailable", None))
        }
    }

    #[tokio::test]
    async fn validation_initialization_skips_tools_while_production_bootstrap_lists_once() {
        let tools_list_calls = Arc::new(AtomicUsize::new(0));
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_calls = tools_list_calls.clone();
        let server_handle = tokio::spawn(async move {
            let service = FailingToolsServer {
                tools_list_calls: server_calls,
            }
            .serve(server_transport)
            .await?;
            service.waiting().await?;
            Ok::<(), anyhow::Error>(())
        });
        let cancellation = CancellationToken::new();

        let service = initialize_service_with_timeout(
            client_transport,
            cancellation.clone(),
            "validation",
            std::time::Duration::from_secs(1),
        )
        .await
        .expect("initialize validation owner");

        assert!(service.peer_info().is_some());
        assert_eq!(tools_list_calls.load(Ordering::SeqCst), 0);

        let result =
            get_tools_with_timeout(&service, "production", std::time::Duration::from_secs(1), cancellation).await;

        assert!(result.is_err());
        assert_eq!(tools_list_calls.load(Ordering::SeqCst), 1);

        service.cancel().await.expect("cancel validation owner");
        server_handle
            .await
            .expect("join validation server")
            .expect("validation server shutdown");
    }
}
