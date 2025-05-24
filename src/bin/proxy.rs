use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use mcpmate::{
    api::{ApiServer, handlers::system::initialize_server_start_time},
    conf::{database::Database, operations::server},
    core::{ConnectionStatus, TransportType, events, loader::load_server_config},
    http::HttpProxyServer,
};
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

    // Migrate runtime configurations to ensure consistent path formats
    if let Err(e) = mcpmate::runtime::migration::migrate_runtime_configs(&db.pool).await {
        tracing::warn!("Failed to migrate runtime configurations: {}", e);
        tracing::warn!("This may cause issues with runtime environment management");
    } else {
        tracing::info!("Runtime configurations migrated successfully");
    }

    // Initialize runtime environment on first startup
    if let Err(e) = initialize_runtime_environment(&db).await {
        tracing::warn!("Runtime environment initialization failed: {}", e);
        tracing::info!(
            "MCPMate will continue startup - runtime environments can be installed later"
        );
    }

    // Detect and install missing runtimes (prioritize MCPMate-managed runtimes)
    if let Err(e) = detect_and_install_missing_runtimes(&db).await {
        tracing::warn!("Runtime detection and installation failed: {}", e);
        tracing::info!("MCPMate will continue startup - runtimes can be installed later");
    }

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
    mcpmate::http::proxy::set_proxy_server(proxy_arc.clone());

    // Set the global instance for the event system
    let proxy_mutex = Arc::new(tokio::sync::Mutex::new(proxy.clone()));
    HttpProxyServer::set_global(proxy_mutex);

    // Initialize the event system
    events::init();

    // Get a reference to the connection pool
    let connection_pool = Arc::clone(&proxy.connection_pool);
    let proxy_arc_clone = Arc::clone(&proxy_arc);

    // Connect to all servers in the background
    tokio::spawn(async move {
        // Wait for a short time to ensure the SSE server is started
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connect to all servers
        let mut pool = connection_pool.lock().await;

        // Connect to all servers in parallel
        if let Err(e) = pool.connect_all().await {
            tracing::error!("Error in parallel connection process: {}", e);
        }

        // Get the total number of servers in the database
        let total_server_count_in_db = if let Some(db) = &proxy_arc_clone.database {
            match server::get_all_servers(&db.pool).await {
                Ok(servers) => servers.len(),
                Err(e) => {
                    tracing::error!("Failed to get servers from database: {}", e);
                    0 // If failed, use 0 and we'll fall back to connections.len() below
                }
            }
        } else {
            0 // If database not available, use 0
        };

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

        // Use database count if available, otherwise fall back to connections length
        let total_count = if total_server_count_in_db > 0 {
            total_server_count_in_db
        } else {
            pool.connections.len()
        };

        // Display system-wide server count, showing ratio of connected vs all configured servers
        // This is consistent with the /api/system/status endpoint which shows total_servers as all servers in the system
        tracing::info!(
            "Connected to {}/{} upstream servers",
            connected_count,
            total_count
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

/// Initialize runtime environment on first startup
/// This function checks for common runtime requirements and attempts to install them
async fn initialize_runtime_environment(_db: &mcpmate::conf::database::Database) -> Result<()> {
    use mcpmate::runtime::{RuntimeManager, RuntimePaths, RuntimeType};

    tracing::info!("Initializing runtime environment...");

    // Create necessary directories
    let runtime_paths = RuntimePaths::new().context("Failed to create runtime paths manager")?;

    // Create directories for Node.js and uv
    if let Err(e) = runtime_paths.create_directories(RuntimeType::Node, None) {
        tracing::warn!("Failed to create Node.js runtime directories: {}", e);
        tracing::warn!("This may cause issues with Node.js runtime environment");
    }

    if let Err(e) = runtime_paths.create_directories(RuntimeType::Uv, None) {
        tracing::warn!("Failed to create uv runtime directories: {}", e);
        tracing::warn!("This may cause issues with Python runtime environment");
    }

    tracing::info!("Runtime directories created successfully");

    // Create runtime manager
    let runtime_manager = RuntimeManager::new().context("Failed to create runtime manager")?;

    // Critical runtimes list
    let critical_runtimes = [
        (RuntimeType::Node, "Node.js (for npx commands)"),
        (RuntimeType::Uv, "uv (for Python package management)"),
    ];

    // Check each runtime and output status
    for (runtime_type, description) in critical_runtimes.iter() {
        match runtime_manager.is_runtime_available(*runtime_type, None) {
            Ok(true) => {
                let path = runtime_manager.get_runtime_path(*runtime_type, None)?;
                tracing::info!(
                    "✓ Runtime {} is available at {}",
                    description,
                    path.display()
                );
            }
            Ok(false) => {
                tracing::info!(
                    "✗ Runtime {} is not available - will be installed when needed",
                    description
                );
            }
            Err(e) => {
                tracing::warn!("! Failed to check runtime {}: {}", description, e);
            }
        }
    }

    tracing::info!("Runtime environment check completed");
    tracing::info!("Missing runtimes will be automatically installed when first needed");

    Ok(())
}

/// Detect and install missing runtimes with proper priority strategy
/// Priority: MCPMate-managed runtimes > System runtimes
async fn detect_and_install_missing_runtimes(db: &mcpmate::conf::database::Database) -> Result<()> {
    use mcpmate::runtime::constants::get_mcpmate_dir;
    use mcpmate::runtime::{RuntimeManager, RuntimeType};

    tracing::info!("Detecting and installing missing runtimes with priority strategy...");

    let runtime_manager = RuntimeManager::new().context("Failed to create runtime manager")?;
    let mcpmate_dir = get_mcpmate_dir().context("Failed to get MCPMate directory")?;

    // Runtime types to check
    let runtime_types = [RuntimeType::Node, RuntimeType::Uv, RuntimeType::Bun];
    let mut installed_count = 0;

    for runtime_type in runtime_types {
        tracing::debug!("Processing runtime: {:?}", runtime_type);

        // Strategy 1: Check for MCPMate-managed runtime (highest priority)
        let managed_available = runtime_manager
            .is_runtime_available(runtime_type, None)
            .unwrap_or(false);

        if managed_available {
            // MCPMate-managed runtime exists, register it
            if let Ok(managed_path) = runtime_manager.get_runtime_path(runtime_type, None) {
                if let Err(e) = register_runtime_in_database(
                    &db.pool,
                    runtime_type,
                    &managed_path,
                    "latest",
                    &mcpmate_dir,
                )
                .await
                {
                    tracing::warn!(
                        "Failed to register managed {:?} runtime: {}",
                        runtime_type,
                        e
                    );
                } else {
                    tracing::info!(
                        "✓ Using managed {:?} runtime: {}",
                        runtime_type,
                        managed_path.display()
                    );
                    installed_count += 1;
                }
                continue;
            }
        }

        // Strategy 2: MCPMate-managed runtime not available, try to install it
        tracing::info!(
            "MCPMate-managed {:?} runtime not found, attempting to install...",
            runtime_type
        );

        // Convert RuntimeType to command string
        let command_str = match runtime_type {
            RuntimeType::Node => "npx",
            RuntimeType::Uv => "uvx",
            RuntimeType::Bun => "bunx",
        };

        match runtime_manager
            .ensure_runtime_for_command(command_str, Some(&db.pool))
            .await
        {
            Ok(_) => {
                // Installation successful, register the managed runtime
                if let Ok(managed_path) = runtime_manager.get_runtime_path(runtime_type, None) {
                    if let Err(e) = register_runtime_in_database(
                        &db.pool,
                        runtime_type,
                        &managed_path,
                        "latest",
                        &mcpmate_dir,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to register newly installed {:?} runtime: {}",
                            runtime_type,
                            e
                        );
                    } else {
                        tracing::info!(
                            "✓ Successfully installed and registered managed {:?} runtime: {}",
                            runtime_type,
                            managed_path.display()
                        );
                        installed_count += 1;
                    }
                    continue;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to install managed {:?} runtime: {}",
                    runtime_type,
                    e
                );
            }
        }

        // Strategy 3: Fallback to system runtime (lowest priority)
        tracing::info!("Falling back to system runtime for {:?}...", runtime_type);
        if let Some(system_path) = check_system_runtime(runtime_type).await {
            if let Err(e) = register_runtime_in_database(
                &db.pool,
                runtime_type,
                &system_path,
                "system",
                &mcpmate_dir,
            )
            .await
            {
                tracing::warn!(
                    "Failed to register system {:?} runtime: {}",
                    runtime_type,
                    e
                );
            } else {
                tracing::info!(
                    "⚠ Using system {:?} runtime as fallback: {}",
                    runtime_type,
                    system_path.display()
                );
                installed_count += 1;
            }
        } else {
            tracing::warn!(
                "No {:?} runtime available (neither managed nor system)",
                runtime_type
            );
        }
    }

    tracing::info!(
        "Runtime detection and installation completed: {} runtimes registered",
        installed_count
    );
    Ok(())
}

/// Check for system runtime
async fn check_system_runtime(
    runtime_type: mcpmate::runtime::RuntimeType
) -> Option<std::path::PathBuf> {
    // Try to find the system runtime executable
    let executable_path = match runtime_type {
        mcpmate::runtime::RuntimeType::Node => {
            // Try to find npx first, then node
            if let Ok(npx_path) = which::which("npx") {
                npx_path
            } else if let Ok(node_path) = which::which("node") {
                node_path
            } else {
                tracing::debug!("Neither npx nor node found in system PATH");
                return None;
            }
        }
        mcpmate::runtime::RuntimeType::Uv => {
            // Try to find uvx first, then uv
            if let Ok(uvx_path) = which::which("uvx") {
                uvx_path
            } else if let Ok(uv_path) = which::which("uv") {
                uv_path
            } else {
                tracing::debug!("Neither uvx nor uv found in system PATH");
                return None;
            }
        }
        mcpmate::runtime::RuntimeType::Bun => {
            if let Ok(bun_path) = which::which("bun") {
                bun_path
            } else {
                tracing::debug!("bun not found in system PATH");
                return None;
            }
        }
    };

    tracing::debug!(
        "Found system runtime for {:?}: {}",
        runtime_type,
        executable_path.display()
    );
    Some(executable_path)
}

/// Register runtime in database
async fn register_runtime_in_database(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    runtime_type: mcpmate::runtime::RuntimeType,
    runtime_path: &std::path::Path,
    version: &str,
    mcpmate_dir: &std::path::Path,
) -> Result<()> {
    use mcpmate::runtime::config::{RuntimeConfig, save_config};

    // Determine the path to store in database
    let relative_path = if runtime_path.starts_with(mcpmate_dir) {
        // Managed runtime - store relative path (relative to mcpmate_dir)
        runtime_path
            .strip_prefix(mcpmate_dir)
            .unwrap_or(runtime_path)
            .to_string_lossy()
            .to_string()
    } else {
        // System runtime - store absolute path as-is
        // The runtime_env module will handle this correctly
        runtime_path.to_string_lossy().to_string()
    };

    // Create runtime config
    let config = RuntimeConfig::new(runtime_type, version, &relative_path);

    // Save to database (this will upsert - insert or update)
    save_config(pool, &config).await?;

    tracing::debug!(
        "Registered {:?} runtime in database: {} -> {}",
        runtime_type,
        runtime_path.display(),
        relative_path
    );

    Ok(())
}
