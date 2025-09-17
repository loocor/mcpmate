//! Initialization logic for core proxy server
//!
//! This module handles the setup and initialization of the proxy server using core modules.

use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{self, EnvFilter};

use super::{Args, ProxyServer, args::StartupMode};

// Import required types and modules from core and other modules
use crate::{
    api::handlers::system,
    config::database::Database,
    core::{capability::naming, foundation::loader},
};

/// Setup logging based on command line arguments
/// This function is safe to call multiple times - it will only initialize once
pub fn setup_logging(args: &Args) -> Result<()> {
    // Create environment filter with smart defaults
    let (env_filter, log_config_msg) = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        // If RUST_LOG is set, respect it completely - no overrides
        let msg = format!("Using RUST_LOG environment variable: {} (full control)", rust_log);
        (EnvFilter::from_default_env(), msg)
    } else {
        // If RUST_LOG is not set, use application defaults with noise reduction
        let default_level = args.log_level.parse().unwrap_or(tracing::Level::INFO.into());
        let msg = format!(
            "No RUST_LOG set, using application default: {} (with noise reduction)",
            args.log_level
        );

        // Create filter with noise reduction for common noisy modules
        let filter = EnvFilter::from_default_env()
            .add_directive(default_level)
            // Keep the really noisy third-party modules quiet
            .add_directive("sqlx=warn".parse().unwrap()) // SQL queries are too verbose
            .add_directive("rmcp=warn".parse().unwrap()) // MCP protocol noise
            .add_directive("hyper=warn".parse().unwrap()) // HTTP client noise
            .add_directive("reqwest=warn".parse().unwrap()) // HTTP requests
            .add_directive("tokio=warn".parse().unwrap()); // Async runtime noise

        (filter, msg)
    };

    let result = tracing_subscriber::fmt().with_env_filter(env_filter).try_init();

    match result {
        Ok(()) => {
            tracing::info!("Logging system initialized successfully");
            tracing::info!("{}", log_config_msg);
        }
        Err(_) => {
            // Global subscriber already set, this is fine for FFI mode
            tracing::debug!("Logging system already initialized, skipping");
        }
    }

    Ok(())
}

/// Setup database connection and perform necessary migrations
pub async fn setup_database() -> Result<Database> {
    // Initialize server start time
    system::initialize_server_start_time();

    // Debug database path before initialization
    use crate::common::paths::global_paths;
    let db_path = global_paths().database_path();
    tracing::info!("FFI Database setup - Expected path: {}", db_path.display());
    tracing::info!("FFI Database setup - File exists: {}", db_path.exists());

    // Initialize database
    let db = match Database::new().await {
        Ok(db) => {
            tracing::info!("FFI Database initialized successfully");
            db
        }
        Err(e) => {
            tracing::error!("FFI Failed to initialize database: {}", e);
            return Err(anyhow::anyhow!("Failed to initialize database: {}", e));
        }
    };

    // Initialize naming store once the database is ready
    naming::initialize(db.pool.clone());

    // Runtime migration removed - simplified runtime management
    tracing::debug!("Runtime management simplified - no migration needed");

    Ok(db)
}

/// Setup proxy server with startup parameters
pub async fn setup_proxy_server_with_params(
    db: Database,
    startup_mode: &StartupMode,
) -> Result<(Arc<ProxyServer>, Arc<ProxyServer>)> {
    // Load configuration from database using core loader with startup parameters
    let config = loader::load_server_config_with_params(&db, startup_mode).await?;

    tracing::info!(
        "Loaded configuration from database with startup mode: {:?}",
        startup_mode
    );
    tracing::info!(
        "Found {} enabled MCP servers in database configuration",
        config.mcp_servers.len()
    );

    // Create proxy server using core implementation
    let mut proxy = ProxyServer::new(Arc::new(config));

    // Use the existing database connection
    proxy.set_database(db).await?;
    tracing::info!("Using database connection for tool-level configuration.");

    // Create Arc wrappers for the proxy server
    let proxy_arc = Arc::new(proxy.clone());
    ProxyServer::set_global(Arc::new(tokio::sync::Mutex::new(proxy)));

    // Event system will be initialized in proxy.set_database() with proper handlers
    tracing::info!("Proxy server created, event system will be initialized with handlers");

    Ok((proxy_arc.clone(), proxy_arc))
}

/// Setup proxy server with database and configuration using core modules (legacy function for backward compatibility)
pub async fn setup_proxy_server(db: Database) -> Result<(Arc<ProxyServer>, Arc<ProxyServer>)> {
    setup_proxy_server_with_params(db, &StartupMode::Default).await
}
