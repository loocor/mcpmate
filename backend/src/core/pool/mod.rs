//! Pool - connection pool management layer
//!
//! Provides connection pool management for upstream MCP servers, including:
//! - connection lifecycle management
//! - health monitoring and reconnection
//! - parallel connection capabilities
//! - resource monitoring and limits

mod connection;
mod executor;
mod health;
mod monitoring;

// Business logic managers (separated from core pool logic)
mod config;
mod database;
mod helpers;
mod sync;
mod types;

pub use connection::UpstreamConnectionPool;

// Re-export selected types for external coordination
pub use database::CapSyncFlags;
pub use types::{FailureKind, UpstreamConnection};
