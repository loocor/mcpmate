use anyhow::Result;
use clap::Parser;
use mcpmate::common::MCPMatePaths;
use std::{env, fs::OpenOptions, io::Write};

use mcpmate::config::registry::start_registry_sync_service;
use mcpmate::core::proxy::{
    Args,
    init::{setup_audit_database, setup_database, setup_logging, setup_proxy_server_with_params},
    startup::{start_api_server, start_background_connections, start_proxy_server},
};
use mcpmate::system::config::init_port_config;

fn write_bootstrap_log(message: &str) {
    let paths = MCPMatePaths::new().ok();
    let log_path = paths
        .as_ref()
        .map(|paths| paths.logs_dir().join("mcpmate-core-bootstrap.log"))
        .or_else(|| dirs::home_dir().map(|home| home.join(".mcpmate").join("logs").join("mcpmate-core-bootstrap.log")));

    if let Some(log_path) = log_path {
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = writeln!(file, "{}", message);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    let current_dir = env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|err| format!("<error:{err}>"));
    let current_exe = env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|err| format!("<error:{err}>"));
    let data_dir = env::var("MCPMATE_DATA_DIR").unwrap_or_else(|_| "<unset>".to_string());
    let desktop_token = env::var("MCPMATE_DESKTOP_MANAGED_TOKEN").unwrap_or_else(|_| "<unset>".to_string());
    write_bootstrap_log(&format!(
        "bootstrap:start pid={} exe={} cwd={} data_dir={} api_port={} mcp_port={} desktop_token={}",
        std::process::id(),
        current_exe,
        current_dir,
        data_dir,
        args.api_port,
        args.mcp_port,
        desktop_token,
    ));

    // Validate command line arguments
    if let Err(e) = args.validate() {
        write_bootstrap_log(&format!("bootstrap:invalid_args error={}", e));
        eprintln!("Invalid arguments: {}", e);
        std::process::exit(1);
    }

    // Get startup mode from arguments
    let startup_mode = args.get_startup_mode();
    tracing::info!("Starting MCPMate with mode: {:?}", startup_mode);

    // Initialize runtime port configuration from command line arguments
    init_port_config(args.api_port, args.mcp_port);

    // Setup logging
    setup_logging(&args)?;
    write_bootstrap_log("bootstrap:logging_initialized");

    // Initialize metrics reporting
    mcpmate::core::foundation::monitor::initialize_metrics_reporting();

    // Setup database
    let db = setup_database().await?;
    write_bootstrap_log("bootstrap:database_initialized");
    let audit_db = setup_audit_database().await?;
    write_bootstrap_log("bootstrap:audit_database_initialized");

    // Start registry sync service (background task)
    let registry_pool = db.pool.clone();
    tokio::spawn(async move {
        start_registry_sync_service(registry_pool);
    });

    // Setup proxy server with startup parameters
    let (proxy_arc1, proxy_arc2) = setup_proxy_server_with_params(db, audit_db, &startup_mode).await?;
    write_bootstrap_log("bootstrap:proxy_initialized");

    // Start background connections
    start_background_connections(&proxy_arc1, proxy_arc2.clone()).await?;
    write_bootstrap_log("bootstrap:background_connections_started");

    // Start proxy server - we need to get a mutable reference from Arc
    let mut proxy_clone = (*proxy_arc1).clone();
    let mcp_server_handle = start_proxy_server(&mut proxy_clone, &args).await?;
    write_bootstrap_log("bootstrap:mcp_server_started");

    // Start API server
    let (api_task, api_cancellation_token) = start_api_server(proxy_arc2.clone(), &args).await?;
    write_bootstrap_log("bootstrap:api_server_started");

    tracing::info!("Servers started. Press Ctrl+C to stop.");
    write_bootstrap_log("bootstrap:startup_complete");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down...");

    // Step 1: Initiate MCP server shutdown first
    tracing::info!("Step 1: Initiating MCP server shutdown...");
    proxy_clone.initiate_shutdown().await?;

    // Step 2: Wait for MCP server to complete gracefully (if handle is available)
    if let Some(mcp_handle) = mcp_server_handle {
        match tokio::time::timeout(std::time::Duration::from_secs(5), mcp_handle).await {
            Ok(Ok(Ok(()))) => {
                tracing::info!("MCP server shutdown completed successfully");
            }
            Ok(Ok(Err(e))) => {
                tracing::warn!("MCP server completed with error: {}", e);
            }
            Ok(Err(e)) => {
                tracing::warn!("MCP server task panicked: {}", e);
            }
            Err(_) => {
                tracing::warn!("MCP server shutdown timed out after 5 seconds");
            }
        }
    } else {
        tracing::info!("MCP server doesn't support graceful shutdown monitoring, proceeding...");
    }

    // Step 2: Shutdown API server after MCP server is done
    tracing::info!("Step 2: Initiating API server shutdown...");
    api_cancellation_token.cancel();

    match tokio::time::timeout(std::time::Duration::from_secs(5), api_task).await {
        Ok(Ok(())) => {
            tracing::info!("API server shutdown completed successfully");
        }
        Ok(Err(e)) => {
            tracing::warn!("API server task completed with error: {}", e);
        }
        Err(_) => {
            tracing::warn!("API server shutdown timed out after 5 seconds");
        }
    }

    // Step 4: Complete proxy server cleanup (connections, etc.)
    tracing::info!("Step 3: Completing proxy server cleanup...");
    proxy_clone.complete_shutdown().await?;

    Ok(())
}
