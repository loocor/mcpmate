//! # Cherry DB Manager
//!
//! A library for managing Cherry Studio LevelDB configurations.
//!
//! This crate provides a simple interface to read and write MCP (Model Context Protocol)
//! server configurations stored in Cherry Studio's LevelDB format.
//!
//! ## Example
//!
//! ```rust,no_run
//! use cherry_db_manager::{CherryDbManager, DefaultCherryDbManager};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = DefaultCherryDbManager::new();
//! let config = manager.read_mcp_config("./leveldb")?;
//! println!("Found {} servers", config.servers.len());
//! # Ok(())
//! # }
//! ```

mod error;
mod manager;
mod types;
mod utils;

pub use error::{CherryDbError, Result};
pub use manager::{CherryDbManager, DefaultCherryDbManager};
pub use types::{
    McpConfigRequest, McpConfigResponse, ServerListResponse, ServerRequest, ServerResponse,
};
