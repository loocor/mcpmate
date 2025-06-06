// Client handlers module
// Provides HTTP API handlers for client management functionality

pub mod config;
pub mod database;
pub mod handlers;
pub mod import;
pub mod models;

// Re-export the main handler functions for use in routes
pub use handlers::{get_clients, get_config, manage_config};
