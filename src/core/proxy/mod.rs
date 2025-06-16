//! Proxy module for core
//!
//! Contains the main proxy server implementation and startup logic using core modules.
//! This module provides a complete proxy server implementation that is independent of core modules.

pub mod args;
pub mod init;
pub mod server;
pub mod startup;

// Re-export main components
pub use args::Args;
pub use server::ProxyServer;
