//! Foundation - infrastructure layer
//!
//! provides the basic functionality that all other modules depend on, including:
//! - error handling
//! - core type definitions
//! - tool functions
//! - process monitoring
//! - pagination handling

pub mod error;
pub mod loader;
pub mod monitor;
pub mod pagination;
pub mod types;
pub mod utils;

// re-export commonly used types and functions
pub use error::{RecoreError, RecoreResult};
pub use loader::*;
pub use types::*;
pub use utils::*;
