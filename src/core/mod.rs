// Core module for MCPMan
// Contains shared interfaces and types used by different transport modes

// Module declarations
pub mod config;
pub mod error;
pub mod tool;
pub mod types;
pub mod upstream;

// Re-exports
pub use config::*;
pub use error::*;
pub use tool::*;
pub use types::*;
pub use upstream::*;
