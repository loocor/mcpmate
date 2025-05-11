use anyhow::Result;
use clap::Parser;
use mcpmate::{
    api::{handlers::system::initialize_server_start_time, ApiServer},
    conf::Database,
    core::loader::load_server_config,
    core::{ConnectionStatus, TransportType},
    http::HttpProxyServer,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};

use tracing_subscriber::{self, EnvFilter};

/// Command line arguments for the MCP proxy server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on for MCP server
    #[arg(short, long, default_value = "8000")]
    port: u16,

    /// Port to listen on for API server
    #[arg(long, default_value = "8080")]
    api_port: u16,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Transport type (sse, str, or uni)
    #[arg(long, alias = "trans", default_value = "uni")]
    transport: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                args.log_level
                    .parse()
                    .unwrap_or(tracing::Level::INFO.into()),
            ),
        )
        .init();

    // Load environment variables from .env file
    if let Ok(path) = std::env::current_dir().map(|p| p.join(".env")) {
        if path.exists() {
            match dotenvy::from_path(&path) {
                Ok(_) => {
                    tracing::info!("Loaded environment from {}", path.display());
                }
                Err(e) => {
                    tracing::error!("Error loading .env file: {}", e);
                }
            }
        } else {
            tracing::warn!("No .env file found at {}", path.display());
        }
    }

    // Initialize server start time
    initialize_server_start_time();

    // Initialize database
    let db = match Database::new().await {
        Ok(db) => {
            tracing::info!("Database initialized successfully");
            db
        }
        Err(e) => {
            tracing::error!("Failed to initialize database: {}", e);
            return Err(anyhow::anyhow!("Failed to initialize database: {}", e));
        }
    };

    // Note: Migration from files to database is automatically performed if the database is empty and config/mcp.json exists

    // Load configuration from database
    let config = load_server_config(&db).await?;

    tracing::info!("Loaded configuration from database");
    tracing::info!(
        "Found {} enabled MCP servers in database configuration",
        config.mcp_servers.len()
    );

    // Create HTTP proxy server
    let mut proxy = HttpProxyServer::new(Arc::new(config));

    // Use the existing database connection
    proxy.set_database(db).await?;
    tracing::info!("Using database connection for tool-level configuration.");

    // Create an Arc for the proxy server and set the global instance
    let proxy_arc = Arc::new(proxy.clone());
    mcpmate::http::proxy::set_proxy_server(proxy_arc);

    // Get a reference to the connection pool
    let connection_pool = Arc::clone(&proxy.connection_pool);

    // Connect to all servers in the background
    tokio::spawn(async move {
        // Wait for a short time to ensure the SSE server is started
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect to all servers
        let mut pool = connection_pool.lock().await;

        // Connect to all servers in parallel
        if let Err(e) = pool.connect_all().await {
            tracing::error!("Error in parallel connection process: {}", e);
        }

        // Record the connection status
        let connected_count = pool
            .connections
            .values()
            .filter(|instances| {
                instances
                    .values()
                    .any(|conn| matches!(conn.status, ConnectionStatus::Ready))
            })
            .count();

        tracing::info!(
            "Connected to {}/{} upstream servers",
            connected_count,
            pool.connections.len()
        );
    });

    // Start proxy server with specified transport
    let mcp_bind_address = format!("127.0.0.1:{}", args.port).parse()?;

    // Check if using unified mode
    if args.transport == "unified" || args.transport == "uni" {
        tracing::info!(
            "Starting MCP proxy server on {} with unified transport (both SSE and Streamable HTTP)",
            mcp_bind_address
        );

        // Start the unified server
        if let Err(e) = proxy.start_unified(mcp_bind_address).await {
            tracing::error!("Failed to start unified proxy server: {}", e);
            return Err(e);
        }
    } else {
        // Parse transport type for non-unified mode
        let transport_type = match args.transport.as_str() {
            "sse" => TransportType::Sse,
            "streamable_http" | "streamablehttp" | "str" => TransportType::StreamableHttp,
            _ => {
                tracing::warn!(
                    "Unknown transport type: {}, defaulting to SSE",
                    args.transport
                );
                TransportType::Sse
            }
        };

        tracing::info!(
            "Starting MCP proxy server on {} with transport type {:?}",
            mcp_bind_address,
            transport_type
        );

        // Start the server with specific transport
        let path = match transport_type {
            TransportType::Sse => "/sse",
            TransportType::StreamableHttp => "/mcp", // Path for Streamable HTTP
            _ => "/sse",                             // Default
        };

        tracing::info!(
            "Using path '{}' for transport type {:?}",
            path,
            transport_type
        );

        if let Err(e) = proxy.start(mcp_bind_address, path, transport_type).await {
            tracing::error!("Failed to start proxy server: {}", e);
            return Err(e);
        }
    }

    // Start API server
    let api_bind_address: SocketAddr = format!("127.0.0.1:{}", args.api_port).parse()?;
    tracing::info!("Starting API server on {}", api_bind_address);

    let api_server = ApiServer::new(api_bind_address);
    let connection_pool_clone = Arc::clone(&proxy.connection_pool);
    let proxy_clone = Arc::new(proxy.clone());

    // Start API server in a separate task
    let api_task = tokio::spawn(async move {
        if let Err(e) = api_server
            .start(connection_pool_clone, Some(proxy_clone))
            .await
        {
            tracing::error!("API server error: {}", e);
        }
    });

    tracing::info!("API server started with HTTP proxy server reference");

    tracing::info!("Servers started. Press Ctrl+C to stop.");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down...");

    // Cancel API server task
    api_task.abort();

    // Disconnect from all servers
    {
        let mut pool = proxy.connection_pool.lock().await;
        pool.disconnect_all().await?;
        tracing::info!("Disconnected from all upstream servers");
    }

    Ok(())
}
