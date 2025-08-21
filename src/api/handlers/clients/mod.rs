// Client handlers module
// Provides HTTP API handlers for client management functionality

pub mod config;
pub mod database;
pub mod handlers;
pub mod import;

// Re-export the main handler functions for use in routes
pub use handlers::{check, details, update};
