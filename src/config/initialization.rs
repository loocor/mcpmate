// Database initialization coordinator for MCPMate
// Coordinates initialization of all database modules

use crate::config::client::init::initialize_client_apps;
use crate::config::server::init::initialize_server_tables;
use crate::config::suit::init::initialize_suit_tables;
use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

/// Run database initialization
pub async fn run_initialization(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::info!("Running database initialization");

    // Initialize server-related tables
    tracing::debug!("Initializing server-related tables");
    initialize_server_tables(pool).await?;

    // Initialize config suit-related tables
    tracing::debug!("Initializing config suit-related tables");
    initialize_suit_tables(pool).await?;

    // Initialize client applications tables and data
    tracing::debug!("Initializing client applications tables and data");
    initialize_client_apps(pool).await?;

    tracing::info!("Database initialization completed successfully");
    Ok(())
}
