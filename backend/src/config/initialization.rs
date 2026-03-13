// Database initialization coordinator for MCPMate
// Coordinates initialization of all database modules

use crate::config::client::init::initialize_client_table;
use crate::config::profile::init::initialize_profile_tables;
use crate::config::server::init::initialize_server_tables;
use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

/// Run database initialization
pub async fn run_initialization(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::info!("Running database initialization");

    // Initialize server-related tables
    tracing::debug!("Initializing server-related tables");
    initialize_server_tables(pool).await?;

    // Initialize client management table
    tracing::debug!("Initializing client management table");
    initialize_client_table(pool).await?;

    // Initialize profile-related tables
    tracing::debug!("Initializing profile-related tables");
    initialize_profile_tables(pool).await?;

    tracing::info!("Database initialization completed successfully");
    Ok(())
}
