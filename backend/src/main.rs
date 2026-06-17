use anyhow::Result;
use clap::Parser;

use mcpmate::common::constants::ports;
use mcpmate::common::startup_diagnostics;
use mcpmate::core::proxy::{
    Args,
    init::{setup_audit_database, setup_database, setup_logging, setup_proxy_server_with_params},
    startup::{start_api_server, start_background_connections, start_proxy_server},
};
use mcpmate::system::config::init_port_config;
use mcpmate::system::port_recovery::{LocalhostPortSelection, recover_available_localhost_ports_selectively};
use mcpmate::system::settings::{
    apply_settings_with_effects_for_paths, get_settings_sync, spawn_mcp_port_reapply_result_logger,
};

#[derive(Debug, Clone, Copy, Default)]
struct ExplicitPortArgs {
    api_port: bool,
    mcp_port: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let explicit_port_args = explicit_port_args();
    let mut args = Args::parse();
    let settings = get_settings_sync()?;

    if !explicit_port_args.api_port && args.api_port == ports::API_PORT {
        args.api_port = settings.api_port;
    }
    if !explicit_port_args.mcp_port && args.mcp_port == ports::MCP_PORT {
        args.mcp_port = settings.mcp_port;
    }

    // Validate command line arguments
    if let Err(e) = args.validate() {
        eprintln!("Invalid arguments: {}", e);
        std::process::exit(1);
    }

    // Setup logging
    setup_logging(&args)?;

    if !explicit_port_args.api_port || !explicit_port_args.mcp_port {
        let recovered = recover_available_localhost_ports_selectively(
            args.api_port,
            args.mcp_port,
            !explicit_port_args.api_port,
            !explicit_port_args.mcp_port,
        )?;
        if recovered.changed_from(args.api_port, args.mcp_port) {
            record_port_recovery(args.api_port, args.mcp_port, recovered);
            let mut next_settings = settings.clone();
            if !explicit_port_args.api_port {
                next_settings.api_port = recovered.api_port;
            }
            if !explicit_port_args.mcp_port {
                next_settings.mcp_port = recovered.mcp_port;
            }
            let applied = apply_settings_with_effects_for_paths(
                mcpmate::common::paths::global_paths(),
                &settings,
                &next_settings,
                None,
            )
            .await?;
            if let Some(task) = applied.client_reapply_task {
                spawn_mcp_port_reapply_result_logger(task);
            }
            args.api_port = recovered.api_port;
            args.mcp_port = recovered.mcp_port;
        }
    }

    // Initialize runtime port configuration from command line arguments
    init_port_config(args.api_port, args.mcp_port);

    // Get startup mode from arguments
    let startup_mode = args.get_startup_mode();
    tracing::info!("Starting MCPMate with mode: {:?}", startup_mode);

    // Initialize metrics reporting
    mcpmate::core::foundation::monitor::initialize_metrics_reporting();

    // Setup database
    let db = match setup_database().await {
        Ok(db) => db,
        Err(error) => {
            startup_diagnostics::error_fatal("database_setup", "database_setup_failed", &error);
            return Err(error);
        }
    };
    let audit_db = setup_audit_database().await?;

    // Setup proxy server with startup parameters
    let (proxy_arc1, proxy_arc2) = match setup_proxy_server_with_params(db, audit_db, &startup_mode).await {
        Ok(proxy) => proxy,
        Err(error) => {
            startup_diagnostics::error_fatal("proxy_setup", "proxy_setup_failed", &error);
            return Err(error);
        }
    };

    // Start background connections
    if let Err(error) = start_background_connections(&proxy_arc1, proxy_arc2.clone()).await {
        startup_diagnostics::error_fatal(
            "background_connections_start",
            "background_connections_start_failed",
            &error,
        );
        return Err(error);
    }

    // Start proxy server - we need to get a mutable reference from Arc
    let mut proxy_clone = (*proxy_arc1).clone();
    let mcp_server_handle = match start_proxy_server(&mut proxy_clone, &args).await {
        Ok(handle) => handle,
        Err(error) => {
            startup_diagnostics::error_fatal("mcp_server_start", "mcp_server_start_failed", &error);
            return Err(error);
        }
    };

    // Start API server
    let (api_task, api_cancellation_token) = match start_api_server(proxy_arc2.clone(), &args).await {
        Ok(api) => api,
        Err(error) => {
            startup_diagnostics::error_fatal("api_server_start", "api_server_start_failed", &error);
            return Err(error);
        }
    };

    tracing::info!("Servers started. Press Ctrl+C to stop.");

    // Wait for shutdown signal: Ctrl+C or SIGTERM
    #[cfg(unix)]
    {
        let mut term_signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C, shutting down...");
            }
            _ = term_signal.recv() => {
                tracing::info!("Received SIGTERM, shutting down...");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        tracing::info!("Received Ctrl+C, shutting down...");
    }

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

fn explicit_port_args() -> ExplicitPortArgs {
    explicit_port_args_from(std::env::args_os().skip(1))
}

fn explicit_port_args_from<I>(args: I) -> ExplicitPortArgs
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    let mut explicit = ExplicitPortArgs::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let Some(raw) = arg.to_str() else {
            continue;
        };
        match raw {
            "--api-port" => {
                explicit.api_port = true;
                let _ = args.next();
            }
            "--mcp-port" | "-m" => {
                explicit.mcp_port = true;
                let _ = args.next();
            }
            _ if raw.starts_with("--api-port=") => explicit.api_port = true,
            _ if raw.starts_with("--mcp-port=") => explicit.mcp_port = true,
            _ if raw.starts_with("-m") && raw.len() > 2 => explicit.mcp_port = true,
            _ => {}
        }
    }

    explicit
}

fn record_port_recovery(
    previous_api_port: u16,
    previous_mcp_port: u16,
    recovered: LocalhostPortSelection,
) {
    let detail = format!(
        "api_port {} -> {}, mcp_port {} -> {}",
        previous_api_port, recovered.api_port, previous_mcp_port, recovered.mcp_port
    );
    startup_diagnostics::warn_degraded_reason(
        startup_diagnostics::component::MAIN,
        "localhost_port_recovery",
        "occupied_localhost_port",
        "advanced_localhost_ports",
        "runtime_ports",
        &detail,
        "Recovered occupied localhost startup ports",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn explicit_args(args: &[&str]) -> ExplicitPortArgs {
        explicit_port_args_from(args.iter().map(std::ffi::OsString::from))
    }

    #[test]
    fn explicit_port_args_detects_mcp_short_value_forms() {
        assert!(explicit_args(&["-m", "8001"]).mcp_port);
        assert!(explicit_args(&["-m8001"]).mcp_port);
    }

    #[test]
    fn explicit_port_args_detects_long_value_forms() {
        let inline = explicit_args(&["--api-port=8081", "--mcp-port=8001"]);
        assert!(inline.api_port);
        assert!(inline.mcp_port);

        let separated = explicit_args(&["--api-port", "8081", "--mcp-port", "8001"]);
        assert!(separated.api_port);
        assert!(separated.mcp_port);
    }
}
