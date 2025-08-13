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
    core::{events, foundation::loader},
    // runtime::migration removed - simplified runtime management
};

/// Setup logging based on command line arguments
/// This function is safe to call multiple times - it will only initialize once
pub fn setup_logging(args: &Args) -> Result<()> {
    // Use try_init() to avoid panic on repeated calls
    let result = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(
                    args.log_level
                        .parse()
                        .unwrap_or(tracing::Level::WARN.into()), // Changed default from INFO to WARN
                )
                // Reduce noise from specific modules
                .add_directive("rmcp=warn".parse().unwrap())
                .add_directive("mcpmate::core::cache=warn".parse().unwrap())
                .add_directive("mcpmate::core::transport=warn".parse().unwrap())
                .add_directive("mcpmate::system::metrics=error".parse().unwrap())
                .add_directive("sqlx=error".parse().unwrap())
        )
        .try_init();

    match result {
        Ok(()) => {
            tracing::info!("Logging system initialized successfully");
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

    // Runtime migration removed - simplified runtime management
    tracing::debug!("Runtime management simplified - no migration needed");

    Ok(db)
}

/// Setup proxy server with startup parameters
pub async fn setup_proxy_server_with_params(
    db: Database,
    startup_mode: &StartupMode,
) -> Result<(ProxyServer, Arc<ProxyServer>)> {
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

    // Create an Arc for the proxy server and set the global instance
    let proxy_arc = Arc::new(proxy.clone());
    ProxyServer::set_global(Arc::new(tokio::sync::Mutex::new(proxy.clone())));

    // Initialize the event system using core
    let _ = events::init();
    tracing::info!("Event system initialized using core");

    Ok((proxy, proxy_arc))
}

/// Setup proxy server with database and configuration using core modules (legacy function for backward compatibility)
pub async fn setup_proxy_server(db: Database) -> Result<(ProxyServer, Arc<ProxyServer>)> {
    setup_proxy_server_with_params(db, &StartupMode::Default).await
}
