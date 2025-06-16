//! Initialization logic for core proxy server
//!
//! This module handles the setup and initialization of the proxy server using core modules.

use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{self, EnvFilter};

use super::{Args, ProxyServer};

// Import required types and modules from core and other modules
use crate::{
    api::handlers::system,
    config::database::Database,
    core::{events, foundation::loader},
    runtime::migration,
};

/// Setup logging based on command line arguments
pub fn setup_logging(args: &Args) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                args.log_level
                    .parse()
                    .unwrap_or(tracing::Level::INFO.into()),
            ),
        )
        .init();
    Ok(())
}

/// Setup database connection and perform necessary migrations
pub async fn setup_database() -> Result<Database> {
    // Initialize server start time
    system::initialize_server_start_time();

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
    if let Err(e) = migration::migrate_runtime_configs(&db.pool).await {
        tracing::warn!("Failed to migrate runtime configurations: {}", e);
        tracing::warn!("This may cause issues with runtime environment management");
    } else {
        tracing::info!("Runtime configurations migrated successfully");
    }

    Ok(db)
}

/// Setup proxy server with database and configuration using core modules
pub async fn setup_proxy_server(db: Database) -> Result<(ProxyServer, Arc<ProxyServer>)> {
    // Load configuration from database using core loader
    let config = loader::load_server_config(&db).await?;

    tracing::info!("Loaded configuration from database");
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
