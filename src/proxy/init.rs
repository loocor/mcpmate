use anyhow::Result;
use mcpmate::{
    api::handlers::system::initialize_server_start_time,
    conf::database::Database,
    core::{events, loader::load_server_config},
    http::HttpProxyServer,
};
use std::sync::Arc;
use tracing_subscriber::{self, EnvFilter};

use super::Args;

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

/// Setup environment variables from .env file
pub fn setup_environment() -> Result<()> {
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
    Ok(())
}

/// Setup database connection and perform necessary migrations
pub async fn setup_database() -> Result<Database> {
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

    Ok(db)
}

/// Setup proxy server with database and configuration
pub async fn setup_proxy_server(db: Database) -> Result<(HttpProxyServer, Arc<HttpProxyServer>)> {
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

    Ok((proxy, proxy_arc))
}
